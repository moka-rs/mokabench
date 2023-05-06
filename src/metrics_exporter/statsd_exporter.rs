use super::MetricsNames;
use crate::Report;

use std::{
    fmt,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::Duration,
};

use metrics::{counter, register_gauge, Gauge};
use metrics_exporter_dogstatsd::StatsdBuilder;
use once_cell::sync::OnceCell;

static EPOCH: AtomicU64 = AtomicU64::new(0);

pub(crate) async fn init(endpoint: &str) {
    if SHARED.get().is_none() {
        let mut exporter = MetricsExporter::default();
        exporter.init_metrics_exporter(endpoint).await;

        SHARED
            .set(exporter)
            .expect("Failed to initialize MetricsExporter. {e}");
    }

    SHARED.get().unwrap().init();
}

pub(crate) fn shutdown() {
    MetricsExporter::shared().shutdown();
}

pub(crate) fn report_stats(metrics_names: &MetricsNames, diff: &Report) {
    counter!(metrics_names.insert_count, diff.insert_count);
    counter!(metrics_names.read_count, diff.read_count);
    counter!(metrics_names.hit_count, diff.hit_count);
    counter!(metrics_names.invalidation_count, diff.invalidation_count);
}

// The epoch will be advanced periodically in the `run` method of the
// `AllocationReporter`.
pub(crate) fn current_epoch() -> u64 {
    EPOCH.load(Ordering::Relaxed)
}

static SHARED: OnceCell<MetricsExporter> = OnceCell::new();

#[derive(Default)]
struct MetricsExporter {
    reporter: Mutex<Option<(Arc<AllocationReporter>, JoinHandle<()>)>>,
}

impl fmt::Debug for MetricsExporter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MetricsExporter").finish()
    }
}

impl MetricsExporter {
    fn shared() -> &'static MetricsExporter {
        SHARED.get().expect("MetricsExporter is not initialized")
    }
}

#[cfg(feature = "metrics")]
impl MetricsExporter {
    async fn init_metrics_exporter(&mut self, endpoint: &str) {
        StatsdBuilder::new()
            // Cannot send to IPV4 address on macOS if it is bound to an IPV6 address.
            // https://users.rust-lang.org/t/udpsocket-connect-fails-on-unspecified-host/69100
            // https://github.com/dialtone/metrics-exporter-dogstatsd/pull/3
            .with_push_gateway(endpoint, Duration::from_millis(498))
            .expect("Failed to set the push gateway to statsd exporter")
            .set_global_prefix("mokabench")
            .install()
            .expect("Failed to install statsd exporter");
    }

    fn init(&self) {
        let mut reporter = self.reporter.lock().unwrap();
        if reporter.is_some() {
            return;
        }

        let ar = Arc::new(AllocationReporter::default());

        // TODO: Give a name to the thread.
        let t = {
            let ar2 = Arc::clone(&ar);
            std::thread::spawn(move || ar2.run())
        };

        *reporter = Some((ar, t));
    }

    fn shutdown(&self) {
        if let Some((r, h)) = self.reporter.lock().unwrap().take() {
            r.shutdown();
            h.join().unwrap();
        }
    }
}

// #[cfg()]
#[derive(Default)]
struct AllocationReporter {
    is_shutting_down: AtomicBool,
}

impl AllocationReporter {
    fn run(&self) {
        let resident_gauge = register_gauge!("memory.resident_mb");
        let allocated_gauge = register_gauge!("memory.allocated_mb");

        loop {
            if self.is_shutting_down.load(Ordering::Acquire) {
                break;
            }
            EPOCH.fetch_add(1, Ordering::AcqRel);
            Self::report_allocation_info(&resident_gauge, &allocated_gauge);
            std::thread::sleep(Duration::from_millis(98));
        }

        Self::run_deferred();
        Self::report_allocation_info(&resident_gauge, &allocated_gauge);
    }

    fn shutdown(&self) {
        self.is_shutting_down.store(true, Ordering::Release);
    }

    fn report_allocation_info(resident_gauge: &Gauge, allocated_gauge: &Gauge) {
        use tikv_jemalloc_ctl::{epoch, stats};

        let e = epoch::mib().unwrap();
        e.advance().unwrap();
        let resident = stats::resident::read().unwrap();
        let allocated = stats::allocated::read().unwrap();
        let resident_mb = resident as f64 / 1024.0 / 1024.0;
        let allocated_mb = allocated as f64 / 1024.0 / 1024.0;

        // println!("allocation,{:.4},{:.4}", resident_mb, allocated_mb);
        resident_gauge.set(resident_mb);
        allocated_gauge.set(allocated_mb);
    }

    /// Runs deferred destructors in crossbeam-epoch and prints the current allocation
    /// info.
    fn run_deferred() {
        use tikv_jemalloc_ctl::{epoch, stats};

        let mut allocated = std::usize::MAX;
        let mut unchanged_count = 0usize;
        loop {
            crossbeam_epoch::pin().flush();

            let e = epoch::mib().unwrap();
            e.advance().unwrap();
            let new_allocated = stats::allocated::read().unwrap();

            if new_allocated == allocated {
                unchanged_count += 1;
                if unchanged_count > 50 {
                    break;
                }
            } else {
                allocated = new_allocated;
                unchanged_count = 0;
            }

            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
}
