//! Default execution layer: one builder, a default worker and `'static` writers.
//!
//! ```no_run
//! use dot_logger::logger::{Budget, Logger, Record, Retention, Schema, EncodeError, WriterQuota};
//! use std::time::Duration;
//!
//! struct Pose([u8; 8]);
//! impl Record for Pose {
//!     const SCHEMA: Schema = Schema::new(1, 1, "pose");
//!     fn encode(&self, out: &mut [u8]) -> Result<usize, EncodeError> {
//!         out.get_mut(..8).ok_or(EncodeError)?.copy_from_slice(&self.0);
//!         Ok(8)
//!     }
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let logger = Logger::builder()
//!     .budget(Budget::new()
//!         .memory_bytes(64 << 20)
//!         .sync_every(Duration::from_millis(100))
//!         .drop_deadline(Duration::from_secs(3))
//!         .writer_quota(WriterQuota::small(256, 64)))
//!     .file_output("/data/logs", Retention::ring(1 << 30))
//!     .channel::<Pose>("robot/pose")
//!     .writer("control")
//!     .start()?;
//! let pose = logger.channel::<Pose>("robot/pose")?;
//! let mut writer = logger.take_writer("control")?;
//! std::thread::spawn(move || { let _ = writer.try_log(pose, &Pose([0; 8])); }).join().ok();
//! let report = logger.shutdown(Duration::from_secs(3))?;
//! # let _ = report;
//! # Ok(()) }
//! ```
//!
//! The detailed modules remain the advanced path. Ownership follows ADR-012:
//! the shared capture core lives behind one management-held `Arc`; each
//! [`Writer`](crate::logger::Writer) owns its endpoint and only builds borrowed views of the core.

mod budget;
mod host;
mod writer;

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub use budget::{Budget, Retention, WriterQuota};
pub use writer::{Channel, EncodeError, LogError, Logged, Record, Schema, Writer};

use crate::capture::{CaptureBuildError, CaptureOwner, ChannelConfig};
use crate::completion::ProducerRange;
use crate::config::{ConfigError, Resource, SessionBuilder};
use crate::control::{ControlError, ShutdownReport};
use crate::platform::PlatformCapabilities;
use crate::routing::RoutingError;
use crate::storage::{RetentionError, StorageChannel, StorageError};

use host::{Command, Flags, HostPlan, Progress};
use writer::Owned;

/// A loss value or declaration that `start()` requires but did not receive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Missing {
    /// No `budget(...)` (or preset) was supplied.
    Budget,
    /// `Budget::memory_bytes`.
    Memory,
    /// `Budget::sync_every`.
    SyncEvery,
    /// `Budget::drop_deadline`.
    DropDeadline,
    /// A writer has no quota and the budget has no default `writer_quota`.
    WriterQuota,
    /// No `file_output`.
    FileOutput,
    /// No `channel`.
    Channel,
    /// No `writer`.
    Writer,
}

/// Why `start()` failed. Nothing is left running and no namespace remains.
#[derive(Debug)]
pub enum StartError {
    /// Required values or declarations are absent; all of them are listed.
    Missing(Vec<Missing>),
    /// A topic or writer name was declared twice, or a name is empty.
    Duplicate,
    /// A value is zero or inconsistent (for example raw slots without bytes).
    InvalidBudget,
    /// The memory budget is short: `resource` needs `required` bytes.
    Budget {
        /// Partition that does not fit.
        resource: Resource,
        /// Bytes it needs.
        required: usize,
        /// Bytes it was given.
        available: usize,
    },
    /// A retention size is below the smallest valid segment.
    Retention {
        /// Smallest acceptable retention bytes.
        required: u64,
        /// Configured retention bytes.
        available: u64,
    },
    /// Platform or configuration validation failed.
    Config(ConfigError),
    /// Ingress construction failed.
    Capture(CaptureBuildError),
    /// Routing construction failed.
    Routing(RoutingError),
    /// Control construction or operation failed.
    Control(ControlError),
    /// File output construction failed.
    Storage(RetentionError),
    /// Thread or filesystem error.
    Io(std::io::Error),
    /// The default worker stopped before it was ready.
    WorkerFailed,
}

impl fmt::Display for StartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(missing) => write!(f, "missing required values: {missing:?}"),
            Self::Budget {
                resource,
                required,
                available,
            } => write!(
                f,
                "{resource:?} needs {required} bytes but the budget gives {available}"
            ),
            Self::Retention {
                required,
                available,
            } => write!(
                f,
                "retention of {available} bytes is below the minimum {required}"
            ),
            other => write!(f, "logger start failed: {other:?}"),
        }
    }
}
impl std::error::Error for StartError {}

fn budget_error(error: &ConfigError) -> Option<StartError> {
    match *error {
        ConfigError::BudgetExceeded {
            resource,
            required,
            available,
        } => Some(StartError::Budget {
            resource,
            required,
            available,
        }),
        _ => None,
    }
}
impl From<ConfigError> for StartError {
    fn from(error: ConfigError) -> Self {
        budget_error(&error).unwrap_or(Self::Config(error))
    }
}
impl From<CaptureBuildError> for StartError {
    fn from(error: CaptureBuildError) -> Self {
        match &error {
            CaptureBuildError::Config(e) => budget_error(e).unwrap_or(Self::Capture(error)),
            _ => Self::Capture(error),
        }
    }
}
impl From<RoutingError> for StartError {
    fn from(error: RoutingError) -> Self {
        match &error {
            RoutingError::Config(e) => budget_error(e).unwrap_or(Self::Routing(error)),
            _ => Self::Routing(error),
        }
    }
}
impl From<ControlError> for StartError {
    fn from(error: ControlError) -> Self {
        match &error {
            ControlError::Config(e) => budget_error(e).unwrap_or(Self::Control(error)),
            _ => Self::Control(error),
        }
    }
}
impl From<crate::completion::CompletionError> for StartError {
    fn from(error: crate::completion::CompletionError) -> Self {
        Self::Control(ControlError::Completion(error))
    }
}
impl From<RetentionError> for StartError {
    fn from(error: RetentionError) -> Self {
        match error {
            RetentionError::Storage(StorageError::Budget {
                required,
                available,
            }) => Self::Budget {
                resource: Resource::Output,
                required,
                available,
            },
            other => Self::Storage(other),
        }
    }
}

/// Channel or writer lookup failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LookupError {
    /// No such topic or writer name.
    Unknown,
    /// The writer was already taken.
    AlreadyTaken,
    /// The topic was registered with another record type.
    SchemaMismatch,
}
impl fmt::Display for LookupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "logger lookup failed: {self:?}")
    }
}
impl std::error::Error for LookupError {}

/// Shutdown could not report a result.
#[derive(Debug)]
pub enum ShutdownError {
    /// The default worker stopped (panic or fatal error) before closing.
    WorkerFailed,
    /// No progress arrived well after the deadline; the worker keeps cleaning up.
    Unresponsive(PendingShutdown),
}
impl fmt::Display for ShutdownError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkerFailed => f.write_str("logger worker failed before closing"),
            Self::Unresponsive(_) => f.write_str("logger worker did not report in time"),
        }
    }
}
impl std::error::Error for ShutdownError {}

/// Shutdown result. Inspect `is_success()`: Closed alone does not mean every
/// record was synchronized.
#[derive(Clone, Debug)]
pub struct FacadeReport {
    /// Core drain/sync/cleanup report.
    pub core: ShutdownReport,
    /// Writers not yet dropped. Their memory stays with the worker until they are.
    pub writers_outstanding: usize,
    /// The default worker panicked or stopped on an internal error.
    pub worker_failed: bool,
    /// A file write, sync or final publication failed.
    pub io_failed: bool,
    /// Producer ranges frozen at close whose synchronization was not confirmed.
    /// Empty on success.
    pub unconfirmed: Vec<ProducerRange>,
}

impl FacadeReport {
    /// On-time closure with every required record written and synchronized,
    /// and no worker or file failure.
    pub fn is_success(&self) -> bool {
        self.core.is_success() && !self.worker_failed && !self.io_failed
    }
}

/// Result of [`Logger::shutdown`].
#[derive(Debug)]
pub enum Shutdown {
    /// Cleanup finished.
    Closed(FacadeReport),
    /// The deadline passed first. The worker keeps draining without releasing
    /// memory or IO in use; `pending` waits for the final report.
    Incomplete {
        /// State at the deadline.
        report: FacadeReport,
        /// Wait for the final result; dropping it detaches.
        pending: PendingShutdown,
    },
}

/// Remaining shutdown after a deadline. Dropping it detaches: the worker
/// finishes and frees everything on its own management thread.
#[derive(Debug)]
pub struct PendingShutdown {
    progress: Receiver<Progress>,
}

impl fmt::Debug for Progress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Progress")
    }
}

impl PendingShutdown {
    /// Wait up to `timeout` for the next report.
    pub fn wait(self, timeout: Duration) -> Result<Shutdown, ShutdownError> {
        match self.progress.recv_timeout(timeout) {
            Ok(Progress::Closed(report)) => Ok(Shutdown::Closed(report)),
            Ok(Progress::Incomplete(report)) => Ok(Shutdown::Incomplete {
                report,
                pending: self,
            }),
            Err(RecvTimeoutError::Timeout) => Err(ShutdownError::Unresponsive(self)),
            Err(RecvTimeoutError::Disconnected) => Err(ShutdownError::WorkerFailed),
        }
    }
}

/// Grace beyond a shutdown deadline for the worker to report Incomplete.
const REPORT_GRACE: Duration = Duration::from_secs(5);

struct ChannelSpec {
    topic: String,
    schema: Schema,
}

/// Collects declarations; allocates nothing until [`start`](Self::start).
#[derive(Default)]
pub struct LoggerBuilder {
    budget: Option<Budget>,
    outputs: Vec<(PathBuf, Retention)>,
    channels: Vec<ChannelSpec>,
    writers: Vec<(String, Option<WriterQuota>)>,
}

impl LoggerBuilder {
    /// Explicit budget; required.
    pub fn budget(mut self, budget: Budget) -> Self {
        self.budget = Some(budget);
        self
    }
    /// Add a file output below an existing directory. A fresh session/target
    /// namespace is created inside it. May be called more than once.
    pub fn file_output(mut self, directory: impl AsRef<Path>, retention: Retention) -> Self {
        self.outputs
            .push((directory.as_ref().to_path_buf(), retention));
        self
    }
    /// Register a channel for records of type `T`, routed to every file output.
    pub fn channel<T: Record + ?Sized>(mut self, topic: &str) -> Self {
        self.channels.push(ChannelSpec {
            topic: topic.into(),
            schema: T::SCHEMA,
        });
        self
    }
    /// Declare a writer using the budget's default quota.
    pub fn writer(mut self, name: &str) -> Self {
        self.writers.push((name.into(), None));
        self
    }
    /// Declare a writer with its own quota.
    pub fn writer_with(mut self, name: &str, quota: WriterQuota) -> Self {
        self.writers.push((name.into(), Some(quota)));
        self
    }

    fn missing(&self) -> Vec<Missing> {
        let mut missing = match &self.budget {
            None => vec![Missing::Budget],
            Some(budget) => {
                let quotas: Vec<_> = self.writers.iter().map(|(_, q)| *q).collect();
                budget::missing(budget, &quotas)
            }
        };
        for (absent, item) in [
            (self.outputs.is_empty(), Missing::FileOutput),
            (self.channels.is_empty(), Missing::Channel),
            (self.writers.is_empty(), Missing::Writer),
        ] {
            if absent {
                missing.push(item);
            }
        }
        missing
    }

    /// Validate, preallocate, start the default worker and create the output
    /// namespaces. On error nothing keeps running and nothing is left on disk.
    pub fn start(self) -> Result<Logger, StartError> {
        let missing = self.missing();
        if !missing.is_empty() {
            return Err(StartError::Missing(missing));
        }
        let duplicate = |names: Vec<&str>| {
            names
                .iter()
                .enumerate()
                .any(|(i, name)| name.is_empty() || names[..i].contains(name))
        };
        if duplicate(self.channels.iter().map(|c| c.topic.as_str()).collect())
            || duplicate(self.writers.iter().map(|(n, _)| n.as_str()).collect())
        {
            return Err(StartError::Duplicate);
        }
        for (directory, _) in &self.outputs {
            if !directory.is_dir() {
                return Err(StartError::Io(std::io::ErrorKind::NotFound.into()));
            }
        }
        let budget = self.budget.ok_or(StartError::InvalidBudget)?;
        let storage: Vec<StorageChannel> = self
            .channels
            .iter()
            .enumerate()
            .map(|(index, spec)| {
                Ok(StorageChannel {
                    channel: ChannelConfig::new(
                        u32::try_from(index + 1).map_err(|_| StartError::InvalidBudget)?,
                        spec.schema.id(),
                        spec.schema.version(),
                    ),
                    topic: spec.topic.clone(),
                    payload_encoding: spec.schema.encoding().into(),
                    schema: None,
                })
            })
            .collect::<Result<_, StartError>>()?;
        let quotas: Vec<_> = self.writers.iter().map(|(_, q)| *q).collect();
        let retention: Vec<Retention> = self.outputs.iter().map(|(_, r)| *r).collect();
        let derived = budget::derive(&budget, &quotas, &storage, &retention)?;
        let plan =
            SessionBuilder::new(derived.session).validate(&PlatformCapabilities::default())?;
        let session = new_session_id();
        let configs: Vec<ChannelConfig> = storage.iter().map(|c| c.channel).collect();
        let owner = CaptureOwner::new_tracked(
            &plan,
            session,
            1,
            &configs,
            &derived.routing,
            derived.completion,
        )?;
        let slot_bytes: Vec<usize> = plan
            .config()
            .producers
            .iter()
            .map(|p| p.small.slot_bytes)
            .collect();
        let (core, writes, reads) = owner.into_parts();
        let core = Arc::new(core);
        let origin = Instant::now();
        let mut mailboxes = Vec::with_capacity(writes.len());
        let mut writers = Vec::with_capacity(writes.len());
        for (id, ((write, (name, _)), bytes)) in writes
            .into_iter()
            .zip(&self.writers)
            .zip(slot_bytes)
            .enumerate()
        {
            let (mailbox, returned) = rtrb::RingBuffer::new(1);
            mailboxes.push(returned);
            let owned = Owned {
                write,
                scratch: vec![0; bytes].into_boxed_slice(),
            };
            writers.push((
                name.clone(),
                Some(Writer::new(owned, Arc::clone(&core), mailbox, id, origin)),
            ));
        }
        let flags = Arc::new(Flags::default());
        let (commands, receive) = mpsc::channel();
        let drop_deadline = derived.tuning.drop_deadline;
        let host = HostPlan {
            core,
            reads,
            mailboxes,
            plan,
            routing: derived.routing,
            control: derived.control,
            writer: derived.writer,
            retention: derived.retention,
            channels: storage,
            outputs: self.outputs.iter().map(|(d, _)| d.clone()).collect(),
            tuning: derived.tuning,
            origin,
            flags: Arc::clone(&flags),
        };
        let (directories, _detached) = match host::spawn(host, receive) {
            Ok(started) => started,
            Err(error) => {
                // Dropping the writers returns their state to the (now dead)
                // host's mailboxes on this management thread.
                drop(writers);
                return Err(error);
            }
        };
        Ok(Logger {
            commands: Some(commands),
            writers: Mutex::new(writers),
            channels: self.channels,
            session,
            directories,
            drop_deadline,
            flags,
        })
    }
}

fn new_session_id() -> u128 {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    nanos
        ^ (u128::from(std::process::id()) << 96)
        ^ (u128::from(NEXT.fetch_add(1, Ordering::Relaxed)) << 64)
}

/// A running logger. Management handle: create, query and shut it down on
/// ordinary threads; hand [`Writer`]s to RT threads.
pub struct Logger {
    commands: Option<Sender<Command>>,
    writers: Mutex<Vec<(String, Option<Writer>)>>,
    channels: Vec<ChannelSpec>,
    session: u128,
    directories: Vec<PathBuf>,
    drop_deadline: Duration,
    flags: Arc<Flags>,
}

impl Logger {
    /// Start declaring a logger.
    pub fn builder() -> LoggerBuilder {
        LoggerBuilder::default()
    }

    /// Copyable tag for a registered topic whose record type is `T`.
    pub fn channel<T: Record + ?Sized>(&self, topic: &str) -> Result<Channel<T>, LookupError> {
        let index = self
            .channels
            .iter()
            .position(|c| c.topic == topic)
            .ok_or(LookupError::Unknown)?;
        if self.channels[index].schema != T::SCHEMA {
            return Err(LookupError::SchemaMismatch);
        }
        let id = u32::try_from(index + 1).map_err(|_| LookupError::Unknown)?;
        Ok(Channel::new(self.session, id))
    }

    /// Take a declared writer. Each writer can be taken once.
    pub fn take_writer(&self, name: &str) -> Result<Writer, LookupError> {
        let mut writers = self
            .writers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (_, slot) = writers
            .iter_mut()
            .find(|(n, _)| n == name)
            .ok_or(LookupError::Unknown)?;
        slot.take().ok_or(LookupError::AlreadyTaken)
    }

    /// Session namespace directory of every file output, in declaration order.
    pub fn directories(&self) -> &[PathBuf] {
        &self.directories
    }

    /// Whether the default worker has failed; new records are then rejected.
    pub fn worker_failed(&self) -> bool {
        self.flags.worker_failed.load(Ordering::Acquire)
    }

    /// Close admission, drain, synchronize and publish every file, then report.
    /// After `deadline` the current state is returned as `Incomplete`.
    pub fn shutdown(mut self, deadline: Duration) -> Result<Shutdown, ShutdownError> {
        self.request_shutdown(deadline)
    }

    fn request_shutdown(&mut self, deadline: Duration) -> Result<Shutdown, ShutdownError> {
        // Untaken writers return their state before the report counts them.
        self.writers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
        let commands = self.commands.take().ok_or(ShutdownError::WorkerFailed)?;
        let (reply, progress) = mpsc::channel();
        commands
            .send(Command::Shutdown {
                deadline: Instant::now() + deadline,
                reply,
            })
            .map_err(|_| ShutdownError::WorkerFailed)?;
        PendingShutdown { progress }.wait(deadline + REPORT_GRACE)
    }
}

impl Drop for Logger {
    fn drop(&mut self) {
        if self.commands.is_some() {
            // Bounded shutdown; the report cannot be returned from Drop.
            if let Ok(Shutdown::Incomplete { pending, .. }) =
                self.request_shutdown(self.drop_deadline)
            {
                drop(pending);
            }
        }
    }
}

impl fmt::Debug for Logger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Logger")
            .field("session", &format_args!("{:032x}", self.session))
            .field("directories", &self.directories)
            .finish_non_exhaustive()
    }
}
