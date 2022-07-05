use super::{BuildFnvHasher, CacheSet, Counters};
use crate::{config::Config, parser::ArcTraceEntry, report::Report};
use crate::moka::unsync::{Cache, CacheBuilder};

use std::sync::Arc;

pub struct UnsyncCache {
    config: Config,
    cache: Cache<usize, (u32, Arc<[u8]>), BuildFnvHasher>,
}

impl UnsyncCache {
    pub fn new(config: &Config, max_cap: u64, init_cap: usize) -> Self {
        let mut builder = CacheBuilder::new(max_cap).initial_capacity(init_cap);
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
            config: config.clone(),
            cache: builder.build_with_hasher(BuildFnvHasher::default()),
        }
    }

    fn get(&mut self, key: &usize) -> bool {
        self.cache.get(key).is_some()
    }

    fn insert(&mut self, key: usize, req_id: usize) {
        let value = super::make_value(&self.config, key, req_id);
        super::sleep_thread_for_insertion(&self.config);
        self.cache.insert(key, value);
    }

    fn invalidate_entries_if(&mut self, entry: &ArcTraceEntry) {
        for block in entry.range() {
            self.cache
                .invalidate_entries_if(move |_k, (_s, v)| v[0] == (block % 256) as u8)
        }
    }
}

impl CacheSet<ArcTraceEntry> for UnsyncCache {
    fn get_or_insert(&mut self, entry: &ArcTraceEntry, report: &mut Report) {
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

    fn get_or_insert_once(&mut self, entry: &ArcTraceEntry, report: &mut Report) {
        self.get_or_insert(entry, report);
    }

    fn update(&mut self, entry: &ArcTraceEntry, report: &mut Report) {
        let mut counters = Counters::default();
        let mut req_id = entry.line_number();

        for block in entry.range() {
            self.insert(block, req_id);
            counters.inserted();
            req_id += 1;
        }

        counters.add_to_report(report);
    }

    fn invalidate(&mut self, entry: &ArcTraceEntry) {
        for block in entry.range() {
            self.cache.invalidate(&block);
        }
    }

    fn invalidate_all(&mut self) {
        self.cache.invalidate_all();
    }

    fn invalidate_entries_if(&mut self, entry: &ArcTraceEntry) {
        self.invalidate_entries_if(entry);
    }

    fn iterate(&mut self) {
        self.cache.iter().count();
    }
}
