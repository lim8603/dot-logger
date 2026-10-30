use super::{Lease, RetentionError, WriterBudget};
use std::sync::Arc;

/// Explicit retention policy. Continuous mode never deletes implicitly.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetentionMode {
    /// Stop at the configured size/age/catalog limit.
    Continuous,
    /// Delete the oldest unprotected closed files to make room.
    Circular,
    /// Circular files with explicit pre/post event protection.
    Event,
}
/// Management-path limits; all capacities and times are caller supplied.
#[derive(Clone, Copy, Debug)]
pub struct RetentionConfig {
    /// Continuous, circular, or event policy.
    pub mode: RetentionMode,
    /// Total file bytes, including a full active-file reservation.
    pub disk_bytes: u64,
    /// Filesystem free bytes that must remain outside a full active-file reserve.
    pub disk_reserve_bytes: u64,
    /// Closed-file age limit in host monotonic nanoseconds.
    pub max_age_ns: u64,
    /// Conservative encoded-file byte ceiling, including summary/footer.
    pub max_segment_bytes: u64,
    /// Rotate on the first append at or after this segment age.
    pub max_segment_ns: u64,
    /// Conservative live MCAP message/chunk/auxiliary index bytes per segment.
    pub max_index_bytes: usize,
    /// Unique protected bytes; an overlapping active file charges its full ceiling.
    pub protected_bytes: u64,
    /// Requested event history in session monotonic nanoseconds.
    pub pre_ns: u64,
    /// Requested event future in session monotonic nanoseconds.
    pub post_ns: u64,
    /// Maximum active plus closed file entries.
    pub max_segments: usize,
    /// Maximum simultaneously protected event windows.
    pub max_events: usize,
    /// Maximum outstanding single-file read pins.
    pub max_pins: usize,
    /// Per-pin cumulative read byte limit.
    pub max_pin_bytes: u64,
    /// Per-pin maximum read lifetime; expiry denies reads, release still unpins.
    pub max_pin_ns: u64,
}
impl RetentionConfig {
    pub(super) fn validate(&self) -> Result<(), RetentionError> {
        if self.max_segments == 0
            || self.max_segment_bytes == 0
            || self.max_segment_ns == 0
            || self.max_index_bytes < 1088
            || self.max_age_ns == 0
            || self.disk_bytes < self.max_segment_bytes
            || self
                .disk_reserve_bytes
                .checked_add(self.max_segment_bytes)
                .is_none()
            || (self.mode == RetentionMode::Event
                && (self.max_events == 0 || self.protected_bytes == 0))
            || (self.max_pins > 0 && (self.max_pin_ns == 0 || self.max_pin_bytes == 0))
        {
            return Err(RetentionError::InvalidConfig);
        }
        Ok(())
    }
}
/// A catalog entry published only after file and directory synchronization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SegmentInfo {
    /// System-generated file sequence within the session/target.
    pub sequence: u64,
    /// Physical closed-file length including footer/summary.
    pub bytes: u64,
    /// Host monotonic open time, used for rotation/retention age.
    pub opened_ns: u64,
    /// Host monotonic publication time.
    pub closed_ns: u64,
    /// Stored message count.
    pub records: usize,
    /// Session monotonic record timestamp bounds; None for an empty file.
    pub mono_range: Option<(u64, u64)>,
}
/// Observed event window. Bounds describe retained record extents, not gap-free data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventReport {
    /// Monotonically allocated event identity.
    pub event_id: u64,
    /// Requested inclusive session monotonic interval.
    pub requested: (u64, u64),
    /// Actual retained record extents intersected with the request, if present.
    pub actual: Option<(u64, u64)>,
    /// The earliest retained record does not reach the requested history start.
    pub pre_truncated: bool,
    /// Host time has not yet reached the requested post-window end.
    pub post_pending: bool,
    /// Explicit expiry after which expire_events may release protection.
    pub expires_ns: u64,
}
/// Opaque single-file read permit; release explicitly after the actual read ends.
/// Expiry denies further reads but never silently drops an in-flight file pin.
#[derive(Debug)]
pub struct ReadPin {
    owner: Arc<()>,
    id: u64,
}
#[derive(Clone, Copy)]
struct Event {
    id: u64,
    start: u64,
    end: u64,
    expires: u64,
}
#[derive(Clone, Copy)]
struct Pin {
    id: u64,
    sequence: u64,
    deadline: u64,
    remaining: u64,
}

pub(super) struct Catalog {
    pub config: RetentionConfig,
    pub closed: Vec<SegmentInfo>,
    events: Vec<Event>,
    pins: Vec<Pin>,
    next_event: u64,
    next_pin: u64,
    identity: Arc<()>,
    _lease: Lease,
}
fn intersects(a: Option<(u64, u64)>, start: u64, end: u64) -> bool {
    a.is_some_and(|(lo, hi)| lo <= end && start <= hi)
}
impl Catalog {
    pub fn new(
        budget: &WriterBudget,
        config: RetentionConfig,
        extra: usize,
    ) -> Result<Self, RetentionError> {
        config.validate()?;
        let mut bytes = extra
            .checked_add(1024)
            .ok_or(RetentionError::InvalidConfig)?;
        for (count, width) in [
            (config.max_segments, std::mem::size_of::<SegmentInfo>()),
            (config.max_events, std::mem::size_of::<Event>()),
            (config.max_pins, std::mem::size_of::<Pin>()),
        ] {
            bytes = bytes
                .checked_add(
                    count
                        .checked_mul(width)
                        .ok_or(RetentionError::InvalidConfig)?,
                )
                .ok_or(RetentionError::InvalidConfig)?;
        }
        let lease = budget.reserve(bytes)?;
        Ok(Self {
            config,
            closed: Vec::with_capacity(config.max_segments),
            events: Vec::with_capacity(config.max_events),
            pins: Vec::with_capacity(config.max_pins),
            next_event: 1,
            next_pin: 1,
            identity: Arc::new(()),
            _lease: lease,
        })
    }
    pub fn protected(&self, range: Option<(u64, u64)>) -> bool {
        self.events
            .iter()
            .any(|e| intersects(range, e.start, e.end))
    }
    pub fn protected_bytes(&self, active: Option<(u64, u64)>) -> u64 {
        self.closed
            .iter()
            .filter(|s| self.protected(s.mono_range))
            .map(|s| s.bytes)
            .sum::<u64>()
            + if self.protected(active) {
                self.config.max_segment_bytes
            } else {
                0
            }
    }
    pub fn check_protection(&self, active: Option<(u64, u64)>) -> Result<(), RetentionError> {
        if self.protected_bytes(active) > self.config.protected_bytes {
            Err(RetentionError::EventQuota)
        } else {
            Ok(())
        }
    }
    pub fn pinned(&self, seq: u64) -> bool {
        self.pins.iter().any(|p| p.sequence == seq)
    }
    pub fn oldest_deletable(&self) -> Option<usize> {
        self.closed
            .iter()
            .position(|s| !self.pinned(s.sequence) && !self.protected(s.mono_range))
    }
    pub fn add_event(
        &mut self,
        at: u64,
        expires: u64,
        active: Option<(u64, u64)>,
        now: u64,
    ) -> Result<EventReport, RetentionError> {
        if self.config.mode != RetentionMode::Event {
            return Err(RetentionError::InvalidConfig);
        }
        let end = at
            .checked_add(self.config.post_ns)
            .ok_or(RetentionError::InvalidConfig)?;
        if at > now || expires < end || expires <= now {
            return Err(RetentionError::InvalidConfig);
        }
        if self.events.len() == self.config.max_events {
            return Err(RetentionError::EventQuota);
        }
        let id = self.next_event;
        let next = id.checked_add(1).ok_or(RetentionError::EventQuota)?;
        self.events.push(Event {
            id,
            start: at.saturating_sub(self.config.pre_ns),
            end,
            expires,
        });
        if let Err(e) = self.check_protection(active) {
            self.events.pop();
            return Err(e);
        }
        self.next_event = next;
        self.event_report(id, active, now)
    }
    pub fn event_report(
        &self,
        id: u64,
        active: Option<(u64, u64)>,
        now: u64,
    ) -> Result<EventReport, RetentionError> {
        let event = self
            .events
            .iter()
            .find(|e| e.id == id)
            .ok_or(RetentionError::UnknownToken)?;
        let mut actual: Option<(u64, u64)> = None;
        for range in self
            .closed
            .iter()
            .map(|s| s.mono_range)
            .chain(std::iter::once(active))
            .flatten()
        {
            if intersects(Some(range), event.start, event.end) {
                let lo = range.0.max(event.start);
                let hi = range.1.min(event.end);
                actual = Some(actual.map_or((lo, hi), |(a, b)| (a.min(lo), b.max(hi))));
            }
        }
        Ok(EventReport {
            event_id: id,
            requested: (event.start, event.end),
            actual,
            pre_truncated: actual.is_none_or(|(lo, _)| lo > event.start),
            post_pending: now < event.end,
            expires_ns: event.expires,
        })
    }
    pub fn release_event(&mut self, id: u64) -> Result<(), RetentionError> {
        let index = self
            .events
            .iter()
            .position(|e| e.id == id)
            .ok_or(RetentionError::UnknownToken)?;
        self.events.remove(index);
        Ok(())
    }
    pub fn expire_events(&mut self, now: u64) -> usize {
        let before = self.events.len();
        self.events.retain(|e| e.expires > now);
        before - self.events.len()
    }
    pub fn pin(
        &mut self,
        seq: u64,
        bytes: u64,
        now: u64,
        deadline: u64,
    ) -> Result<ReadPin, RetentionError> {
        if !self.closed.iter().any(|s| s.sequence == seq) {
            return Err(RetentionError::UnknownToken);
        }
        if self.pins.len() == self.config.max_pins
            || bytes == 0
            || bytes > self.config.max_pin_bytes
            || deadline <= now
            || deadline - now > self.config.max_pin_ns
        {
            return Err(RetentionError::PinLimit);
        }
        let id = self.next_pin;
        self.next_pin = id.checked_add(1).ok_or(RetentionError::PinLimit)?;
        self.pins.push(Pin {
            id,
            sequence: seq,
            deadline,
            remaining: bytes,
        });
        Ok(ReadPin {
            owner: Arc::clone(&self.identity),
            id,
        })
    }
    pub fn consume_pin(
        &mut self,
        token: &ReadPin,
        bytes: u64,
        now: u64,
    ) -> Result<(), RetentionError> {
        if !Arc::ptr_eq(&token.owner, &self.identity) {
            return Err(RetentionError::UnknownToken);
        }
        let pin = self
            .pins
            .iter_mut()
            .find(|p| p.id == token.id)
            .ok_or(RetentionError::UnknownToken)?;
        if now >= pin.deadline {
            return Err(RetentionError::Expired);
        }
        if bytes > pin.remaining {
            return Err(RetentionError::PinLimit);
        }
        pin.remaining -= bytes;
        Ok(())
    }
    pub fn release_pin(&mut self, token: ReadPin) -> Result<(), RetentionError> {
        if !Arc::ptr_eq(&token.owner, &self.identity) {
            return Err(RetentionError::UnknownToken);
        }
        let index = self
            .pins
            .iter()
            .position(|p| p.id == token.id)
            .ok_or(RetentionError::UnknownToken)?;
        self.pins.remove(index);
        Ok(())
    }
}
