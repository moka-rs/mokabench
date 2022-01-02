# Mokabench &mdash; Change Log

## Version 0.1.0

### Added

- Added load generators for the following Moka cache implementations (up to Moka v0.6.x):
    - `future::Cache`
    - `sync::Cache`
    - `sync::SegmentedCache`
    - `unsync::Cache`
- Added the following CLI options:
    - `ttl` (Time-to-live in seconds)
    - `tti` (Time-to-idle in seconds)
    - `enable-insert-once`
    - `enable-invalidate`
    - `enable-invalidate-all`
    - `enable-invalidate-entries-if`
