# Mokabench

A load generator for Moka, a concurrent cache library for Rust.

## Usage

### Install a Rust Toolchain

Install a recent stable [Rust toolchain][rustup].

### Download the Trace Dataset

Mokabench uses a trace dataset called `S3.lis`. This trace was prepared for a research
paper "ARC: A Self-Tuning, Low Overhead Replacement Cache" by Nimrod Megiddo and
Dharmendra S. Modha. The trace is described as "disk read accesses initiated by a
large commercial search engine in response to various web search requests."

Download `S3.lis` from [a author's web site][arc-paper-site].

### Run Benchmarks

Put `S3.lis` to the `./datasets` directory and run the benchmarks with the following
commands:

```console
$ cargo build --release
```

```console
## Call `get` and `insert` with time-to-live = 3 seconds and
## time-to-idle = 1 second.
$ ./target/release/mokabench --ttl 3 --tti 1

## Also call `get_or_insert_with`.
$ ./target/release/mokabench --ttl 3 --tti 1 --enable-insert-once

## Call `get`, `insert` and `invalidate`.
$ ./target/release/mokabench --enable-invalidate

## Call `get`, `insert` and `invalidate_all`.
$ ./target/release/mokabench --enable-invalidate-all

## Call `get`, `insert` and `invalidate_entries-if`.
$ ./target/release/mokabench --enable-invalidate-entries-if

## Run with everything.
$ ./target/release/mokabench --ttl 3 --tti 1 \
    --enable-insert-once --enable-invalidate \
    --enable-invalidate-all --enable-invalidate-entries-if
```

[arc-paper-site]: https://researcher.watson.ibm.com/researcher/view_person_subpage.php?id=4700
[rustup]: https://rustup.rs
