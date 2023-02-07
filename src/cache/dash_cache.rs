use super::{CacheSet, Counters, DefaultHasher};
use crate::{
    config::Config,
    parser::TraceEntry,
    report::Report,
};

#[cfg(feature = "mini-moka")]
use mini_moka::sync::Cache;

#[cfg(all(not(feature = "mini-moka"), any(feature = "moka-v09", feature = "moka-v08")))]
use crate::moka::dash::Cache;

use std::sync::Arc;

pub struct DashCache {
    config: Config,
    cache: Cache<usize, (u32, Arc<[u8]>), DefaultHasher>,
}

impl Clone for DashCache {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            cache: self.cache.clone(),
        }
    }
}

impl DashCache {
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
            config: config.clone(),
            cache: builder.build_with_hasher(DefaultHasher::default()),
        }
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

impl CacheSet<TraceEntry> for DashCache {
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

    fn invalidate_entries_if(&mut self, _entry: &TraceEntry) {
        unimplemented!();
    }

    // fn invalidate_entries_if(&mut self, entry: &TraceEntry) {
    //     for block in entry.range() {
    //         self.cache
    //             .invalidate_entries_if(move |_k, (_s, v)| v[0] == (block % 256) as u8)
    //             .expect("invalidate_entries_if failed");
    //     }
    // }

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

pub struct SharedDashCache(DashCache);

impl SharedDashCache {
    pub fn new(config: &Config, max_cap: u64, init_cap: usize) -> Self {
        Self(DashCache::new(config, max_cap, init_cap))
    }
}

impl Clone for SharedDashCache {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl CacheSet<TraceEntry> for SharedDashCache {
    fn get_or_insert(&mut self, entry: &TraceEntry, report: &mut Report) {
        self.0.get_or_insert(entry, report);
    }

    fn get_or_insert_once(&mut self, entry: &TraceEntry, report: &mut Report) {
        // self.0.get_or_insert_once(entry, report);
        self.0.get_or_insert(entry, report);
    }

    fn update(&mut self, entry: &TraceEntry, report: &mut Report) {
        self.0.update(entry, report);
    }

    fn invalidate(&mut self, entry: &TraceEntry) {
        self.0.invalidate(entry);
    }

    fn invalidate_all(&mut self) {
        self.0.invalidate_all();
    }

    fn invalidate_entries_if(&mut self, _entry: &TraceEntry) {
        // DO NOTHING FOR NOW.

        // self.0.invalidate_entries_if(entry);
    }

    fn iterate(&mut self) {
        self.0.iterate();
    }
}
