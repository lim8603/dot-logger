//! Bounded management-path fanout. No threads or IO are started here.
//!
//! All methods and delivery destruction belong on general/background threads.
//! Shared guards preserve CaptureOwner's lifetime without modifying its unsafe
//! boundary. Small copies/shared headers allocate only after bounded reservation;
//! allocator overhead belongs to explicit worker headroom. Target queues and retry
//! rings are preallocated. Event buffers belong to the caller. A full event buffer
//! leaves work in place instead of losing an outcome. Router Drop is general
//! cleanup, not a successful drain, completion acknowledgment, or shutdown.

use crate::capture::{CaptureConsumer, CapturedRecord, ReadError};
use crate::completion::{
    AbandonMarker, Boundary, Completion, CompletionError, ProducerRange, QueryOutcome, Update,
};
use crate::config::{ConfigError, Resource, ValidatedConfig};
use crate::pool::RawRecord;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

mod types;
pub use types::*;

#[derive(Default)]
pub(crate) struct SharedLedger {
    records: AtomicUsize,
    small: AtomicUsize,
    raw: AtomicUsize,
    pub(crate) storage_limit: usize,
    pub(crate) storage_used: AtomicUsize,
}
#[derive(Default)]
pub(crate) struct TargetLedger {
    records: AtomicUsize,
    bytes: AtomicUsize,
    queued: AtomicUsize,
    pending: AtomicUsize,
    cancelled: AtomicBool,
    pub(crate) writer_active: AtomicBool,
    pub(crate) files: crate::storage::FileLedger,
}
impl TargetLedger {
    fn snapshot(&self) -> TargetSnapshot {
        let records = self.records.load(Ordering::Acquire);
        let queued = self.queued.load(Ordering::Acquire);
        let pending = self.pending.load(Ordering::Acquire);
        TargetSnapshot {
            queued,
            pending,
            in_flight: records.saturating_sub(queued.saturating_add(pending)),
            retained_records: records,
            retained_bytes: self.bytes.load(Ordering::Acquire),
            cancelled: self.cancelled.load(Ordering::Acquire),
        }
    }
}

enum Payload<'owner> {
    Small(Box<[u8]>),
    Raw(RawRecord<'owner>),
}
struct SharedPermit {
    ledger: Arc<SharedLedger>,
    small: usize,
    raw: usize,
}
#[cfg(test)]
thread_local! {
    /// Permit releases on this thread, in order (release-order tests only).
    static RELEASES: std::cell::RefCell<Vec<&'static str>> =
        const { std::cell::RefCell::new(Vec::new()) };
}
#[cfg(test)]
fn note_release(what: &'static str) {
    RELEASES.with(|r| r.borrow_mut().push(what));
}
impl Drop for SharedPermit {
    fn drop(&mut self) {
        #[cfg(test)]
        note_release("shared");
        self.ledger.small.fetch_sub(self.small, Ordering::Release);
        self.ledger.raw.fetch_sub(self.raw, Ordering::Release);
        self.ledger.records.fetch_sub(1, Ordering::Release);
    }
}
struct SharedPayload<'owner> {
    // Field order releases the payload before releasing the corresponding permit.
    payload: Payload<'owner>,
    record: CapturedRecord,
    _permit: SharedPermit,
    _plan: Option<crate::control::config_update::PlanGuard<'owner>>,
}
impl SharedPayload<'_> {
    fn bytes(&self) -> &[u8] {
        match &self.payload {
            Payload::Small(bytes) => bytes,
            Payload::Raw(raw) => raw.payload(),
        }
    }
}
struct TargetPermit {
    ledger: Arc<TargetLedger>,
    bytes: usize,
}
impl Drop for TargetPermit {
    fn drop(&mut self) {
        #[cfg(test)]
        note_release("target");
        self.ledger.bytes.fetch_sub(self.bytes, Ordering::Release);
        self.ledger.records.fetch_sub(1, Ordering::Release);
    }
}

/// Immutable management/IO guard; cannot be cloned or outlive its capture owner.
/// Keep it until an active IO operation actually finishes or confirms cancellation.
/// Dropping it releases this target's retention, without reporting write success.
///
/// ```compile_fail
/// use dot_logger::routing::Delivery;
/// fn duplicate(item: Delivery<'_>) { let _ = item.clone(); }
/// ```
/// ```compile_fail
/// use dot_logger::routing::Delivery;
/// fn escape(item: Delivery<'_>) -> &[u8] { item.payload() }
/// ```
pub struct Delivery<'owner> {
    // Field order releases this target's permit before the shared record: the
    // reverse lets the dispatcher take a fresh shared record while this
    // target's ledger still counts the old one and drop it as Capacity.
    _permit: TargetPermit,
    shared: Arc<SharedPayload<'owner>>,
    target_id: u32,
    required: bool,
    started: Instant,
    completion: Option<Completion<'owner>>,
    abandon: Option<AbandonMarker<'owner>>,
}
impl Delivery<'_> {
    /// Original bytes; raw fanout preserves the capture payload pointer.
    pub fn payload(&self) -> &[u8] {
        self.shared.bytes()
    }
    /// Original metadata, ingress identity, and immutable admission revision.
    pub fn record(&self) -> CapturedRecord {
        self.shared.record
    }
    /// Whether the payload is a zero-copy original (raw) rather than a small copy.
    #[cfg(feature = "remote")]
    pub(crate) fn is_raw(&self) -> bool {
        matches!(self.shared.payload, Payload::Raw(_))
    }
    /// Target retaining this guard.
    pub fn target_id(&self) -> u32 {
        self.target_id
    }
    /// Required/optional role from the frozen plan.
    pub fn required(&self) -> bool {
        self.required
    }
    /// Report an adapter boundary/failure without releasing this IO guard.
    /// Busy changes nothing; retain the guard and retry on the general path.
    /// Releasing an unreported tracked guard becomes Unknown, never success.
    pub fn report(&self, update: Update) -> Result<(), CompletionError> {
        self.completion.ok_or(CompletionError::Untracked)?.report(
            self.record().receipt,
            self.target_id,
            update,
        )
    }
    fn dropped_event(mut self, reason: DropReason) -> DispatchEvent {
        if let Some(marker) = &mut self.abandon {
            marker.disarm();
        }
        self.event(DispatchOutcome::Dropped(reason))
    }
    fn event(&self, outcome: DispatchOutcome) -> DispatchEvent {
        DispatchEvent {
            receipt: self.record().receipt,
            revision: self.record().revision,
            target_id: Some(self.target_id),
            required: self.required,
            outcome,
        }
    }
}
struct Pending<'owner> {
    delivery: Delivery<'owner>,
    attempts: usize,
}
struct PendingQueue<'owner> {
    slots: Box<[Option<Pending<'owner>>]>,
    head: usize,
    len: usize,
}
impl<'owner> PendingQueue<'owner> {
    fn new(capacity: usize) -> Self {
        Self {
            slots: std::iter::repeat_with(|| None).take(capacity).collect(),
            head: 0,
            len: 0,
        }
    }
    fn full(&self) -> bool {
        self.len == self.slots.len()
    }
    fn push_back(&mut self, item: Pending<'owner>) {
        let index = (self.head + self.len) % self.slots.len();
        self.slots[index] = Some(item);
        self.len += 1;
    }
    fn push_front(&mut self, item: Pending<'owner>) {
        self.head = (self.head + self.slots.len() - 1) % self.slots.len();
        self.slots[self.head] = Some(item);
        self.len += 1;
    }
    fn pop(&mut self) -> Option<Pending<'owner>> {
        if self.len == 0 {
            return None;
        }
        let result = self.slots[self.head].take();
        self.head = (self.head + 1) % self.slots.len();
        self.len -= 1;
        result
    }
}
struct Input<'owner> {
    consumer: CaptureConsumer<'owner>,
    deficit: usize,
    max_payload: usize,
    /// Highest sequence taken out of this ingress; every lower one left too.
    taken: u64,
}
struct Target<'owner> {
    config: TargetConfig,
    write: rtrb::Producer<Delivery<'owner>>,
    read: rtrb::Consumer<Delivery<'owner>>,
    pending: PendingQueue<'owner>,
    ledger: Arc<TargetLedger>,
    outcomes_write: Option<rtrb::Producer<DispatchEvent>>,
    outcomes_read: Option<rtrb::Consumer<DispatchEvent>>,
    /// Per producer: every sequence up to here is written or terminal on this
    /// target (monotonic sync readiness watermark).
    written: Box<[u64]>,
}
struct TargetWriter<'scope, 'owner> {
    config: &'scope TargetConfig,
    write: &'scope mut rtrb::Producer<Delivery<'owner>>,
    pending: &'scope mut PendingQueue<'owner>,
    ledger: &'scope Arc<TargetLedger>,
    outcomes: Option<&'scope mut rtrb::Consumer<DispatchEvent>>,
    completion: Option<Completion<'owner>>,
    written: &'scope mut [u64],
}
#[derive(Default)]
struct Cursor {
    next: usize,
    last_now: Option<Instant>,
    closed: bool,
}

struct Feedback {
    slots: Box<[Option<DispatchEvent>]>,
    head: usize,
    len: usize,
}
impl Feedback {
    fn new(capacity: usize) -> Self {
        Self {
            slots: vec![None; capacity].into_boxed_slice(),
            head: 0,
            len: 0,
        }
    }
    fn available(&self) -> usize {
        self.slots.len() - self.len
    }
    fn push(&mut self, event: DispatchEvent) {
        let index = (self.head + self.len) % self.slots.len();
        self.slots[index] = Some(event);
        self.len += 1;
    }
    fn flush(&mut self, completion: Completion<'_>, limit: usize) -> Result<(), CompletionError> {
        for _ in 0..self.len.min(limit) {
            let Some(event) = self.slots[self.head] else {
                break;
            };
            match completion.apply_dispatch(event) {
                Ok(()) | Err(CompletionError::Expired) => {}
                Err(CompletionError::Busy) => break,
                Err(error) => return Err(error),
            }
            self.slots[self.head] = None;
            self.head = (self.head + 1) % self.slots.len();
            self.len -= 1;
        }
        Ok(())
    }
}
/// Internal resource snapshot used only by the concrete shutdown coordinator.
#[derive(Default)]
pub(crate) struct DrainState {
    pub(crate) ingress: usize,
    pub(crate) queued: usize,
    pub(crate) pending: usize,
    pub(crate) in_flight: usize,
    pub(crate) feedback: usize,
    pub(crate) live: usize,
    pub(crate) raw_loaned: usize,
    pub(crate) raw_in_use: usize,
    pub(crate) raw_queued: usize,
    pub(crate) raw_return_pending: usize,
}
impl DrainState {
    pub(crate) fn is_empty(&self) -> bool {
        self.ingress == 0
            && self.queued == 0
            && self.pending == 0
            && self.in_flight == 0
            && self.feedback == 0
            && self.live == 0
            && self.raw_loaned == 0
            && self.raw_in_use == 0
            && self.raw_queued == 0
            && self.raw_return_pending == 0
    }
}

/// Owns fixed output FIFOs, retry rings, route metadata, and borrowed capture inputs.
/// Allocate/drop on a management thread inside CaptureOwner::scope. Output guards
/// may survive a routing scope but remain counted and bound to the capture owner.
///
/// ```compile_fail
/// use dot_logger::capture::CaptureOwner;
/// use dot_logger::config::ValidatedConfig;
/// use dot_logger::routing::{Router, RoutingConfig};
/// fn escape(owner: &mut CaptureOwner, plan: &ValidatedConfig, config: RoutingConfig) {
///     let router = owner.scope(|scope| Router::new(plan, scope.map(|(_, c)| c).collect(), config));
///     drop(router);
/// }
/// ```
pub struct Router<'owner> {
    inputs: Box<[Input<'owner>]>,
    targets: Box<[Target<'owner>]>,
    routes: Box<[Route]>,
    config: Limits,
    ledger: Arc<SharedLedger>,
    cursor: Cursor,
    output_reserved_bytes: usize,
    raw_reserved_bytes: usize,
    session_id: u128,
    revision: u64,
    updates: Option<&'owner crate::control::config_update::State>,
    completion: Option<Completion<'owner>>,
    feedback: Feedback,
}
#[derive(Clone, Copy)]
struct Limits {
    schedule: Schedule,
    shared_records: usize,
    small_copy_bytes: usize,
}

fn array_bytes(count: usize, size: usize) -> Result<usize, RoutingError> {
    count
        .checked_mul(size)
        .filter(|&n| n <= isize::MAX as usize)
        .ok_or(RoutingError::Config(ConfigError::ArithmeticOverflow))
}
fn add(sum: &mut usize, n: usize) -> Result<(), RoutingError> {
    *sum = sum
        .checked_add(n)
        .ok_or(RoutingError::Config(ConfigError::ArithmeticOverflow))?;
    Ok(())
}
// Conservative Arc header/padding allowance. Allocator bookkeeping/rounding is
// worker headroom, consistent with CaptureOwner's concrete reservation contract.
fn arc_bytes<T>() -> usize {
    size_of::<T>() + 2 * size_of::<usize>() + align_of::<T>()
}

impl<'owner> Router<'owner> {
    /// Output bytes `new` reserves for these counts; `feedback` is the
    /// completion feedback capacity when tracked. Shared with the facade sizing
    /// so both use one formula.
    pub(crate) fn required(
        producers: usize,
        config: &RoutingConfig,
        feedback: Option<usize>,
    ) -> Result<usize, RoutingError> {
        let mut reserved = size_of::<Self>()
            + size_of::<Dispatcher<'_, '_>>()
            + size_of::<Vec<TargetConsumer<'_, '_>>>()
            + arc_bytes::<SharedLedger>();
        add(
            &mut reserved,
            array_bytes(producers, size_of::<Input<'_>>())?,
        )?;
        add(
            &mut reserved,
            array_bytes(config.routes.len(), size_of::<Route>())?,
        )?;
        for route in &config.routes {
            add(
                &mut reserved,
                array_bytes(route.targets.len(), size_of::<u32>())?,
            )?;
        }
        add(
            &mut reserved,
            array_bytes(
                config.targets.len(),
                size_of::<Target<'_>>()
                    + size_of::<TargetWriter<'_, '_>>()
                    + size_of::<TargetConsumer<'_, '_>>()
                    + arc_bytes::<TargetLedger>()
                    + size_of::<rtrb::RingBuffer<Delivery<'_>>>(),
            )?,
        )?;
        for target in &config.targets {
            add(
                &mut reserved,
                array_bytes(target.queue_slots, size_of::<Delivery<'_>>())?,
            )?;
            add(&mut reserved, array_bytes(producers, size_of::<u64>())?)?;
            add(
                &mut reserved,
                array_bytes(producers, crate::storage::FileLedger::PRODUCER_BYTES)?,
            )?;
            add(
                &mut reserved,
                array_bytes(target.pending_slots, size_of::<Option<Pending<'_>>>())?,
            )?;
        }
        add(
            &mut reserved,
            array_bytes(config.shared_records, arc_bytes::<SharedPayload<'_>>())?,
        )?;
        add(&mut reserved, array_bytes(config.small_copy_bytes, 1)?)?;
        let feedback_capacity = feedback.unwrap_or(0);
        add(
            &mut reserved,
            array_bytes(feedback_capacity, size_of::<Option<DispatchEvent>>())?,
        )?;
        if feedback.is_some() {
            for _ in &config.targets {
                add(
                    &mut reserved,
                    array_bytes(feedback_capacity, size_of::<DispatchEvent>())?,
                )?;
                add(&mut reserved, size_of::<rtrb::RingBuffer<DispatchEvent>>())?;
            }
        }
        Ok(reserved)
    }

    /// Validate the full concrete reservation before output allocation. `inputs`
    /// must contain each producer once in ID order from the same capture session.
    /// Every registered channel needs an explicit route, including empty routes.
    pub fn new(
        plan: &ValidatedConfig,
        inputs: Vec<CaptureConsumer<'owner>>,
        config: RoutingConfig,
    ) -> Result<Self, RoutingError> {
        if inputs.len() != plan.config().producers.len() || inputs.is_empty() {
            return Err(RoutingError::InputsMismatch);
        }
        let first = inputs[0].registration();
        let session_id = first.session_id;
        let revision = first.revision;
        let channels = first.channels;
        let completion = first.completion;
        if completion.is_some_and(|completion| !completion.registry.matches(&config)) {
            return Err(RoutingError::InputsMismatch);
        }
        for (index, input) in inputs.iter().enumerate() {
            let registration = input.registration();
            let quota = &plan.config().producers[index];
            if registration.session_id != session_id
                || registration.producer_id as usize != index
                || registration.revision != revision
                || registration.channels != channels
                || !match (completion, registration.completion) {
                    (Some(a), Some(b)) => std::ptr::eq(a.registry, b.registry),
                    (None, None) => true,
                    _ => false,
                }
                || registration.memory != &plan.config().memory
                || registration.small_layout != (quota.small.slots, quota.small.slot_bytes)
                || registration.raw_layout
                    != quota.raw.as_ref().map(|raw| (raw.slots, raw.slot_bytes))
            {
                return Err(RoutingError::InputsMismatch);
            }
        }
        let schedule = config.schedule;
        let largest = plan
            .config()
            .producers
            .iter()
            .map(|p| {
                p.raw.as_ref().map_or(p.small.slot_bytes, |raw| {
                    raw.slot_bytes.max(p.small.slot_bytes)
                })
            })
            .max()
            .unwrap_or(1);
        let largest_small = plan
            .config()
            .producers
            .iter()
            .map(|p| p.small.slot_bytes)
            .max()
            .unwrap_or(1);
        if config.shared_records == 0
            || config.targets.is_empty()
            || config.small_copy_bytes < largest_small
            || schedule.record_quantum == 0
            || schedule.byte_quantum == 0
            || schedule.records_per_turn == 0
            || schedule.bytes_per_turn < largest
            || schedule.retries_per_target == 0
            || schedule.reclaim_slots_per_producer == 0
            || config.routes.len() != channels.len()
        {
            return Err(RoutingError::InvalidConfig);
        }
        for (i, target) in config.targets.iter().enumerate() {
            if target.queue_slots == 0
                || target.retained_bytes == 0
                || target.max_attempts == 0
                || target.max_age.is_zero()
                || target
                    .queue_slots
                    .checked_add(target.pending_slots)
                    .is_none()
                || config.targets[..i]
                    .iter()
                    .any(|other| other.id == target.id)
            {
                return Err(RoutingError::InvalidConfig);
            }
        }
        for (i, route) in config.routes.iter().enumerate() {
            if !channels
                .iter()
                .any(|channel| channel.channel_id == route.channel_id)
                || config.routes[..i]
                    .iter()
                    .any(|other| other.channel_id == route.channel_id)
                || route.targets.iter().enumerate().any(|(j, id)| {
                    route.targets[..j].contains(id)
                        || !config.targets.iter().any(|target| target.id == *id)
                })
            {
                return Err(RoutingError::InvalidConfig);
            }
        }
        let feedback_capacity =
            completion.map_or(0, |completion| completion.registry.limits.feedback);
        let reserved = Self::required(
            inputs.len(),
            &config,
            completion.map(|completion| completion.registry.limits.feedback),
        )?;
        if reserved > plan.config().memory.output_bytes {
            return Err(RoutingError::Config(ConfigError::BudgetExceeded {
                resource: Resource::Output,
                required: reserved,
                available: plan.config().memory.output_bytes,
            }));
        }
        let inputs = inputs
            .into_iter()
            .zip(&plan.config().producers)
            .map(|(consumer, p)| Input {
                consumer,
                deficit: 0,
                taken: 0,
                max_payload: p.raw.as_ref().map_or(p.small.slot_bytes, |raw| {
                    raw.slot_bytes.max(p.small.slot_bytes)
                }),
            })
            .collect::<Box<[_]>>();
        let producers = inputs.len();
        let targets = config
            .targets
            .into_iter()
            .map(|config| {
                let (write, read) = rtrb::RingBuffer::new(config.queue_slots);
                let pending = PendingQueue::new(config.pending_slots);
                let (outcomes_write, outcomes_read) = if completion.is_some() {
                    let (write, read) = rtrb::RingBuffer::new(feedback_capacity);
                    (Some(write), Some(read))
                } else {
                    (None, None)
                };
                Target {
                    outcomes_write,
                    outcomes_read,
                    config,
                    write,
                    read,
                    pending,
                    ledger: Arc::new(TargetLedger {
                        files: crate::storage::FileLedger::new(producers),
                        ..TargetLedger::default()
                    }),
                    written: vec![0; producers].into_boxed_slice(),
                }
            })
            .collect();
        // Rebox ID lists so caller Vec spare capacity is not an unaccounted reservation.
        let routes = config
            .routes
            .into_iter()
            .map(|mut route| {
                route.targets = route.targets.into_boxed_slice().into_vec();
                route
            })
            .collect();
        Ok(Self {
            inputs,
            targets,
            routes,
            config: Limits {
                schedule,
                shared_records: config.shared_records,
                small_copy_bytes: config.small_copy_bytes,
            },
            ledger: Arc::new(SharedLedger {
                storage_limit: plan.config().memory.output_bytes - reserved,
                ..SharedLedger::default()
            }),
            cursor: Cursor::default(),
            output_reserved_bytes: reserved,
            raw_reserved_bytes: plan.raw_pool_bytes(),
            session_id,
            revision,
            updates: first.updates,
            completion,
            feedback: Feedback::new(feedback_capacity),
        })
    }

    /// Share the output bytes remaining after concrete routing reservations.
    /// All writers created from this router compete for the same finite budget.
    pub fn writer_budget(&self, target: u32) -> Result<crate::storage::WriterBudget, RoutingError> {
        let target_state = self
            .targets
            .iter()
            .find(|t| t.config.id == target)
            .ok_or(RoutingError::InputsMismatch)?;
        Ok(crate::storage::WriterBudget {
            ledger: Arc::clone(&self.ledger),
            session: self.session_id,
            revision: self.revision,
            target,
            producers: self.inputs.len(),
            target_state: Arc::clone(&target_state.ledger),
        })
    }
    /// Create an explicit file budget for a currently retained configuration
    /// revision. Writers remain single-revision: rotate/open a separate writer
    /// rather than mixing records under incorrect MCAP metadata. This does not
    /// start a writer or provide an automatic worker/rotation backend.
    pub fn writer_budget_for_revision(
        &self,
        target: u32,
        revision: u64,
    ) -> Result<crate::storage::WriterBudget, RoutingError> {
        if !self
            .updates
            .map_or(revision == self.revision, |u| u.known_revision(revision))
        {
            return Err(RoutingError::RecordMismatch);
        }
        let mut budget = self.writer_budget(target)?;
        budget.revision = revision;
        Ok(budget)
    }
    /// Lend a unique dispatcher and independent target FIFO readers. Reader IO can
    /// run on scoped threads; dispatcher operations never invoke a reader callback.
    pub fn scope<R>(
        &mut self,
        callback: impl FnOnce(Dispatcher<'_, 'owner>, Vec<TargetConsumer<'_, 'owner>>) -> R,
    ) -> R {
        let mut writers = Vec::with_capacity(self.targets.len());
        let mut readers = Vec::with_capacity(self.targets.len());
        for target in &mut self.targets {
            writers.push(TargetWriter {
                config: &target.config,
                write: &mut target.write,
                pending: &mut target.pending,
                ledger: &target.ledger,
                outcomes: target.outcomes_read.as_mut(),
                completion: self.completion,
                written: &mut target.written,
            });
            readers.push(TargetConsumer {
                config: &target.config,
                read: &mut target.read,
                ledger: &target.ledger,
                outcomes: target.outcomes_write.as_mut(),
            });
        }
        callback(
            Dispatcher {
                inputs: &mut self.inputs,
                targets: writers,
                routes: &self.routes,
                config: self.config,
                ledger: &self.ledger,
                cursor: &mut self.cursor,
                output_reserved_bytes: self.output_reserved_bytes,
                raw_reserved_bytes: self.raw_reserved_bytes,
                session_id: self.session_id,
                revision: self.revision,
                updates: self.updates,
                completion: self.completion,
                feedback: &mut self.feedback,
            },
            readers,
        )
    }
}

/// Single borrowed general-path dispatcher; all work is driven explicitly by turn.
pub struct Dispatcher<'scope, 'owner> {
    inputs: &'scope mut [Input<'owner>],
    targets: Vec<TargetWriter<'scope, 'owner>>,
    routes: &'scope [Route],
    config: Limits,
    ledger: &'scope Arc<SharedLedger>,
    cursor: &'scope mut Cursor,
    output_reserved_bytes: usize,
    raw_reserved_bytes: usize,
    session_id: u128,
    revision: u64,
    updates: Option<&'owner crate::control::config_update::State>,
    completion: Option<Completion<'owner>>,
    feedback: &'scope mut Feedback,
}
impl Dispatcher<'_, '_> {
    /// Sample distinct physical payload usage separately from per-target references.
    pub fn snapshot(&self) -> RoutingSnapshot {
        RoutingSnapshot {
            raw_reserved_bytes: self.raw_reserved_bytes,
            output_reserved_bytes: self.output_reserved_bytes,
            live_records: self.ledger.records.load(Ordering::Acquire),
            small_copy_bytes: self.ledger.small.load(Ordering::Acquire),
            raw_retained_bytes: self.ledger.raw.load(Ordering::Acquire),
        }
    }
    /// Sample one output's queue, pending, and still-active IO retention.
    pub fn target_snapshot(&self, id: u32) -> Option<TargetSnapshot> {
        self.targets
            .iter()
            .find(|target| target.config.id == id)
            .map(|target| target.ledger.snapshot())
    }
    /// Stop consuming new ingress; retries and reclamation remain driven by turn.
    /// This is not capture closure, target cancellation, or a successful shutdown.
    pub fn close(&mut self) {
        self.cursor.closed = true;
    }
    /// Request target cancellation. Pending work is explicitly reported by turn;
    /// readers must poll their FIFO for cancellation events. Active IO guards stay valid.
    pub fn cancel_target(&mut self, id: u32) -> bool {
        let Some(target) = self.targets.iter().find(|target| target.config.id == id) else {
            return false;
        };
        target.ledger.cancelled.store(true, Ordering::Release);
        true
    }
    pub(crate) fn tracking(&self) -> Option<Completion<'_>> {
        self.completion
    }
    pub(crate) fn drain_state(&mut self) -> DrainState {
        let mut state = DrainState {
            live: self.ledger.records.load(Ordering::Acquire),
            feedback: self.feedback.len,
            ..DrainState::default()
        };
        for input in self.inputs.iter_mut() {
            input
                .consumer
                .reclaim_raw(self.config.schedule.reclaim_slots_per_producer);
            state.ingress += input.consumer.pending_len();
            if let Some(raw) = input.consumer.raw_snapshot() {
                state.raw_loaned += raw.loaned;
                state.raw_in_use += raw.in_use;
                state.raw_queued += raw.queued;
                state.raw_return_pending += raw.return_pending;
            }
        }
        for target in &mut self.targets {
            let snapshot = target.ledger.snapshot();
            state.queued += snapshot.queued;
            state.pending += snapshot.pending;
            state.in_flight += snapshot.in_flight;
            if let Some(queue) = &mut target.outcomes {
                state.feedback += queue.slots();
            }
        }
        state
    }
    pub(crate) fn ready_for_sync(
        &mut self,
        completion: Completion<'_>,
        target: u32,
        ranges: &[ProducerRange],
    ) -> Result<bool, RoutingError> {
        if !self
            .completion
            .is_some_and(|ours| std::ptr::eq(ours.registry, completion.registry))
        {
            return Err(RoutingError::InputsMismatch);
        }
        let Some(writer) = self
            .targets
            .iter_mut()
            .find(|writer| writer.config.id == target)
        else {
            return Err(RoutingError::InputsMismatch);
        };
        // An ended record proves Written only when this target's completion
        // contract needs Written first; an Enqueued contract ends a record
        // (and lets its detail expire) before any write (REV-052 RV-129).
        let written = completion
            .targets()
            .iter()
            .find(|contract| contract.id == target)
            .is_some_and(|contract| {
                contract.supported.contains(Boundary::Written)
                    && matches!(contract.completion, Boundary::Written | Boundary::Synced)
            });
        if !written {
            return self.drained_for_sync(completion, target, ranges);
        }
        // Ready once every record up to the cutoff routed here was written
        // (or ended), so continuous load cannot starve the barrier the way
        // waiting for an empty queue did (TASK-023 S2b ④, Human 2026-11-07).
        let window = completion.registry.limits.window as u64;
        let now = Instant::now();
        for range in ranges {
            if range.last == 0 {
                continue;
            }
            let p = range.producer_id as usize;
            let input = self.inputs.get(p).ok_or(RoutingError::InputsMismatch)?;
            // `accepted` (the cutoff) is published before the ingress envelope,
            // so a record up to the cutoff may not be visible yet: wait until
            // it was taken out of this producer's ingress.
            if input.taken < range.last {
                return Ok(false);
            }
            let mark = writer
                .written
                .get_mut(p)
                .ok_or(RoutingError::InputsMismatch)?;
            // A completion slot is reused only after its record ended on every
            // target, so only the newest `window` sequences can be pending.
            // `mark` claims every sequence up to it, so it only follows a
            // checked run that starts right after it: a range starting past
            // that point is checked on its own and leaves `mark` alone
            // (REV-052 RV-129).
            let resume = (*mark + 1).max(range.last.saturating_sub(window.saturating_sub(1)));
            let mut seq = resume.max(range.first);
            let contiguous = seq == resume;
            while seq <= range.last {
                let receipt = crate::capture::CaptureReceipt {
                    session_id: self.session_id,
                    producer_id: range.producer_id,
                    accepted_seq: seq,
                };
                match completion.poll_at(receipt, target, Boundary::Written, now) {
                    Ok(status)
                        if !matches!(
                            status.outcome,
                            QueryOutcome::Pending | QueryOutcome::Busy
                        ) =>
                    {
                        if contiguous {
                            *mark = seq;
                        }
                        seq += 1;
                    }
                    Ok(_) | Err(CompletionError::Busy) => return Ok(false),
                    Err(error) => return Err(RoutingError::Completion(error)),
                }
            }
        }
        Ok(true)
    }
    /// Readiness for a target without a Written boundary: everything up to
    /// the cutoff left ingress and this target holds nothing.
    fn drained_for_sync(
        &mut self,
        completion: Completion<'_>,
        target: u32,
        ranges: &[ProducerRange],
    ) -> Result<bool, RoutingError> {
        if completion.active_calls() != 0 || self.feedback.len != 0 {
            return Ok(false);
        }
        for writer in &mut self.targets {
            if writer
                .outcomes
                .as_mut()
                .is_some_and(|queue| queue.slots() != 0)
            {
                return Ok(false);
            }
        }
        let Some(writer) = self
            .targets
            .iter()
            .find(|writer| writer.config.id == target)
        else {
            return Err(RoutingError::InputsMismatch);
        };
        if writer.ledger.records.load(Ordering::Acquire) != 0 {
            return Ok(false);
        }
        for (index, input) in self.inputs.iter_mut().enumerate() {
            if let Some(range) = ranges
                .iter()
                .find(|range| range.producer_id as usize == index)
            {
                if let Some(head) = input.consumer.peek()? {
                    if head.receipt.accepted_seq <= range.last {
                        return Ok(false);
                    }
                }
            }
        }
        Ok(true)
    }
    fn flush_feedback(&mut self) -> Result<(), CompletionError> {
        let Some(completion) = self.completion else {
            return Ok(());
        };
        let limit = completion.registry.limits.updates;
        self.feedback.flush(completion, limit)?;
        for target in &mut self.targets {
            if let Some(queue) = &mut target.outcomes {
                for _ in 0..limit {
                    let Ok(chunk) = queue.read_chunk(1) else {
                        break;
                    };
                    let (a, b) = chunk.as_slices();
                    let Some(event) = a.first().or_else(|| b.first()).copied() else {
                        break;
                    };
                    match completion.apply_dispatch(event) {
                        Ok(()) | Err(CompletionError::Expired) => chunk.commit_all(),
                        Err(CompletionError::Busy) => break,
                        Err(error) => return Err(error),
                    }
                }
            }
        }
        Ok(())
    }
    fn finish(
        &mut self,
        events: &[Option<DispatchEvent>],
        mut report: TurnReport,
    ) -> Result<TurnReport, RoutingError> {
        if self.completion.is_some() {
            for event in events[..report.events].iter().flatten() {
                self.feedback.push(*event);
            }
            if let Err(error) = self.flush_feedback() {
                report.fault = Some(RoutingError::Completion(error));
            }
        }
        Ok(report)
    }
    /// Perform bounded retries, round-robin raw reclamation, and record/byte-deficit
    /// scheduling. Only events[..report.events] is initialized; storage elsewhere
    /// is untouched. If event space/shared capacity is insufficient, ingress stays
    /// queued. Enqueued is a routing-stage acknowledgment only. Use a nondecreasing
    /// monotonic `now`; progress/expiry requires the application to drive turns/readers.
    pub fn turn(
        &mut self,
        now: Instant,
        events: &mut [Option<DispatchEvent>],
    ) -> Result<TurnReport, RoutingError> {
        if self.cursor.last_now.is_some_and(|previous| now < previous) {
            return Err(RoutingError::ClockWentBackwards);
        }
        self.cursor.last_now = Some(now);
        let mut report = TurnReport::default();
        if let Err(error) = self.flush_feedback() {
            report.fault = Some(RoutingError::Completion(error));
            return Ok(report);
        }
        if let Some(completion) = self.completion {
            completion.reconcile(completion.registry.limits.reconcile);
        }
        let capacity = if self.completion.is_some() {
            events.len().min(self.feedback.available())
        } else {
            events.len()
        };
        let events = &mut events[..capacity];
        for target in &mut self.targets {
            target.retry(
                now,
                self.config.schedule.retries_per_target,
                events,
                &mut report,
            );
        }
        for input in self.inputs.iter_mut() {
            let reclaimed = input
                .consumer
                .reclaim_raw(self.config.schedule.reclaim_slots_per_producer);
            report.reclaim_inspected += reclaimed.inspected;
            report.reclaimed += reclaimed.reclaimed;
        }
        if self.cursor.closed {
            return self.finish(events, report);
        }
        let first = self.cursor.next;
        'visit: for offset in 0..self.inputs.len() {
            let index = (first + offset) % self.inputs.len();
            self.cursor.next = (index + 1) % self.inputs.len();
            let input = &mut self.inputs[index];
            input.deficit = input
                .deficit
                .saturating_add(self.config.schedule.byte_quantum)
                // Preserve a full configured quantum even when every record is
                // smaller. Bound idle credit while allowing a large FIFO head
                // to accumulate enough credit over successive visits.
                .min(
                    input
                        .max_payload
                        .saturating_add(self.config.schedule.byte_quantum),
                );
            for _ in 0..self.config.schedule.record_quantum {
                let head = match input.consumer.peek() {
                    Ok(Some(head)) => head,
                    Ok(None) => {
                        input.deficit = 0;
                        break;
                    }
                    Err(error) => {
                        report.fault = Some(error.into());
                        break 'visit;
                    }
                };
                let Some(route) = self
                    .routes
                    .iter()
                    .find(|route| route.channel_id == head.metadata.channel_id)
                else {
                    report.fault = Some(RoutingError::RecordMismatch);
                    break 'visit;
                };
                if head.receipt.session_id != self.session_id
                    || head.receipt.producer_id as usize != index
                    || !self.updates.map_or(head.revision == self.revision, |u| {
                        u.contains(head.revision)
                    })
                    || head.payload_len > input.max_payload
                {
                    report.fault = Some(RoutingError::RecordMismatch);
                    break 'visit;
                }
                if report.records == self.config.schedule.records_per_turn
                    || head.payload_len > self.config.schedule.bytes_per_turn - report.bytes
                {
                    report.backpressured = true;
                    break 'visit;
                }
                let cost = head.payload_len.max(1);
                if cost > input.deficit {
                    report.backpressured = true;
                    break;
                }
                // Reserve the worst possible event count before consuming ingress.
                let selected_count = self
                    .targets
                    .iter()
                    .filter(|t| {
                        self.updates
                            .map_or(route.targets.contains(&t.config.id), |u| {
                                u.selected(head.revision, head.metadata.channel_id, t.config.id)
                            })
                    })
                    .count();
                if events.len() - report.events < selected_count.max(1) {
                    report.backpressured = true;
                    break 'visit;
                }
                let shared = match take_shared(input, head, self.config, self.ledger) {
                    Ok(Some(shared)) => shared,
                    Ok(None) => {
                        report.backpressured = true;
                        continue 'visit;
                    }
                    Err(error) => {
                        report.fault = Some(error);
                        break 'visit;
                    }
                };
                input.deficit -= cost;
                report.records += 1;
                report.bytes += head.payload_len;
                if selected_count == 0 {
                    emit(
                        events,
                        &mut report,
                        DispatchEvent {
                            receipt: head.receipt,
                            revision: head.revision,
                            target_id: None,
                            required: false,
                            outcome: DispatchOutcome::Unrouted,
                        },
                    );
                } else {
                    // Iterate configured targets without allocating a fanout list.
                    for target in &mut self.targets {
                        if self
                            .updates
                            .map_or(route.targets.contains(&target.config.id), |u| {
                                u.selected(
                                    head.revision,
                                    head.metadata.channel_id,
                                    target.config.id,
                                )
                            })
                        {
                            target.submit(&shared, now, events, &mut report);
                        }
                    }
                }
            }
        }
        self.finish(events, report)
    }
}

/// Borrowed, general-path output FIFO reader; no file IO is performed by this type.
pub struct TargetConsumer<'scope, 'owner> {
    config: &'scope TargetConfig,
    read: &'scope mut rtrb::Consumer<Delivery<'owner>>,
    ledger: &'scope Arc<TargetLedger>,
    outcomes: Option<&'scope mut rtrb::Producer<DispatchEvent>>,
}
impl<'owner> TargetConsumer<'_, 'owner> {
    /// Take one live IO guard or one explicit queued cancellation/expiry event.
    /// A stalled reader retains a bounded queue; drive it after cancellation to drain.
    /// Once taken, a guard cannot be invalidated even when its deadline expires.
    pub fn try_next(&mut self, now: Instant) -> Option<Result<Delivery<'owner>, DispatchEvent>> {
        if self
            .outcomes
            .as_mut()
            .is_some_and(|queue| queue.slots() == 0)
        {
            return None;
        }
        let delivery = self.read.pop().ok()?;
        self.ledger.queued.fetch_sub(1, Ordering::Release);
        let reason = if self.ledger.cancelled.load(Ordering::Acquire) {
            Some(DropReason::Cancelled)
        } else if now.saturating_duration_since(delivery.started) >= self.config.max_age {
            Some(DropReason::Expired)
        } else {
            None
        };
        Some(match reason {
            Some(reason) => {
                let event = delivery.dropped_event(reason);
                if let Some(queue) = &mut self.outcomes {
                    let _ = queue.push(event);
                }
                Err(event)
            }
            None => Ok(delivery),
        })
    }
    /// Retention includes records already taken by an IO operation.
    pub fn snapshot(&self) -> TargetSnapshot {
        self.ledger.snapshot()
    }
}

fn emit(events: &mut [Option<DispatchEvent>], report: &mut TurnReport, event: DispatchEvent) {
    events[report.events] = Some(event);
    report.events += 1;
}

fn take_shared<'owner>(
    input: &mut Input<'owner>,
    head: CapturedRecord,
    limits: Limits,
    ledger: &Arc<SharedLedger>,
) -> Result<Option<Arc<SharedPayload<'owner>>>, RoutingError> {
    if ledger.records.load(Ordering::Acquire) >= limits.shared_records {
        return Ok(None);
    }
    let plan_guard = input.consumer.retain_plan(head.revision);
    let (payload, small, raw) = match input.consumer.try_take_raw() {
        Ok(Some(raw)) => (Payload::Raw(raw), 0, head.payload_len),
        Ok(None) => return Err(RoutingError::Read(ReadError::InconsistentQueue)),
        Err(ReadError::PayloadKindMismatch) => {
            if head.payload_len
                > limits
                    .small_copy_bytes
                    .saturating_sub(ledger.small.load(Ordering::Acquire))
            {
                return Ok(None);
            }
            let mut copy = vec![0; head.payload_len].into_boxed_slice();
            if input.consumer.try_read(&mut copy)? != Some(head) {
                return Err(RoutingError::Read(ReadError::InconsistentQueue));
            }
            (Payload::Small(copy), head.payload_len, 0)
        }
        Err(error) => return Err(error.into()),
    };
    ledger.records.fetch_add(1, Ordering::Relaxed);
    ledger.small.fetch_add(small, Ordering::Relaxed);
    ledger.raw.fetch_add(raw, Ordering::Relaxed);
    input.taken = head.receipt.accepted_seq;
    Ok(Some(Arc::new(SharedPayload {
        payload,
        record: head,
        _plan: plan_guard,
        _permit: SharedPermit {
            ledger: Arc::clone(ledger),
            small,
            raw,
        },
    })))
}

impl<'owner> TargetWriter<'_, 'owner> {
    fn push(&mut self, delivery: Delivery<'owner>) -> Result<(), Delivery<'owner>> {
        self.ledger.queued.fetch_add(1, Ordering::Relaxed);
        match self.write.push(delivery) {
            Ok(()) => Ok(()),
            Err(rtrb::PushError::Full(delivery)) => {
                self.ledger.queued.fetch_sub(1, Ordering::Release);
                Err(delivery)
            }
        }
    }
    fn submit(
        &mut self,
        shared: &Arc<SharedPayload<'owner>>,
        now: Instant,
        events: &mut [Option<DispatchEvent>],
        report: &mut TurnReport,
    ) {
        let event = |outcome| DispatchEvent {
            receipt: shared.record.receipt,
            revision: shared.record.revision,
            target_id: Some(self.config.id),
            required: self.config.required,
            outcome,
        };
        let bytes = shared.record.payload_len;
        let reason = if self.ledger.cancelled.load(Ordering::Acquire) {
            Some(DropReason::Cancelled)
        } else if self.ledger.records.load(Ordering::Acquire)
            >= self.config.queue_slots + self.config.pending_slots
            || bytes
                > self
                    .config
                    .retained_bytes
                    .saturating_sub(self.ledger.bytes.load(Ordering::Acquire))
        {
            Some(DropReason::Capacity)
        } else {
            None
        };
        if let Some(reason) = reason {
            emit(events, report, event(DispatchOutcome::Dropped(reason)));
            return;
        }
        self.ledger.records.fetch_add(1, Ordering::Relaxed);
        self.ledger.bytes.fetch_add(bytes, Ordering::Relaxed);
        let delivery = Delivery {
            shared: Arc::clone(shared),
            _permit: TargetPermit {
                ledger: Arc::clone(self.ledger),
                bytes,
            },
            target_id: self.config.id,
            required: self.config.required,
            started: now,
            completion: self.completion,
            abandon: self
                .completion
                .and_then(|completion| completion.marker(shared.record.receipt, self.config.id)),
        };
        let delivery = if self.pending.len == 0 {
            match self.push(delivery) {
                Ok(()) => {
                    emit(events, report, event(DispatchOutcome::Enqueued));
                    return;
                }
                Err(delivery) => delivery,
            }
        } else {
            delivery
        };
        let reason = if self.config.max_attempts == 1 {
            Some(DropReason::RetryExhausted)
        } else if self.pending.full() {
            Some(DropReason::PendingFull)
        } else {
            None
        };
        if let Some(reason) = reason {
            emit(events, report, delivery.dropped_event(reason));
        } else {
            self.pending.push_back(Pending {
                delivery,
                attempts: 1,
            });
            self.ledger.pending.fetch_add(1, Ordering::Relaxed);
        }
    }
    fn retry(
        &mut self,
        now: Instant,
        limit: usize,
        events: &mut [Option<DispatchEvent>],
        report: &mut TurnReport,
    ) {
        let visits = self.pending.len.min(limit);
        for _ in 0..visits {
            if report.events == events.len() {
                report.backpressured = true;
                break;
            }
            let Some(mut pending) = self.pending.pop() else {
                break;
            };
            self.ledger.pending.fetch_sub(1, Ordering::Release);
            report.retries += 1;
            let reason = if self.ledger.cancelled.load(Ordering::Acquire) {
                Some(DropReason::Cancelled)
            } else if now.saturating_duration_since(pending.delivery.started) >= self.config.max_age
            {
                Some(DropReason::Expired)
            } else {
                None
            };
            if let Some(reason) = reason {
                emit(events, report, pending.delivery.dropped_event(reason));
                continue;
            }
            pending.attempts += 1;
            let success = pending.delivery.event(DispatchOutcome::Enqueued);
            match self.push(pending.delivery) {
                Ok(()) => emit(events, report, success),
                Err(delivery) if pending.attempts >= self.config.max_attempts => emit(
                    events,
                    report,
                    delivery.dropped_event(DropReason::RetryExhausted),
                ),
                Err(delivery) => {
                    self.pending.push_front(Pending {
                        delivery,
                        attempts: pending.attempts,
                    });
                    self.ledger.pending.fetch_add(1, Ordering::Relaxed);
                    break; // Do not overtake a blocked FIFO head or spin on it.
                }
            }
        }
    }
}

#[cfg(test)]
mod release_order {
    use super::*;
    use crate::capture::{CaptureReceipt, Level, RecordMetadata};

    /// TASK-023 S2b finding ②: a delivery must give back its target permit
    /// before its shared record. The other order lets the dispatcher take a
    /// fresh shared record while this target's ledger still counts the old
    /// one, see the target as full and drop a required record as Capacity.
    #[test]
    fn delivery_releases_its_target_permit_before_the_shared_record() {
        let shared_ledger = Arc::new(SharedLedger::default());
        shared_ledger.records.store(1, Ordering::Relaxed);
        shared_ledger.small.store(1, Ordering::Relaxed);
        let target_ledger = Arc::new(TargetLedger::default());
        target_ledger.records.store(1, Ordering::Relaxed);
        target_ledger.bytes.store(1, Ordering::Relaxed);
        let record = CapturedRecord {
            receipt: CaptureReceipt {
                session_id: 1,
                producer_id: 0,
                accepted_seq: 1,
            },
            metadata: RecordMetadata {
                channel_id: 1,
                schema_id: 0,
                schema_version: 0,
                mono_ns: 0,
                real_ns: 0,
                source_time: None,
                level: Level::Info,
                importance: None,
            },
            revision: 1,
            payload_len: 1,
        };
        let shared = Arc::new(SharedPayload {
            payload: Payload::Small(Box::new([7])),
            record,
            _permit: SharedPermit {
                ledger: Arc::clone(&shared_ledger),
                small: 1,
                raw: 0,
            },
            _plan: None,
        });
        let delivery = Delivery {
            shared,
            _permit: TargetPermit {
                ledger: Arc::clone(&target_ledger),
                bytes: 1,
            },
            target_id: 1,
            required: true,
            started: Instant::now(),
            completion: None,
            abandon: None,
        };
        RELEASES.with(|r| r.borrow_mut().clear());
        drop(delivery);
        assert_eq!(RELEASES.with(|r| r.borrow().clone()), ["target", "shared"]);
        assert_eq!(target_ledger.records.load(Ordering::Relaxed), 0);
        assert_eq!(shared_ledger.records.load(Ordering::Relaxed), 0);
    }
}
