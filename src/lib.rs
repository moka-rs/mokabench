use itertools::Itertools;
use std::io::prelude::*;
use std::{fs::File, io::BufReader, time::Instant};

mod cache_set;
mod parser;
mod report;

use cache_set::{CacheSet, Moka, SharedMoka};
use parser::TraceParser;
pub use report::Report;

pub fn run_single(capacity: usize) -> Result<Report, Box<dyn std::error::Error>> {
    let f = File::open("../trace-data/S3.lis")?;
    let reader = BufReader::new(f);
    let mut parser = parser::ArcTraceParser;
    let mut cache_set = Moka::new(capacity);
    let mut report = Report::new(capacity, None);

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

pub fn run_multi(capacity: usize, num_workers: u16) -> Result<Report, Box<dyn std::error::Error>> {
    let f = File::open("../trace-data/S3.lis")?;
    let reader = BufReader::new(f);
    let cache_set = SharedMoka::new(capacity);

    const BATCH_SIZE: usize = 200;

    let (send, receive) = crossbeam_channel::bounded::<Vec<String>>(100);

    let handles = (0..num_workers)
        .map(|_| {
            let mut cache = cache_set.clone();
            let ch = receive.clone();

            std::thread::spawn(move || {
                let mut parser = parser::ArcTraceParser;
                let mut report = Report::new(capacity, None);
                loop {
                    match ch.recv() {
                        Ok(lines) => {
                            for line in lines {
                                if let Ok(entry) = parser.parse(&line) {
                                    cache.process(&entry, &mut report);
                                }
                            }
                        }
                        Err(_) => {
                            // No more tasks.
                            break;
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
    let mut report = Report::new(capacity, Some(num_workers));
    report.duration = Some(elapsed);
    reports.iter().for_each(|r| report.merge(r));

    Ok(report)
}
