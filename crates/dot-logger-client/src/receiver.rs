use dot_logger::{
    adapters::remote::{Ack, Chunk, Reassembler, ReceivedRecord, RemoteError},
    capture::CapturedRecord,
    query::QueryBudget,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// Receiver storage limits; validation and reservations use the core authority.
#[derive(Clone, Copy, Debug)]
pub struct ReceiverConfig {
    /// Concurrent incomplete plus complete-but-unaccepted records; also gate slots.
    pub records: usize,
    /// Maximum payload bytes per record.
    pub record_bytes: usize,
    /// Maximum incomplete/unaccepted record age.
    pub ttl: Duration,
}

/// Opaque identity of one transport connection, retained by its transport owner.
/// Old identities cannot match a new connection, even if server IDs are reused.
#[derive(Clone)]
pub struct ConnectionId(Arc<()>);

/// Verified record retaining its core permit and originating connection identity.
/// Only [`ClientReceiver::handoff`] can produce its acknowledgement.
pub struct PendingRecord {
    connection: ConnectionId,
    inner: ReceivedRecord,
}

/// Accepted receipt still tied to its original transport connection.
/// Extract it with [`ClientReceiver::take_ack`] immediately before transport send.
pub struct Acknowledgement {
    connection: ConnectionId,
    ack: Ack,
}

/// Client lifecycle failures; messages contain no credential or payload data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientError {
    /// Local cancellation; an already posted host operation may still be running.
    Cancelled,
    /// Consumer rejected a verified record; no receiving acknowledgement was sent.
    ConsumerRejected,
    /// Consumer accepted, but the receiving acknowledgement could not be confirmed.
    AcknowledgementUnknown,
    /// No current connection exists.
    Disconnected,
    /// End the existing connection before starting another.
    AlreadyConnected,
    /// The input belongs to another or an earlier connection.
    StaleConnection,
    /// Structured core receiver failure, including gate or reservation limits.
    Remote(RemoteError),
}
impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => {
                f.write_str("operation cancelled; posted host IO may still be running")
            }
            Self::ConsumerRejected => f.write_str("consumer rejected the record; no ACK sent"),
            Self::AcknowledgementUnknown => {
                f.write_str("consumer accepted; acknowledgement outcome is unknown")
            }
            Self::Disconnected => f.write_str("no active receiver connection"),
            Self::AlreadyConnected => f.write_str("end the active connection before reconnecting"),
            Self::StaleConnection => f.write_str("input belongs to a stale or foreign connection"),
            Self::Remote(error) => write!(f, "receiver: {error}"),
        }
    }
}
impl std::error::Error for ClientError {}

struct LiveConnection {
    id: ConnectionId,
    receiver: Reassembler,
}

/// General-path receiver state owned alongside a transport connection.
/// Caller-owned identity/receipt metadata and transport buffers need their own
/// headroom; payload reservations remain in the shared [`QueryBudget`].
/// This type opens no sockets and starts no worker or runtime.
#[derive(Default)]
pub struct ClientReceiver {
    current: Option<LiveConnection>,
}
impl ClientReceiver {
    /// Create disconnected state without reserving payload storage.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach a fresh receiver after the caller has opened a real connection.
    /// A failed reservation leaves disconnected state intact. Do not call this
    /// merely to reset generation gates on a still-active transport connection.
    pub fn begin_connection(
        &mut self,
        budget: &QueryBudget,
        config: ReceiverConfig,
    ) -> Result<ConnectionId, ClientError> {
        if self.current.is_some() {
            return Err(ClientError::AlreadyConnected);
        }
        let receiver = Reassembler::new(budget, config.records, config.record_bytes, config.ttl)
            .map_err(ClientError::Remote)?;
        // A retained old identity/record keeps its Arc allocation alive, so pointer
        // reuse cannot make an old connection equal a fresh one.
        let id = ConnectionId(Arc::new(()));
        self.current = Some(LiveConnection {
            id: id.clone(),
            receiver,
        });
        Ok(id)
    }

    fn connection(&self, id: &ConnectionId) -> Result<&LiveConnection, ClientError> {
        let current = self.current.as_ref().ok_or(ClientError::Disconnected)?;
        if !Arc::ptr_eq(&current.id.0, &id.0) {
            return Err(ClientError::StaleConnection);
        }
        Ok(current)
    }

    /// Stop routing this connection's fragments and release its incomplete frames.
    /// Retained records keep their actual payload permit until they are dropped.
    /// This is local cleanup, not a claim that remote delivery was cancelled.
    pub fn end_connection(&mut self, id: &ConnectionId) -> Result<(), ClientError> {
        self.connection(id)?;
        self.current = None;
        Ok(())
    }

    /// Validate a fragment from the specified transport, rejecting old connections
    /// before decoding/reassembly state is mutated. Transport decode storage is
    /// separately budgeted; the core also checks fragment and record size limits.
    pub fn push(
        &mut self,
        id: &ConnectionId,
        chunk: Chunk,
        now: Instant,
    ) -> Result<Option<PendingRecord>, ClientError> {
        let current = self.current.as_mut().ok_or(ClientError::Disconnected)?;
        if !Arc::ptr_eq(&current.id.0, &id.0) {
            return Err(ClientError::StaleConnection);
        }
        current
            .receiver
            .push(chunk, now)
            .map(|record| {
                record.map(|inner| PendingRecord {
                    connection: id.clone(),
                    inner,
                })
            })
            .map_err(ClientError::Remote)
    }

    /// Advance the bounded generation gate for this connection's subscription.
    /// Gate exhaustion stays an explicit `Remote(Limit)`; gates are never evicted.
    pub fn discard_generations(
        &mut self,
        id: &ConnectionId,
        subscription: u64,
        generation: u64,
    ) -> Result<(), ClientError> {
        self.connection(id)?;
        // The preceding shared check establishes identity; no user code runs
        // before this exclusive lookup, so the current connection cannot change.
        let current = self.current.as_mut().ok_or(ClientError::Disconnected)?;
        current
            .receiver
            .discard_generations(subscription, generation)
            .map_err(ClientError::Remote)
    }

    /// Accept only a record from the current connection and current core generation.
    /// Rejection/expiry/stale identity returns the original owner without an ACK.
    /// The borrow prevents local disconnect while the synchronous consumer runs.
    // Preserve the bounded owner instead of allocating an error box.
    #[allow(clippy::result_large_err)]
    pub fn handoff(
        &self,
        record: PendingRecord,
        consumer: impl FnOnce(&CapturedRecord, &[u8]) -> bool,
    ) -> Result<Acknowledgement, PendingRecord> {
        if self.connection(&record.connection).is_err() {
            return Err(record);
        }
        let PendingRecord { connection, inner } = record;
        match inner.handoff(consumer) {
            Ok(ack) => Ok(Acknowledgement { connection, ack }),
            Err(inner) => Err(PendingRecord { connection, inner }),
        }
    }

    /// Check connection identity immediately before sending the native ACK on that
    /// exact transport. A reconnect between handoff and extraction rejects it.
    /// Already extracted/sent ACKs and accepted consumer data cannot be revoked.
    pub fn take_ack(
        &self,
        id: &ConnectionId,
        receipt: Acknowledgement,
    ) -> Result<Ack, ClientError> {
        self.connection(id)?;
        if !Arc::ptr_eq(&receipt.connection.0, &id.0) {
            return Err(ClientError::StaleConnection);
        }
        Ok(receipt.ack)
    }
}
