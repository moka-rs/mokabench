#[cfg(all(feature = "moka-v09", feature = "moka-v08"))]
compile_error!(
    "You cannot specify both `moka-v09` and `moka-v8` features at the same time.\n\
                You might need `--no-default-features` too."
);

#[cfg(feature = "moka-v09")]
pub(crate) use moka09 as moka;

#[cfg(all(feature = "moka-v08", not(feature = "moka-v09")))]
pub(crate) use moka08 as moka;

use config::Config;
use crossbeam_channel::Receiver;
use itertools::Itertools;
use std::io::prelude::*;
use std::{fs::File, io::BufReader, time::Instant};

mod cache;
pub mod config;
mod parser;
mod report;
mod trace_file;

use cache::{
    async_cache::SharedAsyncCache, sync_cache::SharedSyncCache,
    sync_segmented::SharedSegmentedMoka, unsync_cache::UnsyncCache, AsyncCacheSet, CacheSet,
};
use parser::{TraceEntry, TraceParser};

pub use report::Report;
pub use trace_file::TraceFile;

use crate::cache::dash_cache::SharedDashCache;
#[cfg(feature = "hashlink")]
use crate::cache::hashlink::HashLink;
#[cfg(feature = "quick_cache")]
use crate::cache::quick_cache::QuickCache;
#[cfg(feature = "stretto")]
use crate::cache::stretto::StrettoCache;

#[cfg(feature = "moka-v09")]
mod eviction_counters;

#[cfg(feature = "moka-v09")]
pub(crate) use eviction_counters::EvictionCounters;

pub(crate) enum Op {
    GetOrInsert(String, usize),
    GetOrInsertOnce(String, usize),
    Update(String, usize),
    Invalidate(String, usize),
    InvalidateAll,
    InvalidateEntriesIf(String, usize),
    Iterate,
}

pub fn run_single(config: &Config, capacity: usize) -> anyhow::Result<Report> {
    let mut max_cap = capacity.try_into().unwrap();
    if config.size_aware {
        max_cap *= 2u64.pow(15);
    }
    let mut cache_set = UnsyncCache::new(config, max_cap, capacity);
    let mut report = Report::new("Moka Unsync Cache", max_cap, Some(1));
    let mut counter = 0;

    let instant = Instant::now();

    for _ in 0..(config.repeat.unwrap_or(1)) {
        let f = File::open(config.trace_file.path())?;
        let reader = BufReader::new(f);
        let mut parser = parser::GenericTraceParser;
        for line in reader.lines() {
            let entry = if let Some(entry) = parser.parse(&line?, counter)? {
                entry
            } else {
                continue;
            };
            counter += 1;
            if config.invalidate_all && counter % 100_000 == 0 {
                cache_set.invalidate_all();
                cache_set.get_or_insert(&entry, &mut report);
            } else if config.size_aware && counter % 11 == 0 {
                cache_set.update(&entry, &mut report);
            } else if config.invalidate && counter % 8 == 0 {
                cache_set.invalidate(&entry);
            } else if config.insert_once && counter % 3 == 0 {
                cache_set.get_or_insert_once(&entry, &mut report);
            } else {
                cache_set.get_or_insert(&entry, &mut report);
            }

            if config.iterate && counter % 50_000 == 0 {
                cache_set.iterate();
            }
        }
    }

    let elapsed = instant.elapsed();
    report.duration = Some(elapsed);

    Ok(report)
}

fn generate_commands<I>(
    config: &Config,
    max_chunk_size: usize,
    counter: &mut usize,
    chunk: I,
) -> anyhow::Result<Vec<Op>>
where
    I: Iterator<Item = std::io::Result<String>>,
{
    let mut ops = Vec::with_capacity(max_chunk_size);
    for line in chunk {
        let line = line?;
        *counter += 1;
        if config.invalidate_all && *counter % 100_000 == 0 {
            ops.push(Op::InvalidateAll);
            ops.push(Op::GetOrInsert(line, *counter));
        } else if config.invalidate_entries_if && *counter % 5_000 == 0 {
            ops.push(Op::InvalidateEntriesIf(line, *counter));
        } else if config.size_aware && *counter % 11 == 0 {
            ops.push(Op::Update(line, *counter));
        } else if config.invalidate && *counter % 8 == 0 {
            ops.push(Op::Invalidate(line, *counter));
        } else if config.insert_once && *counter % 3 == 0 {
            ops.push(Op::GetOrInsertOnce(line, *counter));
        } else {
            ops.push(Op::GetOrInsert(line, *counter));
        }

        if config.iterate && *counter % 50_000 == 0 {
            ops.push(Op::Iterate);
        }
    }
    Ok(ops)
}

fn process_commands(
    receiver: Receiver<Vec<Op>>,
    cache: &mut impl CacheSet<TraceEntry>,
    report: &mut Report,
) {
    let mut parser = parser::GenericTraceParser;
    while let Ok(ops) = receiver.recv() {
        for op in ops {
            match op {
                Op::GetOrInsert(line, line_number) => {
                    if let Some(entry) = parser.parse(&line, line_number).unwrap() {
                        cache.get_or_insert(&entry, report);
                    }
                }
                Op::GetOrInsertOnce(line, line_number) => {
                    if let Some(entry) = parser.parse(&line, line_number).unwrap() {
                        cache.get_or_insert_once(&entry, report);
                    }
                }
                Op::Update(line, line_number) => {
                    if let Some(entry) = parser.parse(&line, line_number).unwrap() {
                        cache.update(&entry, report);
                    }
                }
                Op::Invalidate(line, line_number) => {
                    if let Some(entry) = parser.parse(&line, line_number).unwrap() {
                        cache.invalidate(&entry);
                    }
                }
                Op::InvalidateAll => cache.invalidate_all(),
                Op::InvalidateEntriesIf(line, line_number) => {
                    if let Some(entry) = parser.parse(&line, line_number).unwrap() {
                        cache.invalidate_entries_if(&entry);
                    }
                }
                Op::Iterate => cache.iterate(),
            }
        }
    }
}

async fn process_commands_async(
    receiver: Receiver<Vec<Op>>,
    cache: &mut impl AsyncCacheSet<TraceEntry>,
    report: &mut Report,
) {
    let mut parser = parser::GenericTraceParser;
    while let Ok(ops) = receiver.recv() {
        for op in ops {
            match op {
                Op::GetOrInsert(line, line_number) => {
                    if let Some(entry) = parser.parse(&line, line_number).unwrap() {
                        cache.get_or_insert(&entry, report).await;
                    }
                }
                Op::GetOrInsertOnce(line, line_number) => {
                    if let Some(entry) = parser.parse(&line, line_number).unwrap() {
                        cache.get_or_insert_once(&entry, report).await;
                    }
                }
                Op::Update(line, line_number) => {
                    if let Some(entry) = parser.parse(&line, line_number).unwrap() {
                        cache.update(&entry, report).await;
                    }
                }
                Op::Invalidate(line, line_number) => {
                    if let Some(entry) = parser.parse(&line, line_number).unwrap() {
                        cache.invalidate(&entry).await;
                    }
                }
                Op::InvalidateAll => cache.invalidate_all(),
                Op::InvalidateEntriesIf(line, line_number) => {
                    if let Some(entry) = parser.parse(&line, line_number).unwrap() {
                        cache.invalidate_entries_if(&entry);
                    }
                }
                Op::Iterate => cache.iterate().await,
            }
        }
    }
}

#[allow(clippy::needless_collect)] // on the `handles` variable.
pub fn run_multi_threads(
    config: &Config,
    capacity: usize,
    num_clients: u16,
) -> anyhow::Result<Report> {
    let mut max_cap = capacity.try_into().unwrap();
    if config.size_aware {
        max_cap *= 2u64.pow(15);
    }
    let cache_set = SharedSyncCache::new(config, max_cap, capacity);

    const BATCH_SIZE: usize = 200;

    let (send, receive) = crossbeam_channel::bounded::<Vec<Op>>(100);

    let handles = (0..num_clients)
        .map(|_| {
            let mut cache = cache_set.clone();
            let ch = receive.clone();

            std::thread::spawn(move || {
                let mut report = Report::new("Moka Cache", max_cap, None);
                process_commands(ch, &mut cache, &mut report);
                report
            })
        })
        .collect::<Vec<_>>();

    let mut counter = 0;
    let instant = Instant::now();

    for _ in 0..(config.repeat.unwrap_or(1)) {
        let f = File::open(config.trace_file.path())?;
        let reader = BufReader::new(f);
        for chunk in reader.lines().chunks(BATCH_SIZE).into_iter() {
            let commands = generate_commands(config, BATCH_SIZE, &mut counter, chunk)?;
            send.send(commands)?;
        }
    }

    // Drop the sender channel to notify the workers that we are finished.
    std::mem::drop(send);

    // Wait for the workers to finish and collect their reports.
    let reports = handles
        .into_iter()
        .map(|h| h.join().expect("Failed"))
        .collect::<Vec<_>>();
    let elapsed = instant.elapsed();

    // Merge the reports into one.
    let mut report = Report::new("Moka Sync Cache", max_cap, Some(num_clients));
    report.duration = Some(elapsed);
    reports.iter().for_each(|r| report.merge(r));

    if cfg!(feature = "moka-v09") && config.is_eviction_listener_enabled() {
        report.add_eviction_counts(cache_set.eviction_counters().as_ref().unwrap());
    }

    Ok(report)
}

#[allow(clippy::needless_collect)] // on the `handles` variable.
pub fn run_multi_threads_dash_cache(
    config: &Config,
    capacity: usize,
    num_clients: u16,
) -> anyhow::Result<Report> {
    let mut max_cap = capacity.try_into().unwrap();
    if config.size_aware {
        max_cap *= 2u64.pow(15);
    }
    let cache_set = SharedDashCache::new(config, max_cap, capacity);

    const BATCH_SIZE: usize = 200;

    let (send, receive) = crossbeam_channel::bounded::<Vec<Op>>(100);

    let handles = (0..num_clients)
        .map(|_| {
            let mut cache = cache_set.clone();
            let ch = receive.clone();

            std::thread::spawn(move || {
                let mut report = Report::new("Moka Cache", max_cap, None);
                process_commands(ch, &mut cache, &mut report);
                report
            })
        })
        .collect::<Vec<_>>();

    let mut counter = 0;
    let instant = Instant::now();

    for _ in 0..(config.repeat.unwrap_or(1)) {
        let f = File::open(config.trace_file.path())?;
        let reader = BufReader::new(f);
        for chunk in reader.lines().chunks(BATCH_SIZE).into_iter() {
            let commands = generate_commands(config, BATCH_SIZE, &mut counter, chunk)?;
            send.send(commands)?;
        }
    }

    // Drop the sender channel to notify the workers that we are finished.
    std::mem::drop(send);

    // Wait for the workers to finish and collect their reports.
    let reports = handles
        .into_iter()
        .map(|h| h.join().expect("Failed"))
        .collect::<Vec<_>>();
    let elapsed = instant.elapsed();

    // Merge the reports into one.
    let mut report = Report::new("Moka Dash Cache", max_cap, Some(num_clients));
    report.duration = Some(elapsed);
    reports.iter().for_each(|r| report.merge(r));

    Ok(report)
}

#[cfg(feature = "hashlink")]
#[allow(clippy::needless_collect)] // on the `handles` variable.
pub fn run_multi_threads_hashlink(
    config: &Config,
    capacity: usize,
    num_clients: u16,
) -> anyhow::Result<Report> {
    let cache_set = HashLink::new(config, capacity);

    const BATCH_SIZE: usize = 200;

    let (send, receive) = crossbeam_channel::bounded::<Vec<Op>>(100);

    let handles = (0..num_clients)
        .map(|_| {
            let mut cache = cache_set.clone();
            let ch = receive.clone();

            std::thread::spawn(move || {
                let mut report = Report::new("HashLink", capacity as _, None);
                process_commands(ch, &mut cache, &mut report);
                report
            })
        })
        .collect::<Vec<_>>();

    let mut counter = 0;
    let instant = Instant::now();

    for _ in 0..(config.repeat.unwrap_or(1)) {
        let f = File::open(config.trace_file.path())?;
        let reader = BufReader::new(f);
        for chunk in reader.lines().chunks(BATCH_SIZE).into_iter() {
            let commands = generate_commands(config, BATCH_SIZE, &mut counter, chunk)?;
            send.send(commands)?;
        }
    }

    // Drop the sender channel to notify the workers that we are finished.
    std::mem::drop(send);

    // Wait for the workers to finish and collect their reports.
    let reports = handles
        .into_iter()
        .map(|h| h.join().expect("Failed"))
        .collect::<Vec<_>>();
    let elapsed = instant.elapsed();

    // Merge the reports into one.
    let mut report = Report::new("HashLink", capacity as _, Some(num_clients));
    report.duration = Some(elapsed);
    reports.iter().for_each(|r| report.merge(r));

    Ok(report)
}

#[cfg(feature = "quick_cache")]
#[allow(clippy::needless_collect)] // on the `handles` variable.
pub fn run_multi_threads_quick_cache(
    config: &Config,
    capacity: usize,
    num_clients: u16,
) -> anyhow::Result<Report> {
    let cache_set = QuickCache::new(config, capacity);

    const BATCH_SIZE: usize = 200;

    let (send, receive) = crossbeam_channel::bounded::<Vec<Op>>(100);

    let handles = (0..num_clients)
        .map(|_| {
            let mut cache = cache_set.clone();
            let ch = receive.clone();

            std::thread::spawn(move || {
                let mut report = Report::new("QuickCache", capacity as _, None);
                process_commands(ch, &mut cache, &mut report);
                report
            })
        })
        .collect::<Vec<_>>();

    let mut counter = 0;
    let instant = Instant::now();

    for _ in 0..(config.repeat.unwrap_or(1)) {
        let f = File::open(config.trace_file.path())?;
        let reader = BufReader::new(f);
        for chunk in reader.lines().chunks(BATCH_SIZE).into_iter() {
            let commands = generate_commands(config, BATCH_SIZE, &mut counter, chunk)?;
            send.send(commands)?;
        }
    }

    // Drop the sender channel to notify the workers that we are finished.
    std::mem::drop(send);

    // Wait for the workers to finish and collect their reports.
    let reports = handles
        .into_iter()
        .map(|h| h.join().expect("Failed"))
        .collect::<Vec<_>>();
    let elapsed = instant.elapsed();

    // Merge the reports into one.
    let mut report = Report::new("QuickCache", capacity as _, Some(num_clients));
    report.duration = Some(elapsed);
    reports.iter().for_each(|r| report.merge(r));

    Ok(report)
}

#[cfg(feature = "stretto")]
#[allow(clippy::needless_collect)] // on the `handles` variable.
pub fn run_multi_threads_stretto(
    config: &Config,
    capacity: usize,
    num_clients: u16,
) -> anyhow::Result<Report> {
    let cache_set = StrettoCache::new(config, capacity);

    const BATCH_SIZE: usize = 200;

    let (send, receive) = crossbeam_channel::bounded::<Vec<Op>>(100);

    let handles = (0..num_clients)
        .map(|_| {
            let mut cache = cache_set.clone();
            let ch = receive.clone();

            std::thread::spawn(move || {
                let mut report = Report::new("Stretto", capacity as _, None);
                process_commands(ch, &mut cache, &mut report);
                report
            })
        })
        .collect::<Vec<_>>();

    let mut counter = 0;
    let instant = Instant::now();

    for _ in 0..(config.repeat.unwrap_or(1)) {
        let f = File::open(config.trace_file.path())?;
        let reader = BufReader::new(f);
        for chunk in reader.lines().chunks(BATCH_SIZE).into_iter() {
            let commands = generate_commands(config, BATCH_SIZE, &mut counter, chunk)?;
            send.send(commands)?;
        }
    }

    // Drop the sender channel to notify the workers that we are finished.
    std::mem::drop(send);

    // Wait for the workers to finish and collect their reports.
    let reports = handles
        .into_iter()
        .map(|h| h.join().expect("Failed"))
        .collect::<Vec<_>>();
    let elapsed = instant.elapsed();

    // Merge the reports into one.
    let mut report = Report::new("Stretto", capacity as _, Some(num_clients));
    report.duration = Some(elapsed);
    reports.iter().for_each(|r| report.merge(r));

    Ok(report)
}

#[allow(clippy::needless_collect)] // on the `handles` variable.
pub fn run_multi_thread_segmented(
    config: &Config,
    capacity: usize,
    num_clients: u16,
    num_segments: usize,
) -> anyhow::Result<Report> {
    let mut max_cap = capacity.try_into().unwrap();
    if config.size_aware {
        max_cap *= 2u64.pow(15);
    }
    let cache_set = SharedSegmentedMoka::new(config, max_cap, capacity, num_segments);

    const BATCH_SIZE: usize = 200;

    let (send, receive) = crossbeam_channel::bounded::<Vec<Op>>(100);

    let handles = (0..num_clients)
        .map(|_| {
            let mut cache = cache_set.clone();
            let ch = receive.clone();

            std::thread::spawn(move || {
                let mut report = Report::new("Moka SegmentedCache", max_cap, None);
                process_commands(ch, &mut cache, &mut report);
                report
            })
        })
        .collect::<Vec<_>>();

    let mut counter = 0;
    let instant = Instant::now();

    for _ in 0..(config.repeat.unwrap_or(1)) {
        let f = File::open(config.trace_file.path())?;
        let reader = BufReader::new(f);
        for chunk in reader.lines().chunks(BATCH_SIZE).into_iter() {
            let commands = generate_commands(config, BATCH_SIZE, &mut counter, chunk)?;
            send.send(commands)?;
        }
    }

    // Drop the sender channel to notify the workers that we are finished.
    std::mem::drop(send);

    // Wait for the workers to finish and collect their reports.
    let reports = handles
        .into_iter()
        .map(|h| h.join().expect("Failed"))
        .collect::<Vec<_>>();
    let elapsed = instant.elapsed();

    // Merge the reports into one.
    let mut report = Report::new(
        &format!("Moka SegmentedCache({})", num_segments),
        max_cap,
        Some(num_clients),
    );
    report.duration = Some(elapsed);
    reports.iter().for_each(|r| report.merge(r));

    if cfg!(feature = "moka-v09") && config.is_eviction_listener_enabled() {
        report.add_eviction_counts(cache_set.eviction_counters().as_ref().unwrap());
    }

    Ok(report)
}

pub async fn run_multi_tasks(
    config: &Config,
    capacity: usize,
    num_clients: u16,
) -> anyhow::Result<Report> {
    let mut max_cap = capacity.try_into().unwrap();
    if config.size_aware {
        max_cap *= 2u64.pow(15);
    }
    let cache_set = SharedAsyncCache::new(config, max_cap, capacity);

    const BATCH_SIZE: usize = 200;

    let (send, receive) = crossbeam_channel::bounded::<Vec<Op>>(100);

    let handles = (0..num_clients)
        .map(|_| {
            let mut cache = cache_set.clone();
            let ch = receive.clone();

            tokio::task::spawn(async move {
                let mut report = Report::new("Moka Async Cache", max_cap, None);
                process_commands_async(ch, &mut cache, &mut report).await;
                report
            })
        })
        .collect::<Vec<_>>();

    let mut counter = 0;
    let instant = Instant::now();

    for _ in 0..(config.repeat.unwrap_or(1)) {
        let f = File::open(config.trace_file.path())?;
        let reader = BufReader::new(f);
        for chunk in reader.lines().chunks(BATCH_SIZE).into_iter() {
            let commands = generate_commands(config, BATCH_SIZE, &mut counter, chunk)?;
            send.send(commands)?;
        }
    }

    // Drop the sender channel to notify the workers that we are finished.
    std::mem::drop(send);

    // Wait for the workers to finish and collect their reports.
    let reports = futures_util::future::join_all(handles).await;
    let elapsed = instant.elapsed();

    // Merge the reports into one.
    let mut report = Report::new("Moka Async Cache", max_cap, Some(num_clients));
    report.duration = Some(elapsed);
    reports
        .iter()
        .for_each(|r| report.merge(r.as_ref().expect("Failed")));

    if cfg!(feature = "moka-v09") && config.is_eviction_listener_enabled() {
        report.add_eviction_counts(cache_set.eviction_counters().as_ref().unwrap());
    }

    Ok(report)
}
