use config::Config;
use crossbeam_channel::Receiver;
use itertools::Itertools;
use std::io::prelude::*;
use std::{fs::File, io::BufReader, time::Instant};

mod cache;
pub mod config;
mod parser;
mod report;

use cache::{
    async_cache::SharedAsyncCache, sync_cache::SharedSyncCache,
    sync_segmented::SharedSegmentedMoka, unsync_cache::UnsyncCache, AsyncCacheSet, CacheSet,
};
use parser::{ArcTraceEntry, TraceParser};

pub use report::Report;

pub const TRACE_FILE: &str = "./datasets/S3.lis";

pub(crate) enum Op {
    GetOrInsert(String),
    Invalidate(String),
    InvalidateAll,
}

pub fn run_single(config: &Config, capacity: usize) -> anyhow::Result<Report> {
    let f = File::open(TRACE_FILE)?;
    let reader = BufReader::new(f);
    let mut parser = parser::ArcTraceParser;
    let mut cache_set = UnsyncCache::new(config, capacity);
    let mut report = Report::new("Moka Unsync Cache", capacity, None);
    let mut counter = 0;

    let instant = Instant::now();

    for line in reader.lines() {
        let line = line?;
        counter += 1;
        let entry = parser.parse(&line)?;
        if config.enable_invalidate_all && counter % 100_000 == 0 {
            cache_set.invalidate_all();
            cache_set.get_or_insert(&entry, &mut report);
        } else if config.enable_invalidate && counter % 8 == 0 {
            cache_set.invalidate(&entry);
        } else {
            cache_set.get_or_insert(&entry, &mut report);
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
        if config.enable_invalidate_all && *counter % 100_000 == 0 {
            ops.push(Op::InvalidateAll);
            ops.push(Op::GetOrInsert(line));
        } else if config.enable_invalidate && *counter % 8 == 0 {
            ops.push(Op::Invalidate(line));
        } else {
            ops.push(Op::GetOrInsert(line));
        }
    }
    Ok(ops)
}

fn process_commands(
    receiver: Receiver<Vec<Op>>,
    cache: &mut impl CacheSet<ArcTraceEntry>,
    report: &mut Report,
) {
    let mut parser = parser::ArcTraceParser;
    while let Ok(ops) = receiver.recv() {
        for op in ops {
            match op {
                Op::GetOrInsert(line) => {
                    if let Ok(entry) = parser.parse(&line) {
                        cache.get_or_insert(&entry, report);
                    }
                }
                Op::Invalidate(line) => {
                    if let Ok(entry) = parser.parse(&line) {
                        cache.invalidate(&entry);
                    }
                }
                Op::InvalidateAll => cache.invalidate_all(),
            }
        }
    }
}

async fn process_commands_async(
    receiver: Receiver<Vec<Op>>,
    cache: &mut impl AsyncCacheSet<ArcTraceEntry>,
    report: &mut Report,
) {
    let mut parser = parser::ArcTraceParser;
    while let Ok(ops) = receiver.recv() {
        for op in ops {
            match op {
                Op::GetOrInsert(line) => {
                    if let Ok(entry) = parser.parse(&line) {
                        cache.get_or_insert(&entry, report).await;
                    }
                }
                Op::Invalidate(line) => {
                    if let Ok(entry) = parser.parse(&line) {
                        cache.invalidate(&entry).await;
                    }
                }
                Op::InvalidateAll => cache.invalidate_all(),
            }
        }
    }
}

#[allow(clippy::needless_collect)] // on the `handles` variable.
pub fn run_multi_threads(
    config: &Config,
    capacity: usize,
    num_workers: u16,
) -> anyhow::Result<Report> {
    let f = File::open(TRACE_FILE)?;
    let reader = BufReader::new(f);
    let cache_set = SharedSyncCache::new(config, capacity);

    const BATCH_SIZE: usize = 200;

    let (send, receive) = crossbeam_channel::bounded::<Vec<Op>>(100);

    let handles = (0..num_workers)
        .map(|_| {
            let mut cache = cache_set.clone();
            let ch = receive.clone();

            std::thread::spawn(move || {
                let mut report = Report::new("Moka Cache", capacity, None);
                process_commands(ch, &mut cache, &mut report);
                report
            })
        })
        .collect::<Vec<_>>();

    let mut counter = 0;
    let instant = Instant::now();
    for chunk in reader.lines().chunks(BATCH_SIZE).into_iter() {
        let commands = generate_commands(config, BATCH_SIZE, &mut counter, chunk)?;
        send.send(commands)?;
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
    let mut report = Report::new("Moka Cache", capacity, Some(num_workers));
    report.duration = Some(elapsed);
    reports.iter().for_each(|r| report.merge(r));

    Ok(report)
}

#[allow(clippy::needless_collect)] // on the `handles` variable.
pub fn run_multi_thread_segmented(
    config: &Config,
    capacity: usize,
    num_workers: u16,
    num_segments: usize,
) -> anyhow::Result<Report> {
    let f = File::open(TRACE_FILE)?;
    let reader = BufReader::new(f);
    let cache_set = SharedSegmentedMoka::new(config, capacity, num_segments);

    const BATCH_SIZE: usize = 200;

    let (send, receive) = crossbeam_channel::bounded::<Vec<Op>>(100);

    let handles = (0..num_workers)
        .map(|_| {
            let mut cache = cache_set.clone();
            let ch = receive.clone();

            std::thread::spawn(move || {
                let mut report = Report::new("Moka SegmentedCache", capacity, None);
                process_commands(ch, &mut cache, &mut report);
                report
            })
        })
        .collect::<Vec<_>>();

    let mut counter = 0;
    let instant = Instant::now();
    for chunk in reader.lines().chunks(BATCH_SIZE).into_iter() {
        let commands = generate_commands(config, BATCH_SIZE, &mut counter, chunk)?;
        send.send(commands)?;
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
    let mut report = Report::new("Moka SegmentedCache", capacity, Some(num_workers));
    report.duration = Some(elapsed);
    reports.iter().for_each(|r| report.merge(r));

    Ok(report)
}

pub async fn run_multi_tasks(
    config: &Config,
    capacity: usize,
    num_workers: u16,
) -> anyhow::Result<Report> {
    let f = File::open(TRACE_FILE)?;
    let reader = BufReader::new(f);
    let cache_set = SharedAsyncCache::new(config, capacity);

    const BATCH_SIZE: usize = 200;

    let (send, receive) = crossbeam_channel::bounded::<Vec<Op>>(100);

    let handles = (0..num_workers)
        .map(|_| {
            let mut cache = cache_set.clone();
            let ch = receive.clone();

            tokio::task::spawn(async move {
                let mut report = Report::new("Moka Async Cache", capacity, None);
                process_commands_async(ch, &mut cache, &mut report).await;
                report
            })
        })
        .collect::<Vec<_>>();

    let mut counter = 0;
    let instant = Instant::now();
    for chunk in reader.lines().chunks(BATCH_SIZE).into_iter() {
        let commands = generate_commands(config, BATCH_SIZE, &mut counter, chunk)?;
        send.send(commands)?;
    }

    // Drop the sender channel to notify the workers that we are finished.
    std::mem::drop(send);

    // Wait for the workers to finish and collect their reports.
    let reports = futures::future::join_all(handles).await;
    let elapsed = instant.elapsed();

    // Merge the reports into one.
    let mut report = Report::new("Moka Async Cache", capacity, Some(num_workers));
    report.duration = Some(elapsed);
    reports
        .iter()
        .for_each(|r| report.merge(r.as_ref().expect("Failed")));

    Ok(report)
}
