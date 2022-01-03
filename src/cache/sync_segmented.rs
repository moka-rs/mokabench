use crate::{
    cache::{CacheSet, InitClosureError1, InitClosureError2},
    config::Config,
    parser::ArcTraceEntry,
    report::Report,
};

use moka::sync::{CacheBuilder, SegmentedCache};
use parking_lot::RwLock;
use std::sync::Arc;

use super::{BuildFnvHasher, Counters, InitClosureType};

pub struct SegmentedMoka {
    config: Config,
    cache: SegmentedCache<usize, Arc<[u8]>, BuildFnvHasher>,
}

impl Clone for SegmentedMoka {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            cache: self.cache.clone(),
        }
    }
}

impl SegmentedMoka {
    pub fn new(config: &Config, capacity: usize, num_segments: usize) -> Self {
        #[allow(clippy::useless_conversion)]
        let max_capacity = capacity.try_into().unwrap();
        let mut builder = CacheBuilder::new(max_capacity)
            .initial_capacity(capacity)
            .segments(num_segments);
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

    fn get_or_insert_with(&self, key: usize, counters: Arc<RwLock<Counters>>) {
        self.cache.get_or_insert_with(key, || {
            super::sleep_thread_for_insertion(&self.config);
            counters.write().inserted();
            super::make_value(key)
        });
    }

    fn get_or_try_insert_with(
        &self,
        ty: InitClosureType,
        key: usize,
        counters: Arc<RwLock<Counters>>,
    ) {
        match ty {
            InitClosureType::GetOrTryInsertWithError1 => self
                .cache
                .get_or_try_insert_with(key, || {
                    super::sleep_thread_for_insertion(&self.config);
                    counters.write().inserted();
                    Ok(super::make_value(key)) as Result<_, InitClosureError1>
                })
                .is_ok(),
            InitClosureType::GetOrTyyInsertWithError2 => self
                .cache
                .get_or_try_insert_with(key, || {
                    super::sleep_thread_for_insertion(&self.config);
                    counters.write().inserted();
                    Ok(super::make_value(key)) as Result<_, InitClosureError2>
                })
                .is_ok(),
            _ => unreachable!(),
        };
    }
}

impl CacheSet<ArcTraceEntry> for SegmentedMoka {
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
        let counters = Arc::new(RwLock::new(Counters::default()));

        for block in entry.0.clone() {
            {
                let counters2 = Arc::clone(&counters);
                match InitClosureType::select(block) {
                    InitClosureType::GetOrInsert => self.get_or_insert_with(block, counters2),
                    ty => self.get_or_try_insert_with(ty, block, counters2),
                }
            }
            counters.write().read();
        }

        counters.read().add_to_report(report);
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

pub struct SharedSegmentedMoka(SegmentedMoka);

impl SharedSegmentedMoka {
    pub fn new(config: &Config, capacity: usize, num_segments: usize) -> Self {
        Self(SegmentedMoka::new(config, capacity, num_segments))
    }
}

impl Clone for SharedSegmentedMoka {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl CacheSet<ArcTraceEntry> for SharedSegmentedMoka {
    fn get_or_insert(&mut self, entry: &ArcTraceEntry, report: &mut Report) {
        self.0.get_or_insert(entry, report)
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
