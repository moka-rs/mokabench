use super::{Counters, DefaultHasher, InitClosureType};
use crate::{
    cache::{CacheSet, InitClosureError1, InitClosureError2},
    config::Config,
    config::RemovalNotificationMode,
    moka::sync::SegmentedCache,
    parser::TraceEntry,
    report::Report,
};

#[cfg(feature = "moka-v09")]
use crate::EvictionCounters;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct SegmentedMoka {
    config: Config,
    cache: SegmentedCache<usize, (u32, Arc<[u8]>), DefaultHasher>,
    #[cfg(feature = "moka-v09")]
    eviction_counters: Option<Arc<EvictionCounters>>,
}

impl Clone for SegmentedMoka {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            cache: self.cache.clone(),
            #[cfg(feature = "moka-v09")]
            eviction_counters: self.eviction_counters.as_ref().map(Arc::clone),
        }
    }
}

impl SegmentedMoka {
    pub fn new(config: &Config, max_cap: u64, init_cap: usize, num_segments: usize) -> Self {
        let mut builder = SegmentedCache::builder(num_segments)
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

        #[cfg(feature = "moka-v09")]
        {
            use crate::moka::notification::{Configuration, DeliveryMode};

            let eviction_counters;

            if config.is_eviction_listener_enabled() {
                let c0 = Arc::new(EvictionCounters::default());
                let c1 = Arc::clone(&c0);

                let mode = match config.eviction_listener {
                    RemovalNotificationMode::Immediate => DeliveryMode::Immediate,
                    RemovalNotificationMode::Queued => DeliveryMode::Queued,
                    RemovalNotificationMode::None => unreachable!(),
                };

                let conf = Configuration::builder().delivery_mode(mode).build();

                builder = builder.eviction_listener_with_conf(
                    move |_k, _v, cause| {
                        c1.increment(cause);
                    },
                    conf,
                );

                eviction_counters = Some(c0);
            } else {
                eviction_counters = None;
            }

            Self {
                config: config.clone(),
                cache: builder.build_with_hasher(DefaultHasher::default()),
                eviction_counters,
            }
        }

        #[cfg(not(feature = "moka-v09"))]
        {
            Self {
                config: config.clone(),
                cache: builder.build_with_hasher(DefaultHasher::default()),
            }
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

impl CacheSet<TraceEntry> for SegmentedMoka {
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

    fn get_or_insert_once(&mut self, entry: &TraceEntry, report: &mut Report) {
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

    fn invalidate_entries_if(&mut self, entry: &TraceEntry) {
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

pub struct SharedSegmentedMoka(SegmentedMoka);

impl SharedSegmentedMoka {
    pub fn new(config: &Config, max_cap: u64, init_cap: usize, num_segments: usize) -> Self {
        Self(SegmentedMoka::new(config, max_cap, init_cap, num_segments))
    }

    #[cfg(feature = "moka-v09")]
    pub(crate) fn eviction_counters(&self) -> Option<Arc<EvictionCounters>> {
        self.0.eviction_counters.as_ref().map(Arc::clone)
    }
}

impl Clone for SharedSegmentedMoka {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl CacheSet<TraceEntry> for SharedSegmentedMoka {
    fn get_or_insert(&mut self, entry: &TraceEntry, report: &mut Report) {
        self.0.get_or_insert(entry, report)
    }

    fn get_or_insert_once(&mut self, entry: &TraceEntry, report: &mut Report) {
        self.0.get_or_insert_once(entry, report);
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

    fn invalidate_entries_if(&mut self, entry: &TraceEntry) {
        self.0.invalidate_entries_if(entry);
    }

    fn iterate(&mut self) {
        self.0.iterate();
    }
}
