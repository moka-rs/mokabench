use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Config {
    pub ttl: Option<Duration>,
    pub tti: Option<Duration>,
    pub enable_insert_once: bool,
    pub enable_invalidate: bool,
    pub enable_invalidate_all: bool,
    pub enable_invalidate_entries_if: bool,
}

impl Config {
    pub fn new(
        ttl_secs: Option<u64>,
        tti_secs: Option<u64>,
        enable_insert_once: bool,
        enable_invalidate: bool,
        enable_invalidate_all: bool,
        enable_invalidate_entries_if: bool,
    ) -> Self {
        Self {
            ttl: ttl_secs.map(Duration::from_secs),
            tti: tti_secs.map(Duration::from_secs),
            enable_insert_once,
            enable_invalidate,
            enable_invalidate_all,
            enable_invalidate_entries_if,
        }
    }
}
