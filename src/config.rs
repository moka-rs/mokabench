use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Config {
    pub ttl: Option<Duration>,
    pub tti: Option<Duration>,
    pub enable_invalidate: bool,
    pub enable_invalidate_all: bool,
}

impl Config {
    pub fn new(
        ttl_secs: Option<u64>,
        tti_secs: Option<u64>,
        enable_invalidate: bool,
        enable_invalidate_all: bool,
    ) -> Self {
        Self {
            ttl: ttl_secs.map(Duration::from_secs),
            tti: tti_secs.map(Duration::from_secs),
            enable_invalidate,
            enable_invalidate_all,
        }
    }
}
