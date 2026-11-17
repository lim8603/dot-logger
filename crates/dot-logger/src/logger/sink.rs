//! Optional tracing/`log` connection (API §18.10). General path only: user
//! `Debug`/`Display` formatting runs under a lock on the calling thread, so
//! never emit these events from RT threads.

use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, TryLockError};

use crate::adapters::tracing::{log_level, log_record, tracing_event, tracing_level};
use crate::capture::Level;

use super::writer::{Channel, EncodeError, Record, Schema, Writer};

/// Schema of the JSON text payload written by the tracing connection.
pub(crate) const SCHEMA: Schema = Schema::new(99, 1, "json");

thread_local! {
    /// Set while this thread is inside the sink; a nested event is dropped.
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
}

/// Writer and counters shared by every [`TracingSink`] clone and the logger.
pub(crate) struct SinkState {
    writer: Mutex<Option<Writer>>,
    session: u128,
    channel: u32,
    max: ::tracing::Level,
    dropped: AtomicU64,
}

impl SinkState {
    pub(crate) fn new(writer: Writer, session: u128, channel: u32, max: ::tracing::Level) -> Self {
        Self {
            writer: Mutex::new(Some(writer)),
            session,
            channel,
            max,
            dropped: AtomicU64::new(0),
        }
    }

    /// Take the writer back for shutdown; later events are dropped and counted.
    pub(crate) fn reclaim(&self) -> Option<Writer> {
        self.writer
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }

    /// Acknowledge a posted revision without waiting for an event encoder
    /// holding this lock (including reentrant management on that thread).
    pub(crate) fn acknowledge(&self) {
        let mut writer = match self.writer.try_lock() {
            Ok(writer) => Some(writer),
            Err(TryLockError::Poisoned(writer)) => Some(writer.into_inner()),
            Err(TryLockError::WouldBlock) => None,
        };
        if let Some(writer) = writer.as_mut().and_then(|w| w.as_mut()) {
            writer.refresh();
        }
    }

    pub(crate) fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Acquire)
    }

    fn drop_one(&self) {
        self.dropped.fetch_add(1, Ordering::AcqRel);
    }

    fn submit<T: Record + ?Sized>(&self, level: Level, message: &T) {
        if ACTIVE.with(|active| active.replace(true)) {
            // Logged from inside our own formatting: the lock is held here.
            self.drop_one();
            return;
        }
        let _leave = Leave;
        let mut writer = self.writer.lock().unwrap_or_else(PoisonError::into_inner);
        let accepted = writer.as_mut().is_some_and(|w| {
            w.try_log(Channel::new(self.session, self.channel).at(level), message)
                .is_ok()
        });
        if !accepted {
            self.drop_one();
        }
    }
}

/// Clears [`ACTIVE`] on every exit path, including a panicking formatter.
struct Leave;
impl Drop for Leave {
    fn drop(&mut self) {
        ACTIVE.with(|active| active.set(false));
    }
}

struct EventRecord<'a, 'b>(&'a ::tracing::Event<'b>);
impl Record for EventRecord<'_, '_> {
    const SCHEMA: Schema = SCHEMA;
    fn encode(&self, out: &mut [u8]) -> Result<usize, EncodeError> {
        tracing_event(out, self.0)
            .map(|e| e.payload.len())
            .map_err(|_| EncodeError)
    }
}

struct LogRecord<'a, 'b>(&'a log::Record<'b>);
impl Record for LogRecord<'_, '_> {
    const SCHEMA: Schema = SCHEMA;
    fn encode(&self, out: &mut [u8]) -> Result<usize, EncodeError> {
        log_record(out, self.0)
            .map(|e| e.payload.len())
            .map_err(|_| EncodeError)
    }
}

/// A `tracing` subscriber and `log` logger that records events into the
/// logger's tracing channel, from [`Logger::tracing`](super::Logger::tracing).
///
/// Events below the declared minimum level are disabled before formatting.
/// Only each event's own fields are recorded (JSON text, see
/// `adapters::tracing`); spans are not tracked. A full channel, an oversized
/// event, a formatter error, an event logged from inside formatting, or any
/// event after shutdown is dropped and counted in `Health::tracing_dropped`.
/// Never waits for the worker. Install it once, e.g. with
/// `tracing::subscriber::set_global_default` or `log::set_boxed_logger`.
#[derive(Clone)]
pub struct TracingSink {
    pub(crate) state: Arc<SinkState>,
}

impl std::fmt::Debug for TracingSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TracingSink")
            .field("max", &self.state.max)
            .field("dropped", &self.state.dropped())
            .finish()
    }
}

fn from_log(level: log::Level) -> ::tracing::Level {
    match level {
        log::Level::Error => ::tracing::Level::ERROR,
        log::Level::Warn => ::tracing::Level::WARN,
        log::Level::Info => ::tracing::Level::INFO,
        log::Level::Debug => ::tracing::Level::DEBUG,
        log::Level::Trace => ::tracing::Level::TRACE,
    }
}

impl ::tracing::Subscriber for TracingSink {
    fn enabled(&self, metadata: &::tracing::Metadata<'_>) -> bool {
        metadata.is_event() && *metadata.level() <= self.state.max
    }
    fn max_level_hint(&self) -> Option<::tracing::level_filters::LevelFilter> {
        Some(::tracing::level_filters::LevelFilter::from_level(
            self.state.max,
        ))
    }
    fn new_span(&self, _: &::tracing::span::Attributes<'_>) -> ::tracing::span::Id {
        // Spans are disabled by `enabled`; any forced span shares one id.
        ::tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &::tracing::span::Id, _: &::tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &::tracing::span::Id, _: &::tracing::span::Id) {}
    fn event(&self, event: &::tracing::Event<'_>) {
        self.state.submit(
            tracing_level(*event.metadata().level()),
            &EventRecord(event),
        );
    }
    fn enter(&self, _: &::tracing::span::Id) {}
    fn exit(&self, _: &::tracing::span::Id) {}
}

impl log::Log for TracingSink {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        from_log(metadata.level()) <= self.state.max
    }
    fn log(&self, record: &log::Record<'_>) {
        if log::Log::enabled(self, record.metadata()) {
            self.state
                .submit(log_level(record.level()), &LogRecord(record));
        }
    }
    fn flush(&self) {}
}
