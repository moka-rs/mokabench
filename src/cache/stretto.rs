use super::{CacheSet, Counters, DefaultHasher};
use crate::{config::Config, parser::TraceEntry, report::Report};

use std::sync::Arc;

#[derive(Clone)]
pub struct StrettoCache {
    config: Config,
    cache: ::stretto::Cache<
        usize,
        (u32, Arc<[u8]>),
        ::stretto::DefaultKeyBuilder<usize>,
        ::stretto::DefaultCoster<(u32, Arc<[u8]>)>,
        ::stretto::DefaultUpdateValidator<(u32, Arc<[u8]>)>,
        ::stretto::DefaultCacheCallback<(u32, Arc<[u8]>)>,
        DefaultHasher,
    >,
}

impl StrettoCache {
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
            config: config.clone(),
            cache: ::stretto::Cache::builder(capacity * 10, capacity as i64)
                .set_hasher(DefaultHasher)
                .finalize()
                .unwrap(),
        }
    }

    fn get(&self, key: &usize) -> bool {
        self.cache.get(key).is_some()
    }

    fn insert(&self, key: usize, req_id: usize) {
        let value = super::make_value(&self.config, key, req_id);
        super::sleep_thread_for_insertion(&self.config);
        self.cache.insert(key, value, 1);
    }
}

impl CacheSet<TraceEntry> for StrettoCache {
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

    fn invalidate(&mut self, _entry: &TraceEntry) {
        unimplemented!();
    }

    fn invalidate_all(&mut self) {
        unimplemented!();
    }

    fn invalidate_entries_if(&mut self, _entry: &TraceEntry) {
        unimplemented!();
    }

    fn iterate(&mut self) {
        unimplemented!();
    }
}
