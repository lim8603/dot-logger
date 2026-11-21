//! Bounded producer-local completion windows, independent of payload ownership.
//!
//! RT reservation claims a reusable slot with one lifecycle CAS, without a retry
//! loop or waiting for management reports. Atomics contain only metadata; payload
//! safety remains in pool/storage. General reports also claim a short exclusive
//! metadata publication state and return Busy on contention. Sequence and state
//! checks prevent stale updates from mutating a reused slot. Queries are readers
//! with bounded pages and never start IO. Confirmed boundaries survive later errors.
use crate::capture::{CaptureReceipt, ChannelConfig, Importance};
use crate::config::{ConfigError, ProducerQuota, Resource};
use crate::routing::{DispatchEvent, DispatchOutcome, DropReason, RoutingConfig};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;
mod types;
pub use types::*;
mod query;
pub use query::{CompletionQuery, CompletionScope};

const EMPTY: u8 = 0;
const INITIALIZING: u8 = 1;
const ACTIVE: u8 = 2;
const UPDATING: u8 = 3;
const TERMINAL: u8 = 4;
const NOT_ROUTED: u32 = 1 << 16;
const TERM_SHIFT: u32 = 8;
const TERM_MASK: u32 = 15 << TERM_SHIFT;

pub(crate) struct TargetCell {
    word: AtomicU32,
    abandoned: AtomicU64,
}
pub(crate) struct RecordSlot {
    phase: AtomicU8,
    pub(crate) sequence: AtomicU64,
    terminal_at: AtomicU64,
    revision: AtomicU64,
    // Original registered channel; published by phase Release before general
    // output reports Acquire the slot. Never reconstructed from a DispatchEvent.
    channel_index: AtomicUsize,
    cells: Box<[TargetCell]>,
}
struct Retired {
    counts: [AtomicU64; 4],
}
pub(crate) struct ProducerState {
    pub(crate) active: AtomicBool,
    pub(crate) accepted: AtomicU64,
    pub(crate) slots: Box<[RecordSlot]>,
    important_reserve: usize,
    retired: Box<[Retired]>,
    retired_records: AtomicU64,
}
struct RoutePlan {
    channel_id: u32,
    channel_index: usize,
    indices: Box<[usize]>,
}
#[derive(Clone, Copy)]
pub(crate) struct Limits {
    pub(crate) window: usize,
    pub(crate) max_queries: usize,
    pub(crate) page_items: usize,
    pub(crate) feedback: usize,
    pub(crate) updates: usize,
    pub(crate) reconcile: usize,
}
pub(crate) struct Registry {
    pub(crate) loss: Option<std::sync::Arc<crate::loss::Bank>>,
    pub(crate) control_attached: AtomicBool,
    pub(crate) control_epoch: AtomicU64,
    pub(crate) session_id: u128,
    pub(crate) revision: u64,
    pub(crate) targets: Box<[TargetContract]>,
    routes: Box<[RoutePlan]>,
    pub(crate) producers: Box<[ProducerState]>,
    pub(crate) limits: Limits,
    readers: AtomicUsize,
    cursor: AtomicUsize,
    /// Abandoned deliveries so far: the only state no update terminalizes.
    abandons: AtomicU64,
    /// `abandons` when the current sweep started.
    swept: AtomicU64,
    /// Slots the current sweep has still to visit.
    sweep_left: AtomicUsize,
    /// The sweep skipped an abandoned slot that was being updated.
    sweep_missed: AtomicBool,
    /// One reconciler owns the sweep state (`swept`, `sweep_left`,
    /// `sweep_missed`, `cursor`) at a time (REV-053 RV-131).
    sweeping: AtomicBool,
    reserved: usize,
    origin: Instant,
    retention: u64,
}
fn memory_error() -> CompletionError {
    CompletionError::Config(ConfigError::ArithmeticOverflow)
}
fn add_array(total: &mut usize, count: usize, size: usize) -> Result<(), CompletionError> {
    let bytes = count
        .checked_mul(size)
        .filter(|n| *n <= isize::MAX as usize)
        .ok_or_else(memory_error)?;
    *total = total.checked_add(bytes).ok_or_else(memory_error)?;
    Ok(())
}
impl Registry {
    /// Control bytes of the registry for these counts. Shared with the facade
    /// sizing so both use one formula.
    pub(crate) fn required(
        producers: usize,
        window: usize,
        targets: usize,
        routing: &RoutingConfig,
    ) -> Result<usize, CompletionError> {
        let mut reserved = size_of::<Self>();
        add_array(&mut reserved, producers, size_of::<ProducerState>())?;
        let slots = producers.checked_mul(window).ok_or_else(memory_error)?;
        add_array(&mut reserved, slots, size_of::<RecordSlot>())?;
        let cells = slots.checked_mul(targets).ok_or_else(memory_error)?;
        add_array(&mut reserved, cells, size_of::<TargetCell>())?;
        add_array(
            &mut reserved,
            producers.checked_mul(targets).ok_or_else(memory_error)?,
            size_of::<Retired>(),
        )?;
        add_array(&mut reserved, targets, size_of::<TargetContract>())?;
        add_array(&mut reserved, routing.routes.len(), size_of::<RoutePlan>())?;
        for route in &routing.routes {
            add_array(&mut reserved, route.targets.len(), size_of::<usize>())?;
        }
        Ok(reserved)
    }
    pub(crate) fn new(
        session_id: u128,
        revision: u64,
        quotas: &[ProducerQuota],
        channels: &[ChannelConfig],
        routing: &RoutingConfig,
        config: CompletionConfig,
        available: usize,
    ) -> Result<Self, CompletionError> {
        let retention = u64::try_from(config.history_retention.as_nanos())
            .map_err(|_| CompletionError::InvalidConfig)?;
        if retention == 0
            || config.window_records == 0
            || config.max_queries == 0
            || config.page_items == 0
            || config.feedback_events == 0
            || config.updates_per_turn == 0
            || config.reconcile_per_turn == 0
            || config.targets.is_empty()
            // An abandon marker names its target in 16 bits.
            || config.targets.len() > usize::from(u16::MAX)
            || config.targets.len() != routing.targets.len()
            || routing.routes.len() != channels.len()
        {
            return Err(CompletionError::InvalidConfig);
        }
        let producer_count = quotas.len();
        let important_reserve = |quota: &ProducerQuota| {
            quota
                .small
                .important_slots
                .max(quota.raw.as_ref().map_or(0, |raw| raw.important_slots))
        };
        if quotas
            .iter()
            .any(|quota| important_reserve(quota) > config.window_records)
        {
            return Err(CompletionError::InvalidConfig);
        }
        for (i, target) in config.targets.iter().enumerate() {
            if !target.supported.contains(target.completion)
                || !target.supported.contains(Boundary::Enqueued)
                || config.targets[..i]
                    .iter()
                    .any(|other| other.id == target.id)
                || (target.supported.contains(Boundary::Synced)
                    && !target.supported.contains(Boundary::Written))
                || !routing
                    .targets
                    .iter()
                    .any(|r| r.id == target.id && r.required == target.required)
            {
                return Err(CompletionError::InvalidConfig);
            }
        }
        for (i, route) in routing.routes.iter().enumerate() {
            if !channels.iter().any(|c| c.channel_id == route.channel_id)
                || routing.routes[..i]
                    .iter()
                    .any(|r| r.channel_id == route.channel_id)
                || route.targets.len().max(1) > config.feedback_events
                || route.targets.iter().enumerate().any(|(j, id)| {
                    route.targets[..j].contains(id) || !config.targets.iter().any(|t| t.id == *id)
                })
            {
                return Err(CompletionError::InvalidConfig);
            }
        }
        let reserved = Self::required(
            producer_count,
            config.window_records,
            config.targets.len(),
            routing,
        )?;
        if reserved > available {
            return Err(CompletionError::Config(ConfigError::BudgetExceeded {
                resource: Resource::Completion,
                required: reserved,
                available,
            }));
        }
        let targets: Box<[_]> = config.targets.into_boxed_slice();
        let routes = routing
            .routes
            .iter()
            .map(|r| RoutePlan {
                channel_id: r.channel_id,
                channel_index: channels
                    .iter()
                    .position(|c| c.channel_id == r.channel_id)
                    .unwrap_or(0),
                indices: r
                    .targets
                    .iter()
                    .filter_map(|id| targets.iter().position(|t| t.id == *id))
                    .collect(),
            })
            .collect();
        let producers = quotas
            .iter()
            .map(|quota| ProducerState {
                active: AtomicBool::new(false),
                accepted: AtomicU64::new(0),
                important_reserve: important_reserve(quota),
                slots: (0..config.window_records)
                    .map(|_| RecordSlot {
                        phase: AtomicU8::new(EMPTY),
                        sequence: AtomicU64::new(0),
                        terminal_at: AtomicU64::new(u64::MAX),
                        revision: AtomicU64::new(revision),
                        channel_index: AtomicUsize::new(0),
                        cells: (0..targets.len())
                            .map(|_| TargetCell {
                                word: AtomicU32::new(NOT_ROUTED),
                                abandoned: AtomicU64::new(0),
                            })
                            .collect(),
                    })
                    .collect(),
                retired: (0..targets.len())
                    .map(|_| Retired {
                        counts: std::array::from_fn(|_| AtomicU64::new(0)),
                    })
                    .collect(),
                retired_records: AtomicU64::new(0),
            })
            .collect();
        Ok(Self {
            loss: None,
            control_attached: AtomicBool::new(false),
            control_epoch: AtomicU64::new(0),
            session_id,
            revision,
            targets,
            routes,
            producers,
            limits: Limits {
                window: config.window_records,
                max_queries: config.max_queries,
                page_items: config.page_items,
                feedback: config.feedback_events,
                updates: config.updates_per_turn,
                reconcile: config.reconcile_per_turn,
            },
            readers: AtomicUsize::new(0),
            cursor: AtomicUsize::new(0),
            abandons: AtomicU64::new(0),
            swept: AtomicU64::new(0),
            sweep_left: AtomicUsize::new(0),
            sweep_missed: AtomicBool::new(false),
            sweeping: AtomicBool::new(false),
            reserved,
            origin: Instant::now(),
            retention,
        })
    }
    pub(crate) fn reserved(&self) -> usize {
        self.reserved
    }
    fn target(&self, id: u32) -> Result<usize, CompletionError> {
        self.targets
            .iter()
            .position(|t| t.id == id)
            .ok_or(CompletionError::InvalidTarget)
    }
    fn locate(&self, receipt: CaptureReceipt) -> Result<&RecordSlot, CompletionError> {
        let p = self
            .producers
            .get(receipt.producer_id as usize)
            .ok_or(CompletionError::InvalidReceipt)?;
        if receipt.session_id != self.session_id
            || receipt.accepted_seq == 0
            || receipt.accepted_seq > p.accepted.load(Ordering::Acquire)
        {
            return Err(CompletionError::InvalidReceipt);
        }
        Ok(&p.slots[(receipt.accepted_seq % self.limits.window as u64) as usize])
    }
    fn effective(&self, slot: &RecordSlot, target: usize, sequence: u64) -> u32 {
        let cell = &slot.cells[target];
        let value = cell.word.load(Ordering::Acquire);
        if value & NOT_ROUTED == 0
            && value & TERM_MASK == 0
            && !self.reported(value, target)
            && cell.abandoned.load(Ordering::Acquire) == sequence
        {
            value | (9 << TERM_SHIFT)
        } else {
            value
        }
    }
    /// Whether a delivery for `target` reported anything that ends its duty:
    /// any boundary past Enqueued, or the target's own completion boundary
    /// (an Enqueued contract ends at Enqueued, REV-071 RV-185).
    fn reported(&self, word: u32, target: usize) -> bool {
        word & 0b11110 != 0
            || self
                .targets
                .get(target)
                .is_some_and(|t| Boundaries(word & 31).contains(t.completion))
    }
    fn settled(&self, slot: &RecordSlot) -> bool {
        slot.cells.iter().zip(&self.targets).all(|(cell, target)| {
            let word = cell.word.load(Ordering::Acquire);
            word & NOT_ROUTED != 0
                || word & TERM_MASK != 0
                || Boundaries(word & 31).contains(target.completion)
        })
    }
    pub(crate) fn pending_revision(&self, revision: u64) -> bool {
        self.producers.iter().any(|p| {
            p.slots.iter().any(|slot| {
                let phase = slot.phase.load(Ordering::Acquire);
                phase != EMPTY
                    && phase != TERMINAL
                    && slot.revision.load(Ordering::Acquire) == revision
            })
        })
    }
    pub(crate) fn matches(&self, routing: &RoutingConfig) -> bool {
        routing.targets.len() == self.targets.len()
            && routing.routes.len() == self.routes.len()
            && routing.targets.iter().all(|r| {
                self.targets
                    .iter()
                    .any(|t| r.id == t.id && r.required == t.required)
            })
            && routing.routes.iter().all(|r| {
                self.routes.iter().any(|p| {
                    p.channel_id == r.channel_id
                        && p.indices.len() == r.targets.len()
                        && p.indices
                            .iter()
                            .all(|i| r.targets.contains(&self.targets[*i].id))
                })
            })
    }
}

/// Borrowed management view of owner-held completion metadata. Copying it neither
/// clones payloads nor allocates. Its owner must outlive reports, queries, and IO.
#[derive(Clone, Copy)]
pub struct Completion<'owner> {
    pub(crate) registry: &'owner Registry,
}
impl<'owner> Completion<'owner> {
    /// Original session identity, for combining observations from the same owner.
    pub fn session_id(self) -> u128 {
        self.registry.session_id
    }
    /// Initial registration revision, for session observation identity checks.
    pub fn revision(self) -> u64 {
        self.registry.revision
    }

    /// Immutable initial target contracts.
    pub fn targets(self) -> &'owner [TargetContract] {
        &self.registry.targets
    }
    /// Concrete metadata bytes included in CaptureOwner's control reservation.
    pub fn reserved_bytes(self) -> usize {
        self.registry.reserved
    }
    /// Sample one receipt; lookup never starts synchronization or output work.
    pub fn poll(
        self,
        receipt: CaptureReceipt,
        target: u32,
        boundary: Boundary,
    ) -> Result<RecordStatus, CompletionError> {
        self.observe(receipt, target, boundary, Some(Instant::now()))
    }
    /// Observe terminal-detail age in a caller-supplied monotonic clock sample.
    /// Use chronological samples; this is useful for deterministic retention tests.
    pub fn poll_at(
        self,
        receipt: CaptureReceipt,
        target: u32,
        boundary: Boundary,
        now: Instant,
    ) -> Result<RecordStatus, CompletionError> {
        self.observe(receipt, target, boundary, Some(now))
    }
    fn observe(
        self,
        receipt: CaptureReceipt,
        target: u32,
        boundary: Boundary,
        now: Option<Instant>,
    ) -> Result<RecordStatus, CompletionError> {
        let index = self.registry.target(target)?;
        if !self.registry.targets[index].supported.contains(boundary) {
            return Err(CompletionError::UnsupportedBoundary);
        }
        let slot = self.registry.locate(receipt)?;
        let phase = slot.phase.load(Ordering::Acquire);
        if phase == INITIALIZING || phase == UPDATING {
            return Err(CompletionError::Busy);
        }
        if slot.sequence.load(Ordering::Acquire) != receipt.accepted_seq {
            return Ok(RecordStatus {
                confirmed: Boundaries::default(),
                outcome: QueryOutcome::Expired,
            });
        }
        let stamp = slot.terminal_at.load(Ordering::Acquire);
        let aged = now.is_some_and(|now| self.registry.expired(stamp, now));
        let word = self.registry.effective(slot, index, receipt.accepted_seq);
        let after = slot.phase.load(Ordering::Acquire);
        if after == INITIALIZING || after == UPDATING {
            return Err(CompletionError::Busy);
        }
        if slot.sequence.load(Ordering::Acquire) != receipt.accepted_seq {
            return Ok(RecordStatus {
                confirmed: Boundaries::default(),
                outcome: QueryOutcome::Expired,
            });
        }
        if aged && phase == TERMINAL {
            return Ok(RecordStatus {
                confirmed: Boundaries::default(),
                outcome: QueryOutcome::Expired,
            });
        }
        Ok(classify(word, boundary))
    }
    /// Publish one bounded, nonblocking management observation. Busy changes nothing.
    pub fn report(
        self,
        receipt: CaptureReceipt,
        target: u32,
        update: Update,
    ) -> Result<(), CompletionError> {
        let index = self.registry.target(target)?;
        let confirmed = if let Update::Confirmed(boundary) = update {
            if !self.registry.targets[index].supported.contains(boundary) {
                return Err(CompletionError::UnsupportedBoundary);
            }
            let mut mask = Boundaries::only(boundary).0 | Boundaries::only(Boundary::Enqueued).0;
            if boundary == Boundary::Synced {
                mask |= Boundaries::only(Boundary::Written).0;
            }
            mask
        } else {
            0
        };
        let slot = self.registry.locate(receipt)?;
        let phase = slot.phase.load(Ordering::Acquire);
        if phase != ACTIVE && phase != TERMINAL {
            return Err(CompletionError::Busy);
        }
        slot.phase
            .compare_exchange(phase, UPDATING, Ordering::AcqRel, Ordering::Relaxed)
            .map_err(|_| CompletionError::Busy)?;
        let _publication = Publication {
            registry: self.registry,
            slot,
        };
        if slot.sequence.load(Ordering::Acquire) != receipt.accepted_seq {
            return Err(CompletionError::Expired);
        }
        if self
            .registry
            .expired(slot.terminal_at.load(Ordering::Acquire), Instant::now())
        {
            return Err(CompletionError::Expired);
        }
        let value = slot.cells[index].word.load(Ordering::Acquire);
        if value & NOT_ROUTED != 0 {
            return Err(CompletionError::NotRouted);
        }
        let outcome = terminal(update);
        // FIFO publication happens before the dispatcher can publish Enqueued.
        // A fast writer can already have reported failure in that interval.
        // Preserve that terminal outcome while admitting the earlier queue fact;
        // later success boundaries still cannot replace a terminal failure.
        let late_enqueue = update == Update::Confirmed(Boundary::Enqueued);
        if value & TERM_MASK != 0
            && ((outcome != 0 && outcome != value & TERM_MASK)
                || (confirmed & !value != 0 && !late_enqueue))
        {
            return Err(CompletionError::InvalidTransition);
        }
        if update == Update::Failed(FailureReason::SyncFailure)
            && Boundaries(value & 31).contains(Boundary::Synced)
        {
            return Err(CompletionError::InvalidTransition);
        }
        slot.cells[index]
            .word
            .store(value | confirmed | outcome, Ordering::Release);
        if value & TERM_MASK == 0 && outcome != 0 {
            self.registry
                .count_loss(receipt.producer_id as usize, slot, index, outcome);
        }
        Ok(())
    }
    /// Reconcile at most `limit` physical slots after unreported output release.
    /// This never scans an unbounded sequence history or releases payload memory.
    ///
    /// Updates terminalize their own slot; only an abandoned delivery needs a
    /// sweep. So each abandonment starts one sweep over the whole window, and
    /// without one this does no work (sweeping on every turn took about 37 %
    /// of the host thread in a measured load). A sweep that skipped an
    /// abandoned slot under update runs once more.
    ///
    /// A call that overlaps another reconciliation returns 0 without waiting;
    /// the sweep state has one owner, and a newer abandonment stays visible to
    /// the next call.
    pub fn reconcile(self, limit: usize) -> usize {
        let registry = self.registry;
        if registry
            .sweeping
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return 0;
        }
        let _owner = SweepOwner(&registry.sweeping);
        let total = registry.producers.len() * registry.limits.window;
        let generation = registry.abandons.load(Ordering::Acquire);
        if registry.swept.swap(generation, Ordering::AcqRel) != generation {
            registry.sweep_missed.store(false, Ordering::Relaxed);
            registry.sweep_left.store(total, Ordering::Release);
        }
        let left = registry.sweep_left.load(Ordering::Acquire);
        let count = limit.min(left);
        let mut terminalized = 0;
        for _ in 0..count {
            let index = registry.cursor.fetch_add(1, Ordering::Relaxed) % total;
            let slot = &registry.producers[index / registry.limits.window].slots
                [index % registry.limits.window];
            if let Err(phase) =
                slot.phase
                    .compare_exchange(ACTIVE, UPDATING, Ordering::AcqRel, Ordering::Relaxed)
            {
                if phase == UPDATING {
                    let sequence = slot.sequence.load(Ordering::Acquire);
                    if slot
                        .cells
                        .iter()
                        .any(|c| c.abandoned.load(Ordering::Acquire) == sequence)
                    {
                        registry.sweep_missed.store(true, Ordering::Relaxed);
                    }
                }
                continue;
            }
            let sequence = slot.sequence.load(Ordering::Acquire);
            for i in 0..self.registry.targets.len() {
                let old = slot.cells[i].word.load(Ordering::Acquire);
                let effective = self.registry.effective(slot, i, sequence);
                if old & TERM_MASK == 0 && effective & TERM_MASK != 0 {
                    registry.count_loss(
                        index / registry.limits.window,
                        slot,
                        i,
                        effective & TERM_MASK,
                    );
                }
                slot.cells[i].word.store(effective, Ordering::Release);
            }
            let phase = if self.registry.settled(slot) {
                terminalized += 1;
                TERMINAL
            } else {
                ACTIVE
            };
            if phase == TERMINAL {
                self.registry.stamp(slot);
            }
            slot.phase.store(phase, Ordering::Release);
        }
        let rest = left - count;
        #[cfg(test)]
        BEFORE_SWEEP_STORE.with(|hook| {
            if let Some(hook) = hook.borrow_mut().as_mut() {
                hook();
            }
        });
        registry.sweep_left.store(rest, Ordering::Release);
        if rest == 0 && registry.sweep_missed.swap(false, Ordering::AcqRel) {
            registry.sweep_left.store(total, Ordering::Release);
        }
        terminalized
    }
    /// Aggregate retired terminal counts and current windows. Concurrent samples can
    /// include Busy; shutdown uses this only after closing and resource quiescence.
    pub fn totals(self, required_only: bool) -> CompletionTotals {
        self.selected_totals(required_only, None)
    }
    /// Assignments for one fixed target, including retired outcomes. Records and
    /// expired_records retain their session-wide meaning; fanout is not ingress.
    /// A concurrent view may include Busy and is not a global atomic snapshot.
    pub fn target_totals(self, id: u32) -> Option<CompletionTotals> {
        self.registry
            .targets
            .iter()
            .any(|t| t.id == id)
            .then(|| self.selected_totals(false, Some(id)))
    }
    fn selected_totals(self, required_only: bool, target_id: Option<u32>) -> CompletionTotals {
        #[cfg(test)]
        TOTALS_SCANS.with(|n| n.set(n.get() + 1));
        let mut out = CompletionTotals::default();
        for (pid, producer) in self.registry.producers.iter().enumerate() {
            accumulate(
                &mut out.records,
                producer.accepted.load(Ordering::Acquire),
                &mut out.overflowed,
            );
            accumulate(
                &mut out.expired_records,
                producer.retired_records.load(Ordering::Acquire),
                &mut out.overflowed,
            );
            for (i, history) in producer.retired.iter().enumerate() {
                if (required_only && !self.registry.targets[i].required)
                    || target_id.is_some_and(|id| id != self.registry.targets[i].id)
                {
                    continue;
                }
                for j in 0..4 {
                    let count = history.counts[j].load(Ordering::Acquire);
                    accumulate(&mut out.assigned, count, &mut out.overflowed);
                    let field = match j {
                        0 => &mut out.complete,
                        1 => &mut out.failed,
                        2 => &mut out.dropped,
                        _ => &mut out.unknown,
                    };
                    accumulate(field, count, &mut out.overflowed);
                }
            }
            for slot in &producer.slots {
                let seq = slot.sequence.load(Ordering::Acquire);
                if seq == 0 {
                    continue;
                }
                for target in self.registry.targets.iter() {
                    if (required_only && !target.required)
                        || target_id.is_some_and(|id| id != target.id)
                    {
                        continue;
                    }
                    let receipt = CaptureReceipt {
                        session_id: self.registry.session_id,
                        producer_id: pid as u32,
                        accepted_seq: seq,
                    };
                    let status = self.observe(receipt, target.id, target.completion, None);
                    let field = match status {
                        Ok(RecordStatus {
                            outcome: QueryOutcome::NotRouted,
                            ..
                        }) => continue,
                        Ok(RecordStatus {
                            outcome: QueryOutcome::Complete,
                            ..
                        }) => &mut out.complete,
                        Ok(RecordStatus {
                            outcome: QueryOutcome::Failed(_),
                            ..
                        }) => &mut out.failed,
                        Ok(RecordStatus {
                            outcome: QueryOutcome::Dropped(_),
                            ..
                        }) => &mut out.dropped,
                        Ok(RecordStatus {
                            outcome: QueryOutcome::Unknown,
                            ..
                        }) => &mut out.unknown,
                        Ok(RecordStatus {
                            outcome: QueryOutcome::Pending,
                            ..
                        }) => &mut out.pending,
                        _ => &mut out.busy,
                    };
                    accumulate(field, 1, &mut out.overflowed);
                    accumulate(&mut out.assigned, 1, &mut out.overflowed);
                }
            }
        }
        out
    }
    /// Whether the record in physical slot `index` can still take a Synced
    /// update for target position `target`: routed there, Written, and
    /// neither Synced nor terminal. `false` is exactly what `poll` would
    /// skip at this moment, read from one cell word without the receipt
    /// lookup; most cells belong to other targets (TASK-023 S3 finding ⑪).
    pub(crate) fn awaits_sync(self, index: usize, target: usize) -> bool {
        let window = self.registry.limits.window;
        let Some(cell) = self
            .registry
            .producers
            .get(index / window)
            .and_then(|p| p.slots[index % window].cells.get(target))
        else {
            return false;
        };
        let word = cell.word.load(Ordering::Acquire);
        let confirmed = Boundaries(word & 31);
        word & (NOT_ROUTED | TERM_MASK) == 0
            && confirmed.contains(Boundary::Written)
            && !confirmed.contains(Boundary::Synced)
    }
    pub(crate) fn receipt_at(self, index: usize) -> Option<CaptureReceipt> {
        let producer = self
            .registry
            .producers
            .get(index / self.registry.limits.window)?;
        let seq = producer.slots[index % self.registry.limits.window]
            .sequence
            .load(Ordering::Acquire);
        (seq > 0 && seq <= producer.accepted.load(Ordering::Acquire)).then_some(CaptureReceipt {
            session_id: self.registry.session_id,
            producer_id: (index / self.registry.limits.window) as u32,
            accepted_seq: seq,
        })
    }
    pub(crate) fn enter(self, producer: u32) -> ActiveCall<'owner> {
        let state = &self.registry.producers[producer as usize];
        state.active.store(true, Ordering::SeqCst);
        ActiveCall {
            active: &state.active,
        }
    }
    pub(crate) fn reserve(
        self,
        producer: u32,
        sequence: u64,
        channel: u32,
        importance: Importance,
        revision: u64,
        updates: Option<&'owner crate::control::config_update::State>,
    ) -> Result<Reservation<'owner>, CompletionError> {
        let state = &self.registry.producers[producer as usize];
        let index = (sequence % self.registry.limits.window as u64) as usize;
        let slot = &state.slots[index];
        if importance == Importance::Normal {
            if state.important_reserve == self.registry.limits.window {
                return Err(CompletionError::Busy);
            }
            // Keep a contiguous runway after this sequence for Important input.
            // Counting free slots alone fails when a pending cyclic slot blocks
            // the next sequence despite holes elsewhere. Only this producer can
            // turn reusable slots into pending records; reports never erase a
            // confirmed boundary. Updating slots conservatively reject admission.
            let mut next = index;
            for _ in 0..state.important_reserve {
                next = if next + 1 == state.slots.len() {
                    0
                } else {
                    next + 1
                };
                if !matches!(
                    state.slots[next].phase.load(Ordering::Acquire),
                    EMPTY | TERMINAL
                ) {
                    return Err(CompletionError::Busy);
                }
            }
        }
        let phase = slot.phase.load(Ordering::Acquire);
        if phase != EMPTY && phase != TERMINAL {
            return Err(CompletionError::Busy);
        }
        let route = self
            .registry
            .routes
            .iter()
            .find(|r| r.channel_id == channel)
            .ok_or(CompletionError::InvalidConfig)?;
        slot.phase
            .compare_exchange(phase, INITIALIZING, Ordering::AcqRel, Ordering::Relaxed)
            .map_err(|_| CompletionError::Busy)?;
        Ok(Reservation {
            registry: self.registry,
            producer: state,
            producer_id: producer,
            slot,
            sequence,
            route,
            revision,
            updates,
            previous: phase,
            committed: false,
        })
    }
    /// Number of producer calls that may still publish pre-close admission/loan state.
    pub fn active_calls(self) -> usize {
        self.registry
            .producers
            .iter()
            .filter(|p| p.active.load(Ordering::SeqCst))
            .count()
    }
    pub(crate) fn marker(
        self,
        receipt: CaptureReceipt,
        target: u32,
    ) -> Option<AbandonMarker<'owner>> {
        self.registry.locate(receipt).ok()?;
        let index = self.registry.target(target).ok()?;
        Some(AbandonMarker {
            registry: self.registry,
            sequence: receipt.accepted_seq,
            producer: receipt.producer_id,
            target: u16::try_from(index).ok()?,
            armed: true,
        })
    }
    pub(crate) fn apply_dispatch(self, event: DispatchEvent) -> Result<(), CompletionError> {
        if event.revision
            != self
                .registry
                .locate(event.receipt)?
                .revision
                .load(Ordering::Acquire)
        {
            return Err(CompletionError::InvalidReceipt);
        }
        match event.outcome {
            DispatchOutcome::Enqueued => self.report(
                event.receipt,
                event.target_id.ok_or(CompletionError::InvalidTarget)?,
                Update::Confirmed(Boundary::Enqueued),
            ),
            DispatchOutcome::Dropped(reason) => self.report(
                event.receipt,
                event.target_id.ok_or(CompletionError::InvalidTarget)?,
                Update::Dropped(reason),
            ),
            DispatchOutcome::Unrouted => {
                let slot = self.registry.locate(event.receipt)?;
                slot.phase
                    .compare_exchange(TERMINAL, UPDATING, Ordering::AcqRel, Ordering::Relaxed)
                    .map_err(|_| CompletionError::Busy)?;
                let _publication = Publication {
                    registry: self.registry,
                    slot,
                };
                if slot.sequence.load(Ordering::Acquire) != event.receipt.accepted_seq {
                    return Err(CompletionError::Expired);
                }
                if slot
                    .cells
                    .iter()
                    .any(|cell| cell.word.load(Ordering::Acquire) & NOT_ROUTED == 0)
                {
                    return Err(CompletionError::InvalidTransition);
                }
                // No adapter exists to acknowledge an empty route. Dispatch is
                // its management-side terminal observation and retention origin.
                Ok(())
            }
        }
    }
}
pub(crate) struct ActiveCall<'a> {
    active: &'a AtomicBool,
}
impl Drop for ActiveCall<'_> {
    fn drop(&mut self) {
        self.active.store(false, Ordering::SeqCst);
    }
}
pub(crate) struct Reservation<'a> {
    registry: &'a Registry,
    producer: &'a ProducerState,
    producer_id: u32,
    slot: &'a RecordSlot,
    sequence: u64,
    route: &'a RoutePlan,
    revision: u64,
    updates: Option<&'a crate::control::config_update::State>,
    previous: u8,
    committed: bool,
}
impl Reservation<'_> {
    pub(crate) fn commit(mut self) {
        if self.previous == TERMINAL {
            for (i, cell) in self.slot.cells.iter().enumerate() {
                let value = cell.word.load(Ordering::Acquire);
                if value & NOT_ROUTED != 0 {
                    continue;
                }
                let outcome = classify(value, self.registry.targets[i].completion).outcome;
                let index = match outcome {
                    QueryOutcome::Complete => 0,
                    QueryOutcome::Failed(_) => 1,
                    QueryOutcome::Dropped(_) => 2,
                    _ => 3,
                };
                self.producer.retired[i].counts[index].fetch_add(1, Ordering::Relaxed);
            }
            self.producer
                .retired_records
                .fetch_add(1, Ordering::Relaxed);
        }
        for cell in &self.slot.cells {
            cell.word.store(NOT_ROUTED, Ordering::Release);
        }
        let mut selected = 0;
        for (i, target) in self.registry.targets.iter().enumerate() {
            if self.updates.map_or(self.route.indices.contains(&i), |u| {
                u.selected(self.revision, self.route.channel_id, target.id)
            }) {
                self.slot.cells[i].word.store(0, Ordering::Release);
                selected += 1;
            }
        }
        self.slot
            .channel_index
            .store(self.route.channel_index, Ordering::Relaxed);
        if selected == 0 {
            if let Some(bank) = self.registry.loss.as_deref() {
                bank.ingress(
                    self.producer_id,
                    Some(self.route.channel_index),
                    crate::loss::Cause::NoRoute,
                );
            }
        }
        self.slot.revision.store(self.revision, Ordering::Relaxed);
        self.slot.terminal_at.store(u64::MAX, Ordering::Relaxed);
        self.slot.sequence.store(self.sequence, Ordering::Release);
        self.slot.phase.store(
            if selected == 0 { TERMINAL } else { ACTIVE },
            Ordering::Release,
        );
        self.producer
            .accepted
            .store(self.sequence, Ordering::Release);
        self.committed = true;
    }
}
impl Drop for Reservation<'_> {
    fn drop(&mut self) {
        if !self.committed {
            self.slot.phase.store(self.previous, Ordering::Release);
        }
    }
}
/// Kept as small as the cell reference it replaced: every `Delivery`
/// carries one.
pub(crate) struct AbandonMarker<'a> {
    registry: &'a Registry,
    sequence: u64,
    producer: u32,
    target: u16,
    armed: bool,
}
impl AbandonMarker<'_> {
    pub(crate) fn disarm(&mut self) {
        self.armed = false;
    }
}
impl Drop for AbandonMarker<'_> {
    fn drop(&mut self) {
        if self.armed {
            let registry = self.registry;
            let slot = &registry.producers[self.producer as usize].slots
                [(self.sequence % registry.limits.window as u64) as usize];
            let cell = &slot.cells[usize::from(self.target)];
            cell.abandoned.fetch_max(self.sequence, Ordering::Release);
            // Every guard drops armed after its report; only a record still
            // unconfirmed here is abandoned and needs a sweep (`effective`).
            // Counted after the cell, so a sweep started for it sees the mark.
            let word = cell.word.load(Ordering::Acquire);
            if slot.sequence.load(Ordering::Acquire) == self.sequence
                && word & (NOT_ROUTED | TERM_MASK) == 0
                && !registry.reported(word, usize::from(self.target))
            {
                registry.abandons.fetch_add(1, Ordering::Release);
            }
        }
    }
}
fn terminal(update: Update) -> u32 {
    (match update {
        Update::Confirmed(_) => 0,
        Update::Failed(FailureReason::IoFailure) => 1,
        Update::Failed(FailureReason::SyncFailure) => 2,
        Update::Failed(FailureReason::AdapterFailure) => 3,
        Update::Dropped(DropReason::Capacity) => 4,
        Update::Dropped(DropReason::RetryExhausted) => 5,
        Update::Dropped(DropReason::PendingFull) => 6,
        Update::Dropped(DropReason::Expired) => 7,
        Update::Dropped(DropReason::Cancelled) => 8,
        Update::Unknown => 9,
    }) << TERM_SHIFT
}
fn classify(word: u32, boundary: Boundary) -> RecordStatus {
    let confirmed = Boundaries(word & 31);
    let term = (word & TERM_MASK) >> TERM_SHIFT;
    let outcome = if word & NOT_ROUTED != 0 {
        QueryOutcome::NotRouted
    } else if confirmed.contains(boundary)
        // Reaching only Enqueued and then being dropped is a loss, not a
        // completion, whatever the contract (REV-071 RV-185).
        && !(boundary == Boundary::Enqueued && (4..=8).contains(&term))
    {
        QueryOutcome::Complete
    } else {
        match term {
            1 => QueryOutcome::Failed(FailureReason::IoFailure),
            2 => QueryOutcome::Failed(FailureReason::SyncFailure),
            3 => QueryOutcome::Failed(FailureReason::AdapterFailure),
            4 => QueryOutcome::Dropped(DropReason::Capacity),
            5 => QueryOutcome::Dropped(DropReason::RetryExhausted),
            6 => QueryOutcome::Dropped(DropReason::PendingFull),
            7 => QueryOutcome::Dropped(DropReason::Expired),
            8 => QueryOutcome::Dropped(DropReason::Cancelled),
            9 => QueryOutcome::Unknown,
            _ => QueryOutcome::Pending,
        }
    };
    RecordStatus { confirmed, outcome }
}

struct Publication<'a> {
    registry: &'a Registry,
    slot: &'a RecordSlot,
}
impl Drop for Publication<'_> {
    fn drop(&mut self) {
        let phase = if self.registry.settled(self.slot) {
            self.registry.stamp(self.slot);
            TERMINAL
        } else {
            ACTIVE
        };
        self.slot.phase.store(phase, Ordering::Release);
    }
}
impl Registry {
    fn count_loss(&self, producer: usize, slot: &RecordSlot, target: usize, outcome: u32) {
        use crate::loss::Cause;
        let cause = match outcome >> TERM_SHIFT {
            1 => Cause::Io,
            2 => Cause::Sync,
            3 => Cause::Adapter,
            4 => Cause::Capacity,
            5 => Cause::RetryExhausted,
            6 => Cause::PendingFull,
            7 => Cause::Expired,
            8 => Cause::Cancelled,
            9 => Cause::Unconfirmed,
            _ => return,
        };
        if let Some(bank) = self.loss.as_deref() {
            bank.output(
                producer,
                slot.channel_index.load(Ordering::Relaxed),
                target,
                cause,
            );
        }
    }
    fn stamp(&self, slot: &RecordSlot) {
        let elapsed = self
            .origin
            .elapsed()
            .as_nanos()
            .min(u128::from(u64::MAX - 1)) as u64;
        let _ = slot.terminal_at.compare_exchange(
            u64::MAX,
            elapsed,
            Ordering::Release,
            Ordering::Relaxed,
        );
    }
    fn expired(&self, stamp: u64, now: Instant) -> bool {
        stamp != u64::MAX
            && now
                .checked_duration_since(self.origin)
                .is_some_and(|elapsed| {
                    elapsed.as_nanos() >= u128::from(stamp) + u128::from(self.retention)
                })
    }
}
fn accumulate(target: &mut u64, value: u64, overflow: &mut bool) {
    if let Some(sum) = target.checked_add(value) {
        *target = sum;
    } else {
        *target = u64::MAX;
        *overflow = true;
    }
}

/// Releases the reconcile claim, also on unwind.
struct SweepOwner<'a>(&'a AtomicBool);
impl Drop for SweepOwner<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
thread_local! {
    static TOTALS_SCANS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// Runs inside `reconcile` just before it stores the sweep remainder, so a
    /// test can pause one reconciler there (REV-053 RV-131).
    pub(crate) static BEFORE_SWEEP_STORE: std::cell::RefCell<Option<Box<dyn FnMut()>>> =
        const { std::cell::RefCell::new(None) };
}
/// Full ledger counts started on this thread (test-only probe, T5 ⑫).
#[cfg(test)]
pub(crate) fn totals_scans() -> usize {
    TOTALS_SCANS.with(std::cell::Cell::get)
}
