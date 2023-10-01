use async_trait::async_trait;
use thiserror::Error;

use crate::{parser::TraceEntry, Report};

#[cfg_attr(feature = "moka-v012", path = "moka_driver_v2/async_cache.rs")]
#[cfg_attr(
    any(
        feature = "moka-v08",
        feature = "moka-v09",
        feature = "moka-v010",
        feature = "moka-v011"
    ),
    path = "moka_driver_v1/async_cache.rs"
)]
pub(crate) mod async_cache;

#[cfg_attr(feature = "moka-v012", path = "moka_driver_v2/sync_cache.rs")]
#[cfg_attr(
    any(
        feature = "moka-v08",
        feature = "moka-v09",
        feature = "moka-v010",
        feature = "moka-v011"
    ),
    path = "moka_driver_v1/sync_cache.rs"
)]
pub(crate) mod sync_cache;

#[cfg_attr(feature = "moka-v012", path = "moka_driver_v2/sync_segmented.rs")]
#[cfg_attr(
    any(
        feature = "moka-v08",
        feature = "moka-v09",
        feature = "moka-v010",
        feature = "moka-v011"
    ),
    path = "moka_driver_v1/sync_segmented.rs"
)]
pub(crate) mod sync_segmented;

pub(crate) trait GetOrInsertOnce {
    fn get_or_insert_once(&self, entry: &TraceEntry, report: &mut Report);
}

#[async_trait]
trait AsyncGetOrInsertOnce {
    async fn get_or_insert_once(&self, entry: &TraceEntry, report: &mut Report);
}

// https://rust-lang.github.io/rust-clippy/master/index.html#enum_variant_names
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy)]
enum InitClosureType {
    GetOrInsert,
    GetOrTryInsertWithError1,
    GetOrTyyInsertWithError2,
}

impl InitClosureType {
    fn select(block: usize) -> Self {
        match block % 4 {
            0 => Self::GetOrTryInsertWithError1,
            1 => Self::GetOrTyyInsertWithError2,
            _ => Self::GetOrInsert,
        }
    }
}

#[derive(Debug, Error)]
#[error("init closure failed with error one")]
struct InitClosureError1;

#[derive(Debug, Error)]
#[error("init closure failed with error two")]
struct InitClosureError2;

#[cfg(not(any(feature = "moka-v08", feature = "moka-v09", feature = "moka-v010")))]
/// The `expiry` module provides `MokabenchExpiry`, our implementation of
/// `moka::Expiry` trait, to support per-entry expiration.
pub(crate) mod expiry {
    use std::time::{Duration, Instant};

    /// Implements `moka::Expiry` trait to support per-entry expiration. Our
    /// implementation simply emulates the cache level TTL and TTI.
    pub(crate) struct MokabenchExpiry {
        ttl: Option<Duration>,
        tti: Option<Duration>,
    }

    impl MokabenchExpiry {
        pub(crate) fn new(ttl: Option<Duration>, tti: Option<Duration>) -> Self {
            Self { ttl, tti }
        }
    }

    impl<K, V> crate::moka::Expiry<K, V> for MokabenchExpiry {
        fn expire_after_create(
            &self,
            _key: &K,
            _value: &V,
            _current_time: Instant,
        ) -> Option<Duration> {
            match (self.tti, self.ttl) {
                (None, None) => None,
                (tti @ Some(_), None) => tti,
                (None, ttl @ Some(_)) => ttl,
                (Some(tti), Some(ttl)) => Some(tti.min(ttl)),
            }
        }

        fn expire_after_read(
            &self,
            _key: &K,
            _value: &V,
            current_time: Instant,
            current_duration: Option<Duration>,
            last_modified_at: Instant,
        ) -> Option<Duration> {
            match (self.tti, self.ttl) {
                // We do not have TTI. Do not modify the current duration.
                (None, _) => current_duration,
                // We only have TTI. Return the TTI.
                (tti @ Some(_), None) => tti,
                // We have both TTI and TTL. Return the minimum of the TTI and the
                // remaining TTL.
                (Some(tti), Some(ttl)) => {
                    let duration_since_last_modified = current_time
                        .checked_duration_since(last_modified_at)
                        .expect("last_modified_at > current_time");
                    let remaining_ttl = ttl
                        .checked_sub(duration_since_last_modified)
                        .unwrap_or_default();
                    Some(tti.min(remaining_ttl))
                }
            }
        }

        fn expire_after_update(
            &self,
            _key: &K,
            _value: &V,
            _current_time: Instant,
            current_duration: Option<Duration>,
        ) -> Option<Duration> {
            if self.ttl.is_some() {
                self.ttl
            } else {
                current_duration
            }
        }
    }
}
