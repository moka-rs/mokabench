use mokabench::{self, config::Config, Report};

use clap::{App, Arg};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = create_config()?;

    println!("{}", Report::cvs_header());

    const CAPACITIES: [usize; 2] = [100_000, 2_000_000];

    for capacity in &CAPACITIES {
        run_with_capacity(&config, *capacity).await?
    }

    Ok(())
}

fn create_config() -> anyhow::Result<Config> {
    let matches = App::new("Moka Bench")
        .arg(
            Arg::with_name("ttl")
                .long("ttl")
                .help("Time-to-live in seconds")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("tti")
                .long("tti")
                .help("Time-to-idle in seconds")
                .takes_value(true),
        )
        .arg(Arg::with_name("enable-insert-once").long("enable-insert-once"))
        .arg(Arg::with_name("enable-invalidate").long("enable-invalidate"))
        .arg(Arg::with_name("enable-invalidate-all").long("enable-invalidate-all"))
        .arg(Arg::with_name("enable-invalidate-entries-if").long("enable-invalidate-entries-if"))
        .get_matches();

    let ttl = match matches.value_of("ttl") {
        None => None,
        Some(v) => Some(
            v.parse()
                .map_err(|e| anyhow::anyhow!(r#"Cannot parse ttl "{}" as an integer: {}"#, v, e))?,
        ),
    };

    let tti = match matches.value_of("tti") {
        None => None,
        Some(v) => Some(
            v.parse()
                .map_err(|e| anyhow::anyhow!(r#"Cannot parse tti "{}" as an integer: {}"#, v, e))?,
        ),
    };

    let enable_insert_once = matches.is_present("enable-insert-once");
    let enable_invalidate = matches.is_present("enable-invalidate");
    let enable_invalidate_all = matches.is_present("enable-invalidate-all");
    let enable_invalidate_entries_if = matches.is_present("enable-invalidate-entries-if");

    Ok(Config::new(
        ttl,
        tti,
        enable_insert_once,
        enable_invalidate,
        enable_invalidate_all,
        enable_invalidate_entries_if,
    ))
}

async fn run_with_capacity(config: &Config, capacity: usize) -> anyhow::Result<()> {
    // const NUM_WORKERS_ARRAY: [u16; 6] = [1, 2, 4, 8, 16, 32];
    const NUM_WORKERS_ARRAY: [u16; 5] = [16, 24, 32, 40, 48];

    let report = mokabench::run_single(config, capacity)?;
    println!("{}", report.to_csv_record());

    for num_workers in &NUM_WORKERS_ARRAY {
        let report = mokabench::run_multi_threads(config, capacity, *num_workers)?;
        println!("{}", report.to_csv_record());
    }

    for num_workers in &NUM_WORKERS_ARRAY {
        let report = mokabench::run_multi_tasks(config, capacity, *num_workers).await?;
        println!("{}", report.to_csv_record());
    }

    let num_segments = 8;

    for num_workers in &NUM_WORKERS_ARRAY {
        let report =
            mokabench::run_multi_thread_segmented(config, capacity, *num_workers, num_segments)?;
        println!("{}", report.to_csv_record());
    }

    Ok(())
}
