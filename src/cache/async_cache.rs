use super::AsyncCacheSet;
use crate::{parser::ArcTraceEntry, report::Report, TTI_SECS, TTL_SECS};

use async_trait::async_trait;
use moka::future::{Cache, CacheBuilder};
use std::{collections::hash_map::RandomState, sync::Arc, time::Duration};

pub struct AsyncCache(Cache<usize, Arc<Box<[u8]>>, RandomState>);

impl Clone for AsyncCache {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl AsyncCache {
    pub fn new(capacity: usize) -> Self {
        let cache = CacheBuilder::new(capacity)
            .initial_capacity(capacity)
            .time_to_live(Duration::from_secs(TTL_SECS))
            .time_to_idle(Duration::from_secs(TTI_SECS))
            .build();
        Self(cache)
    }

    fn get(&self, key: usize) -> bool {
        self.0.get(&key).is_some()
    }

    async fn insert(&self, key: usize) {
        let value = vec![0; 512].into_boxed_slice();
        // tokio::task::sleep(std::time::Duration::from_micros(500));
        self.0.insert(key, Arc::new(value)).await;
    }
}

#[async_trait]
impl AsyncCacheSet<ArcTraceEntry> for AsyncCache {
    async fn process(&mut self, entry: &ArcTraceEntry, report: &mut Report) {
        let mut read_count = 0;
        let mut hit_count = 0;
        let mut insert_count = 0;

        for block in entry.0.clone() {
            if self.get(block) {
                hit_count += 1;
            } else {
                self.insert(block).await;
                insert_count += 1;
            }
            read_count += 1;
        }

        report.read_count += read_count;
        report.hit_count += hit_count;
        report.insert_count += insert_count;
    }
}

pub struct SharedAsyncCache(AsyncCache);

impl SharedAsyncCache {
    pub fn new(capacity: usize) -> Self {
        Self(AsyncCache::new(capacity))
    }
}

impl Clone for SharedAsyncCache {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[async_trait]
impl AsyncCacheSet<ArcTraceEntry> for SharedAsyncCache {
    async fn process(&mut self, entry: &ArcTraceEntry, report: &mut Report) {
        self.0.process(entry, report).await
    }
}
