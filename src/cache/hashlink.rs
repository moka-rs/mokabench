use hashlink::LruCache;
use parking_lot::Mutex;

use super::{CacheDriver, Counters, DefaultHasher, Key, Value};
use crate::{config::Config, parser::TraceEntry, report::Report};

use std::sync::Arc;

type HashLinkCache = LruCache<Key, Value, DefaultHasher>;

#[derive(Clone)]
pub struct HashLink {
    config: Arc<Config>,
    cache: Arc<Mutex<HashLinkCache>>,
}

impl HashLink {
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

        Self {
            config: Arc::new(config.clone()),
            cache: Arc::new(Mutex::new(LruCache::with_hasher(
                capacity,
                DefaultHasher::default(),
            ))),
        }
    }

    fn get(&self, key: &usize) -> bool {
        self.cache.lock().get(key).cloned().is_some()
    }

    fn insert(&self, key: usize, req_id: usize) {
        let value = super::make_value(&self.config, key, req_id);
        super::sleep_thread_for_insertion(&self.config);
        self.cache.lock().insert(key, value);
    }
}

impl CacheDriver<TraceEntry> for HashLink {
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
