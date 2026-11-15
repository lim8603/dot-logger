//! Event-window protection of ring outputs (API §22.3).
//!
//! `Logger::mark_event` sends one [`EventWork`] per event output straight to
//! the IO thread that owns that output's `SegmentLog`; the host thread is not
//! involved, so dispatch never waits for a mark. Each IO thread keeps a
//! bounded list mapping the facade mark id to its own core event id, so one
//! [`EventMark`] can be released on every output.

use std::fmt;
use std::sync::mpsc::Sender;

use crate::storage::{RetentionError, SegmentLog, SegmentStore};

/// A protected event, returned by
/// [`Logger::mark_event`](super::Logger::mark_event).
///
/// One mark covers every event output: `outputs()` of them protected the
/// same window `[at_ns() - pre, at_ns() + post]` until
/// `at_ns() + post + hold` or [`Logger::release_event`](super::Logger::release_event).
/// A mark only belongs to the logger that issued it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventMark {
    session: u128,
    id: u64,
    at_ns: u64,
    outputs: usize,
}

impl EventMark {
    pub(crate) fn new(session: u128, id: u64, at_ns: u64, outputs: usize) -> Self {
        Self {
            session,
            id,
            at_ns,
            outputs,
        }
    }
    pub(crate) fn session(&self) -> u128 {
        self.session
    }
    /// Facade mark number, unique within its logger.
    pub fn id(&self) -> u64 {
        self.id
    }
    /// Event time in session monotonic nanoseconds, the clock of
    /// [`Writer::now_ns`](super::Writer::now_ns) and `try_log_at`.
    pub fn at_ns(&self) -> u64 {
        self.at_ns
    }
    /// Number of file outputs that protect the window.
    pub fn outputs(&self) -> usize {
        self.outputs
    }
}

/// Why [`Logger::mark_event`](super::Logger::mark_event) or
/// [`Logger::release_event`](super::Logger::release_event) did not succeed.
/// After any error of `mark_event` no output keeps the window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventError {
    /// No file output was declared with
    /// [`Retention::events`](super::Retention::events).
    NotConfigured,
    /// An output already holds `max_events` windows, or protecting this one
    /// would exceed its `protected_bytes`. Release a mark or wait for expiry.
    Quota,
    /// The default worker is stopping or has stopped; nothing was changed.
    Closing,
    /// An output could not protect the window (a file fault, or the answer
    /// did not arrive within `4 × sync_every + 1 s`), or the mark was issued
    /// by another logger.
    Failed,
}

impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::NotConfigured => "no file output protects event windows",
            Self::Quota => "event window limit reached",
            Self::Closing => "logger worker is closing",
            Self::Failed => "event window could not be protected",
        };
        f.write_str(text)
    }
}
impl std::error::Error for EventError {}

/// Window lengths of one event output, in session monotonic nanoseconds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct EventSpec {
    pub(crate) post_ns: u64,
    pub(crate) hold_ns: u64,
    pub(crate) max_events: usize,
}

/// Answer of one output.
pub(crate) type Reply = Sender<Result<(), EventError>>;

/// Request from the `Logger` handle to the IO thread of one event output.
pub(crate) enum EventWork {
    /// Protect the window of facade mark `id` at session time `at`.
    Mark { id: u64, at: u64, reply: Reply },
    /// Release facade mark `id`; an unknown (expired, refused) mark is Ok.
    Release { id: u64, reply: Reply },
}

/// One protected window of one output: facade mark → core event.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Held {
    mark: u64,
    event: u64,
    expires: u64,
}

/// Event state of one output, owned by its IO thread. `held` mirrors the
/// core catalog: an entry is added only when the core accepted the window and
/// removed with the same expiry rule, so it never exceeds `max_events`.
pub(crate) struct Marks {
    spec: EventSpec,
    pub(crate) work: std::sync::mpsc::Receiver<EventWork>,
    held: Vec<Held>,
}

impl Marks {
    pub(crate) fn new(spec: EventSpec, work: std::sync::mpsc::Receiver<EventWork>) -> Self {
        Self {
            spec,
            work,
            held: Vec::with_capacity(spec.max_events),
        }
    }

    /// Protect `[at - pre, at + post]` until `at + post + hold`. `now` is the
    /// IO thread's clock, never earlier than the log's last time.
    pub(crate) fn mark<S: SegmentStore>(
        &mut self,
        log: &mut SegmentLog<S>,
        id: u64,
        at: u64,
        now: u64,
    ) -> Result<(), (EventError, Option<RetentionError>)> {
        // The handle read the clock before sending; never ahead of `now`.
        let at = at.min(now);
        let expires = at
            .saturating_add(self.spec.post_ns)
            .saturating_add(self.spec.hold_ns);
        match log.protect_event(at, expires, now) {
            Ok(report) => {
                self.held.push(Held {
                    mark: id,
                    event: report.event_id,
                    expires,
                });
                Ok(())
            }
            Err(RetentionError::EventQuota) => Err((EventError::Quota, None)),
            Err(error) => Err((EventError::Failed, Some(error))),
        }
    }

    /// Release facade mark `id` if this output still holds it.
    pub(crate) fn release<S: SegmentStore>(&mut self, log: &mut SegmentLog<S>, id: u64) {
        if let Some(index) = self.held.iter().position(|h| h.mark == id) {
            let held = self.held.swap_remove(index);
            // Unknown only if the core expired it first; nothing remains.
            let _ = log.release_event(held.event);
        }
    }

    /// Expire windows whose hold passed, in the core and in `held`.
    pub(crate) fn expire<S: SegmentStore>(
        &mut self,
        log: &mut SegmentLog<S>,
        now: u64,
    ) -> Result<(), RetentionError> {
        if self.held.is_empty() {
            return Ok(());
        }
        log.expire_events(now)?;
        self.held.retain(|h| h.expires > now);
        Ok(())
    }
}

/// Heap of one output's `held` list, counted in the worker partition.
pub(crate) fn held_bytes(max_events: usize) -> Option<usize> {
    max_events.checked_mul(std::mem::size_of::<Held>())
}
