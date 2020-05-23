pub struct Report {
    pub read_count: usize,
    pub hit_count: usize,
}

impl Default for Report {
    fn default() -> Self {
        Self {
            read_count: 0,
            hit_count: 0,
        }
    }
}

impl Report {
    pub fn hit_ratio(&self) -> f64 {
        (self.hit_count as f64) / (self.read_count as f64)
    }
}
