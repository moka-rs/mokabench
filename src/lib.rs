use std::io::prelude::*;
use std::{fs::File, io::BufReader};

mod cache_set;
mod parser;
mod report;

use cache_set::{CacheSet, Moka};
use parser::TraceParser;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let f = File::open("../trace-data/S3.lis")?;
    let reader = BufReader::new(f);
    let mut parser = parser::ArcTraceParser;
    let mut cache_set = Moka::new(100_000);

    const OP_COUNT: usize = 500_000;

    println!("Running a single threaded executor.");
    println!(r#"  Trace Data: "S3.lis""#);
    println!("  Operation Count: {}", OP_COUNT);

    let lines = reader.lines().enumerate().map(|(c, l)| (c + 1, l));

    for (count, line) in lines {
        let line = line?;
        let entry = parser.parse(&line)?;
        cache_set.process(&entry);

        if count % 1_000 == 0 {
            print!(".");
            std::io::stdout().flush()?;
        }
        if count >= OP_COUNT {
            break;
        }
    }
    println!();

    let report = cache_set.report();
    println!(
        "Read count: {}, Hit ratio: {:.4}%",
        report.read_count,
        report.hit_ratio() * 100.0
    );
    Ok(())
}
