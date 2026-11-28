# Changelog

All notable changes to this project are documented here. The project follows
[Semantic Versioning](https://semver.org/). Before 1.0, a breaking change
bumps the minor version (0.1 → 0.2) and every workspace crate shares one
version.

## [Unreleased]

## [0.1.1] — 2026-11-28 (preview)

A patch preview. File output now starts on macOS and Windows, and the
release adds the 24-hour soak, a 4-hour mixed run, fault injection and
cross-OS file exchange to the verification. Still a GitHub pre-release; the
crates are not on crates.io yet.

```toml
[dependencies]
dot-logger = { git = "https://github.com/lim8603/dot-logger", tag = "v0.1.1" }
```

### Added

- `diagnostics::available_bytes`: free space below a path on Linux, macOS
  and Windows. A missing path is an error on every OS.
- `ReceiverConfig::reservation_bytes` (client) and
  `Reassembler::reservation_bytes` (core): the budget bytes one live receiver
  reserves. Size a `LiveBudget` or `QueryBudget` as subscriptions times this
  value; the receiver's bookkeeping differs by platform, so a formula built
  from the payload size alone can fall short on macOS.
- `influxdb_write` example (`--features influxdb`): the advanced-path
  InfluxDB v2 `Writer` writes a few points and reads the numeric ones back.
  `scripts/check-influxdb.py --example` runs it against a throwaway DB.
- `scripts/check-influxdb.py --server v3`: runs the live InfluxDB tests and
  the example against InfluxDB 3 Core 3.12.3 and checks the stored rows
  with SQL.

### Changed

- README: InfluxDB output is on the advanced path (`adapters::influxdb`),
  not the default `Logger`.
- `adapters::influxdb` and README: server version table. InfluxDB 3 Core
  takes the same writes through its v2-compatible API but has no Flux, so
  `query_numeric` needs InfluxDB 2.x. On 3 Core a batch with a type conflict
  is rejected whole (still reported as `Unknown`).

### Fixed

- File output on macOS and Windows: `Logger::start()` failed with
  `Unsupported` because the free-space check and the directory sync were
  Linux-only. They now use `statvfs` on Unix and the `fs4` crate on Windows,
  and Windows skips the directory sync (NTFS journals it). On Windows, keep
  logs on a local NTFS volume: after a power loss a just-removed ring file
  can reappear, and FAT32, exFAT and network shares give no such guarantee.
- Memory accounting on macOS: Apple's std allocates a lock's storage on its
  first use, which the facade, remote hub and live receivers did not count.
  Each warmed lock now adds 64 bytes to its reservation (0 elsewhere), so a
  budget sized exactly to the old figure on macOS may need a little more.
- MCAP files record the writing version in their header (`dot-logger/0.1.1`)
  instead of a fixed `dot-logger/0.1.0`.

### Security

- `Cargo.lock` takes `url` 2.5.9 for RUSTSEC-2026-0592 (stack overflow in
  `Url::origin()` on nested `blob:` URLs), reached through `reqwest` with the
  `influxdb` feature. dot-logger's own requirements are ranges, so a project
  with its own `Cargo.lock` should run `cargo update -p url`.

### Verified

- About 750 tests pass with Rust 1.96.0 and the 1.85 MSRV, on Ubuntu 24.04,
  Windows Server 2025 and macOS 15. File output tests that ran on Linux only
  now run on all three.
- Logs written on Linux, Windows and macOS are read back on all three in CI.
- A 24-hour soak on the PREEMPT_RT kernel (432 million records, about
  2.5 MB/s, 16 GiB ring) finished with no loss, rejection or missed deadline,
  and every remaining file read back intact.
- A 4-hour run with the same load added, at the same time, another process
  reading closed files, two remote subscribers, status polling and event
  protection while the ring deleted old files. No record was lost or
  rejected, no deadline was missed, real-time threads did not allocate, and
  every file read was intact.
- Fault injection on the same PC: a slow disk, stalled subscribers, a full
  disk, a dropped network link and repeated `SIGKILL`, with no missed
  deadline. On the slow and full disk the loss report matched the gaps in the
  files; the other faults lost no record from the files, and after each
  `SIGKILL` the closed files were intact and the recovered records had no
  gaps.

### Known issues

- No production integration yet.
- A subscriber does not notice a connection that stops silently (no
  keepalive); only the command timeout ends it. A closed connection is
  noticed at once.
- After a real `ENOSPC` the file output stays failed for the rest of the
  session (`IoPoisoned`), while real-time capture keeps accepting records and
  counts them as failed. Restart the logger once space is back.
- On InfluxDB 3 Core, `query_numeric` is not available (no Flux); use
  InfluxDB 2.x for queries.
- Power loss, old Lichtblick files, write-error reopening, compression,
  remote pause or split, and the `paste` warning: unchanged from 0.1.0.

## [0.1.0] — 2026-11-22 (preview)

The first preview release of dot-logger, a Rust logging and telemetry library
for robotics and real-time applications. It is a GitHub pre-release: the API
may still change, and the crates are not on crates.io yet.

```toml
[dependencies]
dot-logger = { git = "https://github.com/lim8603/dot-logger", tag = "v0.1.0" }
```

### Added

- **Real-time safe capture.** `try_log` never blocks or allocates. A full
  queue is a counted rejection, and important records can have reserved slots.
- **Bounded resources.** Memory partitions, queues and file budgets are
  declared up front. Control-loop presets compute them, `plan()` previews
  them, and `calibrate` measures a device profile.
- **Native MCAP storage** (`dot-logger/standard/2`). Your own schema and
  encoding are kept, and metadata and loss counters are stored in the
  `.logger/meta` and `.logger/loss` channels. Files open in Foxglove,
  Lichtblick and the `mcap` tools without conversion.
- **Retention.** A ring limit, event protection before and after
  `mark_event`, and rotation at configuration revision boundaries.
- **Runtime control.** Change channel levels and sampling while running
  (`set_channel`, `dot-logger config apply`).
- **Recovery and loss reports.** `recover` restores interrupted files, and
  `loss` reports cumulative loss counters.
- **Remote access.** A gRPC server for subscribe, query, replay, status and
  completion, with batched and pipelined acknowledgements. The `dot-logger`
  CLI and the `dot-logger-client` crate use it.
- **Viewer.** `dot-logger-viewer` provides a Foxglove WebSocket bridge for
  live data and recorded replay, and a read-only operations dashboard.
- **Operations.** Telemetry windows (10/60/300 s), before/after comparison,
  and file system checks (`check`).
- **Power alarm path** (advanced, off by default). On a UPS warning,
  important files are synchronized in priority order.
- **Adapters.** `tracing` and `log` input, InfluxDB v2 output.

### Formats and protocols

| Interface | Version |
|---|---|
| File profile | `dot-logger/standard/2` |
| Offline manifest | `schema_version: 2` |
| Loss report | v2 |
| gRPC protocol | major 1 |
| Viewer WebSocket | `foxglove.websocket.v1` |
| Dashboard data | `dot_logger.viewer.dashboard.v1` |

Development formats are not accepted as input: the `dot-logger/standard/1`
file profile and version 1 manifests.

### Verification

- About 740 tests pass with Rust 1.96.0 and the 1.85 MSRV. The core
  ownership, routing, completion, retention, storage-fault, query and replay
  tests also run under Miri.
- CI passes on Ubuntu 24.04, Windows Server 2025 and macOS 15 with both Rust
  versions, including the real-time allocation tests.
- Performance and power-loss behavior were measured on one Linux x86_64 PC,
  with both a generic and a PREEMPT_RT kernel. The numbers in the README are
  not guarantees for other hardware.

### Known issues

- No 24-hour soak test and no production integration yet.
- Files have not been exchanged between different operating systems
  (written on one, read on another) in a test.
- The power alarm path was verified with simulated UPS signals and real file
  IO, not with a physical UPS. Durability on physical power loss is not
  claimed.
- Some `standard/2` files written before Schema/Channel registration moved
  ahead of the message chunks show a 1970 start in Lichtblick and fail its
  preload. `dot-logger recover` writes a compatible copy.
- Reopening after a write error, compressed writing, and remote pause or
  split are not implemented.
- The indirect dependency `paste` is reported as unmaintained (one RustSec
  warning, no vulnerability). `cargo audit` does not check the vendored MCAP
  crate (`vendor/mcap-0.26.0`); check MCAP advisories by hand.

[0.1.1]: https://github.com/lim8603/dot-logger/releases/tag/v0.1.1
[0.1.0]: https://github.com/lim8603/dot-logger/releases/tag/v0.1.0
