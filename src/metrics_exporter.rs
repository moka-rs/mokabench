#[cfg_attr(feature = "metrics", path = "metrics_exporter/statsd_exporter.rs")]
#[cfg_attr(
    not(feature = "metrics"),
    path = "metrics_exporter/disabled_exporter.rs"
)]
pub(crate) mod exporter;

pub(crate) use exporter::{init, shutdown};
