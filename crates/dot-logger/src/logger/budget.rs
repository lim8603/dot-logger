//! Facade budget inputs and the deterministic rule that derives core configs.
//!
//! Loss values (memory, sync period, drop deadline, slot counts, retention) have
//! no default and must be stated. Performance values (IO threads, batching,
//! idle wait) default here because a wrong choice changes speed, not loss
//! (ADR-012 §6).

use std::time::{Duration, Instant};

use crate::capture::CaptureOwner;
use crate::completion::{Boundaries, Boundary, CompletionConfig, TargetContract};
use crate::completion::{ProducerRange, Registry};
use crate::config::{
    ImportancePolicy, MemoryBudget, PoolQuota, ProducerQuota, Resource, SessionConfig,
    WorkerThreadingConfig,
};
use crate::control::ControlConfig;
use crate::control::ControlOwner;
#[cfg(not(target_os = "linux"))]
use crate::platform::CapabilityStatus;
use crate::platform::{PlatformCapabilities, Requirement};
use crate::routing::{Delivery, DispatchEvent, Router};
use crate::routing::{Route, RoutingConfig, Schedule, TargetConfig};
use crate::storage::{RetentionConfig, RetentionMode, StorageChannel, WriterConfig};
use crate::storage::{log_required, smallest_rotated_segment};
use crate::telemetry::Bank;

use super::event::EventSpec;
use super::writer::Owned;
use super::{Missing, StartError};

/// Stack reserved for every facade thread, counted in the worker partition.
pub(crate) const STACK_BYTES: usize = 2 * 1024 * 1024;
/// Queued deliveries older than this are dropped; only reachable on an IO stall.
const MAX_QUEUE_AGE: Duration = Duration::from_secs(24 * 60 * 60);
const SEGMENT_CEILING: u64 = 256 * 1024 * 1024;
const SEGMENT_FLOOR: u64 = 8 * 1024 * 1024;

/// Fixed ingress slots of one writer. Slot counts bound how much a burst can
/// queue before RT admission is rejected, so they are loss values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WriterQuota {
    /// Small-record slots copied by `try_log`.
    pub small_slots: usize,
    /// Largest encoded small record.
    pub small_slot_bytes: usize,
    /// Zero-copy original slots lent by `Producer::try_acquire`; zero disables.
    pub raw_slots: usize,
    /// Largest original payload.
    pub raw_slot_bytes: usize,
    /// Small slots only Important records may use; 0 (the default) reserves
    /// none. See [`important`](Self::important).
    pub important_slots: usize,
}

impl WriterQuota {
    /// Small records only.
    pub const fn small(slots: usize, slot_bytes: usize) -> Self {
        Self {
            small_slots: slots,
            small_slot_bytes: slot_bytes,
            raw_slots: 0,
            raw_slot_bytes: 0,
            important_slots: 0,
        }
    }

    /// Add a zero-copy original pool.
    pub const fn with_raw(mut self, slots: usize, slot_bytes: usize) -> Self {
        self.raw_slots = slots;
        self.raw_slot_bytes = slot_bytes;
        self
    }

    /// Reserve `slots` of the small slots for Important records (API §22.4,
    /// core `PoolQuota::important_slots`). Normal records are then rejected
    /// with `Capacity` once `small_slots - slots` of them are queued, so the
    /// reserved slots stay free for records marked with
    /// [`Channel::important`] or classified by
    /// [`ChannelOptions::important_from`]. Important records may also use the
    /// Normal slots. Default 0: no reserve, every record competes equally.
    ///
    /// The reserve must leave a Normal slot: `slots >= small_slots` rejects
    /// `start()` and `plan()` with [`StartError::InvalidBudget`]. Only the
    /// small pool (`try_log`) is reserved; the raw pool of
    /// [`with_raw`](Self::with_raw) has no Important reserve, so Important
    /// loans from `Producer::try_acquire` compete with Normal ones.
    ///
    /// [`Channel::important`]: crate::logger::Channel::important
    /// [`ChannelOptions::important_from`]: crate::logger::ChannelOptions::important_from
    /// [`StartError::InvalidBudget`]: crate::logger::StartError::InvalidBudget
    pub const fn important(mut self, slots: usize) -> Self {
        self.important_slots = slots;
        self
    }

    fn records(&self) -> Option<usize> {
        self.small_slots.checked_add(self.raw_slots)
    }

    fn small_bytes(&self) -> Option<usize> {
        self.small_slots.checked_mul(self.small_slot_bytes)
    }

    fn payload_bytes(&self) -> Option<usize> {
        self.small_bytes()?
            .checked_add(self.raw_slots.checked_mul(self.raw_slot_bytes)?)
    }

    fn producer(&self) -> ProducerQuota {
        ProducerQuota {
            small: PoolQuota {
                slots: self.small_slots,
                slot_bytes: self.small_slot_bytes,
                important_slots: self.important_slots,
            },
            raw: (self.raw_slots > 0).then_some(PoolQuota {
                slots: self.raw_slots,
                slot_bytes: self.raw_slot_bytes,
                important_slots: 0,
            }),
        }
    }
}

/// Explicit facade budget. Every loss value must be set before `start()`.
#[derive(Clone, Debug, Default)]
pub struct Budget {
    memory_bytes: Option<usize>,
    sync_every: Option<Duration>,
    drop_deadline: Option<Duration>,
    writer_quota: Option<WriterQuota>,
    io_threads: usize,
    batch_records: usize,
    batch_wait: Duration,
    idle_wait: Duration,
    worker_cpus: Option<(Vec<usize>, Requirement)>,
    /// Memory left to the preset: exact requirement plus 10 %.
    memory_auto: bool,
}

impl Budget {
    /// Empty budget: loss values unset, performance values at facade defaults
    /// (one IO thread, 256-record batches held at most 10 ms, 1 ms idle wait).
    pub fn new() -> Self {
        Self {
            io_threads: 1,
            batch_records: 256,
            batch_wait: Duration::from_millis(10),
            idle_wait: Duration::from_millis(1),
            ..Self::default()
        }
    }
    /// Total logger memory; the facade splits it into the core partitions.
    pub fn memory_bytes(mut self, bytes: usize) -> Self {
        self.memory_bytes = Some(bytes);
        self
    }
    /// Period of the ordinary file synchronization barrier. Records accepted
    /// after the last barrier can be lost on power failure.
    pub fn sync_every(mut self, period: Duration) -> Self {
        self.sync_every = Some(period);
        self
    }
    /// Bounded shutdown performed when a `Logger` is dropped without `shutdown`.
    pub fn drop_deadline(mut self, deadline: Duration) -> Self {
        self.drop_deadline = Some(deadline);
        self
    }
    /// Default quota for writers declared without one.
    pub fn writer_quota(mut self, quota: WriterQuota) -> Self {
        self.writer_quota = Some(quota);
        self
    }
    /// File IO threads; targets are assigned round-robin. Default 1.
    pub fn io_threads(mut self, threads: usize) -> Self {
        self.io_threads = threads;
        self
    }
    /// Largest write batch and longest hold before flushing it. The batch is
    /// capped at half of a target's record budget (all writers' slots); see
    /// `Plan::batch_records`.
    pub fn batching(mut self, records: usize, wait: Duration) -> Self {
        self.batch_records = records;
        self.batch_wait = wait;
        self
    }
    /// Longest sleep of an idle worker. RT writers never wake workers.
    pub fn idle_wait(mut self, wait: Duration) -> Self {
        self.idle_wait = wait;
        self
    }
    /// Restrict every worker thread (host and IO) to these CPUs to reduce
    /// interference with RT threads; this is not isolation. Default: no
    /// restriction. An unknown or duplicate CPU always rejects `start()`. An
    /// `Optional` placement that the platform cannot apply is reported by
    /// `Health::affinity`; a `Required` one rejects `start()`.
    pub fn worker_cpus(mut self, cpus: &[usize], requirement: Requirement) -> Self {
        self.worker_cpus = (!cpus.is_empty()).then(|| (cpus.to_vec(), requirement));
        self
    }
}

impl Budget {
    /// Fill the loss values this budget leaves unset from `sizing`, in a
    /// fixed order, and return which ones were filled. Set values are kept.
    pub(crate) fn apply(&mut self, sizing: &super::preset::Sizing) -> Vec<Missing> {
        let mut filled = Vec::new();
        if self.memory_bytes.is_none() {
            self.memory_auto = true;
            filled.push(Missing::Memory);
        }
        if self.sync_every.is_none() {
            self.sync_every = Some(sizing.sync_every);
            filled.push(Missing::SyncEvery);
        }
        if self.drop_deadline.is_none() {
            self.drop_deadline = Some(sizing.drop_deadline);
            filled.push(Missing::DropDeadline);
        }
        if self.writer_quota.is_none() {
            self.writer_quota = Some(sizing.quota());
            filled.push(Missing::WriterQuota);
        }
        filled
    }
    pub(crate) fn default_quota(&self) -> Option<WriterQuota> {
        self.writer_quota
    }
}

/// File retention policy of one output directory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Retention {
    mode: RetentionMode,
    bytes: u64,
    reserve: u64,
    events: Option<EventWindow>,
}

/// Records kept around a marked event on a ring output (API §22.3), for
/// [`Retention::events`].
///
/// [`Logger::mark_event`] protects the files holding records whose session
/// monotonic time lies in `[at - pre, at + post]` until `at + post + hold`
/// or [`Logger::release_event`]. `hold` and both [`limits`](Self::limits)
/// decide what the ring keeps, so they are loss values without a default:
/// leaving one out rejects `start()` and `plan()` with
/// [`StartError::Missing`] ([`Missing::EventHold`], [`Missing::EventLimits`]).
///
/// [`Logger::mark_event`]: crate::logger::Logger::mark_event
/// [`Logger::release_event`]: crate::logger::Logger::release_event
/// [`StartError::Missing`]: crate::logger::StartError::Missing
/// [`Missing::EventHold`]: crate::logger::Missing::EventHold
/// [`Missing::EventLimits`]: crate::logger::Missing::EventLimits
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventWindow {
    pre: Duration,
    post: Duration,
    hold: Option<Duration>,
    limits: Option<(usize, u64)>,
}

impl EventWindow {
    /// Keep `pre` of history before the mark and `post` after it.
    pub const fn new(pre: Duration, post: Duration) -> Self {
        Self {
            pre,
            post,
            hold: None,
            limits: None,
        }
    }
    /// Keep the window protected for `hold` after its end (`at + post`);
    /// the worker expires it at least once per `sync_every` after that.
    /// `post + hold` must not be zero, and should be well above the time a
    /// mark takes to reach the IO thread (normally under a millisecond),
    /// otherwise the mark fails because its expiry has already passed.
    pub const fn hold(mut self, hold: Duration) -> Self {
        self.hold = Some(hold);
        self
    }
    /// At most `max_events` windows at once, and at most `protected_bytes`
    /// of files protected by all of them together. Files are protected
    /// whole: a closed file counts its size, and the file being written
    /// counts a full segment (ring / 8, between 8 MiB and 256 MiB) while
    /// it holds a protected record. Both must be above zero, and the ring
    /// must keep two unprotected segments: `protected_bytes + 2 × segment`
    /// above the ring size rejects `start()` with [`StartError::Retention`].
    /// A mark beyond either limit is [`EventError::Quota`]; a record that
    /// would extend protection beyond `protected_bytes` fails on that output.
    ///
    /// [`StartError::Retention`]: crate::logger::StartError::Retention
    /// [`EventError::Quota`]: crate::logger::EventError::Quota
    pub const fn limits(mut self, max_events: usize, protected_bytes: u64) -> Self {
        self.limits = Some((max_events, protected_bytes));
        self
    }
}

impl Retention {
    /// Delete the oldest closed files to stay within `bytes`.
    ///
    /// `bytes` must hold two segments (at least 16 MiB); a smaller ring could
    /// never rotate, so start rejects it with [`StartError::Retention`].
    ///
    /// [`StartError::Retention`]: crate::logger::StartError::Retention
    pub const fn ring(bytes: u64) -> Self {
        Self {
            mode: RetentionMode::Circular,
            bytes,
            reserve: 0,
            events: None,
        }
    }
    /// Stop recording (reject at the file stage) once `bytes` are used.
    pub const fn continuous(bytes: u64) -> Self {
        Self {
            mode: RetentionMode::Continuous,
            bytes,
            reserve: 0,
            events: None,
        }
    }
    /// Filesystem free bytes that must remain untouched. Default 0.
    pub const fn reserve(mut self, bytes: u64) -> Self {
        self.reserve = bytes;
        self
    }
    /// Let [`Logger::mark_event`] keep the files around an event on this
    /// ring (API §22.3). The ring still deletes its oldest files, but skips
    /// files with records inside a protected window. Only a
    /// [`ring`](Self::ring) can hold events: on a
    /// [`continuous`](Self::continuous) output, which never deletes,
    /// `start()` and `plan()` fail with [`StartError::InvalidBudget`].
    ///
    /// ```no_run
    /// use dot_logger::logger::{EventWindow, Retention};
    /// use std::time::Duration;
    /// const MIB: u64 = 1 << 20;
    ///
    /// // Keep 30 s before and 10 s after each mark for one more hour;
    /// // at most 8 marks and 512 MiB of protected files at a time.
    /// let retention = Retention::ring(4096 * MIB).events(
    ///     EventWindow::new(Duration::from_secs(30), Duration::from_secs(10))
    ///         .hold(Duration::from_secs(3600))
    ///         .limits(8, 512 * MIB),
    /// );
    /// # let _ = retention;
    /// ```
    ///
    /// [`Logger::mark_event`]: crate::logger::Logger::mark_event
    /// [`StartError::InvalidBudget`]: crate::logger::StartError::InvalidBudget
    pub const fn events(mut self, window: EventWindow) -> Self {
        self.events = Some(window);
        self
    }
    /// Event loss values this output leaves unset, in a fixed order.
    pub(crate) fn missing(&self) -> Vec<Missing> {
        self.events
            .as_ref()
            .map_or_else(Vec::new, EventWindow::missing)
    }
}

impl EventWindow {
    /// Loss values left unset, in a fixed order.
    fn missing(&self) -> Vec<Missing> {
        [
            self.hold.is_none().then_some(Missing::EventHold),
            self.limits.is_none().then_some(Missing::EventLimits),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

/// Core event settings of one ring output: (pre, post, hold) in ns and the
/// limits. Zero limits, a zero `post + hold` and a window too long for
/// session nanoseconds are `InvalidBudget`.
fn event_config(
    window: &EventWindow,
    ring: u64,
    segment: u64,
) -> Result<(u64, EventSpec, u64), StartError> {
    let (Some(hold), Some((max_events, protected))) = (window.hold, window.limits) else {
        return Err(StartError::Missing(window.missing()));
    };
    let ns = |d: Duration| u64::try_from(d.as_nanos()).map_err(|_| StartError::InvalidBudget);
    let (pre_ns, post_ns, hold_ns) = (ns(window.pre)?, ns(window.post)?, ns(hold)?);
    if max_events == 0 || protected == 0 || post_ns.checked_add(hold_ns).is_none_or(|n| n == 0) {
        return Err(StartError::InvalidBudget);
    }
    // Deleting unprotected files must always make room for the rotating
    // segment and the next one (the ring minimum), whatever is protected.
    let required = protected
        .checked_add(segment.saturating_mul(2))
        .ok_or(StartError::InvalidBudget)?;
    if required > ring {
        return Err(StartError::Retention {
            required,
            available: ring,
        });
    }
    let spec = EventSpec {
        post_ns,
        hold_ns,
        max_events,
    };
    Ok((pre_ns, spec, protected))
}

/// Dispatch events the host drains per turn.
pub(crate) const HOST_EVENTS: usize = 256;

/// Heap the facade itself preallocates beside the core (ADR-012, REV-051
/// RV-124): per writer the encode scratch (one small slot) and the 1-slot
/// return mailbox; per output the IO write batch and, for `events` event
/// slots in total, the mark map of its IO thread; on the host the dispatch
/// event buffer and three per-producer sync range arrays. Thread stacks,
/// channel queues and the bounded sync work list are not included.
pub(crate) fn facade_bytes(
    quotas: &[WriterQuota],
    outputs: usize,
    batch_records: usize,
    events: usize,
) -> Option<usize> {
    use std::mem::size_of;
    let mailbox = size_of::<rtrb::RingBuffer<Owned>>() + size_of::<Owned>();
    let writers = quotas.iter().try_fold(0usize, |sum, q| {
        sum.checked_add(q.small_slot_bytes.checked_add(mailbox)?)
    })?;
    let batches = outputs
        .checked_mul(batch_records)?
        .checked_mul(size_of::<Delivery<'static>>())?;
    let host = HOST_EVENTS
        .checked_mul(size_of::<Option<DispatchEvent>>())?
        .checked_add(
            quotas
                .len()
                .checked_mul(3)?
                .checked_mul(size_of::<ProducerRange>())?,
        )?;
    writers
        .checked_add(batches)?
        .checked_add(host)?
        .checked_add(super::event::held_bytes(events)?)
}

/// Longest horizon an `Instant` deadline is clamped to: far enough to mean
/// "unbounded" for any caller, small enough never to overflow (RV-123).
const HORIZON: Duration = Duration::from_secs(100 * 365 * 24 * 60 * 60);

/// `now + by`, clamped to a far horizon instead of overflowing.
pub(crate) fn later(now: Instant, by: Duration) -> Instant {
    now.checked_add(by.min(HORIZON)).unwrap_or(now)
}

/// Deadline span of one ordinary sync: four periods plus one second.
pub(crate) fn sync_overdue(sync_every: Duration) -> Duration {
    sync_every
        .saturating_mul(4)
        .saturating_add(Duration::from_secs(1))
}

/// Facade performance settings resolved from a [`Budget`].
#[derive(Clone, Copy, Debug)]
pub(crate) struct Tuning {
    pub(crate) sync_every: Duration,
    pub(crate) drop_deadline: Duration,
    pub(crate) io_threads: usize,
    pub(crate) batch_records: usize,
    pub(crate) batch_wait: Duration,
    pub(crate) idle_wait: Duration,
}

/// Every core configuration the default worker needs.
pub(crate) struct Derived {
    pub(crate) session: SessionConfig,
    pub(crate) routing: RoutingConfig,
    pub(crate) completion: CompletionConfig,
    pub(crate) control: ControlConfig,
    pub(crate) writer: WriterConfig,
    pub(crate) retention: Vec<RetentionConfig>,
    /// Event window of each output, in declaration order.
    pub(crate) events: Vec<Option<EventSpec>>,
    pub(crate) tuning: Tuning,
    /// Memory the plan needs before any margin.
    pub(crate) required: usize,
    /// Worker CPU list and the platform observation it was validated against.
    pub(crate) cpus: Option<(Vec<usize>, Requirement)>,
    pub(crate) platform: PlatformCapabilities,
}

/// Loss values absent from `budget`, in a fixed order.
pub(crate) fn missing(budget: &Budget, quotas: &[Option<WriterQuota>]) -> Vec<Missing> {
    let mut missing = Vec::new();
    if budget.memory_bytes.is_none() && !budget.memory_auto {
        missing.push(Missing::Memory);
    }
    if budget.sync_every.is_none() {
        missing.push(Missing::SyncEvery);
    }
    if budget.drop_deadline.is_none() {
        missing.push(Missing::DropDeadline);
    }
    if budget.writer_quota.is_none() && quotas.iter().any(Option::is_none) {
        missing.push(Missing::WriterQuota);
    }
    missing
}

fn budget_error(resource: Resource, required: usize, available: usize) -> StartError {
    StartError::Budget {
        resource,
        required,
        available,
    }
}

/// Memory and threads an optional adapter (remote) adds to the plan.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Extra {
    /// Adapter buffers charged to the external partition.
    pub(crate) external_bytes: usize,
    /// Additional worker threads (each with `STACK_BYTES`).
    pub(crate) threads: usize,
    /// Telemetry counters reserved in the control partition.
    pub(crate) telemetry: bool,
}

/// Quotas with the budget default applied; `None` when one is missing.
#[cfg(feature = "remote")]
pub(crate) fn resolve(budget: &Budget, quotas: &[Option<WriterQuota>]) -> Option<Vec<WriterQuota>> {
    quotas.iter().map(|q| q.or(budget.writer_quota)).collect()
}

/// Deterministic derivation; see API §18.2. All counts follow from the quotas.
pub(crate) fn derive(
    budget: &Budget,
    quotas: &[Option<WriterQuota>],
    channels: &[StorageChannel],
    routes: &[Vec<u32>],
    retention: &[Retention],
    extra: Extra,
) -> Result<Derived, StartError> {
    let (Some(sync_every), Some(drop_deadline)) = (budget.sync_every, budget.drop_deadline) else {
        return Err(StartError::Missing(missing(budget, quotas)));
    };
    if budget.memory_bytes.is_none() && !budget.memory_auto {
        return Err(StartError::Missing(missing(budget, quotas)));
    }
    let quotas: Vec<WriterQuota> = quotas
        .iter()
        .map(|q| q.or(budget.writer_quota))
        .collect::<Option<_>>()
        .ok_or_else(|| StartError::Missing(vec![Missing::WriterQuota]))?;
    if sync_every.is_zero()
        || budget.io_threads == 0
        || budget.batch_records == 0
        || quotas.iter().any(|q| {
            q.small_slots == 0
                || q.small_slot_bytes == 0
                || (q.raw_slots > 0) != (q.raw_slot_bytes > 0)
                // The reserve must leave a Normal slot (API §22.4).
                || q.important_slots >= q.small_slots
        })
    {
        return Err(StartError::InvalidBudget);
    }
    let overflow = || StartError::InvalidBudget;
    let producers: Vec<ProducerQuota> = quotas.iter().map(WriterQuota::producer).collect();

    // Memory: exact raw and input, worker stacks and adapter buffers; control
    // and output are sized below from the constructors' own formulas.
    let mut raw = 0usize;
    let mut input = 0usize;
    for p in &producers {
        input = input
            .checked_add(CaptureOwner::input_entry_bytes(&p.small).ok_or_else(overflow)?)
            .ok_or_else(overflow)?;
        if let Some(r) = &p.raw {
            raw = raw
                .checked_add(r.slots.checked_mul(r.slot_bytes).ok_or_else(overflow)?)
                .ok_or_else(overflow)?;
        }
    }
    let threads = budget
        .io_threads
        .checked_add(1 + extra.threads)
        .ok_or_else(overflow)?;
    // A batch never holds more than half of a target's record budget (every
    // target's queue holds all writers' records): otherwise a writer with
    // fewer slots than the batch waits `batch_wait` for each slot-full
    // (TASK-023 S2b ③, Human 2026-11-07).
    let budget_records = quotas
        .iter()
        .try_fold(0usize, |sum, q| sum.checked_add(q.records()?))
        .ok_or_else(overflow)?;
    let batch_records = budget.batch_records.min((budget_records / 2).max(1));
    // Event slots of every output; validated with the retention below.
    let event_slots = retention
        .iter()
        .filter_map(|r| r.events.and_then(|w| w.limits))
        .try_fold(0usize, |sum, (n, _)| sum.checked_add(n))
        .ok_or_else(overflow)?;
    let worker = threads
        .checked_mul(STACK_BYTES)
        .and_then(|n| {
            n.checked_add(facade_bytes(
                &quotas,
                retention.len(),
                batch_records,
                event_slots,
            )?)
        })
        .ok_or_else(overflow)?;
    let fixed = raw
        .checked_add(input)
        .and_then(|n| n.checked_add(worker))
        .and_then(|n| n.checked_add(extra.external_bytes))
        .ok_or_else(overflow)?;
    // Routing: the shared record budget saturates before any target queue, so
    // overload backpressures into ingress and appears as RT Capacity rejection,
    // never as a silent required-target drop.
    let total = |each: fn(&WriterQuota) -> Option<usize>| {
        quotas
            .iter()
            .try_fold(0usize, |sum, q| sum.checked_add(each(q)?))
            .ok_or_else(overflow)
    };
    let records = total(WriterQuota::records)?;
    let bytes = total(WriterQuota::payload_bytes)?;
    let small_copy = total(WriterQuota::small_bytes)?;
    let max_payload = quotas
        .iter()
        .map(|q| q.small_slot_bytes.max(q.raw_slot_bytes))
        .max()
        .ok_or(StartError::InvalidBudget)?;
    // Every per-writer count fits: their sum (`records`) did.
    let per_writer = quotas
        .iter()
        .filter_map(WriterQuota::records)
        .max()
        .ok_or(StartError::InvalidBudget)?;
    let max_raw = quotas.iter().map(|q| q.raw_slots).max().unwrap_or(0).max(1);
    let targets: Vec<u32> = (1..=retention.len() as u32).collect();
    let routing = RoutingConfig {
        targets: targets
            .iter()
            .map(|&id| TargetConfig {
                id,
                required: true,
                queue_slots: records,
                pending_slots: 0,
                retained_bytes: bytes,
                max_attempts: 1,
                max_age: MAX_QUEUE_AGE,
            })
            .collect(),
        routes: channels
            .iter()
            .zip(routes)
            .map(|(c, targets)| Route {
                channel_id: c.channel.channel_id,
                targets: targets.clone(),
            })
            .collect(),
        shared_records: records,
        small_copy_bytes: small_copy,
        schedule: Schedule {
            record_quantum: per_writer,
            byte_quantum: bytes.max(max_payload),
            records_per_turn: records,
            bytes_per_turn: bytes.max(max_payload),
            retries_per_target: 1,
            reclaim_slots_per_producer: max_raw,
        },
    };

    // Completion ends at Written; durability comes from the sync barriers, so
    // the window only spans records between admission and the file write.
    let window = per_writer.checked_mul(2).ok_or_else(overflow)?;
    let feedback = records.checked_mul(2).ok_or_else(overflow)?;
    let reconcile = window.checked_mul(quotas.len()).ok_or_else(overflow)?;
    // Reconcile only marks abandoned deliveries Unknown, so one writer's
    // window per turn sweeps the table every `writers` turns. Sweeping all of
    // it on every turn cost ~3 ms at 32 writers and capped dispatch below
    // 100k small records/s (TASK-023 S2d finding ⑨).
    let sweep = window;
    let completion = CompletionConfig {
        history_retention: Duration::from_secs(60),
        window_records: window,
        max_queries: 1,
        page_items: 64,
        feedback_events: feedback,
        updates_per_turn: feedback,
        reconcile_per_turn: sweep,
        targets: targets
            .iter()
            .map(|&id| TargetContract {
                id,
                required: true,
                supported: Boundaries::FILE,
                completion: Boundary::Written,
            })
            .collect(),
    };
    let control = ControlConfig {
        sync_slots: 1,
        work_per_poll: reconcile,
    };

    let metadata: usize = channels
        .iter()
        .map(|c| {
            c.topic.len()
                + c.payload_encoding.len()
                + c.schema.as_ref().map_or(0, |s| {
                    s.encoding.len() + s.descriptor.len() + s.name.as_ref().map_or(0, String::len)
                })
        })
        .sum();
    let writer = WriterConfig {
        max_payload_bytes: max_payload,
        write_buffer_bytes: 64 * 1024,
        chunk_target_bytes: 256 * 1024,
        max_records: 16384,
        max_chunks: 4096,
        max_metadata_bytes: metadata + 1024,
    };

    let floor = SEGMENT_FLOOR.max((max_payload as u64).saturating_mul(2) + 1024 * 1024);
    let mut events = Vec::with_capacity(retention.len());
    let retention = retention
        .iter()
        .map(|r| {
            if r.bytes < floor {
                return Err(StartError::Retention {
                    required: floor,
                    available: r.bytes,
                });
            }
            let segment = (r.bytes / 8).clamp(floor, SEGMENT_CEILING.max(floor));
            // A ring rotates only when the closing segment and the next one
            // both fit, so below two segments it would fail almost every
            // record after a successful start (TASK-033 S3).
            let ring_minimum = segment.saturating_mul(2);
            if r.mode == RetentionMode::Circular && r.bytes < ring_minimum {
                return Err(StartError::Retention {
                    required: ring_minimum,
                    available: r.bytes,
                });
            }
            // Event protection only makes sense where files are deleted.
            let (mode, (pre_ns, post_ns, max_events, protected_bytes)) = match &r.events {
                None => {
                    events.push(None);
                    (r.mode, (0, 0, 0, 0))
                }
                Some(_) if r.mode != RetentionMode::Circular => {
                    return Err(StartError::InvalidBudget);
                }
                Some(window) => {
                    let (pre, spec, protected) = event_config(window, r.bytes, segment)?;
                    events.push(Some(spec));
                    (
                        RetentionMode::Event,
                        (pre, spec.post_ns, spec.max_events, protected),
                    )
                }
            };
            Ok(RetentionConfig {
                mode,
                disk_bytes: r.bytes,
                disk_reserve_bytes: r.reserve,
                max_age_ns: u64::MAX,
                max_segment_bytes: segment,
                max_segment_ns: 60_000_000_000,
                max_index_bytes: 4 * 1024 * 1024,
                protected_bytes,
                pre_ns,
                post_ns,
                // Small records close segments far below `segment` bytes (by
                // record bounds or `max_records`); size the count for the
                // smallest such segment so the configured bytes are what
                // limits retention (TASK-023 S2b ⑥, Human 2026-11-07).
                max_segments: usize::try_from(
                    r.bytes
                        .div_ceil(smallest_rotated_segment(segment, writer.max_records)),
                )
                .map_err(|_| StartError::InvalidBudget)?
                .checked_add(2)
                .ok_or(StartError::InvalidBudget)?,
                max_events,
                max_pins: 0,
                max_pin_bytes: 0,
                max_pin_ns: 0,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Control and output: the exact reservations the core constructors make
    // for these configs (API §19.2, replaces the fixed 1/4 split of RV-097).
    let producer_count = producers.len();
    let target_count = targets.len();
    let mut raw_metadata = 0usize;
    for p in &producers {
        if let Some(r) = &p.raw {
            raw_metadata = raw_metadata
                .checked_add(CaptureOwner::raw_metadata_bytes(r).ok_or_else(overflow)?)
                .ok_or_else(overflow)?;
        }
    }
    let mut control_need =
        CaptureOwner::registration_bytes(channels.len(), producer_count, raw_metadata)
            .ok_or_else(overflow)?;
    let parts = [
        Registry::required(producer_count, window, target_count, &routing)
            .map_err(|_| overflow())?,
        ControlOwner::own_bytes(control.sync_slots, producer_count, target_count)
            .ok_or_else(overflow)?,
        if extra.telemetry {
            Bank::required(producer_count).ok_or_else(overflow)?
        } else {
            0
        },
    ];
    for part in parts {
        control_need = control_need.checked_add(part).ok_or_else(overflow)?;
    }
    let mut output_need =
        Router::required(producer_count, &routing, Some(feedback)).map_err(|_| overflow())?;
    for r in &retention {
        output_need = output_need
            .checked_add(log_required(writer, channels, producer_count, r)?)
            .ok_or_else(overflow)?;
    }
    let need = fixed
        .checked_add(control_need)
        .and_then(|n| n.checked_add(output_need))
        .ok_or_else(overflow)?;
    // A preset leaves memory to the exact requirement plus 10 % (API §19.1).
    let total = match budget.memory_bytes {
        Some(total) => total,
        None => need.checked_add(need.div_ceil(10)).ok_or_else(overflow)?,
    };
    if need > total {
        return Err(budget_error(Resource::Total, need, total));
    }
    // Spare memory goes to output, where writers use it for buffering.
    let memory = MemoryBudget {
        total_bytes: total,
        input_bytes: input,
        raw_bytes: raw,
        output_bytes: total - fixed - control_need,
        external_bytes: extra.external_bytes,
        control_bytes: control_need,
        worker_bytes: worker,
    };
    let session = SessionConfig {
        memory,
        producers,
        importance: ImportancePolicy::PrioritizeImportant,
        workers: WorkerThreadingConfig {
            count: threads,
            stack_bytes: STACK_BYTES,
            cpu_ids: budget
                .worker_cpus
                .as_ref()
                .map_or_else(Vec::new, |(cpus, _)| cpus.clone()),
            affinity: budget
                .worker_cpus
                .as_ref()
                .map_or(Requirement::Optional, |&(_, r)| r),
        },
        capabilities: Vec::new(),
    };

    Ok(Derived {
        session,
        routing,
        completion,
        control,
        writer,
        retention,
        events,
        tuning: Tuning {
            sync_every,
            drop_deadline,
            io_threads: budget.io_threads.min(targets.len().max(1)),
            batch_records,
            batch_wait: budget.batch_wait,
            idle_wait: budget.idle_wait,
        },
        required: need,
        cpus: budget.worker_cpus.clone(),
        platform: platform(budget.worker_cpus.as_ref()),
    })
}

/// Observe only what placement validation needs. Without a CPU list nothing
/// is probed. Off Linux the requested CPUs are taken as given and placement is
/// Unsupported, so `Optional` falls back and `Required` is rejected.
fn platform(cpus: Option<&(Vec<usize>, Requirement)>) -> PlatformCapabilities {
    let Some((cpus, _)) = cpus else {
        return PlatformCapabilities::default();
    };
    #[cfg(target_os = "linux")]
    {
        let _ = cpus;
        super::affinity::observe()
    }
    #[cfg(not(target_os = "linux"))]
    {
        PlatformCapabilities {
            available_cpu_ids: cpus.clone(),
            worker_affinity: CapabilityStatus::Unsupported,
            ..PlatformCapabilities::default()
        }
    }
}
