use super::{InitClosureError1, InitClosureError2, InitClosureType};
use crate::cache::{Key, Value};
use crate::moka::future::Cache;
use crate::{
    cache::{self, AsyncCacheDriver, Counters, DefaultHasher},
    config::Config,
    parser::TraceEntry,
    report::Report,
    EvictionCounters,
};

use async_trait::async_trait;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct MokaAsyncCache {
    config: Arc<Config>,
    cache: Cache<Key, Value, DefaultHasher>,
    eviction_counters: Option<Arc<EvictionCounters>>,
}

impl Clone for MokaAsyncCache {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            cache: self.cache.clone(),
            eviction_counters: self.eviction_counters.as_ref().map(Arc::clone),
        }
    }
}

impl MokaAsyncCache {
    pub fn new(config: &Config, max_cap: u64, init_cap: usize) -> Self {
        let config = Arc::new(config.clone());

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
            let eviction_counters;

            if config.is_eviction_listener_enabled() {
                let c0 = Arc::new(EvictionCounters::default());
                let c1 = Arc::clone(&c0);

                builder =
                    builder.eviction_listener_with_queued_delivery_mode(move |_k, _v, cause| {
                        c1.increment(cause);
                    });

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

    fn get(&self, key: usize) -> bool {
        self.cache.get(&key).is_some()
    }

    async fn insert(&self, key: usize, req_id: usize) {
        let value = cache::make_value(&self.config, key, req_id);
        cache::sleep_task_for_insertion(&self.config).await;
        self.cache.insert(key, value).await;
    }

    async fn get_with(&self, key: usize, req_id: usize, is_inserted: Arc<AtomicBool>) {
        self.cache
            .get_with(key, async {
                cache::sleep_task_for_insertion(&self.config).await;
                is_inserted.store(true, Ordering::Release);
                cache::make_value(&self.config, key, req_id)
            })
            .await;
    }

    async fn try_get_with(
        &self,
        ty: InitClosureType,
        key: usize,
        req_id: usize,
        is_inserted: Arc<AtomicBool>,
    ) {
        match ty {
            InitClosureType::GetOrTryInsertWithError1 => self
                .cache
                .try_get_with(key, async {
                    cache::sleep_task_for_insertion(&self.config).await;
                    is_inserted.store(true, Ordering::Release);
                    Ok(cache::make_value(&self.config, key, req_id)) as Result<_, InitClosureError1>
                })
                .await
                .is_ok(),
            InitClosureType::GetOrTyyInsertWithError2 => self
                .cache
                .try_get_with(key, async {
                    cache::sleep_task_for_insertion(&self.config).await;
                    is_inserted.store(true, Ordering::Release);
                    Ok(cache::make_value(&self.config, key, req_id)) as Result<_, InitClosureError2>
                })
                .await
                .is_ok(),
            _ => unreachable!(),
        };
    }
}

#[async_trait]
impl AsyncCacheDriver<TraceEntry> for MokaAsyncCache {
    async fn get_or_insert(&mut self, entry: &TraceEntry, report: &mut Report) {
        let mut counters = Counters::default();
        let mut req_id = entry.line_number();

        for block in entry.range() {
            if self.get(block) {
                counters.read_hit();
            } else {
                self.insert(block, req_id).await;
                counters.inserted();
                counters.read_missed();
            }
            req_id += 1;
        }

        counters.add_to_report(report);
    }

    async fn get_or_insert_once(&mut self, entry: &TraceEntry, report: &mut Report) {
        let mut counters = Counters::default();
        let mut req_id = entry.line_number();
        let is_inserted = Arc::new(AtomicBool::default());

        for block in entry.range() {
            {
                let is_inserted2 = Arc::clone(&is_inserted);
                match InitClosureType::select(block) {
                    InitClosureType::GetOrInsert => {
                        self.get_with(block, req_id, is_inserted2).await
                    }
                    ty => self.try_get_with(ty, block, req_id, is_inserted2).await,
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

    async fn update(&mut self, entry: &TraceEntry, report: &mut Report) {
        let mut counters = Counters::default();
        let mut req_id = entry.line_number();

        for block in entry.range() {
            self.insert(block, req_id).await;
            counters.inserted();
            req_id += 1;
        }

        counters.add_to_report(report);
    }

    async fn invalidate(&mut self, entry: &TraceEntry) {
        for block in entry.range() {
            self.cache.invalidate(&block).await;
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

    async fn iterate(&mut self) {
        let mut count = 0usize;
        for _kv in &self.cache {
            count += 1;

            if count % 500 == 0 {
                tokio::task::yield_now().await;
            }
        }
    }

    fn eviction_counters(&self) -> Option<Arc<EvictionCounters>> {
        self.eviction_counters.as_ref().map(Arc::clone)
    }
}
