use crate::{cache_set::CacheSet, parser::ArcTraceEntry, report::Report, TTI_SECS, TTL_SECS};

use moka::sync::{Builder, ConcurrentCache, SegmentedCache};
use std::{collections::hash_map::RandomState, time::Duration};

pub struct SegmentedMoka(SegmentedCache<usize, Box<[u8]>, RandomState>);

impl Clone for SegmentedMoka {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl SegmentedMoka {
    pub fn new(capacity: usize, num_segments: usize) -> Self {
        let cache = Builder::new(capacity)
            .segments(num_segments)
            .time_to_live(Duration::from_secs(TTL_SECS))
            .time_to_idle(Duration::from_secs(TTI_SECS))
            .build();
        Self(cache)
    }

    fn get(&self, key: &usize) -> bool {
        self.0.get(key).is_some()
    }

    fn insert(&self, key: usize) {
        let value = vec![0; 512].into_boxed_slice();
        // std::thread::sleep(std::time::Duration::from_micros(500));
        self.0.insert(key, value);
    }
}

impl CacheSet<ArcTraceEntry> for SegmentedMoka {
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

pub struct SharedSegmentedMoka(SegmentedMoka);

impl SharedSegmentedMoka {
    pub fn new(capacity: usize, num_segments: usize) -> Self {
        Self(SegmentedMoka::new(capacity, num_segments))
    }
}

impl Clone for SharedSegmentedMoka {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl CacheSet<ArcTraceEntry> for SharedSegmentedMoka {
    fn process(&mut self, entry: &ArcTraceEntry, report: &mut Report) {
        self.0.process(entry, report)
    }
}
