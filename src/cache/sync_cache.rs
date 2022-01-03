use super::{BuildFnvHasher, CacheSet, Counters, InitClosureType};
use crate::{
    cache::{InitClosureError1, InitClosureError2},
    config::Config,
    parser::ArcTraceEntry,
    report::Report,
};

use moka::sync::{Cache, CacheBuilder};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct SyncCache {
    config: Config,
    cache: Cache<usize, Arc<[u8]>, BuildFnvHasher>,
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
        if config.enable_invalidate_entries_if {
            builder = builder.support_invalidation_closures();
        }

        Self {
            config: config.clone(),
            cache: builder.build_with_hasher(BuildFnvHasher::default()),
        }
    }

    fn get(&self, key: &usize) -> bool {
        self.cache.get(key).is_some()
    }

    fn insert(&self, key: usize) {
        let value = super::make_value(key);
        super::sleep_thread_for_insertion(&self.config);
        self.cache.insert(key, value);
    }

    fn get_or_insert_with(&self, key: usize, is_inserted: Arc<AtomicBool>) {
        self.cache.get_or_insert_with(key, || {
            super::sleep_thread_for_insertion(&self.config);
            is_inserted.store(true, Ordering::Release);
            super::make_value(key)
        });
    }

    fn get_or_try_insert_with(
        &self,
        ty: InitClosureType,
        key: usize,
        is_inserted: Arc<AtomicBool>,
    ) {
        match ty {
            InitClosureType::GetOrTryInsertWithError1 => self
                .cache
                .get_or_try_insert_with(key, || {
                    super::sleep_thread_for_insertion(&self.config);
                    is_inserted.store(true, Ordering::Release);
                    Ok(super::make_value(key)) as Result<_, InitClosureError1>
                })
                .is_ok(),
            InitClosureType::GetOrTyyInsertWithError2 => self
                .cache
                .get_or_try_insert_with(key, || {
                    super::sleep_thread_for_insertion(&self.config);
                    is_inserted.store(true, Ordering::Release);
                    Ok(super::make_value(key)) as Result<_, InitClosureError2>
                })
                .is_ok(),
            _ => unreachable!(),
        };
    }
}

impl CacheSet<ArcTraceEntry> for SyncCache {
    fn get_or_insert(&mut self, entry: &ArcTraceEntry, report: &mut Report) {
        let mut counters = Counters::default();

        for block in entry.0.clone() {
            if self.get(&block) {
                counters.read_hit();
            } else {
                self.insert(block);
                counters.inserted();
                counters.read_missed();
            }
        }

        counters.add_to_report(report);
    }

    fn get_or_insert_once(&mut self, entry: &ArcTraceEntry, report: &mut Report) {
        let mut counters = Counters::default();
        let is_inserted = Arc::new(AtomicBool::default());

        for block in entry.0.clone() {
            {
                let is_inserted2 = Arc::clone(&is_inserted);
                match InitClosureType::select(block) {
                    InitClosureType::GetOrInsert => self.get_or_insert_with(block, is_inserted2),
                    ty => self.get_or_try_insert_with(ty, block, is_inserted2),
                }
            }

            if is_inserted.load(Ordering::Acquire) {
                counters.inserted();
                counters.read_missed();
                is_inserted.store(false, Ordering::Release);
            } else {
                counters.read_hit();
            }
        }

        counters.add_to_report(report);
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
        for block in entry.0.clone() {
            self.cache
                .invalidate_entries_if(move |_k, v| v[0] == (block % 256) as u8)
                .expect("invalidate_entries_if failed");
        }
    }
}

pub struct SharedSyncCache(SyncCache);

impl SharedSyncCache {
    pub fn new(config: &Config, capacity: usize) -> Self {
        Self(SyncCache::new(config, capacity))
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

    fn invalidate(&mut self, entry: &ArcTraceEntry) {
        self.0.invalidate(entry);
    }

    fn invalidate_all(&mut self) {
        self.0.invalidate_all();
    }

    fn invalidate_entries_if(&mut self, entry: &ArcTraceEntry) {
        self.0.invalidate_entries_if(entry);
    }
}
