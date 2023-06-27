#[cfg_attr(feature = "metrics", path = "metrics_exporter/statsd_exporter.rs")]
#[cfg_attr(
    not(feature = "metrics"),
    path = "metrics_exporter/disabled_exporter.rs"
)]
pub(crate) mod exporter;

pub(crate) use exporter::{init, shutdown};

#[cfg(feature = "metrics")]
pub(crate) use exporter::{current_epoch, report_stats};

#[cfg_attr(not(feature = "metrics"), allow(dead_code))]
#[derive(Clone, Copy, Debug)]
pub(crate) struct MetricsNames {
    pub(crate) insert_count: &'static str,
    pub(crate) read_count: &'static str,
    pub(crate) hit_count: &'static str,
    pub(crate) invalidation_count: &'static str,
}

pub(crate) static METRICS_NAMES_MOKA_SYNC_CACHE: MetricsNames = MetricsNames {
    insert_count: "moka.sync.insert_count",
    read_count: "moka.sync.read_count",
    hit_count: "moka.sync.hit_count",
    invalidation_count: "moka.sync.invalidation_count",
};

pub(crate) static METRICS_NAMES_MOKA_SYNC_SEG_CACHE: MetricsNames = MetricsNames {
    insert_count: "moka.sync_seg.insert_count",
    read_count: "moka.sync_seg.read_count",
    hit_count: "moka.sync_seg.hit_count",
    invalidation_count: "moka.sync_seg.invalidation_count",
};

pub(crate) static METRICS_NAMES_MOKA_ASYNC_CACHE: MetricsNames = MetricsNames {
    insert_count: "moka.async.insert_count",
    read_count: "moka.async.read_count",
    hit_count: "moka.async.hit_count",
    invalidation_count: "moka.async.invalidation_count",
};
