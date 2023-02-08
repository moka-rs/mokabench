use super::{InitClosureError1, InitClosureError2, InitClosureType};
use crate::{
    cache::{self, CacheDriver, Counters, DefaultHasher, Key, Value},
    config::Config,
    moka::sync::SegmentedCache,
    parser::TraceEntry,
    report::Report,
    EvictionCounters,
};

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct MokaSegmentedCache {
    config: Arc<Config>,
    cache: SegmentedCache<Key, Value, DefaultHasher>,
    eviction_counters: Option<Arc<EvictionCounters>>,
}

impl Clone for MokaSegmentedCache {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            cache: self.cache.clone(),
            eviction_counters: self.eviction_counters.as_ref().map(Arc::clone),
        }
    }
}

impl MokaSegmentedCache {
    pub fn new(config: &Config, max_cap: u64, init_cap: usize, num_segments: usize) -> Self {
        let config = Arc::new(config.clone());

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

        #[cfg(feature = "moka-v08")]
        {
            Self {
                config,
                cache: builder.build_with_hasher(DefaultHasher::default()),
                eviction_counters: None,
            }
        }

        #[cfg(not(feature = "moka-v08"))]
        {
            use crate::config::RemovalNotificationMode;
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
                config,
                cache: builder.build_with_hasher(DefaultHasher::default()),
                eviction_counters,
            }
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

    fn get_with(&self, key: usize, req_id: usize, is_inserted: Arc<AtomicBool>) {
        self.cache.get_with(key, || {
            cache::sleep_thread_for_insertion(&self.config);
            is_inserted.store(true, Ordering::Release);
            cache::make_value(&self.config, key, req_id)
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
                    cache::sleep_thread_for_insertion(&self.config);
                    is_inserted.store(true, Ordering::Release);
                    Ok(cache::make_value(&self.config, key, req_id)) as Result<_, InitClosureError1>
                })
                .is_ok(),
            InitClosureType::GetOrTyyInsertWithError2 => self
                .cache
                .try_get_with(key, || {
                    cache::sleep_thread_for_insertion(&self.config);
                    is_inserted.store(true, Ordering::Release);
                    Ok(cache::make_value(&self.config, key, req_id)) as Result<_, InitClosureError2>
                })
                .is_ok(),
            _ => unreachable!(),
        };
    }
}

impl CacheDriver<TraceEntry> for MokaSegmentedCache {
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

    fn eviction_counters(&self) -> Option<Arc<EvictionCounters>> {
        self.eviction_counters.as_ref().map(Arc::clone)
    }
}
