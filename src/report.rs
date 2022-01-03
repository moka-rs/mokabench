use std::time::Duration;

#[derive(Clone)]
pub struct Report {
    pub name: String,
    pub capacity: usize,
    pub num_workers: Option<u16>,
    pub insert_count: usize,
    pub read_count: usize,
    pub hit_count: usize,
    pub duration: Option<Duration>,
}

impl Report {
    pub fn new(name: &str, capacity: usize, num_workers: Option<u16>) -> Self {
        Self {
            name: name.to_string(),
            capacity,
            num_workers,
            insert_count: 0,
            read_count: 0,
            hit_count: 0,
            duration: None,
        }
    }

    pub fn hit_ratio(&self) -> f64 {
        (self.hit_count as f64) / (self.read_count as f64)
    }

    pub fn merge(&mut self, other: &Self) {
        self.insert_count += other.insert_count;
        self.read_count += other.read_count;
        self.hit_count += other.hit_count;
    }

    // Formatting (CSV)

    pub fn cvs_header() -> String {
        "Cache, Max Capacity, Clients, Inserts, Reads, Hit Ratio, Duration Secs".into()
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
