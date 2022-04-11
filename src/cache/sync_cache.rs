use super::{BuildFnvHasher, CacheSet, Counters, InitClosureType};
use crate::{
    cache::{InitClosureError1, InitClosureError2},
    config::Config,
    parser::ArcTraceEntry,
    report::Report,
};

use moka::sync::Cache;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct SyncCache {
    config: Config,
    cache: Cache<usize, (u32, Arc<[u8]>), BuildFnvHasher>,
}

impl Clone for SyncCache {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            cache: self.cache.clone(),
        }
    }
}

impl SyncCache {
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
        if config.invalidate_entries_if {
            builder = builder.support_invalidation_closures();
        }
        if config.size_aware {
            builder = builder.weigher(|_k, (s, _v)| *s);
        }

        Self {
            config: config.clone(),
            cache: builder.build_with_hasher(BuildFnvHasher::default()),
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

    fn get_with(&self, key: usize, req_id: usize, is_inserted: Arc<AtomicBool>) {
        self.cache.get_with(key, || {
            super::sleep_thread_for_insertion(&self.config);
            is_inserted.store(true, Ordering::Release);
            super::make_value(&self.config, key, req_id)
        });
    }

    fn try_get_with(
        &self,
        ty: InitClosureType,
        key: usize,
        req_id: usize,
        is_inserted: Arc<AtomicBool>,
    ) {
        match ty {
            InitClosureType::GetOrTryInsertWithError1 => self
                .cache
                .try_get_with(key, || {
                    super::sleep_thread_for_insertion(&self.config);
                    is_inserted.store(true, Ordering::Release);
                    Ok(super::make_value(&self.config, key, req_id)) as Result<_, InitClosureError1>
                })
                .is_ok(),
            InitClosureType::GetOrTyyInsertWithError2 => self
                .cache
                .try_get_with(key, || {
                    super::sleep_thread_for_insertion(&self.config);
                    is_inserted.store(true, Ordering::Release);
                    Ok(super::make_value(&self.config, key, req_id)) as Result<_, InitClosureError2>
                })
                .is_ok(),
            _ => unreachable!(),
        };
    }
}

impl CacheSet<ArcTraceEntry> for SyncCache {
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
        let mut counters = Counters::default();
        let mut req_id = entry.line_number();
        let is_inserted = Arc::new(AtomicBool::default());

        for block in entry.range() {
            {
                let is_inserted2 = Arc::clone(&is_inserted);
                match InitClosureType::select(block) {
                    InitClosureType::GetOrInsert => self.get_with(block, req_id, is_inserted2),
                    ty => self.try_get_with(ty, block, req_id, is_inserted2),
                }
            }

            if is_inserted.load(Ordering::Acquire) {
                counters.inserted();
                counters.read_missed();
                is_inserted.store(false, Ordering::Release);
            } else {
                counters.read_hit();
            }
            req_id += 1;
        }

        counters.add_to_report(report);
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
        for block in entry.range() {
            self.cache
                .invalidate_entries_if(move |_k, (_s, v)| v[0] == (block % 256) as u8)
                .expect("invalidate_entries_if failed");
        }
    }

    fn iterate(&mut self) {
        let mut count = 0usize;
        for _kv in &self.cache {
            count += 1;

            if count % 500 == 0 {
                std::thread::yield_now();
            }
        }
    }
}

pub struct SharedSyncCache(SyncCache);

impl SharedSyncCache {
    pub fn new(config: &Config, max_cap: u64, init_cap: usize) -> Self {
        Self(SyncCache::new(config, max_cap, init_cap))
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

    fn get_or_insert_once(&mut self, entry: &ArcTraceEntry, report: &mut Report) {
        self.0.get_or_insert_once(entry, report);
    }

    fn update(&mut self, entry: &ArcTraceEntry, report: &mut Report) {
        self.0.update(entry, report);
    }

    fn invalidate(&mut self, entry: &ArcTraceEntry) {
        self.0.invalidate(entry);
    }

    fn invalidate_all(&mut self) {
        self.0.invalidate_all();
    }

    fn invalidate_entries_if(&mut self, entry: &ArcTraceEntry) {
        self.0.invalidate_entries_if(entry);
    }

    fn iterate(&mut self) {
        self.0.iterate();
    }
}
