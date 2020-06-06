use mokabench::{self, Report};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", Report::cvs_header());

    const CAPACITIES: [usize; 2] = [100_000, 2_000_000];

    for capacity in &CAPACITIES {
        run_with_capacity(*capacity)?
    }

    Ok(())
}

fn run_with_capacity(capacity: usize) -> Result<(), Box<dyn std::error::Error>> {
    // const NUM_WORKERS_ARRAY: [u16; 6] = [1, 2, 4, 8, 16, 32];
    const NUM_WORKERS_ARRAY: [u16; 5] = [16, 24, 32, 40, 48];

    // let report = mokabench::run_single(capacity)?;
    // println!("{}", report.to_csv_record());

    for num_workers in &NUM_WORKERS_ARRAY {
        let report = mokabench::run_multi(capacity, *num_workers)?;
        // println!("");
        println!("{}", report.to_csv_record());
    }

    let num_segments = 8;

    for num_workers in &NUM_WORKERS_ARRAY {
        let report = mokabench::run_multi_segmented(capacity, *num_workers, num_segments)?;
        // println!("");
        println!("{}", report.to_csv_record());
    }

    Ok(())
}
