# dot-logger

Bounded logging and telemetry for robotics, in Rust.

Real-time threads hand records off without waiting or allocating. A default
worker writes them to standard [MCAP](https://mcap.dev) files that Foxglove,
Lichtblick and the `mcap` tools read directly. Every queue, pool, file and
reservation has a declared limit, and every record that does not make it is
counted rather than silently lost.

> **Status:** pre-release (0.1). The API may still change. Linux is the
> measured platform, and CI passes on Linux, Windows and macOS; see
> [Verification](#verification). Licensed under MIT or
> Apache-2.0; see [License](#license).

## Features

- **Real-time safe capture.** `try_log` never blocks or allocates. A full
  queue is a counted rejection. Important records can have reserved slots.
- **Bounded memory.** Memory partitions, queues and file budgets are declared
  up front. Presets compute them from a control-loop rate.
- **Standard files.** Records are stored as native MCAP with your own schema
  and encoding (for example protobuf, JSON or raw bytes). Loss counters and
  record metadata are stored in the same file.
- **Retention.** Ring retention, event windows that keep a period before and
  after a mark, rotation when the configuration changes, and recovery of
  interrupted files into new files.
- **Live and recorded access.** An optional gRPC server for subscribe, query,
  replay and status; the `dot-logger` CLI; and a Rust client library.
- **Visualization.** `dot-logger-viewer` streams live or recorded data to
  Foxglove or Lichtblick and serves a read-only operations dashboard.
- **Operations.** Runtime level and sampling changes, telemetry windows,
  before/after comparison, file-system checks, and an emergency flush on a
  power alarm.
- **Integrations.** `tracing`/`log` input and InfluxDB v2 output.

## Workspace

| Crate | What it is |
|---|---|
| [`dot-logger`](crates/dot-logger) | The library: capture, storage, query/replay, adapters and telemetry |
| [`dot-logger-cli`](crates/dot-logger-cli) | The `dot-logger` command: status, query, replay, tail, recover, loss, compare, checks |
| [`dot-logger-client`](crates/dot-logger-client) | Rust client for the gRPC server and for offline files |
| [`dot-logger-viewer`](crates/dot-logger-viewer) | Foxglove WebSocket bridge and operations dashboard |

## Getting started

Rust 1.85 or newer is required. dot-logger is not published on crates.io yet;
use it as a git dependency pinned to a release tag (see
[CHANGELOG.md](CHANGELOG.md)):

```toml
[dependencies]
dot-logger = { git = "https://github.com/lim8603/dot-logger", tag = "v0.1.0" }
```

dot-logger's dependencies are version ranges, so Cargo picks their newest
compatible releases. On Rust 1.85 itself, a project on edition 2021 should set
`resolver = "3"` in its `[package]` (or `[workspace]`) so Cargo picks releases
that still support 1.85; edition 2024 projects already do, and a committed
`Cargo.lock` keeps whatever versions it records.

`dot_logger::logger` is the default path. This is the complete L1 recipe,
[`examples/recipes/l1_file.rs`](examples/recipes/l1_file.rs):

<!-- recipe:l1_file.rs -->
```rust
//! L1 recipe: record typed messages from a real-time thread into
//! files and judge the shutdown. No support module, `unsafe` or lifetimes.
use dot_logger::logger::{EncodeError, Logger, Preset, Record, Retention, Schema};
use std::time::Duration;

/// Your message type and its encoding (not counted as integration code).
struct Pose {
    x: f32,
    y: f32,
}
impl Record for Pose {
    const SCHEMA: Schema = Schema::new(0x504f_5345, 1, "le-f32x2");
    fn encode(&self, out: &mut [u8]) -> Result<usize, EncodeError> {
        let out = out.get_mut(..8).ok_or(EncodeError)?;
        out[..4].copy_from_slice(&self.x.to_le_bytes());
        out[4..].copy_from_slice(&self.y.to_le_bytes());
        Ok(8)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logger = Logger::builder()
        .preset(Preset::control(1000)) // 1 kHz control loop: fills memory, sync and drop limits
        .file_output(std::env::temp_dir(), Retention::ring(64 << 20)) // an existing directory
        .channel::<Pose>("robot/pose")
        .writer("control")
        .start()?;
    let pose = logger.channel::<Pose>("robot/pose")?;
    let mut writer = logger.take_writer("control")?;
    let rt = std::thread::spawn(move || {
        for x in (0..1000).map(|i| i as f32) {
            // Never waits or allocates; a full queue is a counted rejection.
            let _ = writer.try_log(pose, &Pose { x, y: 0.0 });
        }
    });
    rt.join().map_err(|_| "control thread panicked")?;
    match logger.shutdown(Duration::from_secs(5))? {
        shutdown if shutdown.is_success() => Ok(()),
        shutdown => Err(format!("records not all synchronized: {shutdown:?}").into()),
    }
}
```
<!-- /recipe -->

```sh
cargo run --locked -p dot-logger --example recipe_l1_file
```

- `Preset::control(1000)` fills in memory, sync interval, shutdown deadline
  and writer quotas for a 1 kHz loop. Use `Budget` to set them yourself.
- `try_log` never waits. When the queue is full the record is rejected and
  counted.
- `shutdown(..).is_success()` is true only when every record was written and
  synchronized before the deadline.

## Recipes

Each recipe is a complete program kept small on purpose. A check script
counts its lines (after rustfmt, without `use`, comments, blank lines and the
message type) and the `dot_logger` names it uses.

| Recipe | Shows | Limit | Now |
|---|---|---|---|
| [`l1_file.rs`](examples/recipes/l1_file.rs) | Typed records from a real-time thread into files, judged shutdown | ≤ 20 lines, ≤ 6 names | 20, 6 |
| [`l2_rt_writers.rs`](examples/recipes/l2_rt_writers.rs) | Two real-time writers, zero-copy frames from a loaned slot | ≤ 40 lines, ≤ 10 names | 40, 7 |
| [`l3_tracing.rs`](examples/recipes/l3_tracing.rs) | L1 plus `tracing`/`log` input | ≤ 6 lines more than L1 | +3 |
| [`l3_remote.rs`](examples/recipes/l3_remote.rs) | L1 plus a loopback gRPC server | ≤ 6 lines more than L1 | +5 |

```sh
cargo run --locked -p dot-logger --example recipe_l2_rt_writers
cargo run --locked -p dot-logger --features tracing-adapter --example recipe_l3_tracing
cargo run --locked -p dot-logger --features remote --example recipe_l3_remote
```

The L3 remote recipe prints its server address and exits after writing. To
try subscriptions, use the longer-running `live_host` example described in
[Live view and dashboard](#live-view-and-dashboard).

## Options on the default path

Add these only when you need them. They are documented on
`LoggerBuilder`, `ChannelOptions`, `WriterQuota` and `EventWindow`.

```rust
let log_dir = project_root.join(".logger");
std::fs::create_dir_all(&log_dir)?;
let logger = Logger::builder()
    .budget(budget)
    .file_output(&log_dir, Retention::ring(4 << 30)
        .events(EventWindow::new(Duration::from_secs(30), Duration::from_secs(10))
            .hold(Duration::from_secs(3600))
            .limits(8, 512 << 20)))                       // keep 30 s before and 10 s after a mark
    .channel_with::<Status>("robot/status", ChannelOptions::new()
        .minimum(Level::Info)                             // below Info is skipped
        .important_from(Level::Warn))                     // Warn and above are Important
    .writer_with("control", WriterQuota::small(64, 256).important(8)) // reserved Important slots
    .start()?;
writer.try_log(alarm.important(), &msg)?;                 // mark one record Important
let revision = logger.set_channel(alarm, Level::Warn, 3)?; // change level and Normal sampling
let mark = logger.mark_event()?;                          // protect [-30 s, +10 s] around now
```

- **Important records** are protected from saturation only when the writer
  reserves slots (`WriterQuota::important`). Using `important_from` without a
  reserve makes `plan()` report `PlanWarning::NoImportantReserve`.
- **Level and sampling** changes return the posted configuration revision.
  Normal records keep the first and every `sample_every`-th record (1 keeps
  all); Important records are never sampled, but the minimum level applies to
  them too. Files rotate at the revision boundary.
- **Idle writers** apply a new configuration on their next `try_log`. A writer
  you took that stays idle can make a later change return `ConfigError::Busy`;
  call `writer.refresh()` on its thread from time to time. A posted revision
  does not mean the files are synchronized.
- **Event windows:** call `mark_event` on an ordinary thread (a real-time
  thread can signal one). Protection ends after `hold` or on `release_event`.

A host application can pass a `.logger/` directory under its own project root,
as above; the name dot-logger comes from that quiet `.logger/` folder.

## Live view and dashboard

Enable the gRPC server on the default path with `LoggerBuilder::remote`. It
binds to loopback by default, on a port chosen by the system; read the actual
address from `logger.remote_addr()`. Remote hosts need HTTPS and a token.

The viewer turns that server into a Foxglove WebSocket and an operations
dashboard. With the demo host:

```sh
cargo run --locked -p dot-logger-viewer --example live_host -- ./viewer-demo
# In another terminal, use the URL the demo printed:
cargo run --locked -p dot-logger-viewer -- \
    --upstream http://127.0.0.1:PORT --catalog ./viewer-demo/viewer-catalog.json \
    --dashboard 127.0.0.1:0
```

Open the printed `ws://` URL in Foxglove or Lichtblick and the `http://` URL
in a browser. See the [viewer README](crates/dot-logger-viewer/README.md).

From a terminal, the CLI subscribes to the same server. `--record-bytes` must
fit the host's record limit (64 KiB in the demo):

```sh
cargo run -p dot-logger-cli -- tail --endpoint http://127.0.0.1:PORT --record-bytes 65536 --limit 10
```

See the [CLI README](crates/dot-logger-cli/README.md).

## Two layers

- **Default path — `logger`:** a builder, a default worker, `'static` writers
  that can move between threads, presets and machine calibration profiles
  (`calibrate` feature), health and a shutdown report.
- **Advanced path — the other modules:** memory partitions (`config`),
  capture (`capture`), fan-out (`routing`), completion tracking
  (`completion`), sync, shutdown and power-alarm handling (`control`), MCAP
  files, retention and recovery (`storage`), query and replay (`query`,
  `replay`), adapters (`adapters`: gRPC, InfluxDB, tracing), operational
  telemetry and reports (`telemetry`) and checks (`diagnostics`). The default
  path is built on these.

Each module documents its contract in rustdoc:

```sh
cargo doc --open -p dot-logger --all-features
```

## Cargo features

No feature is enabled by default.

| Feature | Adds |
|---|---|
| `remote` | gRPC server and client types (tonic, Tokio) |
| `influxdb` | Bounded InfluxDB v2 writes and numeric queries (reqwest, Tokio) |
| `tracing-adapter` | `tracing` and `log` input |
| `compression-zstd` | zstd support in the MCAP dependency (dot-logger itself writes uncompressed chunks) |
| `calibrate` | Machine calibration and JSON profiles for presets |

## Examples

| Example | Shows |
|---|---|
| [`dot_probe_facade`](examples/dot_probe_facade.rs) | A robot-application integration on the default path only: `tracing` logs, 1 kHz joint telemetry and 30 Hz camera frames into files plus a gRPC server. Modeled on dot-probe, the first application that uses dot-logger |
| [`dot_probe_*`](crates/dot-logger-client/examples) | Advanced-path integrations modeled on the same application, including InfluxDB output and a configuration before/after comparison; see the [client README](crates/dot-logger-client/README.md) |
| [`general_logs`](examples/general_logs.rs) | `tracing`/`log` input on the advanced path |
| [`ups_simulation`](crates/dot-logger/examples/ups_simulation.rs) | Power-alarm and shutdown-deadline simulation (`--plan`, `--run DIR`) |
| [`soak`](crates/dot-logger/examples/soak.rs) | Long-run load with resource samples, and an external power cut: alarm, then a kill 3 s later (Linux) |
| [`os_exchange`](crates/dot-logger/examples/os_exchange.rs) | Files written on one OS and checked record by record on another (`write DIR`, `verify DIR...`) |
| [`live_host`](crates/dot-logger-viewer/examples/live_host.rs), [`operations_host`](crates/dot-logger-viewer/examples/operations_host.rs) | Demo hosts for the viewer and dashboard |
| [`tests/api-consumer`](tests/api-consumer/src/main.rs) | Using the public API from a separate crate |

## Verification

- **Tests.** The workspace has about 740 unit and integration tests, run with
  Rust 1.96.0 and the 1.85 MSRV. The library's unit tests and its pool,
  routing, completion, control, retention, storage-fault, query and replay
  tests also run under Miri.
- **Platform.** Measured on one Linux x86_64 PC, with both a generic and a
  PREEMPT_RT kernel. CI runs the tests, including the real-time allocation
  tests, on Ubuntu 24.04, Windows Server 2025 and macOS 15 with both Rust
  versions, and they pass. A CI job also writes a log on each of the three
  systems and reads all three logs back on each of them. Performance was not
  measured on Windows or macOS.
- **Power loss** was verified with simulated UPS signals and real file IO,
  not with a physical UPS.
- **24-hour soak** on the PREEMPT_RT kernel: 432 million records at about
  2.5 MB/s into a 16 GiB ring, with no loss, no rejected record, no missed
  control-loop deadline and every remaining file read back intact. Open files
  and the live heap stayed flat; the resident size rose 1% over 22 hours as
  the system allocator kept freed pages. It covered recording and rotation;
  queries and remote streaming were not running at the same time.
- **Not yet done:** a production integration.
- **Dependencies** are audited with RustSec. One indirect dependency, `paste`,
  is reported as unmaintained.

Results measured on that PC over a wired 1 GbE LAN (not guarantees for other
hardware):

| Scenario | Result |
|---|---|
| 256-byte records, 10,000 per second, 60,000 in total | All acknowledged by the receiver; acknowledgement p99 4.12 ms |
| 8 MiB records over gRPC | 115.9–116.4 MB/s, 99–100 % of a raw TCP baseline |

Latency of `try_log` on the same PC (AMD Ryzen 7 H 255, SMT and boost on,
`performance` CPU governor). The calling thread runs at `SCHED_FIFO` 70,
pinned to one core, with its memory locked, and records go to a 1 GiB ring on
tmpfs. Each figure is the median of three runs; the maximum is the largest of
the three. Call times include the two clock reads around the call, which take
about 20 ns.

| Measurement | PREEMPT_RT kernel: p50 / p99 / p99.99 / max | Generic kernel: p50 / p99 / p99.99 / max |
|---|---|---|
| One 256-byte call in a warm loop (10 million calls) | 251 / 375 / 4,031 / 49,361 ns | 263 / 375 / 3,711 / 122,739 ns |
| One 256-byte call after a 32 MiB cache sweep (20,000 calls) | 767 / 1,407 / 6,655 / 19,466 ns | 831 / 1,311 / 6,271 / 8,476 ns |
| 50 calls in one cycle of a 1 kHz control loop (60,000 cycles) | 11.0 / 15.1 / 92.2 / 116.7 µs | 11.0 / 14.6 / 86.0 / 115.6 µs |
| 50 calls in one cycle of a 10 ms control loop (6,000 cycles) | 11.5 / 17.9 / 102.4 / 106.2 µs | 11.8 / 22.0 / 31.1 / 34.7 µs |
| Wake-up delay of the 10 ms loop, with the 50 calls and another thread logging 6.5 MB/s to an SSD | 23.0 / 112.6 / 116.7 / 117.3 µs | 23.0 / 112.6 / 116.7 / 195.8 µs |

For single calls the two kernels agree up to p99.99; the difference is in the
maximum. In the warm loop, five of the six generic-kernel results (three
sizes, two governors) exceeded 85 µs, against at most 54 µs on PREEMPT_RT. In
about one cycle in a thousand, the 50 calls of a cycle take 75–115 µs instead
of about 11 µs, on both kernels. This is not the cost of `try_log`: while the
machine is otherwise idle, the measuring core runs about nine times slower for
about 0.5 ms, mostly every 600 ms, and the same episodes appear in the loop's
response time when it logs nothing
([EV-130](.cowork/05_verification/verification_evidence.md#ev-130)). In runs
while other programs were active, the episodes did not occur and the 1 kHz row
had a p99.9 of 32–35 µs and a maximum of 48–54 µs (generic kernel,
`powersave`). With 6,000 cycles per run, the 10 ms loop's p99.99 is close to
each run's maximum, so it depends on whether a run met one of those cycles.
Logging does not change the loop's wake-up delay, which comes from the kernel
and the CPU's idle states. Adding the SSD load raises the 1 kHz row's p99 by
up to 20 % and its maximum to about 145 µs on both kernels. With the default
`powersave` governor, call p50 is 15–45 % higher and p99 about 40 % higher.
Every row, both governors, the rejection path and multiple writers are in
[EV-129](.cowork/05_verification/verification_evidence.md#ev-129), together
with the raw results and the commands to reproduce them.

## Development

The development toolchain is Rust 1.96.0 (`rust-toolchain.toml`); the minimum
supported version is 1.85. `Cargo.lock` pins dependencies.

```sh
cargo test --workspace --all-features --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
python3 scripts/check-feature-matrix.py --toolchain 1.85.0
python3 scripts/check-integration-surface.py   # needs nightly-2026-10-26
cargo run --locked -p dot-logger-api-consumer
```

`check-integration-surface.py` guards the `logger` API surface and the recipe
limits. When it reports a surface change, review it, update the snapshot with
`--bless`, and commit the snapshot with the change.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

The vendored [`mcap`](vendor/mcap-0.26.0) 0.26.0 crate is MIT-licensed by
Foxglove Technologies Inc; its license text is in
[`vendor/mcap-0.26.0/LICENSE`](vendor/mcap-0.26.0/LICENSE), and
[`VENDORED.md`](vendor/mcap-0.26.0/VENDORED.md) lists the local changes.
