use super::{CacheDriver, Counters, DefaultHasher, Key, Value};
use crate::{config::Config, parser::TraceEntry, report::Report};

use ::quick_cache::OptionsBuilder;

use std::sync::Arc;

type QuickCacheImpl = ::quick_cache::sync::Cache<Key, Value, CustomWeighter, DefaultHasher>;

#[derive(Clone)]
pub struct QuickCache {
    config: Arc<Config>,
    cache: Arc<QuickCacheImpl>,
}

#[derive(Clone)]
struct CustomWeighter(bool);

impl ::quick_cache::Weighter<usize, (), (u32, Arc<[u8]>)> for CustomWeighter {
    fn weight(&self, _key: &usize, _version: &(), val: &(u32, Arc<[u8]>)) -> std::num::NonZeroU32 {
        if self.0 {
            std::num::NonZeroU32::new(val.0).unwrap()
        } else {
            std::num::NonZeroU32::new(1).unwrap()
        }
    }
}

impl QuickCache {
    pub fn new(config: &Config, estimated_items_capacity: usize, capacity: u64) -> Self {
        if let Some(_ttl) = config.ttl {
            todo!()
        }
        if let Some(_tti) = config.tti {
            todo!()
        }

        let options = OptionsBuilder::new()
            .estimated_items_capacity(estimated_items_capacity)
            .weight_capacity(capacity)
            .build()
            .unwrap();

        Self {
            config: Arc::new(config.clone()),
            cache: ::quick_cache::sync::Cache::with_options(
                options,
                CustomWeighter(config.size_aware),
                DefaultHasher,
            )
            .into(),
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

impl CacheDriver<TraceEntry> for QuickCache {
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
}
