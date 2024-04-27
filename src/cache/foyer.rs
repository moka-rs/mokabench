use std::sync::Arc;

use crate::{config::Config, parser::TraceEntry};

use super::{CacheDriver, Counters, Key, Value};

use ::foyer::{Cache, CacheBuilder};

#[derive(Clone)]
pub struct FoyerCache {
    config: Arc<Config>,
    cache: Cache<Key, Value>,
}

impl FoyerCache {
    pub fn new(config: &Config, capacity: usize) -> Self {
        if let Some(_ttl) = config.ttl {
            todo!()
        }
        if let Some(_tti) = config.tti {
            todo!()
        }

        let mut builder = CacheBuilder::new(capacity).with_shards(64);

        if config.size_aware {
            builder = builder.with_weighter(|_key: &Key, value: &Value| value.0 as usize);
        }

        let cache = builder.build();
        let config = Arc::new(config.clone());

        Self { config, cache }
    }

    fn get(&self, key: &usize) -> bool {
        self.cache.get(key).is_some()
    }

    fn insert(&self, key: usize, req_id: usize) {
        let value = super::make_value(&self.config, key, req_id);
        super::sleep_thread_for_insertion(&self.config);
        self.cache.insert(key, value);
    }
}

impl CacheDriver<TraceEntry> for FoyerCache {
    fn get_or_insert(&mut self, entry: &TraceEntry, report: &mut crate::Report) {
        let mut counters = Counters::default();
        let mut req_id = entry.line_number();

        for block in entry.range() {
            if self.get(&block) {
                counters.read_hit();
            } else {
                self.insert(block, req_id);
                counters.inserted();
                counters.read_missed();
            }
            req_id += 1;
        }

        counters.add_to_report(report);
    }

    fn get_or_insert_once(&mut self, _entry: &TraceEntry, _report: &mut crate::Report) {
        todo!()
    }

    fn update(&mut self, entry: &TraceEntry, report: &mut crate::Report) {
        let mut counters = Counters::default();
        let mut req_id = entry.line_number();

        for block in entry.range() {
            self.insert(block, req_id);
            counters.inserted();
            req_id += 1;
        }

        counters.add_to_report(report);
    }
}
