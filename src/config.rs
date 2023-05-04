use std::time::Duration;

use crate::trace_file::TraceFile;

#[derive(Clone, Debug)]
pub struct Config {
    pub trace_file: TraceFile,
    pub ttl: Option<Duration>,
    pub tti: Option<Duration>,
    pub num_clients: Option<Vec<u16>>,
    pub repeat: Option<u16>,
    pub insertion_delay: Option<Duration>,
    pub insert_once: bool,
    pub invalidate: bool,
    pub invalidate_all: bool,
    pub invalidate_entries_if: bool,
    pub iterate: bool,
    pub eviction_listener: RemovalNotificationMode,
    pub size_aware: bool,
    pub entry_api: bool,          // Since Moka v0.10
    pub per_key_expiration: bool, // Since Moka v0.11
}

impl Config {
    pub fn new(
        trace_file: TraceFile,
        ttl_secs: Option<u64>,
        tti_secs: Option<u64>,
        num_clients: Option<Vec<u16>>,
        repeat: Option<u16>,
        insertion_delay_micros: Option<u64>,
    ) -> Self {
        Self {
            trace_file,
            ttl: ttl_secs.map(Duration::from_secs),
            tti: tti_secs.map(Duration::from_secs),
            num_clients,
            repeat,
            insertion_delay: insertion_delay_micros.map(Duration::from_micros),
            insert_once: false,
            invalidate: false,
            invalidate_all: false,
            invalidate_entries_if: false,
            iterate: false,
            eviction_listener: RemovalNotificationMode::None,
            size_aware: false,
            entry_api: false,
            per_key_expiration: false,
        }
    }

    pub fn set_insert_once(&mut self, v: bool) {
        self.insert_once = v;
    }

    pub fn set_invalidate(&mut self, v: bool) {
        self.invalidate = v;
    }

    pub fn set_invalidate_all(&mut self, v: bool) {
        self.invalidate_all = v;
    }

    pub fn set_invalidate_entries_if(&mut self, v: bool) {
        self.invalidate_entries_if = v;
    }

    pub fn set_iterate(&mut self, v: bool) {
        self.iterate = v;
    }

    pub fn set_eviction_listener(&mut self, v: RemovalNotificationMode) {
        self.eviction_listener = v;
    }

    pub fn set_size_aware(&mut self, v: bool) {
        self.size_aware = v;
    }

    pub fn set_entry_api(&mut self, v: bool) {
        self.entry_api = v;
    }

    pub fn set_per_key_expiration(&mut self, v: bool) {
        self.per_key_expiration = v;
    }

    pub fn is_eviction_listener_enabled(&self) -> bool {
        self.eviction_listener != RemovalNotificationMode::None
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RemovalNotificationMode {
    None,
    Immediate,
    Queued,
}
