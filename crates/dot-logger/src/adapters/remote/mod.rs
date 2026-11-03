//! Bounded gRPC, host-driven management and receiving-application ACKs.
//! All operations are general-path. The host owns the runtime and actual IO.
mod hub;
mod receiver;
mod server;
mod types;
mod wire;
pub use hub::{Command, CommandInbox, PendingAck, RemoteDelivery, RemoteHub};
pub use receiver::{Reassembler, ReceivedRecord};
pub use server::{REASON_KEY, RemoteServer};
pub use types::*;
#[cfg(test)]
mod tests;
