# Mokabench &mdash; Change Log

## Version 0.10.0

### Added

- Added support for moka v0.12. ([#??](gh-pull-????))


## Version 0.9.0

### Added

- Added support for moka v0.11. ([#8](gh-pull-0008))
    - Added a CLI option `--per-key-expiration`.

## Version 0.8.0

### Added

- Added support for moka v0.10 and mini-moka v0.10. ([#2](gh-pull-0002))
    - Added a CLI option `--entry-api`.

### Changed

- Upgrade stretto to v0.7. ([#2](gh-pull-0002))

## Version 0.7.0

### Added

- Added support for quick_cache, stretto and hashlink caches. ([#1][gh-pull-0001])

## Version 0.6.0

### Added

- Added crate features `moka-v09` and `moka-v08` to depend on Moka v0.9.x and v0.8.x
  respectively. (`moka-v09` is the default)
- Added a CLI option `--eviction-listener`, which enables the eviction listener
  support added to Moka v0.9.x.

## Version 0.5.0

### Added

- Added a CLI option `--iterate`, which enables iterator based scanning.
    - Note: Iterator support was added at the following Moka versions:
        - Moka v0.8.1:
            - `dash::Cache`:
        - Moka v0.8.2
            - `sync::Cache`
            - `sync::SegmentedCache`
            - `future::Cache`
            - `unsync::Cache`

### Removed

- Dropped support for Moka v0.8.1 or earlier.

## Version 0.4.0

### Added

- Added a driver for `dash::Cache`, which was introduced in Moka v0.8.0.

### Removed

- Dropped support for Moka v0.7.x or earlier.

## Version 0.3.0

### Added

- Support more trace data: ARC-DS1 and ARC-OLTP.
- Added a CLI option `--size-aware`, which enables the size aware (weight-based)
  cache management.

## Changed

- Reduced the value size from 256 bytes to 128 bytes.

### Removed

- Dropped support for Moka v0.6.x or earlier.

## Version 0.2.0

### Fixed

- Fixed a bug to increment the read counter twice after inserting a value.

### Added

- Added support for Moka v0.7.x.
- Added a CLI option `--num-workers`.
- Added support for `get_or_try_insert_with` API. (Will be generated
  when a CLI option `--enable-insert-once` is given)

### Changed

- Switched to Rust 2021 edition.

## Version 0.1.0

### Added

- Added load generators for Moka v0.6.x:
    - Moka cache implementations:
        - `future::Cache`
        - `sync::Cache`
        - `sync::SegmentedCache`
        - `unsync::Cache`
    - Moka API:
        - `get`
        - `get_or_insert_with`
        - `insert`
        - `invalidate_all`
        - `invalidate_entries_if`
        - `invalidate`
- Added the following CLI options:
    - `ttl` (Time-to-live in seconds)
    - `tti` (Time-to-idle in seconds)
    - `enable-insert-once`
    - `enable-invalidate`
    - `enable-invalidate-all`
    - `enable-invalidate-entries-if`

<!-- Links -->
[gh-pull-0008]: https://github.com/moka-rs/mokabench/pull/8
[gh-pull-0002]: https://github.com/moka-rs/mokabench/pull/2
[gh-pull-0001]: https://github.com/moka-rs/mokabench/pull/1
