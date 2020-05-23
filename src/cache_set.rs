use crate::{parser::ArcTraceEntry, report::Report};

use moka::{ConcurrentCache, LFUCache};
use std::collections::hash_map::RandomState;

pub trait CacheSet<T> {
    fn process(&mut self, entry: &T);
    fn report(&self) -> &Report;
}

pub struct Moka {
    moka: LFUCache<usize, Box<[u8]>, RandomState>,
    report: Report,
}

impl Moka {
    pub fn new(capacity: usize) -> Self {
        println!(
            "Created a Moka LFUCache with the max capacity {}.",
            capacity
        );
        Self {
            moka: LFUCache::new(capacity),
            report: Report::default(),
        }
    }
}

impl CacheSet<ArcTraceEntry> for Moka {
    fn process(&mut self, entry: &ArcTraceEntry) {
        let mut read_count = 0;
        let mut hit_count = 0;

        for block in entry.0.clone() {
            if self.get(&block) {
                hit_count += 1;
            } else {
                self.insert(block);
            }
            read_count += 1;
        }

        self.report.read_count += read_count;
        self.report.hit_count += hit_count;
    }

    fn report(&self) -> &Report {
        &self.report
    }
}

impl Moka {
    fn get(&mut self, key: &usize) -> bool {
        self.moka.get(key).is_some()
    }

    fn insert(&mut self, key: usize) {
        let value = vec![0; 512].into_boxed_slice();
        self.moka.insert(key, value);
    }
}
