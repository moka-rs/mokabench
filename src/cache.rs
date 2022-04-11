use std::{
    hash::{BuildHasher, Hash, Hasher},
    sync::Arc,
};

use crate::{config::Config, Report};

use async_trait::async_trait;
use fnv::{FnvBuildHasher, FnvHasher};
use thiserror::Error;

pub(crate) mod async_cache;
pub(crate) mod dash_cache;
pub(crate) mod sync_cache;
pub(crate) mod sync_segmented;
pub(crate) mod unsync_cache;

pub trait CacheSet<T> {
    fn get_or_insert(&mut self, entry: &T, report: &mut Report);
    fn get_or_insert_once(&mut self, entry: &T, report: &mut Report);
    fn update(&mut self, entry: &T, report: &mut Report);
    fn invalidate(&mut self, entry: &T);
    fn invalidate_all(&mut self);
    fn invalidate_entries_if(&mut self, entry: &T);
    fn iterate(&mut self);
}

#[async_trait]
pub trait AsyncCacheSet<T> {
    async fn get_or_insert(&mut self, entry: &T, report: &mut Report);
    async fn get_or_insert_once(&mut self, entry: &T, report: &mut Report);
    async fn update(&mut self, entry: &T, report: &mut Report);
    async fn invalidate(&mut self, entry: &T);
    fn invalidate_all(&mut self);
    fn invalidate_entries_if(&mut self, entry: &T);
    async fn iterate(&mut self);
}

#[derive(Default)]
pub(crate) struct Counters {
    insert_count: usize,
    read_count: usize,
    hit_count: usize,
}

impl Counters {
    pub(crate) fn inserted(&mut self) {
        self.insert_count += 1;
    }

    pub(crate) fn read_hit(&mut self) {
        self.read_count += 1;
        self.hit_count += 1;
    }

    pub(crate) fn read_missed(&mut self) {
        self.read_count += 1;
    }

    pub(crate) fn add_to_report(&self, report: &mut Report) {
        report.insert_count += self.insert_count;
        report.read_count += self.read_count;
        report.hit_count += self.hit_count;
    }
}

const VALUE_LEN: usize = 128;

pub(crate) fn make_value(config: &Config, key: usize, req_id: usize) -> (u32, Arc<[u8]>) {
    let policy_weight = if config.size_aware {
        let mut hasher = FnvBuildHasher::default().build_hasher();
        req_id.hash(&mut hasher);
        // len will be [4 .. 2^16)
        (hasher.finish() as u16).max(4) as u32
    } else {
        0
    };
    (policy_weight, do_make_value(key))
}

fn do_make_value(key: usize) -> Arc<[u8]> {
    let mut value = vec![0; VALUE_LEN].into_boxed_slice();
    value[0] = (key % 256) as u8;
    value.into()
}

pub(crate) fn sleep_thread_for_insertion(config: &Config) {
    if let Some(delay) = config.insertion_delay {
        std::thread::sleep(delay);
    }
}

pub(crate) async fn sleep_task_for_insertion(config: &Config) {
    if let Some(delay) = config.insertion_delay {
        async_io::Timer::after(delay).await;
    }
}

const HASH_SEED_KEY: u64 = 982922761776577566;

#[derive(Clone, Default)]
pub(crate) struct BuildFnvHasher;

impl BuildHasher for BuildFnvHasher {
    type Hasher = FnvHasher;

    fn build_hasher(&self) -> Self::Hasher {
        FnvHasher::with_key(HASH_SEED_KEY)
    }
}

// https://rust-lang.github.io/rust-clippy/master/index.html#enum_variant_names
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum InitClosureType {
    GetOrInsert,
    GetOrTryInsertWithError1,
    GetOrTyyInsertWithError2,
}

impl InitClosureType {
    pub(crate) fn select(block: usize) -> Self {
        match block % 4 {
            0 => Self::GetOrTryInsertWithError1,
            1 => Self::GetOrTyyInsertWithError2,
            _ => Self::GetOrInsert,
        }
    }
}

#[derive(Debug, Error)]
#[error("init closure failed with error one")]
pub(crate) struct InitClosureError1;

#[derive(Debug, Error)]
#[error("init closure failed with error two")]
pub(crate) struct InitClosureError2;
