//! Bounded robotics logging: startup contracts and small-record capture.
//!
//! Configuration validation and capture allocation are management operations.
//! Small capture is an isolated ingress stage; raw pools, dispatch, completion,
//! worker startup, storage, and full shutdown belong to subsequent tasks.

pub mod capture;
pub mod config;
pub mod platform;
