//! dot-probe-specific check and bridge code. It is not part of the library
//! (TASK-033 S3, Human decision 2026-11-15); examples and tests include it
//! with `#[path]`, and integration tests also compile it with fault hooks.
#![allow(dead_code, unused_imports)]
#[path = "dot_probe/bridge.rs"]
mod implementation;
pub use implementation::*;
#[path = "dot_probe/check.rs"]
mod check;
pub use check::SnapshotCheck;
