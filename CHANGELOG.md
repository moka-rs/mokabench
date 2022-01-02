# Mokabench &mdash; Change Log

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
