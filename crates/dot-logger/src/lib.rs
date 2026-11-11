//! Bounded robotics logging: tracked capture, routing, completion, and control.
//!
//! Configuration validation and capture allocation are management operations.
//! Capture, routing, completion, and normal shutdown coordination are explicitly
//! driven. Backend worker startup, file IO, and device guarantees are adapter tasks.

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
