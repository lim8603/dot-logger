//! Shared production backend, also compiled with fault hooks by integration tests.
#[path = "../../src/diagnostics/bridge.rs"]
mod implementation;
pub use implementation::*;
