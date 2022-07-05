use anyhow::Context;
use mokabench::{self, config::Config, Report, TraceFile};

use clap::{Arg, Command};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = create_config()?;
    println!("{:?}", config);
    println!();

    println!("{}", Report::cvs_header());

    for capacity in config.trace_file.default_capacities() {
        run_with_capacity(&config, *capacity).await?
    }

    Ok(())
}

async fn run_with_capacity(config: &Config, capacity: usize) -> anyhow::Result<()> {
    const NUM_CLIENTS_ARRAY: &[u16] = &[16, 24, 32, 40, 48];

    let num_clients_opt = config.num_clients.map(|n| [n; 1]);
    let num_clients_slice: &[u16] = if let Some(n) = &num_clients_opt {
        n
    } else {
        NUM_CLIENTS_ARRAY
    };

    if !config.insert_once {
        let report = mokabench::run_single(config, capacity)?;
        println!("{}", report.to_csv_record());
    }

    if capacity >= 2_000_000 {
        return Ok(());
    }

    for num_clients in num_clients_slice {
        let report = mokabench::run_multi_threads(config, capacity, *num_clients)?;
        println!("{}", report.to_csv_record());
    }

    for num_clients in num_clients_slice {
        let report = mokabench::run_multi_tasks(config, capacity, *num_clients).await?;
        println!("{}", report.to_csv_record());
    }

    if !config.insert_once && !config.invalidate_entries_if {
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

fn create_config() -> anyhow::Result<Config> {
    let mut app = Command::new("Moka Bench")
        .arg(
            Arg::new(OPTION_TRACE_FILE)
                .short('f')
                .long(OPTION_TRACE_FILE)
                .help("The trace file (s3, ds1 or oltp). default: s3")
                .takes_value(true)
                .use_value_delimiter(false),
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
                .use_value_delimiter(false),
        )
        .arg(
            Arg::new(OPTION_REPEAT)
                .short('r')
                .long(OPTION_REPEAT)
                .takes_value(true)
                .use_value_delimiter(false),
        )
        .arg(
            Arg::new(OPTION_INSERTION_DELAY)
                .short('d')
                .long(OPTION_INSERTION_DELAY)
                .takes_value(true)
                .use_value_delimiter(false),
        )
        .arg(Arg::new(OPTION_INSERT_ONCE).long(OPTION_INSERT_ONCE))
        .arg(Arg::new(OPTION_INVALIDATE).long(OPTION_INVALIDATE))
        .arg(Arg::new(OPTION_INVALIDATE_ALL).long(OPTION_INVALIDATE_ALL))
        .arg(Arg::new(OPTION_INVALIDATE_IF).long(OPTION_INVALIDATE_IF))
        .arg(Arg::new(OPTION_ITERATE).long(OPTION_ITERATE))
        .arg(Arg::new(OPTION_SIZE_AWARE).long(OPTION_SIZE_AWARE));

    if cfg!(feature = "moka-v09") {
        app = app.arg(Arg::new(OPTION_EVICTION_LISTENER).long(OPTION_EVICTION_LISTENER));
    }

    let matches = app.get_matches();

    let trace_file = matches.value_of(OPTION_TRACE_FILE).unwrap_or("s3");
    let trace_file = TraceFile::try_from(trace_file)?;

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

    let num_clients = match matches.value_of(OPTION_NUM_CLIENTS) {
        None => None,
        Some(v) => Some(v.parse().with_context(|| {
            format!(r#"Cannot parse num_client "{}" as a positive integer"#, v)
        })?),
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

    let eviction_listener =
        cfg!(feature = "moka-v09") && matches.is_present(OPTION_EVICTION_LISTENER);

    Ok(Config::new(
        trace_file,
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
    ))
}
