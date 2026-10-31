//! General-path Rust client building blocks for dot-logger.
//! The optional `remote` feature provides connection-scoped receiver ownership.
//! Transport, runtime and CLI are separate layers; none is started implicitly.
mod cancel;
pub use cancel::Cancellation;
mod offline;
pub use offline::{OfflineConfig, OfflineSource, open_snapshot};
#[cfg(feature = "remote")]
mod codec;
#[cfg(feature = "remote")]
mod transport;
#[cfg(feature = "remote")]
mod wire;
#[cfg(feature = "remote")]
pub use transport::{RemoteClient, RemoteConfig, RemoteEvent, RemoteSubscription};

#[cfg(feature = "remote")]
mod receiver;
#[cfg(feature = "remote")]
pub use receiver::{
    Acknowledgement, ClientError, ClientReceiver, ConnectionId, PendingRecord, ReceiverConfig,
};
