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

fn main() -> anyhow::Result<()> {
    let (trace_files, mut config) = create_config()?;
    for trace_file in trace_files {
        config.trace_file = trace_file;
        println!("{:?}", config);
        println!();

        println!(
            "{}",
            Report::cvs_header(config.is_eviction_listener_enabled())
        );

        for capacity in config.trace_file.default_capacities() {
            run_with_capacity(&config, *capacity)?
        }
    }

    Ok(())
}

fn run_with_capacity(config: &Config, capacity: usize) -> anyhow::Result<()> {
    const DEFAULT_NUM_CLIENTS_ARRAY: &[u16] = &[16, 24, 32, 40, 48];

    let num_clients_slice: &[u16] = if let Some(n) = &config.num_clients {
        n
    } else {
        DEFAULT_NUM_CLIENTS_ARRAY
    };

    println!("allocation,started,0,0");

    // Note that timing results for the unsync cache are not comparable with the rest
    // as it doesn't use the producer/consumer thread pattern as the other caches.
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

    if capacity >= 2_000_000 {
        return Ok(());
    }

    for num_clients in num_clients_slice {
        let report = mokabench::run_multi_threads(config, capacity, *num_clients)?;
        println!("{}", report.to_csv_record());
    }

    for num_clients in num_clients_slice {
        if config.eviction_listener == RemovalNotificationMode::Immediate {
            eprintln!(
                "WARNING: eviction_listener = \"immediate\" is not supported by \
                    the async cache. \"queued\" mode will be used for it."
            );
        }
        let report = mokabench::run_multi_tasks(config, capacity, *num_clients)?;
        println!("{}", report.to_csv_record());
    }

    if !config.insert_once
        && !config.invalidate_entries_if
        && !config.is_eviction_listener_enabled()
    {
        for num_clients in num_clients_slice {
            let report = mokabench::run_multi_threads_dash_cache(config, capacity, *num_clients)?;
            println!("{}", report.to_csv_record());
        }
    }

    let num_segments = 8;

    for num_clients in num_clients_slice {
        let report =
            mokabench::run_multi_thread_segmented(config, capacity, *num_clients, num_segments)?;
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
const OPTION_REPEAT: &str = "repeat";
const OPTION_SIZE_AWARE: &str = "size-aware";

// Since Moka v0.9.0
const OPTION_EVICTION_LISTENER: &str = "eviction-listener";

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

    if cfg!(feature = "moka-v09") {
        app = app.arg(
            Arg::new(OPTION_EVICTION_LISTENER)
                .long(OPTION_EVICTION_LISTENER)
                .takes_value(true)
                .use_value_delimiter(false),
        );
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
                .with_context(|| format!(r#"Cannot parse ttl "{}" as a positive integer"#, v))?,
        ),
    };

    let tti_secs = match matches.value_of(OPTION_TTI) {
        None => None,
        Some(v) => Some(
            v.parse()
                .with_context(|| format!(r#"Cannot parse tti "{}" as a positive integer"#, v))?,
        ),
    };

    let num_clients = match matches.values_of(OPTION_NUM_CLIENTS) {
        None => None,
        Some(v) => Some(
            v.map(|v| {
                v.parse().with_context(|| {
                    format!(r#"Cannot parse num_client "{}" as a positive integer"#, v)
                })
            })
            .collect::<Result<Vec<u16>, _>>()?,
        ),
    };

    let repeat = match matches.value_of(OPTION_REPEAT) {
        None => None,
        Some(v) => Some(
            v.parse()
                .with_context(|| format!(r#"Cannot parse repeat "{}" as a positive integer"#, v))?,
        ),
    };

    let insertion_delay_micros = match matches.value_of(OPTION_INSERTION_DELAY) {
        None => None,
        Some(v) => Some(v.parse().with_context(|| {
            format!(
                r#"Cannot parse insertion-delay "{}" as a positive integer"#,
                v
            )
        })?),
    };

    let insert_once = matches.is_present(OPTION_INSERT_ONCE);
    let invalidate = matches.is_present(OPTION_INVALIDATE);
    let invalidate_all = matches.is_present(OPTION_INVALIDATE_ALL);
    let invalidate_entries_if = matches.is_present(OPTION_INVALIDATE_IF);
    let iterate = matches.is_present(OPTION_ITERATE);
    let size_aware = matches.is_present(OPTION_SIZE_AWARE);

    let eviction_listener = if cfg!(feature = "moka-v09") {
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

    let config = Config::new(
        trace_files[0],
        ttl_secs,
        tti_secs,
        num_clients,
        repeat,
        insertion_delay_micros,
        insert_once,
        invalidate,
        invalidate_all,
        invalidate_entries_if,
        iterate,
        eviction_listener,
        size_aware,
    );
    Ok((trace_files, config))
}
