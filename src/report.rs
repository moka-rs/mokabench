use std::time::Duration;

use crate::eviction_counters::EvictionCounters;

pub struct ReportBuilder {
    name: String,
    capacity: u64,
    num_workers: Option<u16>,
}

impl ReportBuilder {
    pub fn new(name: &str, capacity: u64, num_workers: Option<u16>) -> Self {
        Self {
            name: name.to_string(),
            capacity,
            num_workers,
        }
    }

    pub fn build(&self) -> Report {
        Report::new(&self.name, self.capacity, self.num_workers)
    }
}

#[derive(Clone, Default)]
pub struct Report {
    pub name: String,
    pub capacity: u64,
    pub num_workers: Option<u16>,
    pub has_eviction_counts: bool,
    pub insert_count: u64,
    pub read_count: u64,
    pub hit_count: u64,
    pub invalidation_count: u64,
    // Evicted by size constraint
    pub eviction_count: u64,
    pub expiration_count: u64,
    pub duration: Option<Duration>,
}

impl Report {
    pub fn new(name: &str, capacity: u64, num_workers: Option<u16>) -> Self {
        Self {
            name: name.to_string(),
            capacity,
            num_workers,
            ..Default::default()
        }
    }

    pub fn hit_ratio(&self) -> f64 {
        (self.hit_count as f64) / (self.read_count as f64)
    }

    pub fn merge(&mut self, other: &Self) {
        self.insert_count += other.insert_count;
        self.read_count += other.read_count;
        self.hit_count += other.hit_count;
        if self.has_eviction_counts {
            self.invalidation_count += other.invalidation_count;
            self.eviction_count += other.eviction_count;
            self.expiration_count += other.expiration_count;
        }
    }

    /// Returns a new report with the difference between the two reports.
    pub fn diff(&self, other: &Self) -> Self {
        let (l, s) = if self.insert_count > other.insert_count {
            (self, other)
        } else {
            (other, self)
        };

        let duration = match (l.duration, s.duration) {
            (Some(d_l), Some(d_s)) => Some(d_l - d_s),
            _ => None,
        };

        Self {
            insert_count: l.insert_count - s.insert_count,
            read_count: l.read_count - s.read_count,
            hit_count: l.hit_count - s.hit_count,
            invalidation_count: l.invalidation_count - s.invalidation_count,
            eviction_count: l.eviction_count - s.eviction_count,
            expiration_count: l.expiration_count - s.expiration_count,
            duration,
            ..self.clone()
        }
    }

    pub(crate) fn add_eviction_counts(&mut self, eviction_counters: &EvictionCounters) {
        self.has_eviction_counts = true;
        self.invalidation_count += eviction_counters.explicit();
        self.eviction_count += eviction_counters.size();
        self.expiration_count += eviction_counters.expired();
    }

    // Formatting (CSV)

    pub fn cvs_header(has_eviction_counters: bool) -> String {
        if has_eviction_counters {
            "Cache, Max Capacity, Clients, Inserts, Reads, Hit Ratio, Invalidates, Evicted by Size, Expired, Duration Secs".into()
        } else {
            "Cache, Max Capacity, Clients, Inserts, Reads, Hit Ratio, Duration Secs".into()
        }
    }

    pub fn to_csv_record(&self) -> String {
        let num_workers = if let Some(n) = self.num_workers {
            n.to_string()
        } else {
            "-".to_string()
        };

        let duration = if let Some(d) = self.duration {
            format!("{:.3}", d.as_secs_f64())
        } else {
            "-".to_string()
        };

        if self.has_eviction_counts {
            format!(
                "{}, {}, {}, {}, {}, {:.3}, {}, {}, {}, {}",
                self.name,
                self.capacity,
                num_workers,
                self.insert_count,
                self.read_count,
                self.hit_ratio() * 100.0,
                self.invalidation_count,
                self.eviction_count,
                self.expiration_count,
                duration
            )
        } else {
            format!(
                "{}, {}, {}, {}, {}, {:.3}, {}",
                self.name,
                self.capacity,
                num_workers,
                self.insert_count,
                self.read_count,
                self.hit_ratio() * 100.0,
                duration
            )
        }
    }
}
