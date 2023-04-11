# Mokabench

A load generator for some of the concurrent cache libraries for Rust.


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

To build with the default features, run the following command. This will enables the
latest version of Moka.

```console
$ cargo build --release
```

#### Enabling Other Cache Products

If you want to enable other cache products, use crate features.

For example:

```console
## Enable Mini-Moka and quick_cache in addition to the latest version of Moka.
$ cargo build --release -F mini-moka,quick_cache

## Disable the latest version of Moka, but enable v0.9.x.
$ cargo build --release --no-default-features -F moka-v09
```

| Feature       | Enabled Cache Product |
|:--------------|:----------------------|
| `moka-v010`   | [Moka](https://crates.io/crates/moka) v0.11.x (Enabled by default) |
| `moka-v010`   |  Moka v0.10.x |
| `moka-v09`    |  Moka v0.9.x |
| `moka-v08`    |  Moka v0.8.x |
| `hashlink`    | [HashLink](https://crates.io/crates/hashlink) |
| `mini-moka`   | [Mini-Moka](https://crates.io/crates/mini-moka) |
| `quick_cache` | [quick_cache](https://crates.io/crates/quick_cache) |
| `stretto`     | [Stretto](https://crates.io/crates/stretto) |

NOTES:

- `moka-v011` is enabled by default.
- `moka-v011`, `moka-v010`, `moka-v09` and `moka-v08` are mutually exclusive.
- `mini-moka` cannot be enabled when `moka-v09` or `moka-v08` is enabled.


### Run Benchmarks

Here are some examples to run benchmarks:

```console
## Run with the default (S3.lis) dataset, using single client thread,
## and then 3 client threads, and so on.
$ ./target/release/mokabench --num-clients 1,3,6

## Run with DS1.lis dataset.
$ ./target/release/mokabench --num-clients 1,3,6 --trace-file ds1

## Run with an insertion delay (in microseconds) to simulate more
## realistic workload. The following example will try to add ~1
## microseconds delay before inserting a value to the cache.
##
## Note: It uses `std::thread::sleep` and many operating systems
## have a minimum sleep resolution larger than 1 microsecond. So
## the actual delay may be larger than the specified value.
##
$ ./target/release/mokabench --num-clients 1,3,6 --insertion-delay 1
```

You can also test Moka's advanced features/APIs:

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


## License

Mokabench is distributed under the MIT license. See [LICENSE-MIT](LICENSE-MIT) for details.

<!-- Links -->

[arc-paper-site]: https://researcher.watson.ibm.com/researcher/view_person_subpage.php?id=4700
[rustup]: https://rustup.rs
