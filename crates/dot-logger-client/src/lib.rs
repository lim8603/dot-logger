//! General-path Rust client building blocks for dot-logger.
//! The optional `remote` feature provides connection-scoped receiver ownership.
//! Transport, runtime and CLI are separate layers; none is started implicitly.

#[cfg(feature = "remote")]
mod receiver;
#[cfg(feature = "remote")]
pub use receiver::{
    Acknowledgement, ClientError, ClientReceiver, ConnectionId, PendingRecord, ReceiverConfig,
};
