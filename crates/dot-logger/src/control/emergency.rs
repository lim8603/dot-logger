//! Caller-driven, startup-only emergency plans sharing the normal coordinator.
//! Protected records are accepted before the alarm into existing input/raw quotas.
//! This first policy admits zero extra final snapshots after notification. No
//! side channel bypasses capture receipts, writer bounds, or actual IO lifetimes.
use super::*;
use crate::platform::power::{PowerAlarm, PowerNotice};
use std::time::Duration;

/// One startup-declared protected file. Rank is separate from record importance.
#[derive(Clone, Copy, Debug)]
pub struct EmergencyFile {
    /// Existing file target supporting Synced.
    pub target: u32,
    /// Lower ranks run first; equal ranks use ascending target ID.
    pub rank: u32,
    /// Maximum protected payload bytes; framing/footer use the writer's budget.
    pub protected_bytes: u64,
    /// Host estimate of preceding drain plus file sync, not an enforced IO timeout.
    pub estimated_drain_sync: Duration,
}
/// Explicit enabled startup plan; disabled owners allocate none of this metadata.
pub struct EmergencyConfig {
    /// Nonempty unique file targets, bounded by the configured target count.
    pub files: Vec<EmergencyFile>,
    /// Time reserved before actual cutoff; includes host shutdown safety headroom.
    pub safety_margin: Duration,
}
struct ProtectedFile {
    plan: EmergencyFile,
    index: usize,
    observed_bytes: AtomicU64,
    confirmed_at: AtomicU64,
}
pub(super) struct EmergencyState {
    files: Box<[ProtectedFile]>,
    margin_ns: u64,
    estimate_ns: u64,
    deadline: AtomicU64,
    first_event: AtomicU64,
    first_generated: AtomicU64,
    first_received: AtomicU64,
}
/// File barrier facts; completion receipt failures must also be inspected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmergencyFileState {
    /// No alarm/frozen barrier yet.
    Armed,
    /// Preceding input/output or a higher-rank file has not finished.
    Pending,
    /// A real IO guard is still held, even if its deadline has passed.
    InFlight,
    /// Barrier confirmed and retained receipt updates finished.
    Synced,
    /// Adapter synchronization failed.
    Failed,
    /// IO guard ended without a confirmed result.
    Unknown,
    /// Deadline prevented an unstarted operation.
    Expired,
}
/// Fixed per-file report copied into caller-owned bounded storage.
#[derive(Clone, Copy, Debug)]
pub struct EmergencyFileReport {
    /// Original target identity.
    pub target: u32,
    /// Declared rank.
    pub rank: u32,
    /// Observed barrier lifecycle, not inferred from file Drop.
    pub state: EmergencyFileState,
    /// Explicit frozen payload count supplied by the trusted writer host.
    /// None means unobserved, never zero or evidence of complete protection.
    pub observed_bytes: Option<u64>,
    /// Declared payload limit.
    pub limit_bytes: u64,
    /// Conservative post-IO/receipt-publication observation time.
    pub confirmed_at: Option<Instant>,
}
/// Aggregate power timing and cleanup facts; no hardware certification is implied.
#[derive(Clone, Copy, Debug)]
pub struct EmergencyReport {
    /// First diagnostic ID; timing minima may include subsequent earlier events.
    pub first_event_id: u64,
    /// Earliest observed generation time, independent of the first event ID.
    pub earliest_generated_at: Instant,
    /// Earliest observed receipt time; source-to-receipt is included in the budget.
    pub earliest_received_at: Instant,
    /// Earliest posted cutoff; never extended by duplicates or later alarms.
    pub poweroff_deadline: Instant,
    /// Cutoff less startup safety margin, with no time before origin invented.
    pub work_deadline: Instant,
    /// Estimated work cannot fit the remaining budget, or declared bytes exceeded.
    /// A conservative estimate is separate from the actual completion facts.
    pub unachievable: bool,
    /// All protected file barriers, byte bounds and required receipts confirmed
    /// before the effective deadline. Cleanup of unrelated IO may still be live.
    pub protected_complete: bool,
    /// Same normal shutdown coordinator's actual resource/partial report.
    pub shutdown: ShutdownPoll,
}
impl ControlOwner<'_> {
    /// Install the immutable plan before producers or sync commands are used.
    /// Existing input/raw allocations contain protected records; this reserves
    /// only checked control metadata, not a second payload pool. No late snapshots
    /// are accepted after notify. Reconfiguration requires a new stopped owner.
    pub fn enable_emergency(
        &mut self,
        plan: &ValidatedConfig,
        mut config: EmergencyConfig,
    ) -> Result<(), ControlError> {
        if self.emergency.is_some()
            || self.shutdown.load(Ordering::Acquire) != 0
            || self.capture.memory != &plan.config().memory
            || self.completion.active_calls() != 0
            || self
                .completion
                .registry
                .producers
                .iter()
                .any(|p| p.accepted.load(Ordering::Acquire) != 0)
            || self
                .slots
                .iter()
                .any(|slot| slot.phase.load(Ordering::Acquire) != FREE)
            || config.files.is_empty()
            || config.files.len() > self.completion.targets().len()
        {
            return Err(ControlError::InvalidConfig);
        }
        let margin_ns = u64::try_from(config.safety_margin.as_nanos())
            .map_err(|_| ControlError::InvalidDeadline)?;
        let mut estimate_ns = 0u64;
        let mut protected_bytes = 0u64;
        if self.completion.targets().iter().any(|target| {
            target.required
                && target.completion == Boundary::Synced
                && !config.files.iter().any(|file| file.target == target.id)
        }) {
            return Err(ControlError::InvalidConfig);
        }
        for (n, file) in config.files.iter().enumerate() {
            if file.protected_bytes == 0
                || file.protected_bytes == u64::MAX
                || config.files[..n].iter().any(|f| f.target == file.target)
                || !self
                    .completion
                    .targets()
                    .iter()
                    .any(|t| t.id == file.target && t.supported.contains(Boundary::Synced))
            {
                return Err(ControlError::InvalidConfig);
            }
            estimate_ns = estimate_ns
                .checked_add(
                    u64::try_from(file.estimated_drain_sync.as_nanos())
                        .map_err(|_| ControlError::InvalidDeadline)?,
                )
                .ok_or(ControlError::InvalidDeadline)?;
            protected_bytes = protected_bytes
                .checked_add(file.protected_bytes)
                .ok_or(ControlError::InvalidConfig)?;
        }
        let payload_budget = plan
            .config()
            .memory
            .input_bytes
            .checked_add(plan.config().memory.raw_bytes)
            .ok_or(ControlError::InvalidConfig)?;
        if protected_bytes > payload_budget as u64 {
            return Err(ControlError::InvalidConfig);
        }
        let extra = size_of::<EmergencyState>()
            .checked_add(
                config
                    .files
                    .len()
                    .checked_mul(size_of::<ProtectedFile>())
                    .ok_or(ControlError::InvalidConfig)?,
            )
            .ok_or(ControlError::InvalidConfig)?;
        let reserved = self
            .reserved
            .checked_add(extra)
            .ok_or(ControlError::InvalidConfig)?;
        let required = self
            .capture
            .reserved
            .checked_add(reserved)
            .ok_or(ControlError::InvalidConfig)?;
        if required > plan.config().memory.control_bytes {
            return Err(ControlError::Config(ConfigError::BudgetExceeded {
                resource: Resource::Control,
                required,
                available: plan.config().memory.control_bytes,
            }));
        }
        config
            .files
            .sort_unstable_by_key(|file| (file.rank, file.target));
        let files = config
            .files
            .into_iter()
            .map(|file| ProtectedFile {
                index: self
                    .completion
                    .targets()
                    .iter()
                    .position(|t| t.id == file.target)
                    .unwrap_or(usize::MAX),
                plan: file,
                observed_bytes: AtomicU64::new(u64::MAX),
                confirmed_at: AtomicU64::new(u64::MAX),
            })
            .collect();
        self.emergency = Some(Box::new(EmergencyState {
            files,
            margin_ns,
            estimate_ns,
            deadline: AtomicU64::new(u64::MAX),
            first_event: AtomicU64::new(0),
            first_generated: AtomicU64::new(u64::MAX),
            first_received: AtomicU64::new(u64::MAX),
        }));
        self.reserved = reserved;
        Ok(())
    }
    /// Publish an alarm without allocation, IO, locks, callbacks or retry loops.
    /// The host invokes this outside an OS async-signal handler. Disabled returns
    /// before inspecting the event and has no transition/reservation side effects.
    /// Metadata precedes the release deadline publication; minima are independent,
    /// so no multiword event/timestamp pair is invented under concurrent posts.
    pub fn notify_power(
        &self,
        alarm: PowerAlarm,
        received_at: Instant,
    ) -> Result<PowerNotice, ControlError> {
        let Some(state) = &self.emergency else {
            return Ok(PowerNotice::Disabled);
        };
        let generated = self.tick(alarm.generated_at)?;
        let received = self.tick(received_at)?;
        let deadline = self.tick(alarm.poweroff_deadline)?;
        if alarm.event_id == 0
            || received < generated
            || deadline <= generated
            || deadline == u64::MAX
        {
            return Err(ControlError::InvalidDeadline);
        }
        let _ = state.first_event.compare_exchange(
            0,
            alarm.event_id,
            Ordering::AcqRel,
            Ordering::Relaxed,
        );
        state
            .first_generated
            .fetch_min(generated, Ordering::Relaxed);
        state.first_received.fetch_min(received, Ordering::Relaxed);
        // Close capture before publishing an actionable emergency. An overlapping
        // admitted producer remains tracked until the existing quiescence barrier.
        self.capture.close();
        let previous = state.deadline.fetch_min(deadline, Ordering::AcqRel);
        Ok(if previous == u64::MAX {
            PowerNotice::Posted
        } else if deadline < previous {
            PowerNotice::Tightened
        } else {
            PowerNotice::Unchanged
        })
    }
    /// Record a trusted host's frozen protected payload count after closure and
    /// publication quiescence. Missing/over-limit observations prevent success.
    /// This is observation only; it does not authorize additional input or IO.
    pub fn observe_protected_bytes(&self, target: u32, bytes: u64) -> Result<(), ControlError> {
        let state = self.emergency.as_ref().ok_or(ControlError::InvalidConfig)?;
        if self.shutdown_sync().is_none() || bytes == u64::MAX {
            return Err(ControlError::Busy);
        }
        let file = state
            .files
            .iter()
            .find(|f| f.plan.target == target)
            .ok_or(ControlError::InvalidConfig)?;
        file.observed_bytes
            .compare_exchange(u64::MAX, bytes, Ordering::AcqRel, Ordering::Relaxed)
            .map_err(|_| ControlError::Busy)?;
        Ok(())
    }
    /// Consume the posted alarm on the general host worker. It cancels new work
    /// for optional nonprotected targets, starts/tightens the same shutdown, and
    /// never cancels or drops an already held writer/raw/sync guard.
    /// None means disabled/armed without an event and starts no shutdown.
    pub fn poll_emergency(
        &self,
        now: Instant,
        dispatcher: &mut Dispatcher<'_, '_>,
    ) -> Result<Option<EmergencyReport>, ControlError> {
        let Some(state) = &self.emergency else {
            return Ok(None);
        };
        let deadline = state.deadline.load(Ordering::Acquire);
        if deadline == u64::MAX {
            return Ok(None);
        };
        if self.tick(now)? < state.first_received.load(Ordering::Acquire) {
            return Err(ControlError::ClockWentBackwards);
        }
        self.now(now)?;
        self.tighten_emergency_deadline();
        for target in self.completion.targets() {
            if !target.required && !state.files.iter().any(|f| f.plan.target == target.id) {
                dispatcher.cancel_target(target.id);
            }
        }
        let work_deadline =
            self.origin + Duration::from_nanos(deadline.saturating_sub(state.margin_ns));
        let shutdown = if self.shutdown.load(Ordering::Acquire) == 0 {
            match self.begin_shutdown(
                ShutdownPlan { sync_files: true },
                now,
                work_deadline,
                dispatcher,
            ) {
                Ok(report) => report,
                Err(ControlError::Closing) => self.poll_shutdown(now, dispatcher)?,
                Err(error) => return Err(error),
            }
        } else {
            self.poll_shutdown(now, dispatcher)?
        };
        #[cfg(test)]
        super::tests::pause_emergency_report();
        // An alarm can tighten while normal shutdown is being polled. Reobserve
        // after that work; a report must not evaluate protection against the old
        // deadline. A later post after this snapshot is handled on the next poll.
        let latest = state.deadline.load(Ordering::Acquire);
        let shutdown = if latest != deadline {
            self.poll_shutdown(now, dispatcher)?
        } else {
            shutdown
        };
        let deadline = state.deadline.load(Ordering::Acquire);
        let work_deadline =
            self.origin + Duration::from_nanos(deadline.saturating_sub(state.margin_ns));
        let mut complete = true;
        let mut bounds = true;
        for file in &state.files {
            let observed = file.observed_bytes.load(Ordering::Acquire);
            bounds &= observed == u64::MAX || observed <= file.plan.protected_bytes;
            complete &= observed != u64::MAX
                && observed <= file.plan.protected_bytes
                && self.emergency_file_state(file) == EmergencyFileState::Synced
                && file.confirmed_at.load(Ordering::Acquire)
                    < deadline.saturating_sub(state.margin_ns);
            // A successful empty/file barrier cannot hide a prior record failure.
            let totals = self
                .completion
                .target_totals(file.plan.target)
                .ok_or(ControlError::InvalidConfig)?;
            complete &= totals.pending == 0
                && totals.busy == 0
                && totals.failed == 0
                && totals.dropped == 0
                && totals.unknown == 0
                && totals.expired_records == 0
                && !totals.overflowed;
        }
        let received = state.first_received.load(Ordering::Acquire);
        Ok(Some(EmergencyReport {
            first_event_id: state.first_event.load(Ordering::Acquire),
            earliest_generated_at: self.origin
                + Duration::from_nanos(state.first_generated.load(Ordering::Acquire)),
            earliest_received_at: self.origin
                + Duration::from_nanos(state.first_received.load(Ordering::Acquire)),
            poweroff_deadline: self.origin + Duration::from_nanos(deadline),
            work_deadline,
            unachievable: !bounds
                || state.estimate_ns
                    > deadline
                        .saturating_sub(state.margin_ns)
                        .saturating_sub(received),
            protected_complete: complete,
            shutdown,
        }))
    }
    /// Copy ordered file facts into caller storage, without allocating or claiming
    /// that an IO deadline invalidated an active guard.
    pub fn emergency_files(
        &self,
        buffer: &mut [EmergencyFileReport],
    ) -> Result<usize, ControlError> {
        let Some(state) = &self.emergency else {
            return Ok(0);
        };
        if buffer.len() < state.files.len() {
            return Err(ControlError::BufferTooSmall);
        };
        for (out, file) in buffer.iter_mut().zip(&state.files) {
            let observed = file.observed_bytes.load(Ordering::Acquire);
            let confirmed = file.confirmed_at.load(Ordering::Acquire);
            *out = EmergencyFileReport {
                target: file.plan.target,
                rank: file.plan.rank,
                state: self.emergency_file_state(file),
                observed_bytes: (observed != u64::MAX).then_some(observed),
                limit_bytes: file.plan.protected_bytes,
                confirmed_at: (confirmed != u64::MAX)
                    .then(|| self.origin + Duration::from_nanos(confirmed)),
            };
        }
        Ok(state.files.len())
    }
    fn emergency_file_state(&self, file: &ProtectedFile) -> EmergencyFileState {
        if self.slots[self.config.sync_slots]
            .generation
            .load(Ordering::Acquire)
            == 0
        {
            return EmergencyFileState::Armed;
        }
        match self.slots[self.config.sync_slots].targets[file.index]
            .state
            .load(Ordering::Acquire)
        {
            IN_FLIGHT => EmergencyFileState::InFlight,
            DONE_OK => EmergencyFileState::Synced,
            DONE_FAILED => EmergencyFileState::Failed,
            DONE_UNKNOWN => EmergencyFileState::Unknown,
            EXPIRED | APPLY_EXPIRED => EmergencyFileState::Expired,
            _ => EmergencyFileState::Pending,
        }
    }
    pub(super) fn emergency_active(&self) -> bool {
        self.emergency
            .as_ref()
            .is_some_and(|s| s.deadline.load(Ordering::Acquire) != u64::MAX)
    }
    pub(super) fn tighten_emergency_deadline(&self) {
        let Some(state) = &self.emergency else { return };
        let end = state.deadline.load(Ordering::Acquire);
        if end == u64::MAX {
            return;
        };
        let end = end.saturating_sub(state.margin_ns);
        self.shutdown_deadline.fetch_min(end, Ordering::AcqRel);
        let slot = &self.slots[self.config.sync_slots];
        // FREE slots get initialized from shutdown_deadline; LIVE/EDITING slots
        // only tighten their atomic deadline. No guard state or memory is reset.
        if slot.phase.load(Ordering::Acquire) != FREE {
            slot.deadline.fetch_min(end, Ordering::AcqRel);
        }
    }
    pub(super) fn emergency_target_selected(&self, index: usize) -> bool {
        !self.emergency_active()
            || self
                .emergency
                .as_ref()
                .is_some_and(|s| s.files.iter().any(|f| f.index == index))
    }
    pub(super) fn emergency_sync_allowed(&self, index: usize) -> bool {
        if !self.emergency_active() {
            return true;
        };
        let Some(state) = &self.emergency else {
            return false;
        };
        for file in &state.files {
            if file.index == index {
                return true;
            };
            if matches!(
                self.emergency_file_state(file),
                EmergencyFileState::Armed
                    | EmergencyFileState::Pending
                    | EmergencyFileState::InFlight
            ) {
                return false;
            };
        }
        false
    }
    pub(super) fn mark_emergency_confirmation(&self, slot: &SyncSlot, index: usize, tick: u64) {
        if !std::ptr::eq(slot, &self.slots[self.config.sync_slots]) {
            return;
        };
        if let Some(state) = &self.emergency {
            if let Some(file) = state.files.iter().find(|f| f.index == index) {
                let _ = file.confirmed_at.compare_exchange(
                    u64::MAX,
                    tick,
                    Ordering::AcqRel,
                    Ordering::Relaxed,
                );
            }
        }
    }
}
