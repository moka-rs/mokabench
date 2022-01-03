use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Config {
    pub ttl: Option<Duration>,
    pub tti: Option<Duration>,
    pub num_clients: Option<u16>,
    pub insertion_delay: Option<Duration>,
    pub insert_once: bool,
    pub invalidate: bool,
    pub invalidate_all: bool,
    pub invalidate_entries_if: bool,
    pub size_aware: bool,
}

// https://rust-lang.github.io/rust-clippy/master/index.html#too_many_arguments
#[allow(clippy::too_many_arguments)]
impl Config {
    pub fn new(
        ttl_secs: Option<u64>,
        tti_secs: Option<u64>,
        num_clients: Option<u16>,
        insertion_delay_micros: Option<u64>,
        insert_once: bool,
        invalidate: bool,
        invalidate_all: bool,
        invalidate_entries_if: bool,
        size_aware: bool,
    ) -> Self {
        Self {
            ttl: ttl_secs.map(Duration::from_secs),
            tti: tti_secs.map(Duration::from_secs),
            num_clients,
            insertion_delay: insertion_delay_micros.map(Duration::from_micros),
            insert_once,
            invalidate,
            invalidate_all,
            invalidate_entries_if,
            size_aware,
        }
    }
}
