use itertools::Itertools;
use std::io::prelude::*;
use std::{fs::File, io::BufReader, time::Instant};

mod cache;
mod parser;
mod report;

use cache::{
    async_cache::SharedAsyncCache,
    sync_cache::{SharedSyncCache, SyncCache},
    sync_segmented::SharedSegmentedMoka,
    AsyncCacheSet, CacheSet,
};
use parser::TraceParser;

pub use report::Report;

pub const TRACE_FILE: &str = "../trace-data/S3.lis";

// pub const TTL_SECS: u64 = 120 * 60;
// pub const TTI_SECS: u64 = 60 * 60;
pub const TTL_SECS: u64 = 3;
pub const TTI_SECS: u64 = 1;

pub fn run_single(capacity: usize) -> anyhow::Result<Report> {
    let f = File::open(TRACE_FILE)?;
    let reader = BufReader::new(f);
    let mut parser = parser::ArcTraceParser;
    let mut cache_set = SyncCache::new(capacity);
    let mut report = Report::new("Moka Cache", capacity, None);

    let instant = Instant::now();
    let lines = reader.lines().enumerate().map(|(c, l)| (c + 1, l));

    for (_count, line) in lines {
        let line = line?;
        let entry = parser.parse(&line)?;
        cache_set.process(&entry, &mut report);
    }

    let elapsed = instant.elapsed();
    report.duration = Some(elapsed);

    Ok(report)
}

pub fn run_multi_threads(capacity: usize, num_workers: u16) -> anyhow::Result<Report> {
    let f = File::open(TRACE_FILE)?;
    let reader = BufReader::new(f);
    let cache_set = SharedSyncCache::new(capacity);

    const BATCH_SIZE: usize = 200;

    let (send, receive) = crossbeam_channel::bounded::<Vec<String>>(100);

    let handles = (0..num_workers)
        .map(|_| {
            let mut cache = cache_set.clone();
            let ch = receive.clone();

            std::thread::spawn(move || {
                let mut parser = parser::ArcTraceParser;
                let mut report = Report::new("Moka Cache", capacity, None);
                while let Ok(lines) = ch.recv() {
                    for line in lines {
                        if let Ok(entry) = parser.parse(&line) {
                            cache.process(&entry, &mut report);
                        }
                    }
                }
                report
            })
        })
        .collect::<Vec<_>>();

    let instant = Instant::now();
    for chunk in reader.lines().chunks(BATCH_SIZE).into_iter() {
        let chunk = chunk.collect::<Result<Vec<String>, _>>()?;
        send.send(chunk)?;
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

pub fn run_multi_thread_segmented(
    capacity: usize,
    num_workers: u16,
    num_segments: usize,
) -> anyhow::Result<Report> {
    let f = File::open(TRACE_FILE)?;
    let reader = BufReader::new(f);
    let cache_set = SharedSegmentedMoka::new(capacity, num_segments);

    const BATCH_SIZE: usize = 200;

    let (send, receive) = crossbeam_channel::bounded::<Vec<String>>(100);

    let handles = (0..num_workers)
        .map(|_| {
            let mut cache = cache_set.clone();
            let ch = receive.clone();

            std::thread::spawn(move || {
                let mut parser = parser::ArcTraceParser;
                let mut report = Report::new("Moka SegmentedCache", capacity, None);
                while let Ok(lines) = ch.recv() {
                    for line in lines {
                        if let Ok(entry) = parser.parse(&line) {
                            cache.process(&entry, &mut report);
                        }
                    }
                }
                report
            })
        })
        .collect::<Vec<_>>();

    let instant = Instant::now();
    for chunk in reader.lines().chunks(BATCH_SIZE).into_iter() {
        let chunk = chunk.collect::<Result<Vec<String>, _>>()?;
        send.send(chunk)?;
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

pub async fn run_multi_tasks(capacity: usize, num_workers: u16) -> anyhow::Result<Report> {
    let f = File::open(TRACE_FILE)?;
    let reader = BufReader::new(f);
    let cache_set = SharedAsyncCache::new(capacity);

    const BATCH_SIZE: usize = 200;

    let (send, receive) = crossbeam_channel::bounded::<Vec<String>>(100);

    let handles = (0..num_workers)
        .map(|_| {
            let mut cache = cache_set.clone();
            let ch = receive.clone();

            tokio::task::spawn(async move {
                let mut parser = parser::ArcTraceParser;
                let mut report = Report::new("Moka Async Cache", capacity, None);
                while let Ok(lines) = ch.recv() {
                    for line in lines {
                        if let Ok(entry) = parser.parse(&line) {
                            cache.process(&entry, &mut report).await;
                        }
                    }
                }
                report
            })
        })
        .collect::<Vec<_>>();

    let instant = Instant::now();
    for chunk in reader.lines().chunks(BATCH_SIZE).into_iter() {
        let chunk = chunk.collect::<Result<Vec<String>, _>>()?;
        send.send(chunk)?;
    }

    // Drop the sender channel to notify the workers that we are finished.
    std::mem::drop(send);

    // Wait for the workers to finish and collect their reports.
    let reports = futures::future::join_all(handles).await;
    let elapsed = instant.elapsed();

    // Merge the reports into one.
    let mut report = Report::new("Moka Async Cache", capacity, Some(num_workers));
    report.duration = Some(elapsed);
    reports.iter().for_each(|r| report.merge(r.as_ref().expect("Failed")));

    Ok(report)
}
