use crate::{parser::ArcTraceEntry, report::Report, TTI_SECS, TTL_SECS};

use moka::unsync::{Cache, CacheBuilder};
use std::{collections::hash_map::RandomState, sync::Arc, time::Duration};

use super::CacheSet;

pub struct UnsyncCache(Cache<usize, Arc<Box<[u8]>>, RandomState>);

impl UnsyncCache {
    pub fn new(capacity: usize) -> Self {
        let cache = CacheBuilder::new(capacity)
            .initial_capacity(capacity)
            .time_to_live(Duration::from_secs(TTL_SECS))
            .time_to_idle(Duration::from_secs(TTI_SECS))
            .build();
        Self(cache)
    }

    fn get(&mut self, key: &usize) -> bool {
        self.0.get(key).is_some()
    }

    fn insert(&mut self, key: usize) {
        let value = vec![0; 512].into_boxed_slice();
        // std::thread::sleep(std::time::Duration::from_micros(500));
        self.0.insert(key, Arc::new(value));
    }
}

impl CacheSet<ArcTraceEntry> for UnsyncCache {
    fn process(&mut self, entry: &ArcTraceEntry, report: &mut Report) {
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
}
