//! Bounded robotics logging: startup contracts and bounded small/raw capture.
//!
//! Configuration validation and capture allocation are management operations.
//! Small/raw capture is an isolated ingress stage; dispatch, completion,
//! worker startup, storage, and full shutdown belong to subsequent tasks.

pub mod capture;
pub mod config;
pub mod platform;
pub mod pool;
