use std::sync::atomic::{AtomicU64, Ordering};

use moka09::notification::RemovalCause;

#[derive(Default)]
pub(crate) struct EvictionCounters {
    size: AtomicU64,
    expired: AtomicU64,
    explicit: AtomicU64,
    replaced: AtomicU64,
}

impl EvictionCounters {
    pub(crate) fn increment(&self, cause: RemovalCause) {
        match cause {
            RemovalCause::Size => self.size.fetch_add(1, Ordering::AcqRel),
            RemovalCause::Expired => self.expired.fetch_add(1, Ordering::AcqRel),
            RemovalCause::Explicit => self.explicit.fetch_add(1, Ordering::AcqRel),
            RemovalCause::Replaced => self.replaced.fetch_add(1, Ordering::AcqRel),
        };
    }

    pub(crate) fn size(&self) -> u64 {
        self.size.load(Ordering::Acquire)
    }

    pub(crate) fn expired(&self) -> u64 {
        self.expired.load(Ordering::Acquire)
    }

    pub(crate) fn explicit(&self) -> u64 {
        self.explicit.load(Ordering::Acquire)
    }

    pub(crate) fn replaced(&self) -> u64 {
        self.replaced.load(Ordering::Acquire)
    }

    pub(crate) fn csv_header() -> &'static str {
        "Size, Expired, Explicit, Replaced"
    }

    pub(crate) fn as_csv_line(&self) -> String {
        format!(
            "{}, {}, {}, {}",
            self.size(),
            self.expired(),
            self.explicit(),
            self.replaced()
        )
    }
}
