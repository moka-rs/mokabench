use std::sync::Arc;

use crate::{
    cache::{self, CacheDriver, Counters, DefaultHasher, Key, Value},
    config::Config,
    parser::TraceEntry,
    report::Report,
};

#[cfg(feature = "mini-moka")]
use mini_moka::sync::Cache;

#[cfg(all(
    not(feature = "mini-moka"),
    any(feature = "moka-v09", feature = "moka-v08")
))]
use crate::moka::dash::Cache;

#[derive(Clone)]
pub struct MiniMokSyncCache {
    config: Arc<Config>,
    cache: Cache<Key, Value, DefaultHasher>,
}

impl MiniMokSyncCache {
    pub fn new(config: &Config, max_cap: u64, init_cap: usize) -> Self {
        let mut builder = Cache::builder()
            .max_capacity(max_cap)
            .initial_capacity(init_cap);
        if let Some(ttl) = config.ttl {
            builder = builder.time_to_live(ttl);
        }
        if let Some(tti) = config.tti {
            builder = builder.time_to_idle(tti)
        }
        if config.size_aware {
            builder = builder.weigher(|_k, (s, _v)| *s);
        }

        Self {
            config: Arc::new(config.clone()),
            cache: builder.build_with_hasher(DefaultHasher::default()),
        }
    }

    fn get(&self, key: &usize) -> bool {
        self.cache.get(key).is_some()
    }

    fn insert(&self, key: usize, req_id: usize) {
        let value = cache::make_value(&self.config, key, req_id);
        cache::sleep_thread_for_insertion(&self.config);
        self.cache.insert(key, value);
    }
}

impl CacheDriver<TraceEntry> for MiniMokSyncCache {
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

    fn invalidate(&mut self, entry: &TraceEntry) {
        for block in entry.range() {
            self.cache.invalidate(&block);
        }
    }

    fn invalidate_all(&mut self) {
        self.cache.invalidate_all();
    }

    fn iterate(&mut self) {
        let mut count = 0usize;
        for entry in &self.cache {
            entry.key();
            count += 1;

            if count % 500 == 0 {
                std::thread::yield_now();
            }
        }
    }
}
