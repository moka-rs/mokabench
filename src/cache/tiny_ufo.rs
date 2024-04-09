use std::sync::Arc;

use super::{CacheDriver, Counters, Key, Value};
use crate::{config::Config, parser::TraceEntry, report::Report};

#[derive(Clone)]
pub struct TinyUfoCache {
    config: Arc<Config>,
    cache: Arc<tinyufo::TinyUfo<Key, Value>>,
}

impl TinyUfoCache {
    pub fn new(config: &Config, capacity: usize) -> Self {
        if let Some(_ttl) = config.ttl {
            todo!()
        }
        if let Some(_tti) = config.tti {
            todo!()
        }
        if config.size_aware {
            todo!()
        }

        // TinyUFO does not support custom hasher. Use its default hasher.
        Self {
            config: Arc::new(config.clone()),
            cache: Arc::new(tinyufo::TinyUfo::new(capacity, capacity)),
        }
    }

    fn get(&self, key: &usize) -> bool {
        self.cache.get(key).is_some()
    }

    fn insert(&self, key: usize, req_id: usize) {
        let value = super::make_value(&self.config, key, req_id);
        super::sleep_thread_for_insertion(&self.config);
        self.cache.put(key, value, 1);
    }
}

impl CacheDriver<TraceEntry> for TinyUfoCache {
    fn get_or_insert(&mut self, entry: &TraceEntry, report: &mut Report) {
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

    fn get_or_insert_once(&mut self, _entry: &TraceEntry, _report: &mut Report) {
        unimplemented!();
    }

    fn update(&mut self, entry: &TraceEntry, report: &mut Report) {
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
