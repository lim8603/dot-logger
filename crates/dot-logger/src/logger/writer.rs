//! Owned `'static` writer handles over the shared capture core (ADR-012 §2–3).

use std::cell::Cell;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::capture::{
    CaptureCore, CaptureError, CaptureReceipt, Importance, Level, Producer, RecordMetadata,
    RecordOutcome, WriteState,
};

/// Payload identity stored with every record of a channel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Schema {
    id: u128,
    version: u32,
    encoding: &'static str,
    /// (type name, descriptor encoding, descriptor bytes).
    descriptor: Option<(&'static str, &'static str, &'static [u8])>,
}

impl Schema {
    /// Schema identity, version and payload encoding label (for example
    /// `"protobuf"`, `"cdr"`, or an application-specific name).
    pub const fn new(id: u128, version: u32, encoding: &'static str) -> Self {
        Self {
            id,
            version,
            encoding,
            descriptor: None,
        }
    }
    /// Attach the payload's schema descriptor so standard MCAP tools can
    /// decode it directly from native MCAP files: `name` is the
    /// type the tools resolve (for protobuf the fully-qualified message name,
    /// `data` a serialized `FileDescriptorSet` with imports), `encoding` the
    /// MCAP schema encoding (`"protobuf"`, `"jsonschema"`, ...). Stored once
    /// per file; without it the payload stays opaque bytes.
    pub const fn descriptor(
        mut self,
        name: &'static str,
        encoding: &'static str,
        data: &'static [u8],
    ) -> Self {
        self.descriptor = Some((name, encoding, data));
        self
    }
    /// Schema identity.
    pub const fn id(&self) -> u128 {
        self.id
    }
    /// Schema version.
    pub const fn version(&self) -> u32 {
        self.version
    }
    /// Payload encoding label.
    pub const fn encoding(&self) -> &'static str {
        self.encoding
    }
    pub(crate) fn payload_schema(&self) -> Option<crate::storage::PayloadSchema> {
        self.descriptor
            .map(|(name, encoding, data)| crate::storage::PayloadSchema {
                encoding: encoding.into(),
                descriptor: data.into(),
                name: Some(name.into()),
            })
    }
}

/// The encoder did not fit the record into the supplied buffer.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EncodeError;

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("record does not fit the encode buffer")
    }
}

impl std::error::Error for EncodeError {}

/// A message type a [`Writer`] can log.
///
/// `encode` runs on the RT thread: it must not allocate, block or panic. The
/// buffer is one small slot (`WriterQuota::small_slot_bytes`).
pub trait Record {
    /// Payload identity of every record of this type.
    const SCHEMA: Schema;
    /// Write the encoded payload into `out` and return its length.
    fn encode(&self, out: &mut [u8]) -> Result<usize, EncodeError>;
}

impl Record for [u8] {
    const SCHEMA: Schema = Schema::new(0, 1, "opaque");
    fn encode(&self, out: &mut [u8]) -> Result<usize, EncodeError> {
        out.get_mut(..self.len())
            .ok_or(EncodeError)?
            .copy_from_slice(self);
        Ok(self.len())
    }
}

/// Copyable channel name tag; owns no memory.
pub struct Channel<T: ?Sized> {
    pub(crate) session: u128,
    pub(crate) id: u32,
    pub(crate) level: Level,
    /// Record override; `None` leaves importance to the channel's mapping.
    pub(crate) importance: Option<Importance>,
    _type: PhantomData<fn(&T)>,
}

impl<T: ?Sized> Channel<T> {
    pub(crate) fn new(session: u128, id: u32) -> Self {
        Self {
            session,
            id,
            level: Level::Info,
            importance: None,
            _type: PhantomData,
        }
    }
    /// The same channel logged at another level (default Info).
    pub fn at(mut self, level: Level) -> Self {
        self.level = level;
        self
    }
    /// The same channel with every record marked Important (API §22.4).
    ///
    /// Important records may use the writer's reserved slots
    /// ([`WriterQuota::important`]) when Normal records no longer fit, and
    /// sampling never drops them. This per-record mark wins over the
    /// channel's [`ChannelOptions::important_from`] mapping. Like
    /// [`at`](Self::at) it only changes this copy of the tag: no cost on the
    /// RT path. Without a reserve the mark is still stored with the record
    /// but protects nothing; `plan()` cannot see per-record marks, so it does
    /// not warn about them.
    ///
    /// [`WriterQuota::important`]: crate::logger::WriterQuota::important
    /// [`ChannelOptions::important_from`]: crate::logger::ChannelOptions::important_from
    pub fn important(mut self) -> Self {
        self.importance = Some(Importance::Important);
        self
    }
}
impl<T: Record + ?Sized> Channel<T> {
    /// Metadata of one record in this channel at its level and importance
    /// mark ([`important`](Self::important); `None` otherwise), for the
    /// advanced path: `Producer::try_record_raw` with a loan from
    /// [`Writer::producer`]. A loan must be acquired with the importance the
    /// record resolves to. Reads no clock and allocates nothing.
    ///
    /// ```no_run
    /// # use dot_logger::{capture::{CaptureError, Importance}, logger::{Channel, Writer}};
    /// # fn frame(camera: Channel<[u8]>, writer: &mut Writer, image: &[u8])
    /// #     -> Result<(), CaptureError> {
    /// // camera = logger.channel::<[u8]>("camera")?
    /// let metadata = camera.metadata(writer.now_ns(), 0);
    /// let mut producer = writer.producer();
    /// let mut loan = producer.try_acquire(image.len(), Importance::Normal)?;
    /// loan.payload_mut().copy_from_slice(image);
    /// producer.try_record_raw(metadata, loan).map_err(|e| e.reason)?;
    /// # Ok(()) }
    /// ```
    pub fn metadata(&self, mono_ns: u64, real_ns: i64) -> RecordMetadata {
        RecordMetadata {
            channel_id: self.id,
            schema_id: T::SCHEMA.id(),
            schema_version: T::SCHEMA.version(),
            mono_ns,
            real_ns,
            source_time: None,
            level: self.level,
            importance: self.importance,
        }
    }
}
impl<T: ?Sized> Clone for Channel<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: ?Sized> Copy for Channel<T> {}
impl<T: ?Sized> std::fmt::Debug for Channel<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Channel").field("id", &self.id).finish()
    }
}

/// Admission result. A receipt acknowledges ingress only, not the file write.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Logged {
    /// Accepted into the preallocated ingress.
    Accepted(CaptureReceipt),
    /// Filtered by the channel's minimum level
    /// ([`ChannelOptions::minimum`](crate::logger::ChannelOptions::minimum),
    /// [`Logger::set_channel`](crate::logger::Logger::set_channel)) or its
    /// sampling; nothing is stored.
    Skipped,
}

/// Fixed RT rejection code; no dynamic diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogError {
    /// Shutdown started or the default worker failed.
    Closing,
    /// Ingress or completion window is full; the record was not admitted.
    Capacity,
    /// Encoded record exceeds the small slot.
    TooLarge,
    /// The encoder failed.
    Encode,
    /// Channel belongs to another logger, or the registration is inconsistent.
    InvalidInput,
    /// Channel schema differs from the registration.
    SchemaMismatch,
    /// This writer's receipt sequence is exhausted.
    Exhausted,
}

impl LogError {
    /// Whether retrying later can succeed (the logger is still draining).
    pub fn is_capacity(&self) -> bool {
        matches!(self, Self::Capacity)
    }
}

impl From<CaptureError> for LogError {
    fn from(error: CaptureError) -> Self {
        match error {
            CaptureError::Capacity | CaptureError::CompletionCapacity => Self::Capacity,
            CaptureError::Closing | CaptureError::DeadlineExceeded => Self::Closing,
            CaptureError::TooLarge => Self::TooLarge,
            CaptureError::SchemaMismatch => Self::SchemaMismatch,
            CaptureError::SequenceExhausted | CaptureError::GenerationExhausted => Self::Exhausted,
            CaptureError::InvalidInput | CaptureError::Unsupported => Self::InvalidInput,
        }
    }
}

/// State a writer owns exclusively and returns through its mailbox on Drop.
///
/// The core reference lives here too, so after the push in `Writer::drop` the
/// RT thread holds no reference whose release could free shared memory
/// (REV-051 RV-120).
pub(crate) struct Owned {
    pub(crate) core: Arc<CaptureCore>,
    pub(crate) write: WriteState,
    pub(crate) scratch: Box<[u8]>,
}

/// Unique RT handle of one producer: `'static + Send`, not `Sync`.
///
/// Use one writer per thread. Dropping it moves its state into a preallocated
/// mailbox; the logger's management side frees it, so Drop never allocates or
/// releases memory on the calling thread.
///
/// ```compile_fail
/// fn require_sync<T: Sync>() {}
/// require_sync::<dot_logger::logger::Writer>();
/// ```
///
/// ```
/// fn require_send_static<T: Send + 'static>() {}
/// require_send_static::<dot_logger::logger::Writer>();
/// ```
pub struct Writer {
    state: Option<Owned>,
    mailbox: rtrb::Producer<Owned>,
    id: usize,
    session: u128,
    origin: Instant,
    _not_sync: PhantomData<Cell<()>>,
}

impl Writer {
    pub(crate) fn new(
        state: Owned,
        mailbox: rtrb::Producer<Owned>,
        id: usize,
        origin: Instant,
    ) -> Self {
        let session = state.core.session_id();
        Self {
            state: Some(state),
            mailbox,
            id,
            session,
            origin,
            _not_sync: PhantomData,
        }
    }

    /// Borrowed view for the advanced RT API (`try_acquire`, `Loan`,
    /// `try_record_raw`, `apply_pending_config`). Building it copies references
    /// only: no allocation and no reference-count change.
    pub fn producer(&mut self) -> Producer<'_> {
        match self.state.as_mut() {
            Some(owned) => owned.core.producer(self.id, &mut owned.write),
            // `state` is only taken inside Drop, after which no method can run.
            None => unreachable!("writer state is present until drop"),
        }
    }

    /// Apply the latest channel change posted with
    /// [`Logger::set_channel`](crate::logger::Logger::set_channel) (API §22.2,
    /// D1) and return the configuration revision this writer now applies.
    ///
    /// `try_log` does the same on entry, so a writer that logs regularly never
    /// needs this. A writer that stays idle holds the next change back
    /// ([`ConfigError::Busy`](crate::logger::ConfigError::Busy)) until it logs
    /// or calls `refresh`; call it periodically from such a thread. RT path:
    /// a fixed number of atomic loads and stores, no waiting and no
    /// allocation; it does not drain records or wake the worker.
    pub fn refresh(&mut self) -> u64 {
        self.producer().apply_pending_config()
    }

    /// Session monotonic time of this logger, in nanoseconds.
    pub fn now_ns(&self) -> u64 {
        u64::try_from(self.origin.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }

    /// Encode and admit one record at the current time. Never waits or allocates.
    pub fn try_log<T: Record + ?Sized>(
        &mut self,
        channel: Channel<T>,
        message: &T,
    ) -> Result<Logged, LogError> {
        let real = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_nanos()).unwrap_or(i64::MAX));
        self.try_log_at(channel, message, self.now_ns(), real)
    }

    /// Like [`try_log`](Self::try_log) with caller-supplied session monotonic
    /// and wall-clock nanoseconds; reads no clock.
    pub fn try_log_at<T: Record + ?Sized>(
        &mut self,
        channel: Channel<T>,
        message: &T,
        mono_ns: u64,
        real_ns: i64,
    ) -> Result<Logged, LogError> {
        let Some(owned) = self.state.as_mut() else {
            return Err(LogError::Closing);
        };
        if owned.core.is_closing() {
            return Err(LogError::Closing);
        }
        // The helper rechecks closure inside ActiveCall if shutdown overlaps encoding.
        if channel.session != self.session {
            owned
                .core
                .loss_before_submit(self.id, None, crate::loss::Cause::InvalidInput)?;
            return Err(LogError::InvalidInput);
        }
        let length = match message.encode(&mut owned.scratch) {
            Ok(length) if length <= owned.scratch.len() => length,
            _ => {
                owned.core.loss_before_submit(
                    self.id,
                    Some(channel.id),
                    crate::loss::Cause::Encode,
                )?;
                return Err(LogError::Encode);
            }
        };
        let payload = &owned.scratch[..length];
        let metadata = channel.metadata(mono_ns, real_ns);
        match owned
            .core
            .producer(self.id, &mut owned.write)
            .try_record_small(metadata, payload)?
        {
            RecordOutcome::Accepted(receipt) => Ok(Logged::Accepted(receipt)),
            RecordOutcome::Skipped => Ok(Logged::Skipped),
        }
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        if let Some(owned) = self.state.take() {
            // Capacity one and exactly one push per writer: this cannot fail.
            // Moving into the preallocated slot neither allocates nor frees.
            let _ = self.mailbox.push(owned);
        }
    }
}

#[cfg(test)]
mod closing_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::logger::{Budget, Logger, Retention, WriterQuota};
    #[test]
    #[cfg(target_os = "linux")]
    // Real files and a directory fsync, which Miri does not support (RV-188).
    #[cfg_attr(miri, ignore)]
    fn closing_during_encoding_outranks_encode_and_foreign_channel_errors() {
        struct Closing<'a>(&'a CaptureCore);
        impl Record for Closing<'_> {
            const SCHEMA: Schema = Schema::new(99, 1, "opaque");
            fn encode(&self, _: &mut [u8]) -> Result<usize, EncodeError> {
                self.0.control().close();
                Err(EncodeError)
            }
        }
        let temp = crate::logger::tests::Temp::new();
        let logger = Logger::builder()
            .budget(
                Budget::new()
                    .memory_bytes(32 << 20)
                    .sync_every(std::time::Duration::from_millis(20))
                    .drop_deadline(std::time::Duration::from_secs(3))
                    .writer_quota(WriterQuota::small(8, 16)),
            )
            .file_output(&temp.0, Retention::continuous(64 << 20))
            .channel::<Closing<'_>>("closing")
            .writer("p")
            .start()
            .unwrap();
        let mut channel = logger.channel::<Closing<'_>>("closing").unwrap();
        let mut writer = logger.take_writer("p").unwrap();
        let core = Arc::clone(&writer.state.as_ref().unwrap().core);
        assert_eq!(
            writer.try_log_at(channel, &Closing(&core), 0, 0),
            Err(LogError::Closing)
        );
        channel.session ^= 1;
        assert_eq!(
            writer.try_log_at(channel, &Closing(&core), 0, 0),
            Err(LogError::Closing)
        );
        drop(writer);
        logger.shutdown(std::time::Duration::from_secs(5)).unwrap();
    }
}
