//! Host-supplied power alarm events; no hardware, OS signals, or workers are installed.
use std::time::{Duration, Instant};

/// One alarm in the coordinator's monotonic clock domain.
/// The source supplies the original generation time and absolute cutoff. A host
/// without a measured signal timestamp must conservatively subtract its verified
/// delivery-delay bound before constructing this event; it cannot invent time.
#[derive(Clone, Copy, Debug)]
pub struct PowerAlarm {
    /// Nonzero diagnostic event identity; duplicates never extend the deadline.
    pub event_id: u64,
    /// Original signal generation time, not logger receipt time.
    pub generated_at: Instant,
    /// Absolute simulated/device poweroff deadline in the same clock domain.
    pub poweroff_deadline: Instant,
}
impl PowerAlarm {
    /// Construct a finite alarm; seconds or minutes use the same absolute deadline.
    pub fn new(event_id: u64, generated_at: Instant, hold_up: Duration) -> Option<Self> {
        if event_id == 0 || hold_up.is_zero() {
            return None;
        }
        Some(Self {
            event_id,
            generated_at,
            poweroff_deadline: generated_at.checked_add(hold_up)?,
        })
    }
}
/// Bounded mailbox publication outcome; no IO is performed by notification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PowerNotice {
    /// The default disabled configuration neither reserves nor transitions.
    Disabled,
    /// First accepted deadline was posted and new capture admission closed.
    Posted,
    /// A later notification shortened the deadline.
    Tightened,
    /// An equal/later notification could not extend the deadline.
    Unchanged,
}
