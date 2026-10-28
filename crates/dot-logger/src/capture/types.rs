//! Fixed-size capture inputs, results, and registration contracts.

use std::fmt;

use crate::config::ConfigError;

/// Retention importance, independent of diagnostic severity.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Importance {
    /// Ordinary admission; cannot consume Important reservations.
    #[default]
    Normal,
    /// May consume the producer's reserved capacity.
    Important,
}

/// Diagnostic severity, ordered from least to most severe.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(usize)]
pub enum Level {
    /// Fine-grained diagnostic detail.
    Trace,
    /// Debugging information.
    Debug,
    /// Informational event.
    Info,
    /// Warning event.
    Warn,
    /// Error event.
    Error,
    /// Fatal diagnostic event.
    Fatal,
}

/// Complete, allocation-free channel importance classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImportanceMapping {
    /// Fixed channel default; Normal is the initial default.
    Default(Importance),
    /// One entry for every Level, in declaration order; no incomplete mapping.
    ByLevel([Importance; 6]),
}

impl ImportanceMapping {
    pub(super) fn resolve(self, level: Level) -> Importance {
        match self {
            Self::Default(importance) => importance,
            Self::ByLevel(mapping) => mapping[level as usize],
        }
    }
}

/// Fixed channel contract registered before capture begins.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChannelConfig {
    /// Session-local channel identifier.
    pub channel_id: u32,
    /// Opaque user schema identifier.
    pub schema_id: u128,
    /// Required user schema version.
    pub schema_version: u32,
    /// Less severe records are intentionally skipped after input validation.
    pub minimum_level: Level,
    /// Used when the record does not explicitly specify importance.
    pub importance: ImportanceMapping,
}

impl ChannelConfig {
    /// Register a channel with all levels enabled and Normal importance.
    pub fn new(channel_id: u32, schema_id: u128, schema_version: u32) -> Self {
        Self {
            channel_id,
            schema_id,
            schema_version,
            minimum_level: Level::Trace,
            importance: ImportanceMapping::Default(Importance::Normal),
        }
    }
}

/// Sensor timestamp paired with its opaque clock domain.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceTime {
    /// Timestamp in the source domain's declared units.
    pub ticks: i64,
    /// Registered source clock domain; no automatic wall-clock conversion.
    pub clock_domain: u64,
}

/// Fixed input metadata; times are prepared by the application before RT entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecordMetadata {
    /// Previously registered channel.
    pub channel_id: u32,
    /// Must match the channel's schema.
    pub schema_id: u128,
    /// Must match the channel's schema version.
    pub schema_version: u32,
    /// Application-supplied monotonic time in nanoseconds.
    pub mono_ns: u64,
    /// Signed wall-clock Unix time in nanoseconds; clock jumps are preserved.
    pub real_ns: i64,
    /// Optional sensor timestamp and clock domain.
    pub source_time: Option<SourceTime>,
    /// Diagnostic severity; does not imply retention importance.
    pub level: Level,
    /// Explicit override, otherwise channel mapping then Normal.
    pub importance: Option<Importance>,
}

/// Non-owning ingress identity; not a storage or completion acknowledgment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CaptureReceipt {
    /// Application-supplied unique session identifier.
    pub session_id: u128,
    /// Owner-local producer index, never reassigned within this owner.
    pub producer_id: u32,
    /// Increases only for accepted records and never wraps.
    pub accepted_seq: u64,
}

/// Immediate, allocation-free capture decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordOutcome {
    /// Copied into bounded ingress, without any output acknowledgment.
    Accepted(CaptureReceipt),
    /// Intentionally omitted by the registered level filter.
    Skipped,
}

/// Fixed error codes for small capture and general waiting.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureError {
    /// Channel, producer/pool, or acquired importance does not match the input.
    InvalidInput,
    /// The channel's schema identifier or version does not match.
    SchemaMismatch,
    /// Payload exceeds the producer's registered slot size.
    TooLarge,
    /// The producer's slots or bytes cannot admit this record.
    Capacity,
    /// The producer completion window has no reusable tracking slot.
    CompletionCapacity,
    /// Ingress was closed; already accepted records remain readable.
    Closing,
    /// No more sequence identifiers can be issued.
    SequenceExhausted,
    /// Every eligible raw slot exhausted its generation; no identifier wraps.
    GenerationExhausted,
    /// This producer has no original-payload pool.
    Unsupported,
    /// The general producer's monotonic deadline has elapsed.
    DeadlineExceeded,
}

/// Metadata returned alongside bytes copied by the management consumer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapturedRecord {
    /// Ingress-only receipt.
    pub receipt: CaptureReceipt,
    /// Original input metadata, with resolved importance.
    pub metadata: RecordMetadata,
    /// Immutable initial configuration revision.
    pub revision: u64,
    /// Number of valid bytes in the caller's destination buffer.
    pub payload_len: usize,
}

/// Management consumer error; failures leave the queued record untouched.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadError {
    /// Supply at least this many destination bytes and retry.
    BufferTooSmall(usize),
    /// Internal publication invariant failed; no record is consumed.
    InconsistentQueue,
    /// The head is a small record; consume it before taking a raw original.
    PayloadKindMismatch,
}

/// Error detected before allocating ingress queues.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CaptureBuildError {
    /// Concrete queue and metadata reservations exceed the validated budget.
    Config(ConfigError),
    /// Register at least one channel, without duplicate identifiers.
    InvalidChannels,
    /// Completion limits or initial route/target contracts are inconsistent.
    InvalidCompletion,
    /// The number of producers exceeds the receipt identifier space.
    TooManyProducers,
    /// A newly created queue could not be warmed before use.
    InitializationFailed,
}

impl fmt::Display for CaptureBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => error.fmt(f),
            Self::InvalidCompletion => f.write_str("invalid completion plan"),
            Self::InvalidChannels => f.write_str("channels must be nonempty and unique"),
            Self::TooManyProducers => f.write_str("too many producers for capture identifiers"),
            Self::InitializationFailed => f.write_str("capture queue initialization failed"),
        }
    }
}

impl std::error::Error for CaptureBuildError {}
