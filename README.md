# Mokabench

A load generator for Moka, a concurrent cache library for Rust.

## Usage

### Install a Rust Toolchain

Install a recent stable [Rust toolchain][rustup].


### Download the Trace Datasets

Mokabench uses the following trace datasets from the ARC paper[^1].

| Dataset    | Workload Type | Description |
|:-----------|:--------------|:------------|
| `S3.lis`   | Search Engine | Disk read accesses initiated by a large commercial search engine in response to various web search requests. |
| `DS1.lis`  | ERP Database  | A database server running at a commercial site running an ERP application on top of a commercial database. |
| `OLTP.lis` | CODASYL       | References to a CODASYL database for a one hour period. |

Download them from [author's web site][arc-paper-site] and put them into the `./datasets` directory.

[^1]: "ARC: A Self-Tuning, Low Overhead Replacement Cache" by Nimrod Megiddo and Dharmendra S. Modha.


### Build Mokabench

```console
$ cargo build --release
```


### Run Benchmarks

Run the benchmarks with the following commands:

```console
## Call `get` and `insert` with time-to-live = 3 seconds and
## time-to-idle = 1 second.
$ ./target/release/mokabench --ttl 3 --tti 1

## Also call `get_or_insert_with`.
$ ./target/release/mokabench --ttl 3 --tti 1 --insert-once

## Call `get`, `insert` and `invalidate`.
$ ./target/release/mokabench --invalidate

## Call `get`, `insert` and `invalidate_all`.
$ ./target/release/mokabench --invalidate-all

## Call `get`, `insert` and `invalidate_entries-if`.
$ ./target/release/mokabench --invalidate-entries-if

## Run with everything.
$ ./target/release/mokabench --ttl 3 --tti 1 \
    --insert-once --invalidate \
    --invalidate-all --invalidate-entries-if
```

[arc-paper-site]: https://researcher.watson.ibm.com/researcher/view_person_subpage.php?id=4700
[rustup]: https://rustup.rs


## License

Mokabench is distributed under the MIT license. See [LICENSE-MIT](LICENSE-MIT) for details.
