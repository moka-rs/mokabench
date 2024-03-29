//! Driver for `moka::future::Cache` v0.12.0 or later.

use super::{AsyncGetOrInsertOnce, InitClosureError1, InitClosureError2, InitClosureType};
use crate::cache::{Key, Value};
use crate::moka::future::Cache;
use crate::{
    async_rt_helper as rt,
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

pub struct MokaAsyncCache<I> {
    config: Arc<Config>,
    cache: Cache<Key, Value, DefaultHasher>,
    insert_once_impl: I,
    eviction_counters: Option<Arc<EvictionCounters>>,
}

impl<I: Clone> Clone for MokaAsyncCache<I> {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            cache: self.cache.clone(),
            insert_once_impl: self.insert_once_impl.clone(),
            eviction_counters: self.eviction_counters.as_ref().map(Arc::clone),
        }
    }
}

impl MokaAsyncCache<GetWith> {
    pub(crate) fn new(config: &Config, max_cap: u64, init_cap: usize) -> Self {
        let (cache, eviction_counters) = Self::create_cache(config, max_cap, init_cap);
        let config = Arc::new(config.clone());
        let insert_once_impl = GetWith {
            cache: cache.clone(),
            config: Arc::clone(&config),
        };

        Self {
            config,
            cache,
            insert_once_impl,
            eviction_counters,
        }
    }
}

use entry_api::EntryOrInsertWith;

impl MokaAsyncCache<EntryOrInsertWith> {
    pub(crate) fn with_entry_api(config: &Config, max_cap: u64, init_cap: usize) -> Self {
        let (cache, eviction_counters) = Self::create_cache(config, max_cap, init_cap);
        let config = Arc::new(config.clone());
        let insert_once_impl = EntryOrInsertWith::new(cache.clone(), Arc::clone(&config));

        Self {
            config,
            cache,
            insert_once_impl,
            eviction_counters,
        }
    }
}

impl<I> MokaAsyncCache<I> {
    fn create_cache(
        config: &Config,
        max_cap: u64,
        init_cap: usize,
    ) -> (
        Cache<Key, Value, DefaultHasher>,
        Option<Arc<EvictionCounters>>,
    ) {
        let mut builder = Cache::builder()
            .max_capacity(max_cap)
            .initial_capacity(init_cap);

        if config.per_key_expiration {
            use crate::cache::moka_driver::expiry::MokabenchExpiry;
            let expiry = MokabenchExpiry::new(config.ttl, config.tti);
            builder = builder.expire_after(expiry);
        }

        if let Some(ttl) = config.ttl {
            if !config.per_key_expiration {
                builder = builder.time_to_live(ttl);
            }
        }
        if let Some(tti) = config.tti {
            if !config.per_key_expiration {
                builder = builder.time_to_idle(tti)
            }
        }

        if config.invalidate_entries_if {
            builder = builder.support_invalidation_closures();
        }
        if config.size_aware {
            builder = builder.weigher(|_k, (s, _v)| *s);
        }

        let eviction_counters;

        if config.is_eviction_listener_enabled() {
            let c0 = Arc::new(EvictionCounters::default());
            let c1 = Arc::clone(&c0);

            builder = builder.eviction_listener(move |_k, _v, cause| {
                c1.increment(cause);
            });

            eviction_counters = Some(c0);
        } else {
            eviction_counters = None;
        }

        let cache = builder.build_with_hasher(DefaultHasher);
        (cache, eviction_counters)
    }

    async fn get(&self, key: usize) -> bool {
        self.cache.get(&key).await.is_some()
    }

    async fn insert(&self, key: usize, req_id: usize) {
        let value = cache::make_value(&self.config, key, req_id);
        cache::sleep_task_for_insertion(&self.config).await;
        self.cache.insert(key, value).await;
    }
}

#[async_trait]
impl<I> AsyncCacheDriver<TraceEntry> for MokaAsyncCache<I>
where
    I: AsyncGetOrInsertOnce + Send + Sync,
{
    async fn get_or_insert(&mut self, entry: &TraceEntry, report: &mut Report) {
        let mut counters = Counters::default();
        let mut req_id = entry.line_number();

        for block in entry.range() {
            if self.get(block).await {
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
        self.insert_once_impl
            .get_or_insert_once(entry, report)
            .await;
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
                rt::yield_now().await;
            }
        }
    }

    fn eviction_counters(&self) -> Option<Arc<EvictionCounters>> {
        self.eviction_counters.as_ref().map(Arc::clone)
    }
}

//
// GetWith (implements GetOrInsertOnce)
//
#[derive(Clone)]
pub(crate) struct GetWith {
    cache: Cache<Key, Value, DefaultHasher>,
    config: Arc<Config>,
}

#[async_trait]
impl AsyncGetOrInsertOnce for GetWith {
    async fn get_or_insert_once(&self, entry: &TraceEntry, report: &mut Report) {
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
}

impl GetWith {
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

//
// EntryOrInsertWith (implements GetOrInsertOnce)
//
mod entry_api {
    use super::*;

    #[derive(Clone)]
    pub(crate) struct EntryOrInsertWith {
        cache: Cache<Key, Value, DefaultHasher>,
        config: Arc<Config>,
    }

    impl EntryOrInsertWith {
        pub(crate) fn new(cache: Cache<Key, Value, DefaultHasher>, config: Arc<Config>) -> Self {
            Self { cache, config }
        }
    }

    #[async_trait]
    impl AsyncGetOrInsertOnce for EntryOrInsertWith {
        async fn get_or_insert_once(&self, entry: &TraceEntry, report: &mut Report) {
            let mut counters = Counters::default();
            let mut req_id = entry.line_number();

            for block in entry.range() {
                let is_inserted = match InitClosureType::select(block) {
                    InitClosureType::GetOrInsert => self.entry_or_insert_with(block, req_id).await,
                    ty => self.entry_or_try_insert_with(ty, block, req_id).await,
                };

                if is_inserted {
                    counters.inserted();
                    counters.read_missed();
                } else {
                    counters.read_hit();
                }
                req_id += 1;
            }

            counters.add_to_report(report);
        }
    }

    impl EntryOrInsertWith {
        async fn entry_or_insert_with(&self, key: usize, req_id: usize) -> bool {
            self.cache
                .entry(key)
                .or_insert_with(async {
                    cache::sleep_task_for_insertion(&self.config).await;
                    cache::make_value(&self.config, key, req_id)
                })
                .await
                .is_fresh()
        }

        async fn entry_or_try_insert_with(
            &self,
            ty: InitClosureType,
            key: usize,
            req_id: usize,
        ) -> bool {
            match ty {
                InitClosureType::GetOrTryInsertWithError1 => self
                    .cache
                    .entry(key)
                    .or_try_insert_with(async {
                        cache::sleep_task_for_insertion(&self.config).await;
                        Ok(cache::make_value(&self.config, key, req_id))
                            as Result<_, InitClosureError1>
                    })
                    .await
                    .unwrap()
                    .is_fresh(),
                InitClosureType::GetOrTyyInsertWithError2 => self
                    .cache
                    .entry(key)
                    .or_try_insert_with(async {
                        cache::sleep_task_for_insertion(&self.config).await;
                        Ok(cache::make_value(&self.config, key, req_id))
                            as Result<_, InitClosureError2>
                    })
                    .await
                    .unwrap()
                    .is_fresh(),
                _ => unreachable!(),
            }
        }
    }
}
