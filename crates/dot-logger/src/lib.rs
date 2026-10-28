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
