//! Explicit synchronous general-worker bridge retaining originals through IO close.
use super::*;
use crate::{
    completion::{Boundary, Completion, CompletionError, FailureReason, Update},
    routing::{Delivery, DropReason},
};

/// Explicit host schema, never inferred from raw images or arbitrary bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionEncoding {
    /// Exactly eight little-endian signed integer bytes.
    I64Le,
    /// Exactly eight little-endian IEEE754 bytes, finite values only.
    F64Le,
    /// Exactly one byte: zero or one.
    BoolByte,
    /// Bounded UTF-8 event; existing write limits/control-character checks apply.
    Utf8Event,
}
impl ProjectionEncoding {
    fn kind(self) -> Kind {
        match self {
            Self::I64Le => Kind::Integer,
            Self::F64Le => Kind::Float,
            Self::BoolByte => Kind::Boolean,
            Self::Utf8Event => Kind::Event,
        }
    }
}
/// Frozen mapping from an original registered capture schema to one DB field.
#[derive(Clone, Copy, Debug)]
pub struct ProjectionSchema {
    /// Original product channel.
    pub channel_id: u32,
    /// Original application schema identifier.
    pub schema_id: u128,
    /// Original schema version, never silently converted.
    pub schema_version: u32,
    /// Explicit bounded scalar/event interpretation.
    pub encoding: ProjectionEncoding,
}
/// Static host/projection failure, without credentials or original payload text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionError {
    /// Mismatched contracts, mapping, session or resource bounds at registration.
    InvalidConfig,
    /// Foreign target/session/revision; no completion update may be published.
    Identity,
    /// Missing/foreign application schema; no network request was attempted.
    Schema,
    /// Invalid scalar length/value or event UTF-8.
    Payload,
    /// Nested runtime or runtime construction failure; recover on a general worker.
    Runtime,
    /// Existing finite writer preflight error; no HTTP retry is inferred.
    Write(Error),
}
impl fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InfluxDB routed projection: {self:?}")
    }
}
impl std::error::Error for ProjectionError {}
/// One explicit general-path DB output, bound to a frozen initial revision.
/// It owns no background thread or persistent runtime. Each synchronous call
/// drives a private current-thread runtime, then drops it while still owning
/// the original Delivery. This also waits for local spawned/blocking work to
/// stop; OS/DNS cleanup may exceed the admission TTL. Remote rollback is never
/// inferred. Run on a host-owned general worker, independently of file output.
pub struct RoutedWriter {
    writer: Writer,
    target: u32,
    revision: u64,
    schemas: Vec<ProjectionSchema>,
    _reservation: Reservation,
}
impl RoutedWriter {
    /// Validate fixed schema/type and DatabaseAcknowledged target contracts.
    /// No connection, runtime or worker is started during registration.
    pub fn new(
        writer: Writer,
        completion: Completion<'_>,
        target: u32,
        schemas: Vec<ProjectionSchema>,
    ) -> Result<Self, ProjectionError> {
        let contract = completion
            .targets()
            .iter()
            .find(|t| t.id == target)
            .ok_or(ProjectionError::InvalidConfig)?;
        if writer.config.session_id != completion.session_id()
            || contract.completion != Boundary::DatabaseAcknowledged
            || !contract.supported.contains(Boundary::DatabaseAcknowledged)
            || writer.pending.upgrade().is_some()
            || schemas.len() != writer.config.channels.len()
        {
            return Err(ProjectionError::InvalidConfig);
        }
        for (i, s) in schemas.iter().enumerate() {
            if s.schema_version == 0
                || schemas[..i]
                    .iter()
                    .any(|other| other.channel_id == s.channel_id)
                || !writer
                    .config
                    .channels
                    .iter()
                    .any(|c| c.id == s.channel_id && c.kind == s.encoding.kind())
            {
                return Err(ProjectionError::InvalidConfig);
            }
        }
        let bytes = schemas
            .len()
            .checked_mul(std::mem::size_of::<ProjectionSchema>())
            .and_then(|n| n.checked_add(std::mem::size_of::<Self>()))
            .ok_or(ProjectionError::InvalidConfig)?;
        let reservation = writer
            .reservation
            .reserve_extra(bytes)
            .map_err(|_| ProjectionError::InvalidConfig)?;
        Ok(Self {
            writer,
            target,
            revision: completion.revision(),
            schemas: schemas.into_boxed_slice().into_vec(),
            _reservation: reservation,
        })
    }
    /// Consume exactly one original guard, preserving its metadata and real time.
    /// Return only after local IO/runtime cleanup, keeping the guard in the result
    /// until explicit completion publication/drop. Never call from an async
    /// runtime or RT thread. Cancellation ends admission/wait, not remote writes.
    /// For completion Busy retry only DeliveryAttempt::publish, never HTTP IO.
    pub fn write_delivery<'o>(
        &mut self,
        delivery: Delivery<'o>,
        cancel: &AtomicBool,
    ) -> DeliveryAttempt<'o> {
        let record = delivery.record();
        let result = if delivery.target_id() != self.target
            || record.receipt.session_id != self.writer.config.session_id
            || record.revision != self.revision
        {
            Err(ProjectionError::Identity)
        } else if cancel.load(Ordering::Acquire) {
            Ok(WriteReport {
                outcome: Outcome::NotSent,
                stop: Stop::Cancelled,
                attempts: 0,
                points: 1,
                status: None,
            })
        } else if tokio::runtime::Handle::try_current().is_ok() {
            Err(ProjectionError::Runtime)
        } else {
            (|| {
                let point = self.project(&delivery)?;
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|_| ProjectionError::Runtime)?;
                let report = runtime.block_on(self.writer.write(&[point], cancel));
                // Do not use shutdown_timeout/background: the original must not
                // be returned while this call's local IO work can still run.
                drop(runtime);
                report.map_err(ProjectionError::Write)
            })()
        };
        let update = match result {
            Ok(WriteReport {
                outcome: Outcome::Acknowledged,
                ..
            }) => Some(Update::Confirmed(Boundary::DatabaseAcknowledged)),
            Ok(WriteReport {
                outcome: Outcome::Rejected,
                ..
            }) => Some(Update::Failed(FailureReason::IoFailure)),
            Ok(WriteReport {
                outcome: Outcome::Unknown,
                ..
            }) => Some(Update::Unknown),
            Ok(WriteReport {
                outcome: Outcome::NotSent,
                stop: Stop::Cancelled,
                ..
            }) => Some(Update::Dropped(DropReason::Cancelled)),
            Ok(_) => Some(Update::Dropped(DropReason::Expired)),
            Err(
                ProjectionError::Identity
                | ProjectionError::Runtime
                | ProjectionError::Write(Error::Busy),
            ) => None,
            Err(_) => Some(Update::Failed(FailureReason::AdapterFailure)),
        };
        DeliveryAttempt {
            delivery,
            result,
            update,
        }
    }
    /// Recover the finite low-level writer for explicit numeric queries/cleanup.
    pub fn into_writer(self) -> Writer {
        self.writer
    }
    fn project<'a>(&self, delivery: &'a Delivery<'_>) -> Result<Point<'a>, ProjectionError> {
        let r = delivery.record();
        let schema = self
            .schemas
            .iter()
            .find(|s| s.channel_id == r.metadata.channel_id)
            .ok_or(ProjectionError::Schema)?;
        if schema.schema_id != r.metadata.schema_id
            || schema.schema_version != r.metadata.schema_version
        {
            return Err(ProjectionError::Schema);
        }
        let payload = delivery.payload();
        if r.payload_len != payload.len() {
            return Err(ProjectionError::Payload);
        }
        let value = match schema.encoding {
            ProjectionEncoding::I64Le => Value::Integer(i64::from_le_bytes(
                payload.try_into().map_err(|_| ProjectionError::Payload)?,
            )),
            ProjectionEncoding::F64Le => {
                let v =
                    f64::from_le_bytes(payload.try_into().map_err(|_| ProjectionError::Payload)?);
                if !v.is_finite() {
                    return Err(ProjectionError::Payload);
                }
                Value::Float(v)
            }
            ProjectionEncoding::BoolByte => match payload {
                [0] => Value::Boolean(false),
                [1] => Value::Boolean(true),
                _ => return Err(ProjectionError::Payload),
            },
            ProjectionEncoding::Utf8Event => {
                if payload.len() > self.writer.config.max_event_bytes {
                    return Err(ProjectionError::Payload);
                }
                Value::Event(std::str::from_utf8(payload).map_err(|_| ProjectionError::Payload)?)
            }
        };
        Ok(Point {
            channel: r.metadata.channel_id,
            receipt: r.receipt,
            real_ns: r.metadata.real_ns,
            value,
        })
    }
}
/// Completed local IO attempt retaining the original router guard for reporting.
/// Unpublished Drop uses the existing Delivery Unknown fallback. A successful
/// publish still retains the original until this owner is actually dropped.
pub struct DeliveryAttempt<'o> {
    delivery: Delivery<'o>,
    result: Result<WriteReport, ProjectionError>,
    update: Option<Update>,
}
impl<'o> DeliveryAttempt<'o> {
    /// Original capture receipt, never a synthesized DB record identity.
    pub fn receipt(&self) -> CaptureReceipt {
        self.delivery.record().receipt
    }
    /// Native IO/preflight result; no automatic completion publication.
    pub fn result(&self) -> Result<WriteReport, ProjectionError> {
        self.result
    }
    /// Exact intended completion update; None is recoverable host misuse/Busy.
    pub fn update(&self) -> Option<Update> {
        self.update
    }
    /// One bounded nonblocking publication. On Busy retain this owner and retry
    /// only this method. Foreign identity/runtime/Busy preflight changes nothing.
    pub fn publish(&self) -> Result<(), CompletionError> {
        self.delivery
            .report(self.update.ok_or(CompletionError::InvalidTransition)?)
    }
    /// Recover only a foreign/runtime/Busy preflight guard for the correct worker.
    /// A terminal attempt returns itself unchanged: retry publication, never IO.
    pub fn into_delivery(self) -> Result<Delivery<'o>, Self> {
        if self.update.is_none() {
            Ok(self.delivery)
        } else {
            Err(self)
        }
    }
}
