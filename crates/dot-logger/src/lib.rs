//! Bounded robotics logging: tracked capture, routing, completion, and control.
//!
//! # Two layers
//!
//! - [`logger`] is the default path: one builder, a default worker and owned
//!   `'static` writers. Start here.
//! - The other modules are the advanced path the facade is built on: explicit
//!   memory partitions ([`config`]), capture, routing, completion, control,
//!   storage, query/replay, adapters and telemetry. Each module documents its
//!   own contract. There, configuration validation and capture allocation are
//!   management operations; capture, routing, completion and shutdown are
//!   explicitly driven; worker startup, file IO and device guarantees belong
//!   to the host or an adapter.
//!
//! # Quickstart
//!
//! The L1 recipe (`examples/recipes/l1_file.rs`): typed records from a
//! real-time thread into files, then a judged shutdown.
//!
//! ```no_run
#![doc = include_str!("../../../examples/recipes/l1_file.rs")]
//! ```
//!
//! # Recipes (NFR-008)
//!
//! | Recipe in `examples/recipes/` | Shows | Limit |
//! |---|---|---|
//! | `l1_file.rs` | typed records from an RT thread, judged shutdown | ≤ 20 lines, ≤ 6 concepts |
//! | `l2_rt_writers.rs` | two RT writers, a zero-copy raw loan | ≤ 40 lines, ≤ 10 concepts |
//! | `l3_tracing.rs`, `l3_remote.rs` | L1 plus `tracing`/`log` or a gRPC server | ≤ 6 more lines |
//!
//! `scripts/check-integration-surface.py` enforces these limits and the
//! reviewed [`logger`] surface.
//!
//! # Features
//!
//! None is on by default.
//!
//! | Feature | Adds |
//! |---|---|
//! | `remote` | gRPC server and client types (tonic, Tokio) |
//! | `influxdb` | bounded InfluxDB v2 writes and numeric queries (reqwest, Tokio) |
//! | `tracing-adapter` | `tracing`/`log` input |
//! | `compression-zstd` | MCAP zstd support in the dependency (the product writer is uncompressed) |
//! | `calibrate` | explicit machine calibration and JSON profiles for presets |

pub mod capture;
pub mod config;
pub mod platform;
pub mod pool;

/// Bounded management-path fanout, retries, and output retention accounting.
pub mod routing;

/// Bounded completion admission, observations, queries, and waits.
pub mod completion;

/// Explicit synchronization and deadline-based normal shutdown coordination.
pub mod control;

/// Bounded general-path MCAP envelopes, file writers and chunk indexes.
pub mod storage;

/// Frozen complete-chunk snapshots, bounded query pages and original blob reads.
pub mod query;

/// Caller-driven offline replay, bounded prefetch and generation-based control.
pub mod replay;

/// Optional general-path output and network adapters.
pub mod adapters;

/// Fixed producer counters and bounded caller-driven operational observations.
pub mod telemetry;

/// Load-free diagnostic plans and caller-driven physical write admission.
pub mod diagnostics;

/// Default execution layer: builder, default worker and owned writers (ADR-012).
pub mod logger;

/// Explicit machine calibration and JSON profiles for presets (TASK-028 S2).
#[cfg(feature = "calibrate")]
pub mod calibrate;

/// Facade types for the common integration path.
pub mod prelude {
    pub use crate::logger::{
        Budget, Channel, EncodeError, LogError, Logged, Logger, Preset, Record, Retention, Schema,
        Shutdown, Sizing, Writer, WriterQuota,
    };
}
