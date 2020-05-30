use crate::{parser::ArcTraceEntry, report::Report};

use moka::{ConcurrentCache, LFUCache};
use std::collections::hash_map::RandomState;

pub trait CacheSet<T> {
    fn process(&mut self, entry: &T, report: &mut Report);
}

pub struct Moka(LFUCache<usize, Box<[u8]>, RandomState>);

impl Clone for Moka {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl Moka {
    pub fn new(capacity: usize) -> Self {
        Self(LFUCache::new(capacity))
    }

    fn get(&self, key: &usize) -> bool {
        self.0.get(key).is_some()
    }

    fn insert(&self, key: usize) {
        let value = vec![0; 512].into_boxed_slice();
        self.0.insert(key, value);
    }
}

impl CacheSet<ArcTraceEntry> for Moka {
    fn process(&mut self, entry: &ArcTraceEntry, report: &mut Report) {
        let mut read_count = 0;
        let mut hit_count = 0;
        let mut insert_count = 0;

        for block in entry.0.clone() {
            if self.get(&block) {
                hit_count += 1;
            } else {
                self.insert(block);
                insert_count += 1;
            }
            read_count += 1;
        }

        report.read_count += read_count;
        report.hit_count += hit_count;
        report.insert_count += insert_count;
    }
}

pub struct SharedMoka(Moka);

impl SharedMoka {
    pub fn new(capacity: usize) -> Self {
        Self(Moka::new(capacity))
    }
}

impl Clone for SharedMoka {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl CacheSet<ArcTraceEntry> for SharedMoka {
    fn process(&mut self, entry: &ArcTraceEntry, report: &mut Report) {
        self.0.process(entry, report)
    }
}
