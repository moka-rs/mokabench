use mokabench::{self, Report};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", Report::cvs_header());

    for capacity in &[100_000_usize, 10_000_000_usize] {
        run_with_capacity(*capacity)?
    }

    Ok(())
}

fn run_with_capacity(capacity: usize) -> Result<(), Box<dyn std::error::Error>> {
    const NUM_WORKERS_ARRAY: [u16; 6] = [1, 2, 4, 8, 16, 32];

    let report = mokabench::run_single(capacity)?;
    println!("{}", report.to_csv_record());

    for num_workers in &NUM_WORKERS_ARRAY {
        let report = mokabench::run_multi(capacity, *num_workers)?;
        println!("{}", report.to_csv_record());
    }

    Ok(())
}
