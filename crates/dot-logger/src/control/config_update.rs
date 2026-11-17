//! Explicit publication of bounded, two-generation capture policies.
//!
//! Only management uses the mutex. Release publication precedes producer Acquire;
//! a producer acknowledges after its last old admission. A bank is reused only
//! after every producer has acknowledged, every ingress head has been consumed,
//! completion is terminal and all old payload guards have actually been dropped.
use crate::{
    capture::{ChannelConfig, Level},
    completion::Registry,
    config::{ConfigError, MemoryBudget, Resource},
    routing::{Route, RoutingConfig},
};
use std::sync::{
    Mutex, MutexGuard,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};

/// One channel's explicitly requested level filter and deterministic sampling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChannelPatch {
    /// Existing registered channel; schema and importance mapping stay fixed.
    pub channel_id: u32,
    /// Minimum diagnostic severity, independent of retention importance.
    pub minimum_level: Level,
    /// Keep the first and every Nth eligible Normal record; positive. Important
    /// records bypass sampling. Retried submissions count as separate attempts.
    pub sample_every: u32,
}
/// Partial runtime patch. New channels/targets/pools/workers require restart.
#[derive(Clone, Debug, Default)]
pub struct ConfigPatch {
    /// At most one replacement policy per registered channel.
    pub channels: Vec<ChannelPatch>,
    /// At most one replacement route per channel, using existing target IDs.
    pub routes: Vec<Route>,
    /// A changed memory plan is diagnosed but never applied to live storage.
    pub memory: Option<MemoryBudget>,
}
/// Dry-run differences and the already reserved two-generation memory envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Validation {
    /// Channel policy entries whose effective values would change.
    pub changed_channels: usize,
    /// Channel target sets whose effective values would change.
    pub changed_routes: usize,
    /// Any stricter level, sampling omission or removed target loses information.
    pub information_loss: bool,
    /// Startup memory change cannot be applied without stopping the owner.
    pub restart_required: bool,
    /// Fixed metadata bytes charged to control at explicit enable.
    pub reserved_bytes: usize,
}
/// Stable management error; no change is published on failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigUpdateError {
    /// The base revision is not the latest posted revision.
    Conflict,
    /// Idle producer, queued record, unfinished output or payload holds old plan.
    RevisionBusy,
    /// Invalid/duplicate channel, target, zero interval or empty effective patch.
    InvalidPatch,
    /// Caller destination cannot hold the current route; nothing was written.
    BufferTooSmall,
    /// Live owner has no compatible completion/routing registration.
    Untracked,
    /// The requested field requires owner restart.
    RestartRequired,
    /// Revision must never wrap.
    RevisionExhausted,
    /// Concrete metadata exceeds the explicit control reservation.
    Config(ConfigError),
    /// Poisoned management state cannot be treated as successful publication.
    Unavailable,
    /// Ingress is closed; no new setting may be posted.
    Closing,
}
impl std::fmt::Display for ConfigUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "config update: {self:?}")
    }
}
impl std::error::Error for ConfigUpdateError {}
struct Policy {
    level: AtomicUsize,
    every: AtomicUsize,
}
struct Bank {
    revision: AtomicU64,
    policies: Box<[Policy]>,
    routes: Box<[AtomicUsize]>,
    guards: AtomicUsize,
}
struct Progress {
    applied: AtomicU64,
    cutoff: AtomicU64,
    consumed: AtomicU64,
}
pub(crate) struct Management {
    pub(crate) slot: usize,
    pub(crate) revision: u64,
    previous: Option<u64>,
}
pub(crate) struct State {
    banks: [Bank; 2],
    posted: AtomicUsize,
    progress: Box<[Progress]>,
    channels: Box<[u32]>,
    targets: Box<[u32]>,
    memory: MemoryBudget,
    management: Mutex<Management>,
    reserved: usize,
}
/// Borrowed management control. Publication never claims simultaneous application.
#[derive(Clone, Copy)]
pub struct ConfigUpdates<'a> {
    pub(crate) state: &'a State,
    pub(crate) registry: &'a Registry,
    pub(crate) closing: &'a std::sync::atomic::AtomicBool,
}
impl State {
    pub(crate) fn new(
        channels: &[ChannelConfig],
        producers: usize,
        routing: &RoutingConfig,
        memory: &MemoryBudget,
        revision: u64,
        available: usize,
    ) -> Result<Self, ConfigUpdateError> {
        let overflow = || ConfigUpdateError::Config(ConfigError::ArithmeticOverflow);
        let route_count = channels
            .len()
            .checked_mul(routing.targets.len())
            .ok_or_else(overflow)?;
        let bytes = Self::required(channels.len(), producers, routing.targets.len())
            .ok_or_else(overflow)?;
        if bytes > available {
            return Err(ConfigUpdateError::Config(ConfigError::BudgetExceeded {
                resource: Resource::Control,
                required: bytes,
                available,
            }));
        }
        let bank = || Bank {
            revision: AtomicU64::new(revision),
            policies: channels
                .iter()
                .map(|c| Policy {
                    level: AtomicUsize::new(c.minimum_level as usize),
                    every: AtomicUsize::new(1),
                })
                .collect(),
            routes: {
                let mut routes = Vec::with_capacity(route_count);
                for c in channels {
                    for t in &routing.targets {
                        routes.push(AtomicUsize::new(usize::from(routing.routes.iter().any(
                            |r| r.channel_id == c.channel_id && r.targets.contains(&t.id),
                        ))));
                    }
                }
                routes.into_boxed_slice()
            },
            guards: AtomicUsize::new(0),
        };
        Ok(Self {
            banks: [bank(), bank()],
            posted: AtomicUsize::new(0),
            progress: (0..producers)
                .map(|_| Progress {
                    applied: AtomicU64::new(revision),
                    cutoff: AtomicU64::new(0),
                    consumed: AtomicU64::new(0),
                })
                .collect(),
            channels: channels.iter().map(|c| c.channel_id).collect(),
            targets: routing.targets.iter().map(|t| t.id).collect(),
            memory: memory.clone(),
            management: Mutex::new(Management {
                slot: 0,
                revision,
                previous: None,
            }),
            reserved: bytes,
        })
    }
    /// Control bytes `new` charges for this shape: both banks, progress and
    /// the registered ID lists. `None` on overflow or a layout a boxed slice
    /// cannot represent. Shared with the facade sizing so the derived budget
    /// and the actual reservation cannot diverge.
    pub(crate) fn required(channels: usize, producers: usize, targets: usize) -> Option<usize> {
        let mut bytes = size_of::<Self>();
        for (n, size) in [
            (channels, 2 * size_of::<Policy>() + size_of::<u32>()),
            (channels.checked_mul(targets)?, 2 * size_of::<AtomicUsize>()),
            (targets, size_of::<u32>()),
            (producers, size_of::<Progress>()),
        ] {
            bytes =
                bytes.checked_add(n.checked_mul(size).filter(|n| *n <= isize::MAX as usize)?)?;
        }
        Some(bytes)
    }
    pub(crate) fn reserved(&self) -> usize {
        self.reserved
    }
    pub(crate) fn applied(&self, producer: usize) -> u64 {
        self.progress[producer].applied.load(Ordering::Acquire)
    }
    pub(crate) fn known_revision(&self, revision: u64) -> bool {
        self.management
            .lock()
            .is_ok_and(|m| revision == m.revision || m.previous == Some(revision))
    }
    pub(crate) fn boundary(&self, producer: u32, sequence: u64, local: u64) -> u64 {
        let slot = self.posted.load(Ordering::Acquire);
        let revision = self.banks[slot].revision.load(Ordering::Relaxed);
        if revision != local {
            let p = &self.progress[producer as usize];
            p.cutoff.store(sequence, Ordering::Relaxed);
            p.applied.store(revision, Ordering::Release);
        }
        revision
    }
    fn bank(&self, revision: u64) -> Option<&Bank> {
        // Callers hold an unconsumed ingress record, producer acknowledgement,
        // management mutex or guard; these prevent reuse of this revision bank.
        self.banks
            .iter()
            .find(|b| b.revision.load(Ordering::Acquire) == revision)
    }
    pub(crate) fn policy(&self, revision: u64, channel: u32) -> Option<(usize, u32)> {
        let c = self.channels.iter().position(|id| *id == channel)?;
        let p = &self.bank(revision)?.policies[c];
        Some((
            p.level.load(Ordering::Relaxed),
            p.every.load(Ordering::Relaxed) as u32,
        ))
    }
    pub(crate) fn selected(&self, revision: u64, channel: u32, target: u32) -> bool {
        let Some(c) = self.channels.iter().position(|id| *id == channel) else {
            return false;
        };
        let Some(t) = self.targets.iter().position(|id| *id == target) else {
            return false;
        };
        self.bank(revision)
            .is_some_and(|b| b.routes[c * self.targets.len() + t].load(Ordering::Relaxed) != 0)
    }
    pub(crate) fn contains(&self, revision: u64) -> bool {
        self.bank(revision).is_some()
    }
    pub(crate) fn consumed(&self, producer: u32, sequence: u64) {
        self.progress[producer as usize]
            .consumed
            .store(sequence, Ordering::Release);
    }
    pub(crate) fn retain(&self, revision: u64) -> Option<PlanGuard<'_>> {
        let bank = self.bank(revision)?;
        bank.guards.fetch_add(1, Ordering::Relaxed);
        Some(PlanGuard { bank })
    }
    fn ready(&self, m: &Management, registry: &Registry) -> bool {
        let Some(old) = m.previous else { return true };
        self.progress.iter().all(|p| {
            p.applied.load(Ordering::Acquire) == m.revision
                && p.consumed.load(Ordering::Acquire) >= p.cutoff.load(Ordering::Acquire)
        }) && self.banks[1 - m.slot].guards.load(Ordering::Acquire) == 0
            && !registry.pending_revision(old)
    }
    pub(crate) fn observation(
        &self,
        registry: &Registry,
    ) -> Result<(MutexGuard<'_, Management>, bool), ConfigUpdateError> {
        let m = self
            .management
            .lock()
            .map_err(|_| ConfigUpdateError::Unavailable)?;
        let ready = self.ready(&m, registry);
        Ok((m, ready))
    }
}
pub(crate) struct PlanGuard<'a> {
    bank: &'a Bank,
}
impl Drop for PlanGuard<'_> {
    fn drop(&mut self) {
        self.bank.guards.fetch_sub(1, Ordering::Release);
    }
}
impl ConfigUpdates<'_> {
    /// Latest posted revision, independent of each producer's acknowledgement.
    pub fn posted_revision(self) -> Result<u64, ConfigUpdateError> {
        Ok(self
            .state
            .management
            .lock()
            .map_err(|_| ConfigUpdateError::Unavailable)?
            .revision)
    }
    /// Atomically observed revision at the producer's most recent call boundary.
    pub fn producer_revision(self, producer: u32) -> Option<u64> {
        self.state
            .progress
            .get(producer as usize)
            .map(|p| p.applied.load(Ordering::Acquire))
    }
    /// Read current channel policy at the requested posted base for a concrete
    /// before/after preview. A concurrent newer publication returns Conflict.
    pub fn channel_policy(
        self,
        base_revision: u64,
        channel_id: u32,
    ) -> Result<ChannelPatch, ConfigUpdateError> {
        let m = self
            .state
            .management
            .lock()
            .map_err(|_| ConfigUpdateError::Unavailable)?;
        if base_revision != m.revision {
            return Err(ConfigUpdateError::Conflict);
        }
        let index = self
            .state
            .channels
            .iter()
            .position(|id| *id == channel_id)
            .ok_or(ConfigUpdateError::InvalidPatch)?;
        let policy = &self.state.banks[m.slot].policies[index];
        let minimum_level = match policy.level.load(Ordering::Relaxed) {
            0 => Level::Trace,
            1 => Level::Debug,
            2 => Level::Info,
            3 => Level::Warn,
            4 => Level::Error,
            _ => Level::Fatal,
        };
        Ok(ChannelPatch {
            channel_id,
            minimum_level,
            sample_every: policy.every.load(Ordering::Relaxed) as u32,
        })
    }
    /// Copy the current route in registered target order. An undersized buffer
    /// changes nothing; this management operation never allocates a target list.
    pub fn route_targets(
        self,
        base_revision: u64,
        channel_id: u32,
        destination: &mut [u32],
    ) -> Result<usize, ConfigUpdateError> {
        let m = self
            .state
            .management
            .lock()
            .map_err(|_| ConfigUpdateError::Unavailable)?;
        if base_revision != m.revision {
            return Err(ConfigUpdateError::Conflict);
        }
        if !self.state.channels.contains(&channel_id) {
            return Err(ConfigUpdateError::InvalidPatch);
        }
        let selected = |id: &u32| self.state.selected(m.revision, channel_id, *id);
        let count = self.state.targets.iter().filter(|id| selected(id)).count();
        if destination.len() < count {
            return Err(ConfigUpdateError::BufferTooSmall);
        }
        for (index, id) in self
            .state
            .targets
            .iter()
            .filter(|id| selected(id))
            .enumerate()
        {
            destination[index] = *id;
        }
        Ok(count)
    }
    /// Validate only: does not post, run IO or change any producer's local policy.
    pub fn validate_config(
        self,
        base_revision: u64,
        patch: &ConfigPatch,
    ) -> Result<Validation, ConfigUpdateError> {
        let m = self
            .state
            .management
            .lock()
            .map_err(|_| ConfigUpdateError::Unavailable)?;
        self.validate(&m, base_revision, patch)
    }
    fn validate(
        self,
        m: &Management,
        base: u64,
        patch: &ConfigPatch,
    ) -> Result<Validation, ConfigUpdateError> {
        if base != m.revision {
            return Err(ConfigUpdateError::Conflict);
        }
        if patch.channels.len() > self.state.channels.len()
            || patch.routes.len() > self.state.channels.len()
        {
            return Err(ConfigUpdateError::InvalidPatch);
        }
        let mut result = Validation {
            changed_channels: 0,
            changed_routes: 0,
            information_loss: false,
            restart_required: patch
                .memory
                .as_ref()
                .is_some_and(|memory| memory != &self.state.memory),
            reserved_bytes: self.state.reserved,
        };
        for (i, c) in patch.channels.iter().enumerate() {
            if c.sample_every == 0
                || patch.channels[..i]
                    .iter()
                    .any(|p| p.channel_id == c.channel_id)
            {
                return Err(ConfigUpdateError::InvalidPatch);
            }
            let (level, every) = self
                .state
                .policy(m.revision, c.channel_id)
                .ok_or(ConfigUpdateError::InvalidPatch)?;
            if (level, every) != (c.minimum_level as usize, c.sample_every) {
                result.changed_channels += 1;
            }
            result.information_loss |=
                c.minimum_level as usize > level || c.sample_every > 1 && c.sample_every != every;
        }
        for (i, r) in patch.routes.iter().enumerate() {
            if !self.state.channels.contains(&r.channel_id)
                || patch.routes[..i]
                    .iter()
                    .any(|p| p.channel_id == r.channel_id)
                || r.targets.len() > self.state.targets.len()
                || r.targets.len().max(1) > self.registry.limits.feedback
                || r.targets
                    .iter()
                    .enumerate()
                    .any(|(j, t)| !self.state.targets.contains(t) || r.targets[..j].contains(t))
            {
                return Err(ConfigUpdateError::InvalidPatch);
            }
            let mut changed = false;
            for t in &self.state.targets {
                let old = self.state.selected(m.revision, r.channel_id, *t);
                let new = r.targets.contains(t);
                changed |= old != new;
                result.information_loss |= old && !new;
            }
            result.changed_routes += usize::from(changed);
        }
        if result.changed_channels == 0 && result.changed_routes == 0 && !result.restart_required {
            return Err(ConfigUpdateError::InvalidPatch);
        }
        Ok(result)
    }
    /// Atomically post one validated generation. Conflict takes precedence over
    /// Busy. Idle producers must explicitly poll or enter capture before reuse.
    pub fn apply_config(
        self,
        base_revision: u64,
        patch: &ConfigPatch,
    ) -> Result<u64, ConfigUpdateError> {
        let mut m = self
            .state
            .management
            .lock()
            .map_err(|_| ConfigUpdateError::Unavailable)?;
        let validation = self.validate(&m, base_revision, patch)?;
        if self.closing.load(Ordering::SeqCst) {
            return Err(ConfigUpdateError::Closing);
        }
        if validation.restart_required {
            return Err(ConfigUpdateError::RestartRequired);
        }
        if !self.state.ready(&m, self.registry) {
            return Err(ConfigUpdateError::RevisionBusy);
        }
        let revision = m
            .revision
            .checked_add(1)
            .ok_or(ConfigUpdateError::RevisionExhausted)?;
        let next = 1 - m.slot;
        let source = &self.state.banks[m.slot];
        let destination = &self.state.banks[next];
        for (old, new) in source.policies.iter().zip(&destination.policies) {
            new.level
                .store(old.level.load(Ordering::Relaxed), Ordering::Relaxed);
            new.every
                .store(old.every.load(Ordering::Relaxed), Ordering::Relaxed);
        }
        for (old, new) in source.routes.iter().zip(&destination.routes) {
            new.store(old.load(Ordering::Relaxed), Ordering::Relaxed);
        }
        for c in &patch.channels {
            if let Some(index) = self
                .state
                .channels
                .iter()
                .position(|id| *id == c.channel_id)
            {
                destination.policies[index]
                    .level
                    .store(c.minimum_level as usize, Ordering::Relaxed);
                destination.policies[index]
                    .every
                    .store(c.sample_every as usize, Ordering::Relaxed);
            }
        }
        for r in &patch.routes {
            if let Some(c) = self
                .state
                .channels
                .iter()
                .position(|id| *id == r.channel_id)
            {
                for (t, id) in self.state.targets.iter().enumerate() {
                    destination.routes[c * self.state.targets.len() + t]
                        .store(usize::from(r.targets.contains(id)), Ordering::Relaxed);
                }
            }
        }
        destination.revision.store(revision, Ordering::Release);
        m.previous = Some(m.revision);
        m.revision = revision;
        m.slot = next;
        self.state.posted.store(next, Ordering::Release);
        Ok(revision)
    }
}
