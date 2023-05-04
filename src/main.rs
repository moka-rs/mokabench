use anyhow::Context;
use mokabench::{
    self,
    config::{Config, RemovalNotificationMode},
    Report, TraceFile,
};

use clap::{Arg, Command};

#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[cfg(target_env = "msvc")]
compile_error!("Sorry, Windows MSVC target is not supported because we cannot use jemalloc there");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(feature = "metrics")]
    {
        // Show tracing logs on the console so that we can see any errors in
        // metrics_exporter_dogstatsd crate.
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::INFO)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");

        // tracing::info!("tracing_subscriber::FmtSubscriber enabled");
    }

    let (trace_files, mut config) = create_config()?;

    for trace_file in trace_files {
        config.trace_file = trace_file;
        println!("{config:?}");
        println!();

        println!(
            "{}",
            Report::cvs_header(config.is_eviction_listener_enabled())
        );

        for capacity in config.trace_file.default_capacities() {
            run_with_capacity(&config, *capacity).await?
        }
    }

    Ok(())
}

async fn run_with_capacity(config: &Config, capacity: usize) -> anyhow::Result<()> {
    const DEFAULT_NUM_CLIENTS_ARRAY: &[u16] = &[16, 24, 32, 40, 48];

    let num_clients_slice: &[u16] = if let Some(n) = &config.num_clients {
        n
    } else {
        DEFAULT_NUM_CLIENTS_ARRAY
    };

    // Note that timing results for the unsync cache are not comparable with the rest
    // as it doesn't use the producer/consumer thread pattern as the other caches.

    #[cfg(any(feature = "mini-moka", feature = "moka-v08", feature = "moka-v09"))]
    if !config.insert_once && !config.is_eviction_listener_enabled() {
        let report = mokabench::run_single(config, capacity)?;
        println!("{}", report.to_csv_record());
    }

    #[cfg(feature = "hashlink")]
    if !config.insert_once
        && !config.size_aware
        && !config.invalidate_entries_if
        && !config.is_eviction_listener_enabled()
    {
        for num_clients in num_clients_slice {
            let report = mokabench::run_multi_threads_hashlink(config, capacity, *num_clients)?;
            println!("{}", report.to_csv_record());
        }
    }

    #[cfg(feature = "quick_cache")]
    if !config.insert_once
        && !config.size_aware
        && !config.invalidate_entries_if
        && !config.is_eviction_listener_enabled()
    {
        for num_clients in num_clients_slice {
            let report = mokabench::run_multi_threads_quick_cache(config, capacity, *num_clients)?;
            println!("{}", report.to_csv_record());
        }
    }

    #[cfg(feature = "stretto")]
    if !config.insert_once
        && !config.size_aware
        && !config.invalidate_entries_if
        && !config.is_eviction_listener_enabled()
    {
        for num_clients in num_clients_slice {
            let report = mokabench::run_multi_threads_stretto(config, capacity, *num_clients)?;
            println!("{}", report.to_csv_record());
        }
    }

    #[cfg(any(feature = "mini-moka", feature = "moka-v08", feature = "moka-v09"))]
    if !config.insert_once
        && !config.invalidate_entries_if
        && !config.is_eviction_listener_enabled()
    {
        for num_clients in num_clients_slice {
            let report = mokabench::run_multi_threads_moka_dash(config, capacity, *num_clients)?;
            println!("{}", report.to_csv_record());
        }
    }

    for num_clients in num_clients_slice {
        let report = mokabench::run_multi_threads_moka_sync(config, capacity, *num_clients).await?;
        println!("{}", report.to_csv_record());
    }

    for num_clients in num_clients_slice {
        if config.eviction_listener == RemovalNotificationMode::Immediate {
            eprintln!(
                "WARNING: eviction_listener = \"immediate\" is not supported by \
                    the async cache. \"queued\" mode will be used for it."
            );
        }
        let report = mokabench::run_multi_tasks_moka_async(config, capacity, *num_clients).await?;
        println!("{}", report.to_csv_record());
    }

    let num_segments = 8;

    for num_clients in num_clients_slice {
        let report =
            mokabench::run_multi_threads_moka_segment(config, capacity, *num_clients, num_segments)
                .await?;
        println!("{}", report.to_csv_record());
    }

    Ok(())
}

const OPTION_TRACE_FILE: &str = "trace-file";
const OPTION_TRACE_FILES: &str = "trace-files";
const OPTION_TTL: &str = "ttl";
const OPTION_TTI: &str = "tti";
const OPTION_NUM_CLIENTS: &str = "num-clients";
const OPTION_INSERT_ONCE: &str = "insert-once";
const OPTION_INSERTION_DELAY: &str = "insertion-delay";
const OPTION_INVALIDATE: &str = "invalidate";
const OPTION_INVALIDATE_ALL: &str = "invalidate-all";
const OPTION_INVALIDATE_IF: &str = "invalidate-entries-if";
const OPTION_ITERATE: &str = "iterate";
const OPTION_SIZE_AWARE: &str = "size-aware";
const OPTION_REPEAT: &str = "repeat";

// Since Moka v0.9.0
const OPTION_EVICTION_LISTENER: &str = "eviction-listener";

// Since Moka v0.10.0
const OPTION_ENTRY_API: &str = "entry-api";

// Since Moka v0.11.0
const OPTION_PER_KEY_EXPIRATION: &str = "per-key-expiration";

fn create_config() -> anyhow::Result<(Vec<TraceFile>, Config)> {
    let mut app = Command::new("Moka Bench")
        .arg(
            Arg::new(OPTION_TRACE_FILE)
                .alias(OPTION_TRACE_FILES)
                .short('f')
                .long(OPTION_TRACE_FILE)
                .help("The trace file (e.g. s3, ds1, oltp). default: s3")
                .default_value("s3")
                .default_missing_value("s3")
                .takes_value(true)
                .multiple_values(true)
                .use_value_delimiter(true),
        )
        .arg(
            Arg::new(OPTION_TTL)
                .long(OPTION_TTL)
                .help("Time-to-live in seconds")
                .takes_value(true),
        )
        .arg(
            Arg::new(OPTION_TTI)
                .long(OPTION_TTI)
                .help("Time-to-idle in seconds")
                .takes_value(true),
        )
        .arg(
            Arg::new(OPTION_NUM_CLIENTS)
                .short('n')
                .long(OPTION_NUM_CLIENTS)
                .takes_value(true)
                .multiple_values(true)
                .use_value_delimiter(true),
        )
        .arg(
            Arg::new(OPTION_REPEAT)
                .short('r')
                .long(OPTION_REPEAT)
                .takes_value(true),
        )
        .arg(
            Arg::new(OPTION_INSERTION_DELAY)
                .short('d')
                .long(OPTION_INSERTION_DELAY)
                .takes_value(true),
        )
        .arg(Arg::new(OPTION_INSERT_ONCE).long(OPTION_INSERT_ONCE))
        .arg(Arg::new(OPTION_INVALIDATE).long(OPTION_INVALIDATE))
        .arg(Arg::new(OPTION_INVALIDATE_ALL).long(OPTION_INVALIDATE_ALL))
        .arg(Arg::new(OPTION_INVALIDATE_IF).long(OPTION_INVALIDATE_IF))
        .arg(Arg::new(OPTION_ITERATE).long(OPTION_ITERATE))
        .arg(Arg::new(OPTION_SIZE_AWARE).long(OPTION_SIZE_AWARE));

    if cfg!(not(feature = "moka-v08")) {
        app = app.arg(
            Arg::new(OPTION_EVICTION_LISTENER)
                .long(OPTION_EVICTION_LISTENER)
                .takes_value(true)
                .use_value_delimiter(false),
        );
    }

    if cfg!(not(any(feature = "moka-v09", feature = "moka-v08"))) {
        app = app.arg(Arg::new(OPTION_ENTRY_API).long(OPTION_ENTRY_API));
    }

    if cfg!(not(any(
        feature = "moka-v010",
        feature = "moka-v09",
        feature = "moka-v08"
    ))) {
        app = app.arg(Arg::new(OPTION_PER_KEY_EXPIRATION).long(OPTION_PER_KEY_EXPIRATION));
    }

    let matches = app.get_matches();

    let trace_files = matches
        .values_of(OPTION_TRACE_FILE)
        .unwrap()
        .map(TraceFile::try_from)
        .collect::<Result<Vec<TraceFile>, _>>()?;

    let ttl_secs = match matches.value_of(OPTION_TTL) {
        None => None,
        Some(v) => Some(
            v.parse()
                .with_context(|| format!(r#"Cannot parse ttl "{v}" as a positive integer"#))?,
        ),
    };

    let tti_secs = match matches.value_of(OPTION_TTI) {
        None => None,
        Some(v) => Some(
            v.parse()
                .with_context(|| format!(r#"Cannot parse tti "{v}" as a positive integer"#))?,
        ),
    };

    let num_clients = match matches.values_of(OPTION_NUM_CLIENTS) {
        None => None,
        Some(v) => Some(
            v.map(|v| {
                v.parse().with_context(|| {
                    format!(r#"Cannot parse num_client "{v}" as a positive integer"#)
                })
            })
            .collect::<Result<Vec<u16>, _>>()?,
        ),
    };

    let repeat = match matches.value_of(OPTION_REPEAT) {
        None => None,
        Some(v) => Some(
            v.parse()
                .with_context(|| format!(r#"Cannot parse repeat "{v}" as a positive integer"#))?,
        ),
    };

    let insertion_delay_micros = match matches.value_of(OPTION_INSERTION_DELAY) {
        None => None,
        Some(v) => Some(v.parse().with_context(|| {
            format!(r#"Cannot parse insertion-delay "{v}" as a positive integer"#,)
        })?),
    };

    let insert_once = matches.is_present(OPTION_INSERT_ONCE);
    let invalidate = matches.is_present(OPTION_INVALIDATE);
    let invalidate_all = matches.is_present(OPTION_INVALIDATE_ALL);
    let invalidate_entries_if = matches.is_present(OPTION_INVALIDATE_IF);
    let iterate = matches.is_present(OPTION_ITERATE);
    let size_aware = matches.is_present(OPTION_SIZE_AWARE);

    // Since Moka v0.10
    let entry_api = matches.is_present(OPTION_ENTRY_API);

    // Since Moka v0.11
    let per_key_expiration = matches.is_present(OPTION_PER_KEY_EXPIRATION);

    let eviction_listener = if cfg!(not(feature = "moka-v08")) {
        if let Some(v) = matches.value_of(OPTION_EVICTION_LISTENER) {
            match v {
                "immediate" => RemovalNotificationMode::Immediate,
                "queued" => RemovalNotificationMode::Queued,
                _ => {
                    anyhow::bail!(
                        r#"eviction-listener must be "immediate" or "queued", but got "{}""#,
                        v
                    );
                }
            }
        } else {
            RemovalNotificationMode::None
        }
    } else {
        RemovalNotificationMode::None
    };

    if !entry_api && insert_once && cfg!(not(any(feature = "moka-v08", feature = "moka-v09"))) {
        eprintln!("\nWARNING: Testing Moka's entry API is disabled by default. Use --entry-api to enable it.\n");
    }

    let mut config = Config::new(
        trace_files[0],
        ttl_secs,
        tti_secs,
        num_clients,
        repeat,
        insertion_delay_micros,
    );
    config.set_insert_once(insert_once);
    config.set_invalidate(invalidate);
    config.set_invalidate_all(invalidate_all);
    config.set_invalidate_entries_if(invalidate_entries_if);
    config.set_iterate(iterate);
    config.set_eviction_listener(eviction_listener);
    config.set_size_aware(size_aware);
    config.set_entry_api(entry_api);
    config.set_per_key_expiration(per_key_expiration);

    Ok((trace_files, config))
}
