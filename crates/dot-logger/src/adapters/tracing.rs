//! Explicit ordinary-path hooks for an application's tracing subscriber/log sink.
//!
//! These functions install no global logger, subscriber, worker, queue or clock.
//! The caller supplies scratch storage and invokes its own admission/diagnostics.
//! They encode JSON UTF-8 `{"target":"...","fields":{"name":"value",...}}`;
//! all field values are text, not typed numeric telemetry. `tracing` primitive
//! strings are preserved; other fields use their `Debug` representation. Only
//! the event's own fields are encoded, without span context or log key-values.
//!
//! This API belongs on ordinary application threads. User `Debug`/`Display`
//! implementations may allocate, block or recurse; bounded output does not make
//! formatting RT safe or impose a CPU/time budget on those callbacks. A host
//! should avoid logging recursively from its sink and keep worker scratch within
//! its own memory budget. Do not attach this hook to RT producers.

use crate::capture::{CaptureError, Level, Producer, RecordMetadata, RecordOutcome};
use ::tracing::field::{Field, Visit};
use std::fmt::{self, Write};

/// A complete encoded event borrowing the caller's scratch until capture/copy.
#[derive(Debug)]
pub struct EncodedLog<'a> {
    /// Diagnostic severity mapped from the source, without changing importance.
    pub level: Level,
    /// Complete UTF-8 JSON. No prefix is returned after encoding failure.
    pub payload: &'a [u8],
}
impl EncodedLog<'_> {
    /// Submit once through the native bounded small-record admission API.
    ///
    /// Only `metadata.level` is replaced. The host supplies channel/schema,
    /// clocks, source time and importance; register the textual schema explicitly.
    /// Filtering/sampling, capacity/closure errors and ingress receipts keep their
    /// native meanings. There is no retry or output-completion acknowledgment.
    pub fn capture(
        &self,
        producer: &mut Producer<'_>,
        mut metadata: RecordMetadata,
    ) -> Result<RecordOutcome, CaptureError> {
        metadata.level = self.level;
        producer.try_record_small(metadata, self.payload)
    }
}

/// Whole-event encoding failed; scratch may contain an unusable partial prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodeError {
    /// Encoded bytes exceed the caller's exact scratch length; no truncation.
    TooLarge,
    /// A user formatter returned `fmt::Error` without exhausting scratch.
    Formatting,
}
impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ordinary log encoding: {self:?}")
    }
}
impl std::error::Error for EncodeError {}

/// Encode the event's own fields for an explicit application subscriber hook.
///
/// No span storage or process-wide registration is performed. All primitive
/// values become strings: use the separate typed numeric API for numeric data.
/// The returned borrow prevents scratch reuse while this payload is referenced.
pub fn tracing_event<'a>(
    scratch: &'a mut [u8],
    event: &::tracing::Event<'_>,
) -> Result<EncodedLog<'a>, EncodeError> {
    let mut output = Buffer::new(scratch);
    let mut result = header(&mut output, event.metadata().target());
    if result.is_ok() {
        let mut visitor = Fields {
            output: &mut output,
            result: Ok(()),
            first: true,
        };
        event.record(&mut visitor);
        result = visitor.result;
    }
    finish(output, result, tracing_level(*event.metadata().level()))
}

/// Encode a log record's target and formatted message for an explicit sink hook.
/// Source file/module/line and optional structured key-values are not included.
/// Formatting runs on the caller's ordinary thread, with no logger installation.
pub fn log_record<'a>(
    scratch: &'a mut [u8],
    record: &log::Record<'_>,
) -> Result<EncodedLog<'a>, EncodeError> {
    let mut output = Buffer::new(scratch);
    let result = (|| {
        header(&mut output, record.target())?;
        output.write_str("\"message\":")?;
        quoted(&mut output, *record.args())
    })();
    finish(output, result, log_level(record.level()))
}
fn tracing_level(level: ::tracing::Level) -> Level {
    match level {
        ::tracing::Level::TRACE => Level::Trace,
        ::tracing::Level::DEBUG => Level::Debug,
        ::tracing::Level::INFO => Level::Info,
        ::tracing::Level::WARN => Level::Warn,
        ::tracing::Level::ERROR => Level::Error,
    }
}
fn log_level(level: log::Level) -> Level {
    match level {
        log::Level::Trace => Level::Trace,
        log::Level::Debug => Level::Debug,
        log::Level::Info => Level::Info,
        log::Level::Warn => Level::Warn,
        log::Level::Error => Level::Error,
    }
}
fn header(output: &mut Buffer<'_>, target: &str) -> fmt::Result {
    output.write_str("{\"target\":")?;
    quoted(output, format_args!("{target}"))?;
    output.write_str(",\"fields\":{")
}
fn finish(
    mut output: Buffer<'_>,
    result: fmt::Result,
    level: Level,
) -> Result<EncodedLog<'_>, EncodeError> {
    if result.is_ok() {
        output.write_str("}}").map_err(|_| EncodeError::TooLarge)?;
    }
    // Sticky overflow catches a formatter that swallows the writer's error.
    if output.overflow {
        return Err(EncodeError::TooLarge);
    }
    result.map_err(|_| EncodeError::Formatting)?;
    Ok(EncodedLog {
        level,
        payload: &output.bytes[..output.len],
    })
}
struct Buffer<'a> {
    bytes: &'a mut [u8],
    len: usize,
    overflow: bool,
}
impl<'a> Buffer<'a> {
    fn new(bytes: &'a mut [u8]) -> Self {
        Self {
            bytes,
            len: 0,
            overflow: false,
        }
    }
}
impl Write for Buffer<'_> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if self.overflow || text.len() > self.bytes.len() - self.len {
            self.overflow = true;
            return Err(fmt::Error);
        }
        let end = self.len + text.len();
        self.bytes[self.len..end].copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}
struct Escaped<'a, 'b>(&'a mut Buffer<'b>);
impl Write for Escaped<'_, '_> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for c in text.chars() {
            match c {
                '"' => self.0.write_str("\\\"")?,
                '\\' => self.0.write_str("\\\\")?,
                '\u{0}'..='\u{1f}' => write!(self.0, "\\u{:04x}", c as u32)?,
                _ => self.0.write_char(c)?,
            }
        }
        Ok(())
    }
}
fn quoted(output: &mut Buffer<'_>, value: fmt::Arguments<'_>) -> fmt::Result {
    output.write_char('"')?;
    Escaped(output).write_fmt(value)?;
    output.write_char('"')
}
struct Fields<'a, 'b> {
    output: &'a mut Buffer<'b>,
    result: fmt::Result,
    first: bool,
}
impl Fields<'_, '_> {
    fn field(&mut self, field: &Field, value: fmt::Arguments<'_>) {
        if self.result.is_err() || self.output.overflow {
            return;
        }
        self.result = (|| {
            if !self.first {
                self.output.write_char(',')?;
            }
            self.first = false;
            quoted(self.output, format_args!("{}", field.name()))?;
            self.output.write_char(':')?;
            quoted(self.output, value)
        })();
    }
}
impl Visit for Fields<'_, '_> {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.field(field, format_args!("{value}"));
    }
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.field(field, format_args!("{value:?}"));
    }
}
