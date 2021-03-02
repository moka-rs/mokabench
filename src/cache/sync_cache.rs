use crate::{config::Config, parser::ArcTraceEntry, report::Report};

use moka::sync::{Cache, CacheBuilder};
use std::{collections::hash_map::RandomState, sync::Arc};

use super::CacheSet;

pub struct SyncCache {
    _config: Config,
    cache: Cache<usize, Arc<Box<[u8]>>, RandomState>,
}

impl Clone for SyncCache {
    fn clone(&self) -> Self {
        Self {
            _config: self._config.clone(),
            cache: self.cache.clone(),
        }
    }
}

impl SyncCache {
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

    fn get(&self, key: &usize) -> bool {
        self.cache.get(key).is_some()
    }

    fn insert(&self, key: usize) {
        let value = vec![0; 512].into_boxed_slice();
        // std::thread::sleep(std::time::Duration::from_micros(500));
        self.cache.insert(key, Arc::new(value));
    }
}

impl CacheSet<ArcTraceEntry> for SyncCache {
    fn get_or_insert(&mut self, entry: &ArcTraceEntry, report: &mut Report) {
        let mut read_count = 0;
        let mut hit_count = 0;
        let mut insert_count = 0;

        for block in entry.0.clone() {
            if self.get(&block) {
                hit_count += 1;
            } else {
                self.insert(block);
                insert_count += 1;
            }
            read_count += 1;
        }

        report.read_count += read_count;
        report.hit_count += hit_count;
        report.insert_count += insert_count;
    }

    fn invalidate(&mut self, entry: &ArcTraceEntry) {
        for block in entry.0.clone() {
            self.cache.invalidate(&block);
        }
    }

    fn invalidate_all(&mut self) {
        self.cache.invalidate_all();
    }
}

pub struct SharedSyncCache(SyncCache);

impl SharedSyncCache {
    pub fn new(config: &Config, capacity: usize) -> Self {
        Self(SyncCache::new(config, capacity))
    }
}

impl Clone for SharedSyncCache {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl CacheSet<ArcTraceEntry> for SharedSyncCache {
    fn get_or_insert(&mut self, entry: &ArcTraceEntry, report: &mut Report) {
        self.0.get_or_insert(entry, report);
    }

    fn invalidate(&mut self, entry: &ArcTraceEntry) {
        self.0.invalidate(entry);
    }

    fn invalidate_all(&mut self) {
        self.0.invalidate_all();
    }
}
