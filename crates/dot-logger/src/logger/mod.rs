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
//! std::fs::create_dir_all(".logger")?;
//! let logger = Logger::builder()
//!     .budget(Budget::new()
//!         .memory_bytes(64 << 20)
//!         .sync_every(Duration::from_millis(100))
//!         .drop_deadline(Duration::from_secs(3))
//!         .writer_quota(WriterQuota::small(256, 64)))
//!     .file_output(".logger", Retention::ring(1 << 30))
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
//! The detailed modules remain the advanced path. Ownership: the shared
//! capture core lives behind one management-held `Arc`; each
//! [`crate::logger::Writer`] owns its endpoint and only builds borrowed views of the core.

mod affinity;
mod budget;
mod event;
mod health;
mod host;
mod preset;
mod remote;
#[cfg(feature = "tracing-adapter")]
mod sink;
#[cfg(all(test, target_os = "linux"))]
mod tests;
mod writer;

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Diagnostic severity of a record ([`Channel::at`]) and the threshold of
/// [`ChannelOptions::important_from`]; the same type as
/// `dot_logger::capture::Level`.
pub use crate::capture::Level;
pub use budget::{Budget, EventWindow, Retention, WriterQuota};
pub use event::{EventError, EventMark};
pub use health::{Degradation, Failure, Health, HealthState, OutputHealth};
pub use preset::{Preset, Sizing};
#[cfg(feature = "tracing-adapter")]
pub use sink::TracingSink;
pub use writer::{Channel, EncodeError, LogError, Logged, Record, Schema, Writer};

use crate::capture::{
    CaptureBuildError, CaptureOwner, ChannelConfig, Importance, ImportanceMapping,
};
use crate::completion::ProducerRange;
use crate::config::{
    ConfigError as CoreConfigError, MemoryBudget, Resource, SessionBuilder, ValidatedConfig,
};
use crate::control::config_update::ConfigUpdateError;
use crate::control::{ControlError, ShutdownReport};
use crate::routing::RoutingError;
use crate::storage::{RetentionError, StorageChannel, StorageError};

use health::Shared;
use host::{Command, HostPlan, Progress};
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
    /// An [`EventWindow`] without [`hold`](EventWindow::hold).
    EventHold,
    /// An [`EventWindow`] without [`limits`](EventWindow::limits).
    EventLimits,
}

/// Why `start()` failed. Nothing is left running and no namespace remains.
#[derive(Debug)]
pub enum StartError {
    /// Required values or declarations are absent; all of them are listed.
    Missing(Vec<Missing>),
    /// A topic or writer name was declared twice, or a name is empty.
    Duplicate,
    /// A value is zero or inconsistent (for example raw slots without bytes,
    /// an Important reserve not below the small slots, or an event window
    /// on a continuous output or with zero limits).
    InvalidBudget,
    /// A channel's output selection is empty, repeats an output or names an
    /// output index that was not declared.
    InvalidOutputs,
    /// The memory budget is short: `resource` needs `required` bytes.
    Budget {
        /// Partition that does not fit.
        resource: Resource,
        /// Bytes it needs.
        required: usize,
        /// Bytes it was given.
        available: usize,
    },
    /// A retention size is below the smallest valid segment, or below an
    /// event window's protected bytes plus two segments.
    Retention {
        /// Smallest acceptable retention bytes.
        required: u64,
        /// Configured retention bytes.
        available: u64,
    },
    /// Platform or configuration validation failed.
    Config(CoreConfigError),
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
    /// Remote limits are invalid or the listener could not bind.
    #[cfg(feature = "remote")]
    Remote(crate::adapters::remote::RemoteError),
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

fn budget_error(error: &CoreConfigError) -> Option<StartError> {
    match *error {
        CoreConfigError::BudgetExceeded {
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
impl From<CoreConfigError> for StartError {
    fn from(error: CoreConfigError) -> Self {
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

/// Why [`Logger::set_channel`] could not confirm a change (API §22.2).
/// Fixed codes, no dynamic diagnostics. Busy, Invalid and Closing do not post
/// this request; Failed can leave an unconfirmed posted revision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigError {
    /// The previous change has not settled within the bounded wait
    /// (`Budget::drop_deadline`): a writer taken with
    /// [`Logger::take_writer`] has neither logged nor called
    /// [`Writer::refresh`] since that change, or records admitted before it
    /// are still being written. Also returned if another local request already
    /// occupies the host's single waiting slot. Retry after the writer refreshes
    /// or the waiting call finishes.
    Busy,
    /// `sample_every` is 0, or the channel was issued by another logger.
    Invalid,
    /// Admission is closed (shutdown started, or a worker failed).
    Closing,
    /// The default worker stopped or did not answer in time, or the core
    /// refused the change for an internal reason (exhausted revisions,
    /// unavailable management state). After a missing answer the change may
    /// still have been posted.
    Failed,
}
impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "channel change failed: {self:?}")
    }
}
impl std::error::Error for ConfigError {}

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

impl Shutdown {
    /// Closed and [`FacadeReport::is_success`]: every required record was
    /// written and synchronized on time. `Incomplete` is never success
    /// (dropping it detaches the pending wait, as below).
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Closed(report) if report.is_success())
    }
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
    ///
    /// The worker reports `Incomplete` at most once, when the shutdown
    /// deadline passes; every later report is the final `Closed`.
    ///
    /// # Errors
    ///
    /// [`ShutdownError::Unresponsive`] (returning this handle to wait again)
    /// when no report arrives within `timeout` — after an `Incomplete` this
    /// means the worker has not closed yet, not a second `Incomplete`.
    /// [`ShutdownError::WorkerFailed`] when the worker ended without closing.
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

#[derive(Clone)]
struct ChannelSpec {
    topic: String,
    schema: Schema,
    /// Selected output indices; `None` routes to every output.
    outputs: Option<Vec<usize>>,
    /// Lowest level classified Important; `None` keeps every level Normal.
    important_from: Option<Level>,
    /// Lowest admitted level; `None` admits every level (Trace).
    minimum: Option<Level>,
}

impl ChannelSpec {
    /// Core importance mapping of this channel (API §22.4).
    fn importance(&self) -> ImportanceMapping {
        match self.important_from {
            None => ImportanceMapping::Default(Importance::Normal),
            Some(from) => {
                let mut by_level = [Importance::Normal; 6];
                // `ByLevel` has one entry per `Level` in declaration order.
                for slot in &mut by_level[from as usize..] {
                    *slot = Importance::Important;
                }
                ImportanceMapping::ByLevel(by_level)
            }
        }
    }
}

/// Registration options of one channel (API §18.2).
///
/// `ChannelOptions::new()` keeps the defaults of
/// [`LoggerBuilder::channel`]: every file output, every level Normal.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ChannelOptions {
    outputs: Option<Vec<usize>>,
    important_from: Option<Level>,
    minimum: Option<Level>,
}

impl ChannelOptions {
    /// Defaults: routed to every file output, every level Normal, every
    /// level admitted.
    pub fn new() -> Self {
        Self::default()
    }
    /// Classify records sent at `level` or more severe as Important and the
    /// rest as Normal. A record marked with
    /// [`Channel::important`] stays Important at any level, and an explicit
    /// importance in advanced-path metadata always wins over this mapping.
    ///
    /// Important records only gain protection from a writer reserve
    /// ([`WriterQuota::important`]); when every writer's reserve is 0,
    /// [`plan`](LoggerBuilder::plan) reports
    /// [`PlanWarning::NoImportantReserve`].
    pub fn important_from(mut self, level: Level) -> Self {
        self.important_from = Some(level);
        self
    }
    /// Admit only records sent at `level` or more severe (API §22.2). Lower
    /// records return [`Logged::Skipped`] on
    /// the RT path without waiting, are not stored and are not counted as
    /// accidental losses (they remain intentional Policy omissions in loss
    /// reports). Change the policy while running with [`Logger::set_channel`].
    /// Default: every level (Trace).
    pub fn minimum(mut self, level: Level) -> Self {
        self.minimum = Some(level);
        self
    }
    /// Route only to these file outputs, by `file_output` declaration order
    /// starting at 0. An empty, repeated or undeclared index rejects
    /// `start()` and `plan()` with [`StartError::InvalidOutputs`].
    pub fn outputs(mut self, outputs: &[usize]) -> Self {
        self.outputs = Some(outputs.to_vec());
        self
    }
}

/// What `resolve` decided before any allocation or IO.
struct Resolved {
    channels: Vec<ChannelSpec>,
    writers: Vec<String>,
    storage: Vec<StorageChannel>,
    derived: budget::Derived,
    plan: ValidatedConfig,
    filled: Vec<Missing>,
    default_quota: Option<WriterQuota>,
    warnings: Vec<PlanWarning>,
    #[cfg(feature = "tracing-adapter")]
    tracing: Option<::tracing::Level>,
    #[cfg(feature = "remote")]
    remote: Option<(crate::adapters::remote::RemoteLimits, usize)>,
}

/// Budget a builder resolves to, from [`LoggerBuilder::plan`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Plan {
    /// Exact memory partitions `start()` reserves (total includes any margin).
    pub memory: MemoryBudget,
    /// Memory the core needs for this plan, before the preset's 10 % margin.
    pub required_bytes: usize,
    /// Ordinary sync period.
    pub sync_every: Duration,
    /// Bounded shutdown on `Logger` drop.
    pub drop_deadline: Duration,
    /// Quota of writers declared without their own.
    pub writer_quota: Option<WriterQuota>,
    /// Loss values the budget left unset and the preset filled, in order.
    pub preset_filled: Vec<Missing>,
    /// IO batch actually used: the budget's batch size capped at half of a
    /// target's record budget.
    pub batch_records: usize,
    /// What a calibration profile on the preset flags, and an Important
    /// classification without reserve; never blocks start.
    pub warnings: Vec<PlanWarning>,
}

/// A plan exceeds what a calibration profile observed (API §19.8), or
/// classifies records Important without reserving room for them (API §22.4).
/// These are warnings, not start errors: the profile is a snapshot, and a
/// reserve of 0 is the default (D3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanWarning {
    /// The memory budget is larger than the memory available when measured.
    Memory {
        /// Total memory budget of the plan.
        budget_bytes: usize,
        /// Available memory in the profile (cgroup headroom when lower).
        available_bytes: u64,
    },
    /// The declared load of writers using the preset exceeds 0.6 × the
    /// profile's sustained write estimate.
    Load {
        /// Declared bytes per second of those writers, metadata included.
        offered_bytes_per_second: u64,
        /// 0.6 × the sustained estimate.
        limit_bytes_per_second: u64,
    },
    /// A channel uses [`ChannelOptions::important_from`] but every writer's
    /// [`WriterQuota::important`] reserve is 0, so its Important records get
    /// no protection when the slots are full (API §22.4, D3). Records marked
    /// with [`Channel::important`] are invisible to the plan and never raise
    /// this warning.
    NoImportantReserve,
}

/// Collects declarations; allocates nothing until [`start`](Self::start).
#[derive(Default)]
pub struct LoggerBuilder {
    budget: Option<Budget>,
    outputs: Vec<(PathBuf, Retention)>,
    channels: Vec<ChannelSpec>,
    writers: Vec<(String, Option<WriterQuota>)>,
    telemetry: bool,
    preset: Option<Sizing>,
    #[cfg(feature = "tracing-adapter")]
    tracing: Option<(String, ::tracing::Level, WriterQuota)>,
    #[cfg(feature = "remote")]
    remote: Option<(
        crate::adapters::remote::ServerConfig,
        Option<crate::adapters::remote::RemoteLimits>,
    )>,
}

/// Internal writer of the tracing connection; never returned by `take_writer`.
#[cfg(feature = "tracing-adapter")]
const TRACING_WRITER: &str = "dot-logger/tracing";

impl LoggerBuilder {
    /// Explicit budget; required.
    pub fn budget(mut self, budget: Budget) -> Self {
        self.budget = Some(budget);
        self
    }
    /// Compute the loss values the budget leaves unset (or the whole budget,
    /// without `budget`) from a [`Preset`] or custom [`Sizing`] at start, for
    /// the declared writers, outputs and channels. Values set on the budget
    /// are never overridden; [`plan`](Self::plan) shows the result.
    pub fn preset(mut self, sizing: Sizing) -> Self {
        self.preset = Some(sizing);
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
    ///
    /// Records are sent at `Info` unless [`Channel::at`] picks another level;
    /// the channel itself admits every level (its minimum is `Trace`), so a
    /// lower sending level is never filtered out by the registration. Use
    /// [`ChannelOptions::minimum`] or [`Logger::set_channel`] to filter.
    pub fn channel<T: Record + ?Sized>(self, topic: &str) -> Self {
        self.channel_with::<T>(topic, ChannelOptions::new())
    }
    /// Register a channel for records of type `T` with explicit options, for
    /// example routed to a subset of the file outputs.
    pub fn channel_with<T: Record + ?Sized>(
        mut self,
        topic: &str,
        options: ChannelOptions,
    ) -> Self {
        self.channels.push(ChannelSpec {
            topic: topic.into(),
            schema: T::SCHEMA,
            outputs: options.outputs,
            important_from: options.important_from,
            minimum: options.minimum,
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
    /// Enable fixed producer counters, sampled once per second into
    /// [`Health::telemetry`]. Counters are reserved in the control partition;
    /// each RT record then also updates producer-local atomics.
    pub fn telemetry(mut self) -> Self {
        self.telemetry = true;
        self
    }
    /// Connect `tracing` and `log` events at `min` level or more severe to a
    /// JSON text channel `topic` (schema 99/v1 `"json"`), served by an
    /// internal writer with `quota`. The quota is a loss value: events that
    /// do not fit are dropped and counted. Get the sink from
    /// [`Logger::tracing`]. Ordinary threads only.
    #[cfg(feature = "tracing-adapter")]
    pub fn tracing(mut self, topic: &str, min: ::tracing::Level, quota: WriterQuota) -> Self {
        self.tracing = Some((topic.into(), min, quota));
        self
    }
    /// Serve a gRPC listener (`server.enabled` must be true; loopback-only
    /// unless TLS and tokens are set, as `ServerConfig::validate` requires).
    /// Subscribers receive best-effort copies of records the first file output
    /// wrote, so a channel routed away from output 0 with
    /// [`ChannelOptions::outputs`] is not observed; copies are not completion
    /// targets, and their overflow is their own
    /// Gap/Disconnect. GetStatus and channel level/sampling ApplyConfig are
    /// served by the default worker. Route patches are InvalidInput and memory
    /// changes RestartRequired; other data requests are Unsupported. Compare
    /// has no registered snapshots and cannot produce a comparison.
    /// Limits are derived from the writer quotas (see API §18.11) and their
    /// buffers are charged to `Budget::memory_bytes`.
    #[cfg(feature = "remote")]
    pub fn remote(mut self, server: crate::adapters::remote::ServerConfig) -> Self {
        self.remote = Some((server, None));
        self
    }
    /// Like [`remote`](Self::remote) with explicit limits.
    #[cfg(feature = "remote")]
    pub fn remote_with(
        mut self,
        server: crate::adapters::remote::ServerConfig,
        limits: crate::adapters::remote::RemoteLimits,
    ) -> Self {
        self.remote = Some((server, Some(limits)));
        self
    }

    /// Everything `start()` decides before any allocation or IO; shared with
    /// [`plan`](Self::plan) so both report the same budget.
    fn resolve(&self) -> Result<Resolved, StartError> {
        #[cfg_attr(not(feature = "tracing-adapter"), allow(unused_mut))]
        let mut channels = self.channels.clone();
        #[cfg_attr(not(feature = "tracing-adapter"), allow(unused_mut))]
        let mut writers = self.writers.clone();
        // The tracing connection is one more channel and writer, declared last.
        #[cfg(feature = "tracing-adapter")]
        let tracing = self.tracing.as_ref().map(|(topic, min, quota)| {
            channels.push(ChannelSpec {
                topic: topic.clone(),
                schema: sink::SCHEMA,
                outputs: None,
                important_from: None,
                minimum: None,
            });
            writers.push((TRACING_WRITER.into(), Some(*quota)));
            *min
        });
        let mut budget = self.budget.clone();
        let mut filled = Vec::new();
        if let Some(sizing) = &self.preset {
            filled = budget.get_or_insert_with(Budget::new).apply(sizing);
        }
        let quotas: Vec<_> = writers.iter().map(|(_, q)| *q).collect();
        let mut missing = match &budget {
            None => vec![Missing::Budget],
            Some(budget) => budget::missing(budget, &quotas),
        };
        for item in self.outputs.iter().flat_map(|(_, r)| r.missing()) {
            if !missing.contains(&item) {
                missing.push(item);
            }
        }
        for (absent, item) in [
            (self.outputs.is_empty(), Missing::FileOutput),
            (channels.is_empty(), Missing::Channel),
            (writers.is_empty(), Missing::Writer),
        ] {
            if absent {
                missing.push(item);
            }
        }
        if !missing.is_empty() {
            return Err(StartError::Missing(missing));
        }
        let duplicate = |names: Vec<&str>| {
            names
                .iter()
                .enumerate()
                .any(|(i, name)| name.is_empty() || names[..i].contains(name))
        };
        if duplicate(channels.iter().map(|c| c.topic.as_str()).collect())
            || duplicate(writers.iter().map(|(n, _)| n.as_str()).collect())
        {
            return Err(StartError::Duplicate);
        }
        let budget = budget.ok_or(StartError::InvalidBudget)?;
        // Output targets per channel, 1-based like the core target IDs.
        let routes = channels
            .iter()
            .map(|spec| route_targets(spec.outputs.as_deref(), self.outputs.len()))
            .collect::<Result<Vec<_>, _>>()?;
        let storage: Vec<StorageChannel> = channels
            .iter()
            .enumerate()
            .map(|(index, spec)| {
                Ok(StorageChannel {
                    channel: ChannelConfig {
                        importance: spec.importance(),
                        minimum_level: spec.minimum.unwrap_or(Level::Trace),
                        ..ChannelConfig::new(
                            u32::try_from(index + 1).map_err(|_| StartError::InvalidBudget)?,
                            spec.schema.id(),
                            spec.schema.version(),
                        )
                    },
                    topic: spec.topic.clone(),
                    payload_encoding: spec.schema.encoding().into(),
                    schema: spec.schema.payload_schema(),
                })
            })
            .collect::<Result<_, StartError>>()?;
        let retention: Vec<Retention> = self.outputs.iter().map(|(_, r)| *r).collect();
        #[cfg(feature = "remote")]
        let remote = match &self.remote {
            None => None,
            Some((_, limits)) => {
                let limits = match limits {
                    Some(limits) => limits.clone(),
                    None => remote::default_limits(
                        &budget::resolve(&budget, &quotas).ok_or(StartError::InvalidBudget)?,
                    ),
                };
                let reserve = limits.reservation().map_err(StartError::Remote)?;
                Some((limits, reserve))
            }
        };
        #[allow(unused_mut)]
        let mut extra = budget::Extra {
            telemetry: self.telemetry,
            ..budget::Extra::default()
        };
        #[cfg(feature = "remote")]
        if let Some((_, reserve)) = &remote {
            extra.external_bytes = *reserve;
            extra.threads = 1;
        }
        let derived = budget::derive(&budget, &quotas, &storage, &routes, &retention, extra)?;
        let plan = SessionBuilder::new(derived.session.clone()).validate(&derived.platform)?;
        let mut warnings = self.preset.as_ref().map_or_else(Vec::new, |sizing| {
            warnings(sizing, plan.config().memory.total_bytes, &quotas)
        });
        // Only user writers can log to a classified channel; the tracing
        // writer is excluded. `derive` already checked every quota resolves.
        let reserved = self
            .writers
            .iter()
            .filter_map(|(_, q)| q.or(budget.default_quota()))
            .any(|q| q.important_slots > 0);
        if !reserved && channels.iter().any(|c| c.important_from.is_some()) {
            warnings.push(PlanWarning::NoImportantReserve);
        }
        Ok(Resolved {
            channels,
            writers: writers.into_iter().map(|(name, _)| name).collect(),
            storage,
            derived,
            plan,
            filled,
            default_quota: budget.default_quota(),
            warnings,
            #[cfg(feature = "tracing-adapter")]
            tracing,
            #[cfg(feature = "remote")]
            remote,
        })
    }

    /// The budget `start()` would use, without allocating, binding or creating
    /// files: exact memory partitions, the requirement before any preset
    /// margin, the loss values and which of them the preset filled. Record it
    /// to reproduce a deployment. Errors are those `start()` would return
    /// before IO (missing values, shortfalls with their amounts).
    pub fn plan(&self) -> Result<Plan, StartError> {
        let resolved = self.resolve()?;
        Ok(Plan {
            memory: resolved.plan.config().memory.clone(),
            required_bytes: resolved.derived.required,
            sync_every: resolved.derived.tuning.sync_every,
            drop_deadline: resolved.derived.tuning.drop_deadline,
            writer_quota: resolved.default_quota,
            preset_filled: resolved.filled,
            batch_records: resolved.derived.tuning.batch_records,
            warnings: resolved.warnings,
        })
    }

    /// Validate, preallocate, start the default worker and create the output
    /// namespaces. On error nothing keeps running and nothing is left on disk.
    #[cfg_attr(not(feature = "remote"), allow(unused_mut))]
    pub fn start(mut self) -> Result<Logger, StartError> {
        let Resolved {
            channels,
            writers: names,
            storage,
            derived,
            plan,
            #[cfg(feature = "tracing-adapter")]
            tracing,
            #[cfg(feature = "remote")]
            remote,
            ..
        } = self.resolve()?;
        for (directory, _) in &self.outputs {
            if !directory.is_dir() {
                return Err(StartError::Io(std::io::ErrorKind::NotFound.into()));
            }
        }
        #[cfg(feature = "remote")]
        let remote = match (self.remote.take(), remote) {
            (Some((server, _)), Some((limits, reserve))) => Some((server, limits, reserve)),
            _ => None,
        };
        let session = new_session_id();
        // The listener binds before any namespace exists; dropping it on a
        // later error stops and joins it.
        #[allow(unused_mut)]
        let mut running = remote::Remote::default();
        #[cfg(feature = "remote")]
        let (remote_addr, remote_placed) = match remote {
            None => (None, None),
            Some((server, limits, _)) => {
                let (runtime, placed) =
                    remote::Runtime::start(&plan, server, limits, session, derived.cpus.clone())?;
                let address = runtime.address;
                running.runtime = Some(runtime);
                (Some(address), placed)
            }
        };
        let configs: Vec<ChannelConfig> = storage.iter().map(|c| c.channel).collect();
        let mut owner = CaptureOwner::new_tracked(
            &plan,
            session,
            1,
            &configs,
            &derived.routing,
            derived.completion,
        )?;
        owner.enable_loss_tracking()?;
        if self.telemetry {
            owner.enable_telemetry()?;
        }
        // Runtime level/sampling changes (API §22.2); `derive` sized the banks.
        owner
            .enable_config_updates(&derived.routing)
            .map_err(|error| match error {
                ConfigUpdateError::Config(error) => StartError::from(error),
                // The routing is the one the registry was built from.
                _ => StartError::InvalidBudget,
            })?;
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
        for (id, ((write, name), bytes)) in
            writes.into_iter().zip(&names).zip(slot_bytes).enumerate()
        {
            let (mailbox, returned) = rtrb::RingBuffer::new(1);
            mailboxes.push(returned);
            let owned = Owned {
                core: Arc::clone(&core),
                write,
                scratch: vec![0; bytes].into_boxed_slice(),
            };
            writers.push((name.clone(), Some(Writer::new(owned, mailbox, id, origin))));
        }
        #[cfg(feature = "tracing-adapter")]
        let sink = match (tracing, writers.pop()) {
            (Some(min), Some((_, Some(writer)))) => {
                let channel =
                    u32::try_from(channels.len()).map_err(|_| StartError::InvalidBudget)?;
                Some(Arc::new(sink::SinkState::new(
                    writer, session, channel, min,
                )))
            }
            (None, Some(last)) => {
                writers.push(last);
                None
            }
            _ => None,
        };
        let held = Held {
            writers: Arc::new(crate::sync::warm_mutex(writers)),
            #[cfg(feature = "tracing-adapter")]
            sink,
        };
        let shared = Arc::new(Shared::new(self.outputs.len(), self.telemetry));
        #[cfg(feature = "remote")]
        shared.placed(remote_placed);
        // One request channel per event output, served by its IO thread.
        let mut marks = Vec::with_capacity(derived.events.len());
        let mut events = Vec::new();
        for spec in &derived.events {
            marks.push(spec.map(|spec| {
                let (send, work) = mpsc::channel();
                events.push(send);
                event::Marks::new(spec, work)
            }));
        }
        let (commands, receive) = mpsc::channel();
        let drop_deadline = derived.tuning.drop_deadline;
        let overdue = budget::sync_overdue(derived.tuning.sync_every);
        let host = HostPlan {
            core,
            reads,
            mailboxes,
            plan,
            routing: derived.routing,
            control: derived.control,
            writer: derived.writer,
            retention: derived.retention,
            marks,
            channels: storage,
            outputs: self.outputs.iter().map(|(d, _)| d.clone()).collect(),
            tuning: derived.tuning,
            origin,
            shared: Arc::clone(&shared),
            cpus: derived.cpus,
            remote: running,
            held: held.clone(),
        };
        let (directories, _detached) = match host::spawn(host, receive) {
            Ok(started) => started,
            Err(failed) => {
                // No writer was handed out yet: return every state to the
                // failed host's reaper on this management thread, then wait
                // until it stopped the listener and released the core, so
                // nothing keeps running once `start` returns.
                held.release();
                drop(held);
                if let Some(host) = failed.host {
                    let _ = host.join();
                }
                return Err(failed.error);
            }
        };
        Ok(Logger {
            commands: Some(commands),
            events,
            next_mark: AtomicU64::new(1),
            held,
            channels,
            session,
            directories,
            drop_deadline,
            shared,
            origin,
            overdue,
            #[cfg(feature = "remote")]
            remote_addr,
        })
    }
}

/// Core target IDs (1-based) for a channel's output selection.
fn route_targets(selected: Option<&[usize]>, outputs: usize) -> Result<Vec<u32>, StartError> {
    let all = 1..=u32::try_from(outputs).map_err(|_| StartError::InvalidOutputs)?;
    let Some(selected) = selected else {
        return Ok(all.collect());
    };
    if selected.is_empty()
        || selected
            .iter()
            .enumerate()
            .any(|(i, o)| *o >= outputs || selected[..i].contains(o))
    {
        return Err(StartError::InvalidOutputs);
    }
    selected
        .iter()
        .map(|&o| u32::try_from(o + 1).map_err(|_| StartError::InvalidOutputs))
        .collect()
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

/// Writers the logger holds itself: declared but not taken, and the tracing
/// connection's internal writer. Shared with the default worker, which makes
/// them apply a posted configuration revision (API §22.2, D1).
type HeldWriters = Arc<Mutex<Vec<(String, Option<Writer>)>>>;

#[derive(Clone)]
pub(crate) struct Held {
    writers: HeldWriters,
    #[cfg(feature = "tracing-adapter")]
    sink: Option<Arc<sink::SinkState>>,
}

impl Held {
    /// Apply the latest posted revision without waiting for an application
    /// thread holding a writer. A later management turn retries skipped locks.
    pub(crate) fn acknowledge(&self) {
        let writers = match self.writers.try_lock() {
            Ok(writers) => Some(writers),
            Err(std::sync::TryLockError::Poisoned(writers)) => Some(writers.into_inner()),
            Err(std::sync::TryLockError::WouldBlock) => None,
        };
        if let Some(mut writers) = writers {
            for writer in writers.iter_mut().filter_map(|(_, w)| w.as_mut()) {
                writer.refresh();
            }
        }
        #[cfg(feature = "tracing-adapter")]
        if let Some(sink) = &self.sink {
            sink.acknowledge();
        }
    }

    /// Return every held writer's state (shutdown, failed start); later
    /// tracing events are dropped and counted.
    fn release(&self) {
        self.writers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
        #[cfg(feature = "tracing-adapter")]
        if let Some(sink) = &self.sink {
            drop(sink.reclaim());
        }
    }
}

/// A running logger. Management handle: create, query and shut it down on
/// ordinary threads; hand [`Writer`]s to RT threads.
pub struct Logger {
    commands: Option<Sender<Command>>,
    /// Request channel of each event output's IO thread.
    events: Vec<Sender<event::EventWork>>,
    next_mark: AtomicU64,
    held: Held,
    channels: Vec<ChannelSpec>,
    session: u128,
    directories: Vec<PathBuf>,
    drop_deadline: Duration,
    shared: Arc<Shared>,
    origin: Instant,
    overdue: Duration,
    #[cfg(feature = "remote")]
    remote_addr: Option<std::net::SocketAddr>,
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
            .held
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
        self.shared.worker_failed.load(Ordering::Acquire)
    }

    /// Current worker, output, placement and telemetry status. General path:
    /// it allocates the per-output list and may briefly lock the sample.
    pub fn health(&self) -> Health {
        let now = u64::try_from(self.origin.elapsed().as_nanos()).unwrap_or(u64::MAX);
        #[allow(unused_mut)]
        let mut health = self.shared.health(now, self.overdue);
        #[cfg(feature = "tracing-adapter")]
        {
            health.tracing_dropped = self.held.sink.as_ref().map(|s| s.dropped());
        }
        health
    }

    /// Bound listener address of [`LoggerBuilder::remote`]; `None` without it.
    #[cfg(feature = "remote")]
    pub fn remote_addr(&self) -> Option<std::net::SocketAddr> {
        self.remote_addr
    }

    /// The tracing/`log` sink declared with [`LoggerBuilder::tracing`].
    /// Clones share one internal writer; shutdown reclaims it.
    #[cfg(feature = "tracing-adapter")]
    pub fn tracing(&self) -> Result<TracingSink, LookupError> {
        self.held
            .sink
            .as_ref()
            .map(|state| TracingSink {
                state: Arc::clone(state),
            })
            .ok_or(LookupError::Unknown)
    }

    /// Protect the records around now on every output declared with
    /// [`Retention::events`] (API §22.3). General path: it allocates, sends
    /// one request to each event output's IO thread and waits for the
    /// answers at most `4 × sync_every + 1 s` (the periodic sync's overdue
    /// bound); a failed mark waits once more as long for its rollback. RT
    /// threads do not call this; they notify a thread that does.
    ///
    /// The event time is read here, so the window is
    /// `[at_ns - pre, at_ns + post]` of [`EventMark::at_ns`] on the clock of
    /// [`Writer::now_ns`], also for records that arrive at the file later.
    /// Files holding records in it are not deleted until
    /// `at_ns + post + hold` (expired at least once per `sync_every` after
    /// that) or [`release_event`](Self::release_event).
    ///
    /// # Errors
    ///
    /// [`EventError::NotConfigured`] without an event output;
    /// [`EventError::Quota`] when an output already holds `max_events`
    /// windows or would exceed its `protected_bytes`;
    /// [`EventError::Closing`] when the worker has stopped;
    /// [`EventError::Failed`] on a file fault or no answer in time. After
    /// any error the outputs that did protect the window release it again,
    /// so no output keeps a window the caller has no mark for.
    pub fn mark_event(&self) -> Result<EventMark, EventError> {
        if self.events.is_empty() {
            return Err(EventError::NotConfigured);
        }
        let at = u64::try_from(self.origin.elapsed().as_nanos()).unwrap_or(u64::MAX);
        let id = self.next_mark.fetch_add(1, Ordering::Relaxed);
        let result = self.ask(|reply| event::EventWork::Mark { id, at, reply });
        match result {
            Ok(()) => Ok(EventMark::new(self.session, id, at, self.events.len())),
            Err(error) => {
                // Requests are served in order, so this also releases a
                // late protection of an output that did not answer in time.
                let _ = self.ask_all(|reply| event::EventWork::Release { id, reply });
                Err(error)
            }
        }
    }

    /// Release `mark` on every event output before its hold ends, so the
    /// ring may delete its files again. General path with the same bounded
    /// wait as [`mark_event`](Self::mark_event). A mark that already expired
    /// or was released is `Ok`.
    ///
    /// # Errors
    ///
    /// [`EventError::NotConfigured`] without an event output;
    /// [`EventError::Failed`] for a mark of another logger or no answer in
    /// time; [`EventError::Closing`] when the worker has stopped.
    pub fn release_event(&self, mark: EventMark) -> Result<(), EventError> {
        if self.events.is_empty() {
            return Err(EventError::NotConfigured);
        }
        if mark.session() != self.session {
            return Err(EventError::Failed);
        }
        let id = mark.id();
        self.ask(|reply| event::EventWork::Release { id, reply })
    }

    /// Send one request to every event output and wait for all answers
    /// until the overdue bound; the most severe failure wins.
    fn ask(&self, request: impl Fn(event::Reply) -> event::EventWork) -> Result<(), EventError> {
        if self.worker_failed() {
            return Err(EventError::Closing);
        }
        self.ask_all(request)
    }

    /// Cleanup must reach every live output even after another worker failed.
    fn ask_all(
        &self,
        request: impl Fn(event::Reply) -> event::EventWork,
    ) -> Result<(), EventError> {
        let (reply, answers) = mpsc::channel();
        let mut sent = 0;
        let mut result = Ok(());
        for output in &self.events {
            match output.send(request(reply.clone())) {
                Ok(()) => sent += 1,
                Err(_) => result = worse(result, EventError::Closing),
            }
        }
        drop(reply);
        let deadline = budget::later(Instant::now(), self.overdue);
        for _ in 0..sent {
            let wait = deadline.saturating_duration_since(Instant::now());
            match answers.recv_timeout(wait) {
                Ok(Ok(())) => {}
                Ok(Err(error)) => result = worse(result, error),
                Err(RecvTimeoutError::Timeout) => return worse(result, EventError::Failed),
                // An IO thread ended with the request unanswered.
                Err(RecvTimeoutError::Disconnected) => return worse(result, EventError::Closing),
            }
        }
        result
    }

    /// Change a channel's minimum level and sampling while running (API
    /// §22.2) and return its posted configuration revision. Writers apply it
    /// at their next call boundary; this does not wait for file rotation or sync. Records sent at a less severe level are then
    /// [`Logged::Skipped`]; of the Normal records at or above it, the first
    /// and every `sample_every`-th are kept (1 keeps all), while Important
    /// records are never sampled out. Every stored record carries the
    /// revision it was admitted under, so records before and after a change
    /// stay distinguishable.
    ///
    /// General path: the default worker posts the change and this call waits
    /// for its answer. Each writer applies a posted change at its next
    /// [`Writer::try_log`] or [`Writer::refresh`]; writers still held by the
    /// logger (not taken, the tracing writer) and writers already dropped are
    /// handled by the logger. A change can only be posted once every writer
    /// applied the previous one and the records admitted before it were
    /// written; the worker retries until `Budget::drop_deadline` passes and
    /// then reports [`ConfigError::Busy`]. Asking for the policy already in
    /// effect posts nothing and returns its revision. The first revision is 1.
    ///
    /// ```no_run
    /// # use dot_logger::logger::{Logger, Writer, Level};
    /// # fn adjust(logger: &Logger, writer: &mut Writer) -> Result<(), Box<dyn std::error::Error>> {
    /// let channel = logger.channel::<[u8]>("robot/pose")?;
    /// let _posted = logger.set_channel(channel, Level::Warn, 3)?;
    /// writer.refresh(); // Optional when this writer logs regularly.
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// [`ConfigError::Invalid`] for `sample_every == 0` or a channel of
    /// another logger; [`ConfigError::Busy`], [`ConfigError::Closing`] and
    /// [`ConfigError::Failed`] as documented on [`ConfigError`].
    pub fn set_channel<T: ?Sized>(
        &self,
        channel: Channel<T>,
        minimum: Level,
        sample_every: u32,
    ) -> Result<u64, ConfigError> {
        let known =
            usize::try_from(channel.id).is_ok_and(|id| (1..=self.channels.len()).contains(&id));
        if channel.session != self.session || !known || sample_every == 0 {
            return Err(ConfigError::Invalid);
        }
        let commands = self.commands.as_ref().ok_or(ConfigError::Closing)?;
        // D1: writers the logger holds apply the latest revision first, so
        // only taken writers can hold the change back.
        self.held.acknowledge();
        let (reply, answer) = mpsc::channel();
        commands
            .send(Command::Configure {
                channel: crate::control::config_update::ChannelPatch {
                    channel_id: channel.id,
                    minimum_level: minimum,
                    sample_every,
                },
                deadline: budget::later(Instant::now(), self.drop_deadline),
                reply,
            })
            .map_err(|_| ConfigError::Failed)?;
        // The worker answers by the deadline; the grace covers its turn.
        match answer.recv_timeout(self.drop_deadline.saturating_add(REPORT_GRACE)) {
            Ok(result) => result,
            Err(_) => Err(ConfigError::Failed),
        }
    }

    /// Close admission, drain, synchronize and publish every file, then report.
    /// After `deadline` the current state is returned as `Incomplete`.
    pub fn shutdown(mut self, deadline: Duration) -> Result<Shutdown, ShutdownError> {
        self.request_shutdown(deadline)
    }

    fn request_shutdown(&mut self, deadline: Duration) -> Result<Shutdown, ShutdownError> {
        // Untaken writers return their state before the report counts them.
        self.held.release();
        let commands = self.commands.take().ok_or(ShutdownError::WorkerFailed)?;
        let (reply, progress) = mpsc::channel();
        commands
            .send(Command::Shutdown {
                deadline: budget::later(Instant::now(), deadline),
                reply,
            })
            .map_err(|_| ShutdownError::WorkerFailed)?;
        PendingShutdown { progress }.wait(deadline.saturating_add(REPORT_GRACE))
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

/// Combine per-output answers: Failed over Closing over Quota.
fn worse(result: Result<(), EventError>, error: EventError) -> Result<(), EventError> {
    let rank = |e: &EventError| match e {
        EventError::Failed => 3,
        EventError::Closing => 2,
        EventError::Quota => 1,
        EventError::NotConfigured => 0,
    };
    match result {
        Err(current) if rank(&current) >= rank(&error) => Err(current),
        _ => Err(error),
    }
}

/// Profile warnings for a plan (API §19.8). Writers with their own quota are
/// outside the preset's declared load.
fn warnings(
    sizing: &Sizing,
    budget_bytes: usize,
    quotas: &[Option<WriterQuota>],
) -> Vec<PlanWarning> {
    let Some(machine) = sizing.machine() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if let Some(available) = machine.available_memory {
        if budget_bytes as u64 > available {
            out.push(PlanWarning::Memory {
                budget_bytes,
                available_bytes: available,
            });
        }
    }
    let writers = quotas.iter().filter(|q| q.is_none()).count() as u64;
    let offered = sizing.offered_bytes_per_second().saturating_mul(writers);
    if offered > machine.load_limit {
        out.push(PlanWarning::Load {
            offered_bytes_per_second: offered,
            limit_bytes_per_second: machine.load_limit,
        });
    }
    out
}
