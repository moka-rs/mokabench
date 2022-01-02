use crate::{config::Config, parser::ArcTraceEntry, report::Report};

use moka::unsync::{Cache, CacheBuilder};
use std::{collections::hash_map::RandomState, sync::Arc};

use super::{CacheSet, Counters};

pub struct UnsyncCache {
    _config: Config,
    cache: Cache<usize, Arc<[u8]>, RandomState>,
}

impl UnsyncCache {
    pub fn new(config: &Config, capacity: usize) -> Self {
        #[allow(clippy::useless_conversion)]
        let max_capacity = capacity.try_into().unwrap();
        let mut builder = CacheBuilder::new(max_capacity).initial_capacity(capacity);
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

    fn get(&mut self, key: &usize) -> bool {
        self.cache.get(key).is_some()
    }

    fn insert(&mut self, key: usize) {
        let value = super::make_value(key);
        // std::thread::sleep(std::time::Duration::from_micros(500));
        self.cache.insert(key, value);
    }

    fn invalidate_entries_if(&mut self, entry: &ArcTraceEntry) {
        for block in entry.0.clone() {
            self.cache
                .invalidate_entries_if(move |_k, v| v[0] == (block % 256) as u8)
        }
    }
}

impl CacheSet<ArcTraceEntry> for UnsyncCache {
    fn get_or_insert(&mut self, entry: &ArcTraceEntry, report: &mut Report) {
        let mut counters = Counters::default();

        for block in entry.0.clone() {
            if !self.get(&block) {
                self.insert(block);
                counters.inserted();
            }
            counters.read();
        }

        counters.add_to_report(report);
    }

    fn get_or_insert_once(&mut self, entry: &ArcTraceEntry, report: &mut Report) {
        self.get_or_insert(entry, report);
    }

    fn invalidate(&mut self, entry: &ArcTraceEntry) {
        for block in entry.0.clone() {
            self.cache.invalidate(&block);
        }
    }

    fn invalidate_all(&mut self) {
        self.cache.invalidate_all();
    }

    fn invalidate_entries_if(&mut self, entry: &ArcTraceEntry) {
        self.invalidate_entries_if(entry);
    }
}
