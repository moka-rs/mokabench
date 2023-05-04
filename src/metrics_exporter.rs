use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, RwLock,
    },
    thread::JoinHandle,
    time::Duration,
};

use metrics::{counter, register_gauge, Gauge};
use once_cell::sync::Lazy;

#[cfg(feature = "metrics")]
use metrics_exporter_dogstatsd::StatsdBuilder;

use crate::Report;

static EPOCH: AtomicU64 = AtomicU64::new(0);

pub(crate) async fn init(endpoint: &str) {
    let mut shared = MetricsExporter::shared().write().unwrap();
    if shared.is_initialized {
        return;
    }

    #[cfg(feature = "metrics")]
    shared.do_init(endpoint).await;

    shared.is_initialized = true;
}

pub(crate) fn shutdown() {
    // TODO: FIXME: "this `MutexGuard` is held across an `await` point"
    // https://rust-lang.github.io/rust-clippy/master/index.html#await_holding_lock
    let mut shared = MetricsExporter::shared().write().unwrap();

    #[cfg(feature = "metrics")]
    shared.do_shutdown();

    shared.is_initialized = false;
}

pub(crate) fn report_stats(metrics_names: &MetricsNames, diff: &Report) {
    #[cfg(feature = "metrics")]
    {
        let shared = MetricsExporter::shared().read().unwrap();
        shared.do_report_stats(metrics_names, diff);
    }
}

pub(crate) fn current_epoch() -> u64 {
    EPOCH.load(Ordering::Relaxed)
}

static SHARED: Lazy<RwLock<MetricsExporter>> = Lazy::new(|| RwLock::new(MetricsExporter::new()));

struct MetricsExporter {
    is_initialized: bool,
    is_metrics_installed: bool,
    reporter: Option<Arc<AllocationReporter>>,
    reporter_thread: Option<JoinHandle<()>>,
}

impl MetricsExporter {
    fn new() -> Self {
        Self {
            is_initialized: false,
            is_metrics_installed: false,
            reporter: None,
            reporter_thread: None,
        }
    }

    fn shared() -> &'static RwLock<MetricsExporter> {
        &SHARED
    }
}

#[cfg(feature = "metrics")]
impl MetricsExporter {
    async fn do_init(&mut self, endpoint: &str) {
        if !self.is_metrics_installed {
            StatsdBuilder::new()
                // Cannot send to IPV4 address on macOS if it is bound to an IPV6 address.
                // https://users.rust-lang.org/t/udpsocket-connect-fails-on-unspecified-host/69100
                // https://github.com/dialtone/metrics-exporter-dogstatsd/pull/3
                .with_push_gateway(endpoint, Duration::from_millis(498))
                .expect("Failed to set the push gateway to statsd exporter")
                .set_global_prefix("mokabench")
                .install()
                .expect("Failed to install statsd exporter");

            self.is_metrics_installed = true;
        }

        let alloc_reporter = Arc::new(AllocationReporter::default());
        let alloc_reporter2 = Arc::clone(&alloc_reporter);

        // TODO: Give a name to the thread.
        let t = std::thread::spawn(move || {
            alloc_reporter2.run();
        });

        self.reporter = Some(alloc_reporter);
        self.reporter_thread = Some(t);
    }

    fn do_shutdown(&mut self) {
        match (self.reporter.take(), self.reporter_thread.take()) {
            (Some(r), Some(h)) => {
                r.shutdown();
                h.join().unwrap();
            }
            (None, None) => {}
            _ => unreachable!(),
        }
    }

    fn do_report_stats(&self, metrics_names: &MetricsNames, diff: &Report) {
        counter!(metrics_names.insert_count, diff.insert_count);
        counter!(metrics_names.read_count, diff.read_count);
        counter!(metrics_names.hit_count, diff.hit_count);
        counter!(metrics_names.invalidation_count, diff.invalidation_count);
    }
}

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
