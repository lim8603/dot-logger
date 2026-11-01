//! Optional adapters; core capture never starts their runtimes or IO.
#[cfg(feature = "influxdb")]
pub mod influxdb;
#[cfg(feature = "remote")]
pub mod remote;
#[cfg(feature = "tracing-adapter")]
pub mod tracing;
