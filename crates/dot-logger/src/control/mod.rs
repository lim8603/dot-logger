//! Explicit bounded synchronization and normal shutdown coordination.
//!
//! No filesystem operations or worker threads are started here. A writer takes a
//! SyncWork guard only after the concrete dispatcher has drained preceding work,
//! performs its adapter-specific barrier, then acknowledges the actual result.
//! Guard lifetimes keep control state alive across IO. Deadline expiry never
//! invalidates an in-flight guard or substitutes a successful File Drop.
//!
//! A `SyncWork` from an explicit `ControlOwner` request is held until the real
//! synchronization ends. Past the shutdown deadline, in-flight IO and loans stay
//! valid and the same owner continues cleanup. A shutdown sync that expired
//! before it started settles unconfirmed boundaries as Unknown only after the
//! actual access and preceding syncs end. A `Closed` report still needs
//! `is_success()` and its failure and timeout fields checked.
use crate::capture::CaptureControl;
use crate::completion::{
    Boundary, Completion, CompletionError, CompletionScope, FailureReason, ProducerRange,
    QueryOutcome, Update,
};
use crate::config::{ConfigError, Resource, ValidatedConfig};
use crate::routing::Dispatcher;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;
/// Explicit two-generation filter, sampling and existing-target route updates.
pub mod config_update;
/// Optional startup-only power alarm plans and ranked emergency synchronization.
pub mod emergency;
mod types;
pub use types::*;
const FREE: u8 = 0;
const LIVE: u8 = 1;
const EDITING: u8 = 2;
const RETIRED: u8 = 3;
const NONE: u8 = 0;
const QUEUED: u8 = 1;
const IN_FLIGHT: u8 = 2;
const ACK_OK: u8 = 3;
const ACK_FAILED: u8 = 4;
const ACK_UNKNOWN: u8 = 5;
const DONE_OK: u8 = 6;
const DONE_FAILED: u8 = 7;
const DONE_UNKNOWN: u8 = 8;
const EXPIRED: u8 = 9;
// Shutdown-only metadata cleanup after all preceding IO has actually ended.
// This is not an adapter acknowledgment and never creates a SyncWork guard.
const APPLY_EXPIRED: u8 = 10;
struct AtomicRange {
    first: AtomicU64,
    last: AtomicU64,
}
struct SyncTarget {
    state: AtomicU8,
    cursor: AtomicUsize,
}
struct SyncSlot {
    phase: AtomicU8,
    generation: AtomicU64,
    deadline: AtomicU64,
    finished_at: AtomicU64,
    ranges: Box<[AtomicRange]>,
    targets: Box<[SyncTarget]>,
}
struct SlotPublication<'a>(&'a AtomicU8);
impl Drop for SlotPublication<'_> {
    fn drop(&mut self) {
        self.0.store(LIVE, Ordering::Release);
    }
}
impl SyncSlot {
    fn claim(&self, generation: u64) -> Result<SlotPublication<'_>, ControlError> {
        self.phase
            .compare_exchange(LIVE, EDITING, Ordering::AcqRel, Ordering::Relaxed)
            .map_err(|state| {
                if state == EDITING {
                    ControlError::Busy
                } else {
                    ControlError::InvalidToken
                }
            })?;
        let guard = SlotPublication(&self.phase);
        if self.generation.load(Ordering::Acquire) != generation {
            return Err(ControlError::InvalidToken);
        }
        Ok(guard)
    }
    fn copy_ranges(&self, buffer: &mut [ProducerRange]) -> Result<usize, ControlError> {
        if buffer.len() < self.ranges.len() {
            return Err(ControlError::BufferTooSmall);
        }
        for (i, range) in self.ranges.iter().enumerate() {
            let first = range.first.load(Ordering::Acquire);
            buffer[i] = ProducerRange {
                producer_id: i as u32,
                first: first.max(1),
                last: range.last.load(Ordering::Acquire),
            };
        }
        Ok(self.ranges.len())
    }
    fn includes(&self, range: ProducerRange) -> bool {
        let Some(ours) = self.ranges.get(range.producer_id as usize) else {
            return false;
        };
        let first = ours.first.load(Ordering::Acquire);
        first != 0 && range.first >= first && range.first <= ours.last.load(Ordering::Acquire)
    }
}
struct ControllerLease<'a>(&'a crate::completion::Registry);
impl Drop for ControllerLease<'_> {
    fn drop(&mut self) {
        self.0.control_attached.store(false, Ordering::Release);
    }
}
struct SyncSubmission<'a>(&'a AtomicBool);
impl<'a> SyncSubmission<'a> {
    fn enter(active: &'a AtomicBool) -> Result<Self, ControlError> {
        active
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
            .map_err(|_| ControlError::Busy)?;
        Ok(Self(active))
    }
}
impl Drop for SyncSubmission<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
/// Owns preallocated command/range/application state on the general path. Borrowed
/// SyncWork guards prevent this owner from being dropped while an operation lives.
/// An incomplete shutdown leaves this same owner, dispatcher, and capture owner
/// available for subsequent polls. Drop performs cleanup, not synchronization.
pub struct ControlOwner<'owner> {
    epoch: u64,
    capture: CaptureControl<'owner>,
    completion: Completion<'owner>,
    slots: Box<[SyncSlot]>,
    target_busy: Box<[AtomicBool]>,
    config: ControlConfig,
    origin: Instant,
    clock: AtomicU64,
    reserved: usize,
    shutdown: AtomicU8,
    sync_submitting: AtomicBool,
    shutdown_deadline: AtomicU64,
    shutdown_finished_at: AtomicU64,
    sync_files: AtomicBool,
    emergency: Option<Box<emergency::EmergencyState>>,
    // Last field: release attachment only after all control allocations are freed.
    _attachment: ControllerLease<'owner>,
}
impl<'owner> ControlOwner<'owner> {
    /// Control bytes this owner reserves beyond the capture reservation.
    /// Shared with the facade sizing so both use one formula.
    pub(crate) fn own_bytes(sync_slots: usize, producers: usize, targets: usize) -> Option<usize> {
        let count = sync_slots.checked_add(1)?;
        let mut reserved = size_of::<Self>();
        for (n, size) in [
            (count, size_of::<SyncSlot>()),
            (count.checked_mul(producers)?, size_of::<AtomicRange>()),
            (count.checked_mul(targets)?, size_of::<SyncTarget>()),
        ] {
            let bytes = n.checked_mul(size).filter(|n| *n <= isize::MAX as usize)?;
            reserved = reserved.checked_add(bytes)?;
        }
        reserved.checked_add(targets.checked_mul(size_of::<AtomicBool>())?)
    }

    /// Reserve ordinary request slots plus a dedicated shutdown slot, within the
    /// control bytes remaining after concrete capture/completion reservations.
    pub fn new(
        plan: &ValidatedConfig,
        capture: CaptureControl<'owner>,
        config: ControlConfig,
        origin: Instant,
    ) -> Result<Self, ControlError> {
        let completion = capture.completion.ok_or(ControlError::InvalidConfig)?;
        if capture.memory != &plan.config().memory
            || config.sync_slots == 0
            || config.work_per_poll == 0
        {
            return Err(ControlError::InvalidConfig);
        }
        let count = config
            .sync_slots
            .checked_add(1)
            .ok_or(ControlError::Config(ConfigError::ArithmeticOverflow))?;
        let reserved = Self::own_bytes(
            config.sync_slots,
            completion.registry.producers.len(),
            completion.targets().len(),
        )
        .ok_or(ControlError::Config(ConfigError::ArithmeticOverflow))?;
        let required = capture
            .reserved
            .checked_add(reserved)
            .ok_or(ControlError::Config(ConfigError::ArithmeticOverflow))?;
        if required > plan.config().memory.control_bytes {
            return Err(ControlError::Config(ConfigError::BudgetExceeded {
                resource: Resource::Control,
                required,
                available: plan.config().memory.control_bytes,
            }));
        }
        completion
            .registry
            .control_attached
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .map_err(|_| ControlError::Busy)?;
        let attachment = ControllerLease(completion.registry);
        let Some(epoch) = completion
            .registry
            .control_epoch
            .load(Ordering::Acquire)
            .checked_add(1)
        else {
            return Err(ControlError::Capacity);
        };
        completion
            .registry
            .control_epoch
            .store(epoch, Ordering::Release);
        let slots = (0..count)
            .map(|_| SyncSlot {
                phase: AtomicU8::new(FREE),
                generation: AtomicU64::new(0),
                deadline: AtomicU64::new(0),
                finished_at: AtomicU64::new(u64::MAX),
                ranges: (0..completion.registry.producers.len())
                    .map(|_| AtomicRange {
                        first: AtomicU64::new(0),
                        last: AtomicU64::new(0),
                    })
                    .collect(),
                targets: (0..completion.targets().len())
                    .map(|_| SyncTarget {
                        state: AtomicU8::new(NONE),
                        cursor: AtomicUsize::new(0),
                    })
                    .collect(),
            })
            .collect();
        Ok(Self {
            epoch,
            capture,
            completion,
            slots,
            target_busy: (0..completion.targets().len())
                .map(|_| AtomicBool::new(false))
                .collect(),
            config,
            origin,
            clock: AtomicU64::new(0),
            reserved,
            shutdown: AtomicU8::new(0),
            sync_submitting: AtomicBool::new(false),
            shutdown_deadline: AtomicU64::new(u64::MAX),
            shutdown_finished_at: AtomicU64::new(0),
            sync_files: AtomicBool::new(false),
            emergency: None,
            _attachment: attachment,
        })
    }
    /// Concrete command metadata bytes; allocator bookkeeping belongs to headroom.
    pub fn reserved_bytes(&self) -> usize {
        self.reserved
    }
    fn tick(&self, time: Instant) -> Result<u64, ControlError> {
        let duration = time
            .checked_duration_since(self.origin)
            .ok_or(ControlError::ClockWentBackwards)?;
        u64::try_from(duration.as_nanos()).map_err(|_| ControlError::InvalidDeadline)
    }
    fn now(&self, time: Instant) -> Result<u64, ControlError> {
        let tick = self.tick(time)?;
        let previous = self.clock.fetch_max(tick, Ordering::AcqRel);
        if tick < previous {
            return Err(ControlError::ClockWentBackwards);
        }
        Ok(tick)
    }
    fn slot(&self, token: SyncToken) -> Result<&SyncSlot, ControlError> {
        if token.session_id != self.completion.registry.session_id
            || token.owner_epoch != self.epoch
        {
            return Err(ControlError::InvalidToken);
        }
        if token.session_id != self.completion.registry.session_id
            || token.owner_epoch != self.epoch
        {
            return Err(ControlError::InvalidToken);
        }
        let slot = self
            .slots
            .get(token.slot)
            .ok_or(ControlError::InvalidToken)?;
        if slot.generation.load(Ordering::Acquire) != token.generation
            || slot.phase.load(Ordering::Acquire) == FREE
            || slot.phase.load(Ordering::Acquire) == RETIRED
        {
            return Err(ControlError::InvalidToken);
        }
        Ok(slot)
    }
    fn initialize(
        &self,
        index: usize,
        scope: CompletionScope<'_>,
        targets: &[u32],
        deadline: u64,
    ) -> Result<SyncToken, ControlError> {
        let slot = &self.slots[index];
        slot.phase
            .compare_exchange(FREE, EDITING, Ordering::AcqRel, Ordering::Relaxed)
            .map_err(|_| ControlError::Capacity)?;
        let Some(generation) = slot.generation.load(Ordering::Relaxed).checked_add(1) else {
            slot.phase.store(RETIRED, Ordering::Release);
            return Err(ControlError::Capacity);
        };
        for range in &slot.ranges {
            range.first.store(0, Ordering::Relaxed);
            range.last.store(0, Ordering::Relaxed);
        }
        for range in scope.ranges() {
            let ours = &slot.ranges[range.producer_id as usize];
            ours.first.store(range.first, Ordering::Relaxed);
            ours.last.store(range.last, Ordering::Relaxed);
        }
        for (i, target) in slot.targets.iter().enumerate() {
            target.cursor.store(0, Ordering::Relaxed);
            target.state.store(
                if targets.contains(&self.completion.targets()[i].id) {
                    QUEUED
                } else {
                    NONE
                },
                Ordering::Relaxed,
            );
        }
        slot.finished_at.store(u64::MAX, Ordering::Relaxed);
        slot.deadline.store(deadline, Ordering::Relaxed);
        slot.generation.store(generation, Ordering::Release);
        slot.phase.store(LIVE, Ordering::Release);
        Ok(SyncToken {
            session_id: self.completion.registry.session_id,
            owner_epoch: self.epoch,
            slot: index,
            generation,
        })
    }
    /// Explicitly request synchronization for frozen cutoffs. This only queues work;
    /// completion polling never calls it. All capabilities are checked before queuing.
    /// Concurrent submissions return Busy without waiting. Shutdown includes any
    /// admitted submission before it can finish freezing and drain its requests.
    pub fn request_sync(
        &self,
        scope: CompletionScope<'_>,
        targets: &[u32],
        now: Instant,
        deadline: Instant,
    ) -> Result<SyncToken, ControlError> {
        let tick = self.now(now)?;
        let end = self.tick(deadline)?;
        let _submission = SyncSubmission::enter(&self.sync_submitting).map_err(|error| {
            if self.shutdown.load(Ordering::SeqCst) != 0 || self.emergency_active() {
                ControlError::Closing
            } else {
                error
            }
        })?;
        // SC admission < open check < shutdown close < quiescence observation.
        // If this call saw Running, shutdown must see its live submission until
        // the slot is published (or every error path returns via the RAII guard).
        // A call entering after shutdown observes Closing before touching a slot.
        if self.shutdown.load(Ordering::SeqCst) != 0 || self.emergency_active() {
            return Err(ControlError::Closing);
        }
        #[cfg(test)]
        tests::pause_sync_submission();
        if tick >= end {
            return Err(ControlError::DeadlineExceeded);
        }
        if scope.session_id != self.completion.registry.session_id || targets.is_empty() {
            return Err(ControlError::InvalidConfig);
        }
        self.completion.scope(scope.ranges())?;
        for (i, id) in targets.iter().enumerate() {
            if targets[..i].contains(id)
                || !self
                    .completion
                    .targets()
                    .iter()
                    .any(|target| target.id == *id && target.supported.contains(Boundary::Synced))
            {
                return Err(ControlError::UnsupportedBoundary);
            }
        }
        for index in 0..self.config.sync_slots {
            if let Ok(token) = self.initialize(index, scope, targets, end) {
                return Ok(token);
            }
        }
        Err(ControlError::Capacity)
    }
    /// Take one adapter operation after its preceding ingress/target IO and feedback
    /// drained. Later records may conservatively delay this readiness check. Copy
    /// cutoffs into writer-owned storage before starting asynchronous IO.
    pub fn next_sync<'control>(
        &'control self,
        token: SyncToken,
        target: u32,
        now: Instant,
        dispatcher: &mut Dispatcher<'_, '_>,
        ranges: &mut [ProducerRange],
    ) -> Result<Option<SyncWork<'control, 'owner>>, ControlError> {
        let tick = self.now(now)?;
        self.tighten_emergency_deadline();
        let slot = self.slot(token)?;
        let _publication = slot.claim(token.generation)?;
        let index = self
            .completion
            .targets()
            .iter()
            .position(|contract| contract.id == target)
            .ok_or(ControlError::UnsupportedBoundary)?;
        if token.slot == self.config.sync_slots && !self.emergency_sync_allowed(index) {
            return Ok(None);
        }
        if token.slot != self.config.sync_slots && self.emergency_active() {
            if slot.targets[index].state.load(Ordering::Acquire) == QUEUED {
                slot.targets[index].state.store(EXPIRED, Ordering::Release);
            }
            return Ok(None);
        }
        if slot.targets[index].state.load(Ordering::Acquire) != QUEUED {
            return Ok(None);
        }
        if tick >= slot.deadline.load(Ordering::Acquire) {
            slot.targets[index].state.store(EXPIRED, Ordering::Release);
            return Ok(None);
        }
        let count = slot.copy_ranges(ranges)?;
        if !dispatcher.ready_for_sync(self.completion, target, &ranges[..count])? {
            return Ok(None);
        }
        if self.target_busy[index]
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
        {
            return Ok(None);
        }
        slot.targets[index]
            .state
            .store(IN_FLIGHT, Ordering::Release);
        Ok(Some(SyncWork {
            owner: self,
            token,
            index,
            target,
            armed: true,
        }))
    }
    /// Inspect command progress without issuing IO or dropping active operations.
    pub fn sync_snapshot(
        &self,
        token: SyncToken,
        now: Instant,
    ) -> Result<SyncSnapshot, ControlError> {
        let tick = self.now(now)?;
        let slot = self.slot(token)?;
        let _publication = slot.claim(token.generation)?;
        let finished_at = slot.finished_at.load(Ordering::Acquire);
        let observed = if finished_at == u64::MAX {
            tick
        } else {
            finished_at
        };
        let mut out = SyncSnapshot {
            deadline_elapsed: observed >= slot.deadline.load(Ordering::Acquire),
            ..SyncSnapshot::default()
        };
        for target in &slot.targets {
            match target.state.load(Ordering::Acquire) {
                QUEUED => out.queued += 1,
                IN_FLIGHT => out.in_flight += 1,
                ACK_OK | ACK_FAILED | ACK_UNKNOWN | APPLY_EXPIRED => out.applying += 1,
                DONE_OK => out.succeeded += 1,
                DONE_FAILED => out.failed += 1,
                DONE_UNKNOWN => out.unknown += 1,
                EXPIRED => out.expired += 1,
                _ => {}
            }
        }
        out.finished = out.queued == 0 && out.in_flight == 0 && out.applying == 0;
        Ok(out)
    }
    /// Retire one finished ordinary command. An active IO guard or queued/applying
    /// work prevents reuse. A stale generation can never acknowledge a new request.
    pub fn release_sync(&self, token: SyncToken) -> Result<(), ControlError> {
        if token.slot >= self.config.sync_slots {
            return Err(ControlError::InvalidToken);
        }
        let slot = self.slot(token)?;
        let publication = slot.claim(token.generation)?;
        if slot.targets.iter().any(|target| {
            matches!(
                target.state.load(Ordering::Acquire),
                QUEUED | IN_FLIGHT | ACK_OK | ACK_FAILED | ACK_UNKNOWN | APPLY_EXPIRED
            )
        }) {
            return Err(ControlError::InFlight);
        }
        std::mem::forget(publication);
        slot.phase.store(FREE, Ordering::Release);
        Ok(())
    }
    /// Apply at most work_per_poll physical completion cells for acknowledged
    /// barriers. Expired details remain Expired; newly accepted records beyond the
    /// frozen cutoff are never confirmed by an older barrier acknowledgment.
    pub fn poll(&self, now: Instant) -> Result<ControlProgress, ControlError> {
        let tick = self.now(now)?;
        self.tighten_emergency_deadline();
        let mut out = ControlProgress::default();
        let total =
            self.completion.registry.producers.len() * self.completion.registry.limits.window;
        for slot in &self.slots {
            let generation = slot.generation.load(Ordering::Acquire);
            let Ok(_publication) = slot.claim(generation) else {
                continue;
            };
            for (index, target) in slot.targets.iter().enumerate() {
                let state = target.state.load(Ordering::Acquire);
                let shutdown_slot = std::ptr::eq(slot, &self.slots[self.config.sync_slots]);
                // An alarm leaves an unselected target out of the shutdown
                // barrier, as when it arrives before the freeze (REV-071
                // RV-176); ordinary barriers it cuts short expire.
                if state == QUEUED
                    && shutdown_slot
                    && self.emergency_active()
                    && !self.emergency_target_selected(index)
                {
                    target.state.store(NONE, Ordering::Release);
                    continue;
                }
                let emergency_skip = self.emergency_active() && !shutdown_slot;
                if state == QUEUED
                    && (emergency_skip || tick >= slot.deadline.load(Ordering::Acquire))
                {
                    target.state.store(EXPIRED, Ordering::Release);
                    continue;
                }
                if !matches!(state, ACK_OK | ACK_FAILED | ACK_UNKNOWN | APPLY_EXPIRED) {
                    continue;
                }
                while target.cursor.load(Ordering::Relaxed) < total
                    && out.examined < self.config.work_per_poll
                {
                    let cursor = target.cursor.load(Ordering::Relaxed);
                    out.examined += 1;
                    let receipt = self
                        .completion
                        .awaits_sync(cursor, index)
                        .then(|| self.completion.receipt_at(cursor))
                        .flatten();
                    if let Some(receipt) = receipt {
                        let range = ProducerRange {
                            producer_id: receipt.producer_id,
                            first: receipt.accepted_seq,
                            last: receipt.accepted_seq,
                        };
                        if slot.includes(range) {
                            let status = match self.completion.poll(
                                receipt,
                                self.completion.targets()[index].id,
                                Boundary::Synced,
                            ) {
                                Ok(status) => status,
                                Err(CompletionError::Busy) => break,
                                Err(error) => return Err(error.into()),
                            };
                            if status.confirmed.contains(Boundary::Written)
                                && matches!(status.outcome, QueryOutcome::Pending)
                            {
                                let update = match state {
                                    ACK_OK => Update::Confirmed(Boundary::Synced),
                                    ACK_FAILED => Update::Failed(FailureReason::SyncFailure),
                                    _ => Update::Unknown,
                                };
                                match self.completion.report(
                                    receipt,
                                    self.completion.targets()[index].id,
                                    update,
                                ) {
                                    Ok(()) => out.applied += 1,
                                    Err(CompletionError::Expired) => {}
                                    Err(CompletionError::Busy) => break,
                                    Err(error) => return Err(error.into()),
                                }
                            }
                        }
                    }
                    target.cursor.store(cursor + 1, Ordering::Release);
                }
                if target.cursor.load(Ordering::Acquire) == total {
                    self.mark_emergency_confirmation(slot, index, tick);
                    target.state.store(
                        match state {
                            ACK_OK => DONE_OK,
                            ACK_FAILED => DONE_FAILED,
                            APPLY_EXPIRED => EXPIRED,
                            _ => DONE_UNKNOWN,
                        },
                        Ordering::Release,
                    );
                }
            }
            if slot.targets.iter().all(|target| {
                matches!(
                    target.state.load(Ordering::Acquire),
                    NONE | DONE_OK | DONE_FAILED | DONE_UNKNOWN | EXPIRED
                )
            }) {
                let _ = slot.finished_at.compare_exchange(
                    u64::MAX,
                    tick,
                    Ordering::AcqRel,
                    Ordering::Relaxed,
                );
            }
        }
        Ok(out)
    }
    fn settle_expired_shutdown(&self) -> Result<usize, ControlError> {
        let slot = &self.slots[self.config.sync_slots];
        let _publication = slot.claim(1)?;
        let total =
            self.completion.registry.producers.len() * self.completion.registry.limits.window;
        let mut applying = 0;
        for target in &slot.targets {
            if target.state.load(Ordering::Acquire) == EXPIRED
                && target.cursor.load(Ordering::Acquire) < total
            {
                target.state.store(APPLY_EXPIRED, Ordering::Release);
                applying += 1;
            }
        }
        Ok(applying)
    }
    /// Close admission and begin the declared drain/sync plan. A pre-close producer
    /// call and ordinary sync submission must finish publication before cutoffs
    /// can be frozen and final cleanup can complete.
    pub fn begin_shutdown(
        &self,
        plan: ShutdownPlan,
        now: Instant,
        deadline: Instant,
        dispatcher: &mut Dispatcher<'_, '_>,
    ) -> Result<ShutdownPoll, ControlError> {
        self.now(now)?;
        let end = self.tick(deadline)?;
        if !plan.sync_files
            && self
                .completion
                .targets()
                .iter()
                .any(|target| target.completion == Boundary::Synced)
        {
            return Err(ControlError::UnsupportedBoundary);
        }
        if !dispatcher
            .tracking()
            .is_some_and(|ours| std::ptr::eq(ours.registry, self.completion.registry))
        {
            return Err(ControlError::InvalidConfig);
        }
        self.shutdown
            .compare_exchange(0, 4, Ordering::SeqCst, Ordering::Relaxed)
            .map_err(|_| ControlError::Closing)?;
        self.shutdown_deadline.fetch_min(end, Ordering::AcqRel);
        self.tighten_emergency_deadline();
        self.sync_files.store(plan.sync_files, Ordering::Relaxed);
        self.capture.close();
        self.shutdown.store(1, Ordering::Release);
        self.poll_shutdown(now, dispatcher)
    }
    /// Identify the dedicated shutdown request after producer cutoffs are frozen.
    /// Ordinary request saturation never consumes this reserved slot.
    pub fn shutdown_sync(&self) -> Option<SyncToken> {
        let slot = &self.slots[self.config.sync_slots];
        (slot.phase.load(Ordering::Acquire) == LIVE).then(|| SyncToken {
            session_id: self.completion.registry.session_id,
            owner_epoch: self.epoch,
            slot: self.config.sync_slots,
            generation: slot.generation.load(Ordering::Acquire),
        })
    }
    /// Copy the exact producer cutoffs frozen after admission closed and all
    /// pre-close producer/sync publications finished. Busy means freezing has
    /// not completed.
    pub fn shutdown_ranges(&self, buffer: &mut [ProducerRange]) -> Result<usize, ControlError> {
        let token = self.shutdown_sync().ok_or(ControlError::Busy)?;
        let slot = self.slot(token)?;
        let _publication = slot.claim(token.generation)?;
        slot.copy_ranges(buffer)
    }
    /// Host-turn variant of [`Self::poll_shutdown`]: it drives shutdown the
    /// same way but counts the completion ledger (producers × window ×
    /// targets) only when the result can matter — closure is possible (no
    /// drain or producer call left) or, with `incomplete_due`, the deadline
    /// has passed and the caller still owes an Incomplete report. Otherwise
    /// it returns `None` (still Closing). Closure is decided only on a counted
    /// poll, so the outcome matches `poll_shutdown` (T5 ⑫, EV-066).
    pub(crate) fn poll_shutdown_when_due(
        &self,
        now: Instant,
        dispatcher: &mut Dispatcher<'_, '_>,
        incomplete_due: bool,
    ) -> Result<Option<ShutdownPoll>, ControlError> {
        self.drive_shutdown(now, dispatcher, Some(incomplete_due))
    }
    /// Poll safe cleanup on the same owner after any timeout. This does not perform
    /// writer IO or forcibly cancel IO guards. Drive dispatcher/readers/writers too.
    /// Keep polling after Closing or Incomplete until Closed; inspect its report
    /// for failure, since Closed alone does not mean synchronization succeeded.
    /// Every call counts the completion ledger (producers × window × targets)
    /// for its report; a host that drains between polls should poll at a
    /// bounded rate rather than once per dispatch turn.
    pub fn poll_shutdown(
        &self,
        now: Instant,
        dispatcher: &mut Dispatcher<'_, '_>,
    ) -> Result<ShutdownPoll, ControlError> {
        self.drive_shutdown(now, dispatcher, None)?
            .ok_or(ControlError::Busy)
    }
    /// `when_due: None` counts on every poll (public contract); `Some(due)`
    /// counts only as described on [`Self::poll_shutdown_when_due`].
    fn drive_shutdown(
        &self,
        now: Instant,
        dispatcher: &mut Dispatcher<'_, '_>,
        when_due: Option<bool>,
    ) -> Result<Option<ShutdownPoll>, ControlError> {
        let tick = self.now(now)?;
        self.tighten_emergency_deadline();
        if !dispatcher
            .tracking()
            .is_some_and(|ours| std::ptr::eq(ours.registry, self.completion.registry))
        {
            return Err(ControlError::InvalidConfig);
        }
        let phase = self.shutdown.load(Ordering::Acquire);
        if phase == 0 {
            return Err(ControlError::InvalidConfig);
        }
        if phase == 4 {
            return Err(ControlError::Busy);
        }
        if phase == 1
            && !self.sync_submitting.load(Ordering::SeqCst)
            && self.completion.active_calls() == 0
        {
            self.shutdown
                .compare_exchange(1, 4, Ordering::AcqRel, Ordering::Relaxed)
                .map_err(|_| ControlError::Busy)?;
            let slot = &self.slots[self.config.sync_slots];
            slot.phase.store(EDITING, Ordering::Release);
            for (i, range) in slot.ranges.iter().enumerate() {
                range.first.store(1, Ordering::Relaxed);
                range.last.store(
                    self.completion.registry.producers[i]
                        .accepted
                        .load(Ordering::Acquire),
                    Ordering::Relaxed,
                );
            }
            for (i, target) in slot.targets.iter().enumerate() {
                let sync = self.sync_files.load(Ordering::Relaxed)
                    && self.emergency_target_selected(i)
                    && self.completion.targets()[i]
                        .supported
                        .contains(Boundary::Synced);
                target
                    .state
                    .store(if sync { QUEUED } else { NONE }, Ordering::Relaxed);
                target.cursor.store(0, Ordering::Relaxed);
            }
            slot.generation.store(1, Ordering::Release);
            slot.deadline.store(
                self.shutdown_deadline.load(Ordering::Acquire),
                Ordering::Relaxed,
            );
            slot.finished_at.store(u64::MAX, Ordering::Relaxed);
            slot.phase.store(LIVE, Ordering::Release);
            self.shutdown.store(2, Ordering::Release);
        }
        self.poll(now)?;
        self.completion
            .reconcile(self.completion.registry.limits.reconcile);
        let drain = dispatcher.drain_state();
        // A cheap pre-read; closure below rereads producer calls and also
        // requires this poll to have counted.
        let counted = match when_due {
            None => true,
            Some(incomplete_due) => {
                (drain.is_empty() && self.completion.active_calls() == 0)
                    || self.shutdown.load(Ordering::Acquire) == 3
                    || (incomplete_due && tick >= self.shutdown_deadline.load(Ordering::Acquire))
            }
        };
        let (totals, required) = if counted {
            (self.completion.totals(false), self.completion.totals(true))
        } else {
            Default::default()
        };
        let mut sync_in_flight = 0;
        let mut sync_pending = 0;
        let mut sync_failed = false;
        for slot in &self.slots {
            if slot.phase.load(Ordering::Acquire) == FREE
                || slot.phase.load(Ordering::Acquire) == RETIRED
            {
                continue;
            }
            if slot.phase.load(Ordering::Acquire) == EDITING {
                sync_pending += 1;
                continue;
            }
            for target in &slot.targets {
                match target.state.load(Ordering::Acquire) {
                    IN_FLIGHT => sync_in_flight += 1,
                    QUEUED | ACK_OK | ACK_FAILED | ACK_UNKNOWN | APPLY_EXPIRED => sync_pending += 1,
                    DONE_FAILED | DONE_UNKNOWN | EXPIRED => sync_failed = true,
                    _ => {}
                }
            }
            let finished = slot.finished_at.load(Ordering::Acquire);
            if finished != u64::MAX
                && finished >= slot.deadline.load(Ordering::Acquire)
                && slot
                    .targets
                    .iter()
                    .any(|target| target.state.load(Ordering::Acquire) != NONE)
            {
                sync_failed = true;
            }
        }
        let producer_calls = self.completion.active_calls();
        if self.shutdown.load(Ordering::Acquire) >= 2
            && producer_calls == 0
            && drain.is_empty()
            && sync_in_flight == 0
            && sync_pending == 0
        {
            // A shutdown barrier that never started cannot later acknowledge its
            // records. Settle its unconfirmed sync facts as Unknown, preserving
            // Written, only after writer guards, feedback and ordinary barriers
            // have finished. Ordinary request expiry never changes record state.
            // The cursor makes this cleanup bounded and schedules it exactly once.
            sync_pending += self.settle_expired_shutdown()?;
        }
        if counted
            && self.shutdown.load(Ordering::Acquire) >= 2
            && producer_calls == 0
            && drain.is_empty()
            && sync_in_flight == 0
            && sync_pending == 0
            && totals.pending == 0
            && totals.busy == 0
            && !totals.overflowed
            && self.shutdown.load(Ordering::Acquire) != 3
        {
            self.shutdown_finished_at.store(tick, Ordering::Relaxed);
            self.shutdown.store(3, Ordering::Release);
        }
        let phase = if self.shutdown.load(Ordering::Acquire) == 3 {
            ShutdownPhase::Closed
        } else {
            ShutdownPhase::Closing
        };
        if !counted {
            return Ok(None);
        }
        let observed = if phase == ShutdownPhase::Closed {
            self.shutdown_finished_at.load(Ordering::Acquire)
        } else {
            tick
        };
        let report = ShutdownReport {
            phase,
            deadline_elapsed: observed >= self.shutdown_deadline.load(Ordering::Acquire),
            totals,
            required,
            ingress: drain.ingress,
            queued: drain.queued,
            pending: drain.pending,
            in_flight: drain.in_flight,
            feedback: drain.feedback,
            raw_loaned: drain.raw_loaned,
            raw_in_use: drain.raw_in_use,
            raw_queued: drain.raw_queued,
            raw_return_pending: drain.raw_return_pending,
            producer_calls,
            sync_in_flight,
            sync_failed,
        };
        Ok(Some(if phase == ShutdownPhase::Closed {
            ShutdownPoll::Closed(report)
        } else if report.deadline_elapsed {
            ShutdownPoll::Incomplete(report)
        } else {
            ShutdownPoll::Closing(report)
        }))
    }
}
/// What other files of one target may hold of a producer without a proven
/// sync, as seen by one file (REV-052 RV-128).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Foreign {
    /// Hull of released files still owing their sync, `(0, 0)` when none.
    pub(crate) owed: (u64, u64),
    /// Highest sequence the target's files appended before this file.
    pub(crate) base: u64,
    /// Highest sequence the target's files appended so far.
    pub(crate) appended: u64,
}
impl Foreign {
    /// The parts of `from..=to` another file may hold unsynced: the owed
    /// hull up to `base`, and what later files appended after this file's
    /// `last`. Between `base` and `last` this file was the only writer, so a
    /// record there that it does not hold was never appended to the target.
    fn spans(self, from: u64, to: u64, last: u64) -> impl Iterator<Item = (u64, u64)> {
        let earlier = if self.owed.0 == 0 {
            (1, 0)
        } else {
            (from.max(self.owed.0), to.min(self.owed.1).min(self.base))
        };
        let later = (
            from.max(self.base.max(last).saturating_add(1)),
            to.min(self.appended),
        );
        [earlier, later].into_iter().filter(|(lo, hi)| lo <= hi)
    }
}
/// Adapter-owned synchronization operation. Hold through actual IO completion or
/// confirmed cancellation. Releasing without acknowledgment reports Unknown.
///
/// ```compile_fail
/// use dot_logger::control::SyncWork;
/// fn duplicate(work: SyncWork<'_, '_>) { let _ = work.clone(); }
/// ```
///
/// ```compile_fail
/// use dot_logger::{control::{ControlOwner, SyncToken, SyncWork}, routing::Dispatcher, completion::ProducerRange};
/// use std::time::Instant;
/// fn escape<'a>(owner: ControlOwner<'a>, token: SyncToken, now: Instant,
///     dispatcher: &mut Dispatcher<'_, 'a>, ranges: &mut [ProducerRange]) -> SyncWork<'a, 'a> {
///     owner.next_sync(token, 10, now, dispatcher, ranges).unwrap().unwrap()
/// }
/// ```
pub struct SyncWork<'control, 'owner> {
    owner: &'control ControlOwner<'owner>,
    token: SyncToken,
    index: usize,
    target: u32,
    armed: bool,
}
impl SyncWork<'_, '_> {
    /// Whether this barrier may be acknowledged by syncing one file that
    /// appended `first..=last` of each producer. A record outside that range
    /// is owed to this file only while still pending; one that `foreign` says
    /// another file may hold unsynced needs a positive proof instead.
    pub(crate) fn covers_file(
        &self,
        first: &[u64],
        last: &[u64],
        foreign: impl Fn(usize) -> Foreign,
    ) -> Result<bool, ControlError> {
        let completion = self.owner.completion;
        let slot = &self.owner.slots[self.token.slot];
        if first.len() != slot.ranges.len() || last.len() != slot.ranges.len() {
            return Ok(false);
        }
        let window = completion.registry.limits.window as u64;
        for (p, range) in slot.ranges.iter().enumerate() {
            let start = range.first.load(Ordering::Acquire);
            let end = range.last.load(Ordering::Acquire);
            if start == 0 || end < start {
                continue;
            }
            let foreign = foreign(p);
            let mut outside = [(1, 0); 2];
            if first[p] == 0 {
                outside[0] = (start, end);
            } else {
                if start < first[p] {
                    outside[0] = (start, end.min(first[p] - 1));
                }
                if end > last[p] {
                    outside[1] = (start.max(last[p] + 1), end);
                }
            }
            for (from, to) in outside {
                if to < from {
                    continue;
                }
                // Another file's share: every record must be proven Synced,
                // not routed here, or ended without a write. Details older
                // than the window are gone, so such a span is unproven. This
                // holds under a Synced contract too: a record there may end
                // Unknown or Failed after only Written (REV-071 RV-190).
                for (lo, hi) in foreign.spans(from, to, last[p]) {
                    if hi - lo >= window {
                        return Ok(false);
                    }
                    if !self.settled(p, lo, hi, true)? {
                        return Ok(false);
                    }
                }
                // Never appended by any file: a completion slot is reused only
                // after its record ended on every target, so only the newest
                // `window` sequences can still be owed to this file.
                if !self.settled(
                    p,
                    from.max(to.saturating_sub(window.saturating_sub(1))),
                    to,
                    false,
                )? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
    /// Poll `from..=to` of producer `p` at Synced: pending fails, and with
    /// `strict` so does an expired detail.
    fn settled(&self, p: usize, from: u64, to: u64, strict: bool) -> Result<bool, ControlError> {
        let completion = self.owner.completion;
        for seq in from..=to {
            let receipt = crate::capture::CaptureReceipt {
                session_id: self.token.session_id,
                producer_id: p as u32,
                accepted_seq: seq,
            };
            let status = completion.poll(receipt, self.target, Boundary::Synced)?;
            match status.outcome {
                QueryOutcome::Busy => return Err(ControlError::Busy),
                QueryOutcome::Pending => return Ok(false),
                QueryOutcome::Expired if strict => return Ok(false),
                _ => {}
            }
            // Another file may hold it unsynced: a record that reached
            // Written there but ended Failed or Unknown before Synced is not
            // proven synced anywhere (REV-071 RV-175).
            if strict
                && status.confirmed.contains(Boundary::Written)
                && !status.confirmed.contains(Boundary::Synced)
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    /// Frozen request identity.
    pub fn token(&self) -> SyncToken {
        self.token
    }
    /// Target whose barrier this operation owns.
    pub fn target_id(&self) -> u32 {
        self.target
    }
    /// Copy every producer cutoff into caller/writer-owned bounded storage.
    pub fn copy_ranges(&self, buffer: &mut [ProducerRange]) -> Result<usize, ControlError> {
        self.owner.slots[self.token.slot].copy_ranges(buffer)
    }
    /// Acknowledge the actual adapter outcome, leaving bounded receipt publication
    /// to subsequent control polls. No payload storage is invalidated here.
    pub fn complete(mut self, result: SyncResult) {
        self.owner.slots[self.token.slot].targets[self.index]
            .state
            .store(
                match result {
                    SyncResult::Synced => ACK_OK,
                    SyncResult::Failed => ACK_FAILED,
                    SyncResult::Unknown => ACK_UNKNOWN,
                },
                Ordering::Release,
            );
        self.owner.target_busy[self.index].store(false, Ordering::Release);
        self.armed = false;
    }
}
impl Drop for SyncWork<'_, '_> {
    fn drop(&mut self) {
        if self.armed {
            self.owner.slots[self.token.slot].targets[self.index]
                .state
                .store(ACK_UNKNOWN, Ordering::Release);
            self.owner.target_busy[self.index].store(false, Ordering::Release);
        }
    }
}

#[cfg(test)]
mod tests;
