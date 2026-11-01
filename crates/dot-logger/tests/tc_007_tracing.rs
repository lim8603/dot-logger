//! TASK-020 S1: bounded ordinary-path log encoding and capture admission.
#![cfg(feature = "tracing-adapter")]
#![allow(clippy::unwrap_used)]

use dot_logger::{
    adapters::tracing::{EncodeError, log_record, tracing_event},
    capture::{CaptureError, CaptureOwner, ChannelConfig, Importance, Level, RecordOutcome},
};
use std::fmt;
use tracing::{Event, callsite::Callsite, field::Value};
mod support;

fn event<R>(value: &dyn Value, callback: impl FnOnce(&Event<'_>) -> R) -> R {
    let site = tracing::callsite! {
        name: "ordinary event", kind: tracing::metadata::Kind::EVENT,
        target: "robot\"arm", level: tracing::Level::WARN, fields: message
    };
    let metadata = site.metadata();
    let field = metadata.fields().field("message").unwrap();
    let values = [(&field, Some(value))];
    let values = metadata.fields().value_set(&values);
    callback(&Event::new(metadata, &values))
}

#[test]
fn tracing_preserves_target_fields_unicode_and_json_escaping() {
    let text = "온도\n\"\\\u{0}\t";
    event(&text, |event| {
        let mut scratch = [0; 256];
        let encoded = tracing_event(&mut scratch, event).unwrap();
        assert_eq!(encoded.level, Level::Warn);
        let value: serde_json::Value = serde_json::from_slice(encoded.payload).unwrap();
        assert_eq!(value["target"], "robot\"arm");
        assert_eq!(value["fields"]["message"], text);
    });
}

#[test]
fn log_levels_and_message_rendering_are_explicit() {
    for (source, expected) in [
        (log::Level::Trace, Level::Trace),
        (log::Level::Debug, Level::Debug),
        (log::Level::Info, Level::Info),
        (log::Level::Warn, Level::Warn),
        (log::Level::Error, Level::Error),
    ] {
        let mut scratch = [0; 256];
        let encoded = log_record(
            &mut scratch,
            &log::Record::builder()
                .args(format_args!("joint={}\n", 7))
                .target("robot")
                .level(source)
                .build(),
        )
        .unwrap();
        assert_eq!(encoded.level, expected);
        let value: serde_json::Value = serde_json::from_slice(encoded.payload).unwrap();
        assert_eq!(value["fields"]["message"], "joint=7\n");
    }
}

#[test]
fn exact_size_succeeds_and_short_buffers_never_return_a_prefix() {
    let record = log::Record::builder().args(format_args!("温度")).build();
    let mut scratch = [0; 256];
    let expected = log_record(&mut scratch, &record).unwrap().payload.to_vec();
    for size in 0..expected.len() {
        let mut scratch = vec![0; size];
        assert_eq!(
            log_record(&mut scratch, &record).unwrap_err(),
            EncodeError::TooLarge
        );
    }
    let mut exact = vec![0; expected.len()];
    assert_eq!(log_record(&mut exact, &record).unwrap().payload, expected);
}

struct BadFormat;
impl fmt::Display for BadFormat {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Err(fmt::Error)
    }
}
struct SwallowsOverflow;
impl fmt::Display for SwallowsOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = formatter.write_str(&"x".repeat(2048));
        Ok(())
    }
}
#[test]
fn formatter_errors_and_swallowed_overflow_cannot_be_accepted() {
    let mut scratch = [0; 256];
    assert_eq!(
        log_record(
            &mut scratch,
            &log::Record::builder()
                .args(format_args!("{BadFormat}"))
                .build()
        )
        .unwrap_err(),
        EncodeError::Formatting
    );
    assert_eq!(
        log_record(
            &mut scratch,
            &log::Record::builder()
                .args(format_args!("{SwallowsOverflow}"))
                .build()
        )
        .unwrap_err(),
        EncodeError::TooLarge
    );
    let encoded = log_record(
        &mut scratch,
        &log::Record::builder().args(format_args!("ok")).build(),
    )
    .unwrap();
    let value: serde_json::Value = serde_json::from_slice(encoded.payload).unwrap();
    assert_eq!(value["fields"]["message"], "ok");
}

fn make_owner(slots: usize, slot_bytes: usize, minimum: Level) -> CaptureOwner {
    let mut config = support::plan().config().clone();
    config.producers.truncate(1);
    config.producers[0].small.slots = slots;
    config.producers[0].small.slot_bytes = slot_bytes;
    let plan = dot_logger::config::SessionBuilder::new(config)
        .validate(&dot_logger::platform::PlatformCapabilities::default())
        .unwrap();
    let mut channel = ChannelConfig::new(7, 99, 1);
    channel.minimum_level = minimum;
    CaptureOwner::new(&plan, 42, 1, &[channel]).unwrap()
}

#[test]
fn capture_preserves_host_identity_times_importance_and_native_capacity() {
    let mut owner = make_owner(1, 256, Level::Trace);
    owner.scope(|mut scope| {
        let control = scope.control();
        let (mut producer, mut consumer) = scope.next().unwrap();
        let mut scratch = [0; 256];
        let record = log::Record::builder()
            .args(format_args!("fault"))
            .level(log::Level::Error)
            .build();
        let encoded = log_record(&mut scratch, &record).unwrap();
        let mut metadata = support::meta();
        metadata.importance = Some(Importance::Normal);
        metadata.mono_ns = 123;
        metadata.real_ns = -456;
        let receipt = support::accepted(encoded.capture(&mut producer, metadata).unwrap());
        assert_eq!(receipt.session_id, 42);
        assert_eq!(
            encoded.capture(&mut producer, metadata),
            Err(CaptureError::Capacity)
        );
        let mut bytes = [0; 256];
        let captured = consumer.try_read(&mut bytes).unwrap().unwrap();
        assert_eq!(captured.receipt, receipt);
        assert_eq!(captured.metadata.level, Level::Error);
        assert_eq!(captured.metadata.importance, Some(Importance::Normal));
        assert_eq!(captured.metadata.mono_ns, 123);
        assert_eq!(captured.metadata.real_ns, -456);
        assert_eq!(&bytes[..captured.payload_len], encoded.payload);
        control.close();
        assert_eq!(
            encoded.capture(&mut producer, metadata),
            Err(CaptureError::Closing)
        );
    });
}

#[test]
fn filters_schema_and_slot_limits_keep_native_capture_results() {
    let mut owner = make_owner(2, 256, Level::Warn);
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        let mut scratch = [0; 256];
        let record = log::Record::builder()
            .args(format_args!("info"))
            .level(log::Level::Info)
            .build();
        let encoded = log_record(&mut scratch, &record).unwrap();
        assert_eq!(
            encoded.capture(&mut producer, support::meta()),
            Ok(RecordOutcome::Skipped)
        );
        assert!(consumer.peek().unwrap().is_none());
        let mut bad = support::meta();
        bad.schema_id = 0;
        assert_eq!(
            encoded.capture(&mut producer, bad),
            Err(CaptureError::SchemaMismatch)
        );
    });
    let mut owner = make_owner(2, 16, Level::Trace);
    owner.scope(|mut scope| {
        let (mut producer, mut consumer) = scope.next().unwrap();
        let mut scratch = [0; 256];
        event(&"message", |event| {
            let encoded = tracing_event(&mut scratch, event).unwrap();
            assert_eq!(
                encoded.capture(&mut producer, support::meta()),
                Err(CaptureError::TooLarge)
            );
        });
        assert!(consumer.peek().unwrap().is_none());
    });
}

#[derive(Clone)]
struct Hook(std::sync::Arc<std::sync::Mutex<Vec<(Level, serde_json::Value)>>>);
impl tracing::Subscriber for Hook {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
    fn event(&self, event: &Event<'_>) {
        let mut scratch = [0; 512];
        let encoded = tracing_event(&mut scratch, event).unwrap();
        self.0.lock().unwrap().push((
            encoded.level,
            serde_json::from_slice(encoded.payload).unwrap(),
        ));
    }
}
#[test]
fn actual_scoped_subscriber_gets_all_levels_and_own_fields_without_span_state() {
    let hook = Hook(Default::default());
    tracing::subscriber::with_default(hook.clone(), || {
        let span = tracing::info_span!("host span", secret = "not an event field");
        let _entered = span.enter();
        tracing::trace!(target: "host", count = 17u64, ready = true, ratio = 1.25f64, "tick");
        tracing::debug!("debug");
        tracing::info!("info");
        tracing::warn!("warn");
        tracing::error!("error");
    });
    let values = hook.0.lock().unwrap();
    assert_eq!(
        values.iter().map(|v| v.0).collect::<Vec<_>>(),
        [
            Level::Trace,
            Level::Debug,
            Level::Info,
            Level::Warn,
            Level::Error
        ]
    );
    assert_eq!(values[0].1["fields"]["count"], "17");
    assert_eq!(values[0].1["fields"]["ready"], "true");
    assert_eq!(values[0].1["fields"]["ratio"], "1.25");
    assert_eq!(values[0].1["fields"]["message"], "tick");
    assert!(values[0].1["fields"].get("secret").is_none());
}

struct BadDebug;
impl fmt::Debug for BadDebug {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Err(fmt::Error)
    }
}
#[test]
fn tracing_formatter_failure_and_short_scratch_return_no_partial_event() {
    let mut scratch = [0; 256];
    event(&tracing::field::debug(BadDebug), |event| {
        assert_eq!(
            tracing_event(&mut scratch, event).unwrap_err(),
            EncodeError::Formatting
        );
    });
    event(&"長いメッセージ", |event| {
        let expected = tracing_event(&mut scratch, event).unwrap().payload.to_vec();
        for len in 0..expected.len() {
            let mut scratch = vec![0; len];
            assert_eq!(
                tracing_event(&mut scratch, event).unwrap_err(),
                EncodeError::TooLarge
            );
        }
        assert_eq!(
            tracing_event(&mut scratch[..expected.len()], event)
                .unwrap()
                .payload,
            expected
        );
    });
}
