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
        let mut reserved = size_of::<Self>();
        add_array(&mut reserved, producer_count, size_of::<ProducerState>())?;
        let slots = producer_count
            .checked_mul(config.window_records)
            .ok_or_else(memory_error)?;
        add_array(&mut reserved, slots, size_of::<RecordSlot>())?;
        let cells = slots
            .checked_mul(config.targets.len())
            .ok_or_else(memory_error)?;
        add_array(&mut reserved, cells, size_of::<TargetCell>())?;
        add_array(
            &mut reserved,
            producer_count
                .checked_mul(config.targets.len())
                .ok_or_else(memory_error)?,
            size_of::<Retired>(),
        )?;
        add_array(
            &mut reserved,
            config.targets.len(),
            size_of::<TargetContract>(),
        )?;
        add_array(&mut reserved, routing.routes.len(), size_of::<RoutePlan>())?;
        for route in &routing.routes {
            add_array(&mut reserved, route.targets.len(), size_of::<usize>())?;
        }
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
            && value & 0b11110 == 0
            && cell.abandoned.load(Ordering::Acquire) == sequence
        {
            value | (9 << TERM_SHIFT)
        } else {
            value
        }
    }
    fn settled(&self, slot: &RecordSlot) -> bool {
        slot.cells.iter().zip(&self.targets).all(|(cell, target)| {
            let word = cell.word.load(Ordering::Acquire);
            word & NOT_ROUTED != 0
                || word & TERM_MASK != 0
                || Boundaries(word & 31).contains(target.completion)
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
        Ok(())
    }
    /// Reconcile at most `limit` physical slots after unreported output release.
    /// This never scans an unbounded sequence history or releases payload memory.
    pub fn reconcile(self, limit: usize) -> usize {
        let total = self.registry.producers.len() * self.registry.limits.window;
        let mut terminalized = 0;
        for _ in 0..limit.min(total) {
            let index = self.registry.cursor.fetch_add(1, Ordering::Relaxed) % total;
            let slot = &self.registry.producers[index / self.registry.limits.window].slots
                [index % self.registry.limits.window];
            if slot
                .phase
                .compare_exchange(ACTIVE, UPDATING, Ordering::AcqRel, Ordering::Relaxed)
                .is_err()
            {
                continue;
            }
            let sequence = slot.sequence.load(Ordering::Acquire);
            for i in 0..self.registry.targets.len() {
                let effective = self.registry.effective(slot, i, sequence);
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
        terminalized
    }
    /// Aggregate retired terminal counts and current windows. Concurrent samples can
    /// include Busy; shutdown uses this only after closing and resource quiescence.
    pub fn totals(self, required_only: bool) -> CompletionTotals {
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
                if required_only && !self.registry.targets[i].required {
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
                    if required_only && !target.required {
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
            slot,
            sequence,
            route,
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
        let slot = self.registry.locate(receipt).ok()?;
        let index = self.registry.target(target).ok()?;
        Some(AbandonMarker {
            cell: &slot.cells[index].abandoned,
            sequence: receipt.accepted_seq,
            armed: true,
        })
    }
    pub(crate) fn apply_dispatch(self, event: DispatchEvent) -> Result<(), CompletionError> {
        if event.revision != self.registry.revision {
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
    slot: &'a RecordSlot,
    sequence: u64,
    route: &'a RoutePlan,
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
        for &i in &self.route.indices {
            self.slot.cells[i].word.store(0, Ordering::Release);
        }
        self.slot.terminal_at.store(u64::MAX, Ordering::Relaxed);
        self.slot.sequence.store(self.sequence, Ordering::Release);
        self.slot.phase.store(
            if self.route.indices.is_empty() {
                TERMINAL
            } else {
                ACTIVE
            },
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
pub(crate) struct AbandonMarker<'a> {
    cell: &'a AtomicU64,
    sequence: u64,
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
            self.cell.fetch_max(self.sequence, Ordering::Release);
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
    let outcome = if word & NOT_ROUTED != 0 {
        QueryOutcome::NotRouted
    } else if confirmed.contains(boundary) {
        QueryOutcome::Complete
    } else {
        match (word & TERM_MASK) >> TERM_SHIFT {
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

#[cfg(test)]
mod tests;
