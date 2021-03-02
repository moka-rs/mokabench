use super::AsyncCacheSet;
use crate::{config::Config, parser::ArcTraceEntry, report::Report};

use async_trait::async_trait;
use moka::future::{Cache, CacheBuilder};
use std::{collections::hash_map::RandomState, sync::Arc};

pub struct AsyncCache {
    _config: Config,
    cache: Cache<usize, Arc<Box<[u8]>>, RandomState>,
}

impl Clone for AsyncCache {
    fn clone(&self) -> Self {
        Self {
            _config: self._config.clone(),
            cache: self.cache.clone(),
        }
    }
}

impl AsyncCache {
    pub fn new(config: &Config, capacity: usize) -> Self {
        let mut builder = CacheBuilder::new(capacity).initial_capacity(capacity);
        if let Some(ttl) = config.ttl {
            builder = builder.time_to_live(ttl);
        }
        if let Some(tti) = config.tti {
            builder = builder.time_to_idle(tti)
        }
        Self {
            _config: config.clone(),
            cache: builder.build(),
        }
    }

    fn get(&self, key: usize) -> bool {
        self.cache.get(&key).is_some()
    }

    async fn insert(&self, key: usize) {
        let value = vec![0; 512].into_boxed_slice();
        // tokio::task::sleep(std::time::Duration::from_micros(500));
        self.cache.insert(key, Arc::new(value)).await;
    }
}

#[async_trait]
impl AsyncCacheSet<ArcTraceEntry> for AsyncCache {
    async fn get_or_insert(&mut self, entry: &ArcTraceEntry, report: &mut Report) {
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

    async fn invalidate(&mut self, entry: &ArcTraceEntry) {
        for block in entry.0.clone() {
            self.cache.invalidate(&block).await;
        }
    }

    fn invalidate_all(&mut self) {
        self.cache.invalidate_all();
    }
}

pub struct SharedAsyncCache(AsyncCache);

impl SharedAsyncCache {
    pub fn new(config: &Config, capacity: usize) -> Self {
        Self(AsyncCache::new(config, capacity))
    }
}

impl Clone for SharedAsyncCache {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[async_trait]
impl AsyncCacheSet<ArcTraceEntry> for SharedAsyncCache {
    async fn get_or_insert(&mut self, entry: &ArcTraceEntry, report: &mut Report) {
        self.0.get_or_insert(entry, report).await
    }

    async fn invalidate(&mut self, entry: &ArcTraceEntry) {
        self.0.invalidate(entry).await;
    }

    fn invalidate_all(&mut self) {
        self.0.invalidate_all();
    }
}
