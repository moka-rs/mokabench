# Mokabench

A load generator for some of the concurrent cache libraries for Rust.


## Usage

### Install the Rust Toolchain

Install a recent stable [Rust toolchain][rustup].

### Clone this Repository and the Submodule

Clone this repository:

```console
$ git clone https://github.com/moka-rs/mokabench.git
$ cd mokabench
```

Then clone the submodule [`cache-trace`][git-cache-trace], which contains the trace
datasets. Since these datasets are very large, use the `--depth 1` option to clone
only the latest revision.

```console
$ git submodule init
$ git submodule update --depth 1
```

### Expand the Trace Files

The `cache-trace` submodule has the `arc` directory. It contains the trace datasets
used in the ARC paper[^1]. Here are some examples:

| Dataset    | Workload Type | Description |
|:-----------|:--------------|:------------|
| `S3.lis`   | Search Engine | Disk read accesses initiated by a large commercial search engine in response to various web search requests. |
| `DS1.lis`  | ERP Database  | A database server running at a commercial site running an ERP application on top of a commercial database. |
| `OLTP.lis` | CODASYL       | References to a CODASYL database for a one hour period. |

See [the README][git-cache-trace-arc] in `cache-trace/arc` for more details.

They are compressed with [Zstandard][zstd]. Run the following command to expand.

```console
$ cd cache-trace/arc
$ zstd -d *.lis.zst
$ cat spc1likeread.lis.zst.* | zstd -d - > spc1likeread.lis
```

You may need to install `zstd` first. For example, on Ubuntu:

```console
$ sudo apt install zstd
```

or on macOS using Homebrew:

```console
$ brew install zstd
```

In the future, you will not need to expand the trace files manually. Mokabench will
add support for directly reading the compressed trace files. ([#13])

[git-cache-trace]: https://github.com/moka-rs/cache-trace
[git-cache-trace-arc]: https://github.com/moka-rs/cache-trace/tree/main/arc
[zstd]: https://facebook.github.io/zstd/
[#13]: https://github.com/moka-rs/mokabench/issues/13
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
$ cargo build --release --no-default-features -F moka-v09,rt-tokio
```

**Features to select cache products:**

| Feature       | Enabled Cache Product |
|:--------------|:----------------------|
| `moka-v012`   | [Moka](https://crates.io/crates/moka) v0.12.x (Enabled by default) |
| `moka-v011`   |  Moka v0.11.x |
| `moka-v010`   |  Moka v0.10.x |
| `moka-v09`    |  Moka v0.9.x |
| `moka-v08`    |  Moka v0.8.x |
| `hashlink`    | [HashLink](https://crates.io/crates/hashlink) |
| `mini-moka`   | [Mini-Moka](https://crates.io/crates/mini-moka) |
| `quick_cache` | [quick_cache](https://crates.io/crates/quick_cache) |
| `stretto`     | [Stretto](https://crates.io/crates/stretto) |
| `tiny-ufo`    | [TinyUFO](https://crates.io/crates/TinyUFO) |

**Features to select async runtime:**

Only used by Moka async cache.

| Feature        | Enabled Cache Product |
|:---------------|:----------------------|
| `rt-tokio`     | [Tokio](https://crates.io/crates/tokio) (Enabled by default) |
| `rt-async-std` | [async-std](https://crates.io/crates/async-std) |

NOTES:

- `moka-v012` and `rt-tokio` are enabled by default.
- `moka-v012`, `moka-v011`, `moka-v010`, `moka-v09` and `moka-v08` are mutually
  exclusive.
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

[rustup]: https://rustup.rs
