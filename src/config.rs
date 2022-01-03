use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Config {
    pub ttl: Option<Duration>,
    pub tti: Option<Duration>,
    pub num_clients: Option<u16>,
    pub insertion_delay: Option<Duration>,
    pub enable_insert_once: bool,
    pub enable_invalidate: bool,
    pub enable_invalidate_all: bool,
    pub enable_invalidate_entries_if: bool,
}

// https://rust-lang.github.io/rust-clippy/master/index.html#too_many_arguments
#[allow(clippy::too_many_arguments)]
impl Config {
    pub fn new(
        ttl_secs: Option<u64>,
        tti_secs: Option<u64>,
        num_clients: Option<u16>,
        insertion_delay_micros: Option<u64>,
        enable_insert_once: bool,
        enable_invalidate: bool,
        enable_invalidate_all: bool,
        enable_invalidate_entries_if: bool,
    ) -> Self {
        Self {
            ttl: ttl_secs.map(Duration::from_secs),
            tti: tti_secs.map(Duration::from_secs),
            num_clients,
            insertion_delay: insertion_delay_micros.map(Duration::from_micros),
            enable_insert_once,
            enable_invalidate,
            enable_invalidate_all,
            enable_invalidate_entries_if,
        }
    }
}
