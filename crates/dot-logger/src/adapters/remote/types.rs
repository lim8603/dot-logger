use crate::{
    capture::*,
    completion::*,
    query::Query,
    replay::{ReplayState, Speed},
};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};
/// Incompatible majors are rejected before host dispatch.
pub const PROTOCOL_MAJOR: u32 = 1;
/// Maximum original bytes per fragment, excluding envelope.
pub const MAX_CHUNK_BYTES: usize = 256 * 1024;
/// Fixed admission limits shared by server and host inbox.
#[derive(Clone, Debug)]
pub struct RemoteLimits {
    /// Concurrent TCP connections, including TLS handshakes.
    pub connections: usize,
    /// Concurrent subscription streams globally.
    pub subscriptions: usize,
    /// Outstanding commands, including timed-out commands still owned by host.
    pub commands: usize,
    /// Maximum encoded message, at least 2048 bytes and at most 16 MiB.
    pub message_bytes: usize,
    /// Maximum single original payload.
    pub record_bytes: usize,
    /// Queued plus unacknowledged records per subscription.
    pub subscription_records: usize,
    /// Queued plus unacknowledged payload bytes per subscription.
    pub subscription_bytes: usize,
    /// Record age limit and negotiated idle stream lease.
    pub max_age: Duration,
    /// Host response timeout; does not cancel actual IO.
    pub request_timeout: Duration,
}
impl RemoteLimits {
    /// Conservative adapter buffer reservation. Runtime/TLS/HTTP2 allocations
    /// and host callback buffers require separately declared worker headroom.
    pub fn reservation(&self) -> Result<usize, RemoteError> {
        if self.connections == 0
            || self.subscriptions == 0
            || self.commands == 0
            || self.connections > 1024
            || self.subscriptions > 1024
            || self.commands > 1024
            || self.message_bytes < 2048
            || self.message_bytes > 16 * 1024 * 1024
            || self.record_bytes == 0
            || self.subscription_records == 0
            || self.subscription_bytes < self.record_bytes
            || self.max_age.is_zero()
            || self.request_timeout.is_zero()
            || self.max_age > Duration::from_secs(86400)
            || self.request_timeout > Duration::from_secs(3600)
        {
            return Err(RemoteError::InvalidInput);
        }
        self.subscription_records
            .checked_mul(1024)
            .and_then(|n| n.checked_add(self.subscription_bytes))
            .and_then(|n| n.checked_add(self.message_bytes.checked_mul(2)?))
            .and_then(|n| n.checked_mul(self.subscriptions))
            .and_then(|n| {
                n.checked_add(
                    self.commands
                        .checked_mul(self.message_bytes.checked_mul(4)?)?,
                )
            })
            .and_then(|n| {
                n.checked_add(
                    self.connections
                        .checked_mul(self.commands.checked_add(self.subscriptions)?)?
                        .checked_mul(self.message_bytes)?
                        .checked_mul(2)?,
                )
            })
            .and_then(|n| n.checked_add(self.connections.checked_mul(16384)?))
            .and_then(|n| n.checked_add(16384))
            .ok_or(RemoteError::Limit)
    }
    pub(crate) fn chunk_bytes(&self) -> usize {
        MAX_CHUNK_BYTES.min(self.message_bytes - 1024)
    }
}
/// Observation and mutation permissions are distinct.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Permission {
    /// Query, replay, subscriptions and completion observations.
    Read,
    /// Explicit configuration/check commands and reads.
    Manage,
}
/// Bounded bearer credential; deliberately not Debug.
pub struct AccessToken {
    pub(crate) value: String,
    pub(crate) permission: Permission,
}
impl AccessToken {
    /// Register 16..256 printable non-whitespace bytes.
    pub fn new(value: String, permission: Permission) -> Result<Self, RemoteError> {
        if !(16..=256).contains(&value.len()) || !value.bytes().all(|c| c.is_ascii_graphic()) {
            return Err(RemoteError::InvalidInput);
        }
        Ok(Self { value, permission })
    }
}
/// Certificate/private key PEM; secrets are never formatted.
pub struct TlsIdentity {
    pub(crate) certificate: Vec<u8>,
    pub(crate) private_key: Vec<u8>,
}
impl TlsIdentity {
    /// Explicit credentials, parsed before socket admission.
    pub fn new(certificate: Vec<u8>, private_key: Vec<u8>) -> Result<Self, RemoteError> {
        if certificate.is_empty()
            || private_key.is_empty()
            || certificate.len() > 65536
            || private_key.len() > 65536
        {
            return Err(RemoteError::InvalidInput);
        }
        Ok(Self {
            certificate,
            private_key,
        })
    }
}
/// Explicit listener configuration; default is disabled, loopback, read only.
pub struct ServerConfig {
    /// Explicit enable switch.
    pub enabled: bool,
    /// Literal bind address.
    pub address: SocketAddr,
    /// Anonymous read permission on loopback only.
    pub anonymous_loopback_read: bool,
    /// Bounded static credentials.
    pub tokens: Vec<AccessToken>,
    /// Mandatory for every non-loopback listener.
    pub tls: Option<TlsIdentity>,
}
impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
            anonymous_loopback_read: true,
            tokens: Vec::new(),
            tls: None,
        }
    }
}
impl ServerConfig {
    /// Reject insecure external binds and duplicate credentials.
    pub fn validate(&self) -> Result<(), RemoteError> {
        if self.tokens.len() > 16
            || self
                .tokens
                .iter()
                .enumerate()
                .any(|(i, t)| self.tokens[..i].iter().any(|p| p.value == t.value))
        {
            return Err(RemoteError::InvalidInput);
        }
        if !self.address.ip().is_loopback() && (self.tls.is_none() || self.tokens.is_empty()) {
            return Err(RemoteError::PermissionDenied);
        }
        Ok(())
    }
}
/// Fixed live overload policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OverflowPolicy {
    /// Drop new matching records, then report their contiguous stream positions.
    Gap,
    /// Disconnect only this subscription; unacknowledged records become Unknown.
    Disconnect,
}
/// Native subscription filter; original payloads are explicit opt-in.
#[derive(Clone, Copy, Debug)]
pub struct SubscriptionFilter {
    /// Optional channel.
    pub channel: Option<u32>,
    /// Minimum severity.
    pub minimum_level: Level,
    /// Include originals.
    pub originals: bool,
    /// Overload policy.
    pub overflow: OverflowPolicy,
}
/// Bounded original fragment; validation and consumer handoff precede ACK.
#[derive(Clone, Debug)]
pub struct Chunk {
    /// Owning subscription.
    pub subscription: u64,
    /// Stream-local transfer position.
    pub transfer: u64,
    /// Replay generation, zero for live if selected by host.
    pub generation: u64,
    /// Full original identity and clocks.
    pub record: CapturedRecord,
    /// Repeated complete payload length.
    pub total: u64,
    /// Exact next offset.
    pub offset: u64,
    /// CRC32 of complete payload.
    pub checksum: u32,
    /// Fragment bytes, no more than 256 KiB.
    pub data: Vec<u8>,
    /// Original/raw classification from host.
    pub original: bool,
    /// Earlier mono time than previously sent stream record.
    pub late: bool,
}
impl Chunk {
    /// Build one complete native fragment for receiver integrations.
    pub fn single(
        subscription: u64,
        transfer: u64,
        generation: u64,
        record: CapturedRecord,
        data: &[u8],
        original: bool,
    ) -> Self {
        Self {
            subscription,
            transfer,
            generation,
            record,
            total: data.len() as u64,
            offset: 0,
            checksum: checksum(data),
            data: data.to_vec(),
            original,
            late: false,
        }
    }
}
/// Receiver acknowledgment produced after a successful consumer handoff.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ack {
    /// Owning subscription.
    pub subscription: u64,
    /// Exact stream transfer.
    pub transfer: u64,
    /// Assembled generation.
    pub generation: u64,
    /// Original receipt.
    pub receipt: CaptureReceipt,
}
/// Terminal state of one offered record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AckState {
    /// No receiving-application ACK yet.
    Pending,
    /// Receiver verified and handed off the record.
    Acknowledged,
    /// Stream ended/expired before ACK.
    Unknown,
    /// Explicit overflow discard.
    Dropped,
}
/// Native page item; blob identities are opaque capabilities, never paths.
#[derive(Clone, Debug)]
pub struct RemoteItem {
    /// Original metadata.
    pub record: CapturedRecord,
    /// Inline bytes, empty for nonempty blob references.
    pub inline: Vec<u8>,
    /// Host blob capability; zero for inline data.
    pub blob: u64,
}
/// Remote replay transition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayAction {
    /// Start with a snapshot handle.
    Start,
    /// Freeze time.
    Pause,
    /// Continue time.
    Resume,
    /// Seek and discard old queued generations.
    Seek,
    /// Stop/close reader.
    Stop,
    /// Change positive rational rate.
    Speed,
}
/// Bounded host operations; backend IO stays host-owned.
#[derive(Clone, Debug)]
pub enum ManagementRequest {
    /// Observe current session/revision.
    GetStatus,
    /// Read a frozen page; zero cursor starts, zero snapshot asks host to open.
    Query {
        /// Snapshot identity.
        snapshot: u64,
        /// Opaque continuation.
        cursor: u64,
        /// Native filter.
        query: Query,
    },
    /// Read a bounded subrange of a blob capability.
    ReadBlob {
        /// Snapshot identity.
        snapshot: u64,
        /// Blob capability.
        blob: u64,
        /// Payload offset.
        offset: u64,
        /// Maximum requested bytes.
        length: u32,
    },
    /// Transition a host replay.
    Replay {
        /// Replay/snapshot capability.
        handle: u64,
        /// Transition.
        action: ReplayAction,
        /// Generation precondition.
        generation: u64,
        /// Seek/start mono time.
        from: u64,
        /// Positive rate.
        speed: Speed,
    },
    /// Observe completion; does not initiate sync.
    GetCompletion {
        /// Original receipt.
        receipt: CaptureReceipt,
        /// Fixed target.
        target: u32,
        /// Requested boundary.
        boundary: Boundary,
    },
    /// Manage permission required; host validates its own patch schema.
    ApplyConfig {
        /// Revision precondition.
        base_revision: u64,
        /// Bounded configuration patch.
        patch: Vec<u8>,
    },
    /// Load-free preview of a pre-registered host check capability.
    PlanCheck {
        /// Plan capability; no path or budgets come from the remote caller.
        plan: u64,
    },
    /// Manage permission required; pre-registered bounded plan only.
    RunCheck {
        /// Plan capability.
        plan: u64,
    },
    /// Compare host-owned measurements.
    Compare {
        /// Earlier identity.
        before: u64,
        /// Later identity.
        after: u64,
    },
}
/// Native reply, checked against encoded response budget before transport.
#[derive(Clone, Debug)]
pub enum ManagementReply {
    /// Session status.
    Status {
        /// Session identity.
        session: u128,
        /// Current revision.
        revision: u64,
    },
    /// Bounded page with explicit blob references.
    Page {
        /// Snapshot capability.
        snapshot: u64,
        /// Zero at final page.
        cursor: u64,
        /// Bounded items.
        items: Vec<RemoteItem>,
    },
    /// Bounded blob range.
    Blob(Vec<u8>),
    /// Confirmed local transition; not an in-flight send cancellation.
    Replay {
        /// Current generation.
        generation: u64,
        /// Lifecycle.
        state: ReplayState,
    },
    /// Native completion observation.
    Completion(RecordStatus),
    /// Host-confirmed revision.
    Applied(u64),
    /// Bounded host report bytes, with host-owned schema.
    Report(Vec<u8>),
}
impl ManagementReply {
    pub(crate) fn check_buffer_limit(&self, max: usize) -> Result<(), RemoteError> {
        match self {
            Self::Blob(data) | Self::Report(data) if data.capacity() > max => {
                Err(RemoteError::Limit)
            }
            Self::Page { items, .. } => {
                if items.len() > 1000 {
                    return Err(RemoteError::Limit);
                }
                let requested = items
                    .capacity()
                    .checked_mul(std::mem::size_of::<RemoteItem>())
                    .and_then(|base| {
                        items
                            .iter()
                            .try_fold(base, |n, item| n.checked_add(item.inline.capacity()))
                    });
                if requested.is_none_or(|n| n > max.saturating_mul(2)) {
                    return Err(RemoteError::Limit);
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

/// Fixed failure code; credentials and internal diagnostics are never formatted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemoteError {
    /// Invalid range, limits or metadata.
    InvalidInput,
    /// Incompatible major.
    Version,
    /// Finite capacity exhausted.
    Limit,
    /// Missing or insufficient credentials.
    PermissionDenied,
    /// Explicitly disabled listener.
    Disabled,
    /// Host deadline elapsed; IO may still be running.
    TimedOut,
    /// Transport failed; no success inferred.
    Transport,
    /// Stream or transfer no longer exists.
    Gone,
    /// Host does not implement this operation.
    Unsupported,
    /// Invalid offsets, metadata or checksum.
    Corrupt,
}
impl std::fmt::Display for RemoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidInput => "invalid remote request",
            Self::Version => "incompatible protocol major",
            Self::Limit => "remote capacity exceeded",
            Self::PermissionDenied => "remote permission denied",
            Self::Disabled => "remote server disabled",
            Self::TimedOut => "remote request timed out",
            Self::Transport => "remote transport failed",
            Self::Gone => "remote handle gone",
            Self::Unsupported => "unsupported remote operation",
            Self::Corrupt => "invalid remote record",
        })
    }
}
impl std::error::Error for RemoteError {}
pub(crate) fn checksum(data: &[u8]) -> u32 {
    !crate::storage::recovery::crc_update(!0, data)
}
pub(crate) fn valid_record(r: CapturedRecord) -> bool {
    r.receipt.accepted_seq != 0 && r.metadata.importance.is_some()
}
