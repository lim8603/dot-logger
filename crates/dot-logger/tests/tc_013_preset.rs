//! TC-013 F15 (TASK-028 S1): presets and sizing fill missing loss values
//! deterministically, never override user values, and explain the result.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#![cfg(any(unix, windows))]

use dot_logger::config::Resource;
use dot_logger::logger::{
    Budget, EncodeError, Logger, LoggerBuilder, Missing, Preset, Record, Retention, Schema,
    Shutdown, Sizing, StartError, WriterQuota,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MIB: usize = 1024 * 1024;

struct Pose;
impl Record for Pose {
    const SCHEMA: Schema = Schema::new(0x504f_5345, 1, "le-i32x2");
    fn encode(&self, out: &mut [u8]) -> Result<usize, EncodeError> {
        out.get_mut(..8).ok_or(EncodeError)?.fill(1);
        Ok(8)
    }
}

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "dot-logger-tc013-preset-{}-{nanos}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn declared(temp: &Temp) -> LoggerBuilder {
    Logger::builder()
        .file_output(&temp.0, Retention::ring(256 * MIB as u64))
        .channel::<Pose>("robot/pose")
        .writer("control")
}

#[test]
fn f15_control_preset_fills_every_loss_value_and_starts() {
    let temp = Temp::new();
    let builder = declared(&temp).preset(Preset::control(1000));
    let plan = builder.plan().unwrap();
    // 1 kHz × 1 s absorb × 1.25 slot margin, 256-byte records.
    assert_eq!(plan.writer_quota, Some(WriterQuota::small(1250, 256)));
    assert_eq!(plan.sync_every, Duration::from_secs(1));
    assert_eq!(plan.drop_deadline, Duration::from_secs(3));
    // Memory is the exact requirement plus 10 %.
    assert_eq!(
        plan.memory.total_bytes,
        plan.required_bytes + plan.required_bytes.div_ceil(10)
    );
    assert_eq!(
        plan.preset_filled,
        vec![
            Missing::Memory,
            Missing::SyncEvery,
            Missing::DropDeadline,
            Missing::WriterQuota
        ]
    );
    // Planning is deterministic and touches no file.
    assert_eq!(
        format!("{plan:?}"),
        format!("{:?}", builder.plan().unwrap())
    );
    assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 0);
    let logger = builder.start().unwrap();
    let pose = logger.channel::<Pose>("robot/pose").unwrap();
    let mut writer = logger.take_writer("control").unwrap();
    for _ in 0..100 {
        writer.try_log(pose, &Pose).unwrap();
    }
    drop(writer);
    let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(10)).unwrap() else {
        panic!("shutdown did not close");
    };
    assert!(report.is_success(), "{report:?}");
    assert_eq!(report.core.required.complete, 100);
}

#[test]
fn f15_user_values_are_never_overridden() {
    let temp = Temp::new();
    let plan = declared(&temp)
        .budget(
            Budget::new()
                .memory_bytes(64 * MIB)
                .sync_every(Duration::from_millis(50)),
        )
        .preset(Preset::control(100))
        .plan()
        .unwrap();
    assert_eq!(plan.memory.total_bytes, 64 * MIB);
    assert_eq!(plan.sync_every, Duration::from_millis(50));
    assert_eq!(plan.drop_deadline, Duration::from_secs(3));
    assert_eq!(plan.writer_quota, Some(WriterQuota::small(125, 256)));
    assert_eq!(
        plan.preset_filled,
        vec![Missing::DropDeadline, Missing::WriterQuota]
    );
}

#[test]
fn f15_user_memory_below_the_preset_load_is_rejected_with_the_shortfall() {
    let temp = Temp::new();
    let builder = declared(&temp)
        .budget(Budget::new().memory_bytes(8 * MIB))
        .preset(Preset::control(100_000));
    let error = builder.plan().unwrap_err();
    let StartError::Budget {
        resource: Resource::Total,
        required,
        available,
    } = error
    else {
        panic!("{error:?}");
    };
    assert_eq!(available, 8 * MIB);
    assert!(required > available);
    assert!(matches!(
        builder.start(),
        Err(StartError::Budget { required: r, .. }) if r == required
    ));
    assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 0);
}

#[test]
fn f15_sensor_and_black_box_presets_follow_their_profiles() {
    let temp = Temp::new();
    let sensor = declared(&temp)
        .preset(Preset::sensor(30, 8 * MIB))
        .plan()
        .unwrap();
    // 30 Hz × 1 s × 1.25 = 38 originals of 8 MiB plus small metadata records.
    assert_eq!(
        sensor.writer_quota,
        Some(WriterQuota::small(38, 256).with_raw(38, 8 * MIB))
    );
    assert!(sensor.memory.raw_bytes >= 38 * 8 * MIB);
    let black_box = declared(&temp)
        .preset(Preset::black_box(100))
        .plan()
        .unwrap();
    // Durability first: 100 ms sync and a 2 s absorb.
    assert_eq!(black_box.sync_every, Duration::from_millis(100));
    assert_eq!(black_box.writer_quota, Some(WriterQuota::small(250, 256)));
}

#[test]
fn f15_custom_sizing_mixes_streams_and_overrides_preset_defaults() {
    let temp = Temp::new();
    let plan = declared(&temp)
        .preset(
            Sizing::new(Duration::from_millis(500), Duration::from_secs(2))
                .small(2000, 128)
                .raw(10, MIB)
                .absorb(Duration::from_millis(400)),
        )
        .plan()
        .unwrap();
    // 2000 × 0.4 × 1.25 = 1000; 10 × 0.4 × 1.25 = 5.
    assert_eq!(
        plan.writer_quota,
        Some(WriterQuota::small(1000, 128).with_raw(5, MIB))
    );
    assert_eq!(plan.sync_every, Duration::from_millis(500));
    let tuned = declared(&temp)
        .preset(Preset::control(1000).sync_every(Duration::from_millis(200)))
        .plan()
        .unwrap();
    assert_eq!(tuned.sync_every, Duration::from_millis(200));
}

/// REV-051 RV-123: unbounded loss durations are accepted and shut down
/// normally; they saturate internally instead of overflowing.
#[test]
fn f15_unbounded_durations_start_shut_down_and_drop_without_panic() {
    let temp = Temp::new();
    let unbounded = || {
        Budget::new()
            .memory_bytes(64 * MIB)
            .sync_every(Duration::MAX)
            .drop_deadline(Duration::MAX)
            .writer_quota(WriterQuota::small(64, 64))
    };
    let logger = declared(&temp).budget(unbounded()).start().unwrap();
    let Shutdown::Closed(report) = logger.shutdown(Duration::MAX).unwrap() else {
        panic!("an unbounded deadline must still close");
    };
    assert!(report.is_success(), "{report:?}");
    drop(declared(&temp).budget(unbounded()).start().unwrap());
}

/// REV-051 RV-123: sizes whose totals overflow are rejected as an invalid
/// budget instead of panicking or wrapping.
#[test]
fn f15_overflowing_sizes_are_rejected_not_panicking() {
    let temp = Temp::new();
    let huge = declared(&temp)
        .budget(
            Budget::new()
                .memory_bytes(64 * MIB)
                .sync_every(Duration::from_secs(1))
                .drop_deadline(Duration::from_secs(3))
                .writer_quota(WriterQuota::small(64, 64)),
        )
        .writer_with("other", WriterQuota::small(usize::MAX, 256))
        .plan();
    assert!(matches!(huge, Err(StartError::InvalidBudget)), "{huge:?}");
    let sized = declared(&temp)
        .preset(Preset::control(u32::MAX).absorb(Duration::MAX))
        .plan();
    assert!(matches!(sized, Err(StartError::InvalidBudget)), "{sized:?}");
}

/// REV-051 RV-124 (ADR-012): the facade's own preallocations — each writer's
/// encode scratch and return mailbox, each output's write batch — are part of
/// the worker partition and of `required_bytes`.
#[test]
fn f15_facade_buffers_are_charged_to_the_worker_partition() {
    let temp = Temp::new();
    let plan = |slot_bytes: usize, batch: usize| {
        declared(&temp)
            .budget(
                Budget::new()
                    .memory_bytes(64 * MIB)
                    .sync_every(Duration::from_secs(1))
                    .drop_deadline(Duration::from_secs(3))
                    .writer_quota(WriterQuota::small(64, slot_bytes))
                    .batching(batch, Duration::from_millis(10)),
            )
            .plan()
            .unwrap()
    };
    // Batches stay within the cap of half the 64-record budget (finding ③).
    let base = plan(64, 4);
    let wide = plan(64 * 1024, 4);
    let batched = plan(64, 32);
    assert_eq!((base.batch_records, batched.batch_records), (4, 32));
    // One writer: its scratch grows by exactly the slot growth.
    assert_eq!(
        wide.memory.worker_bytes - base.memory.worker_bytes,
        64 * 1024 - 64
    );
    // One output: a larger batch reserves more worker memory.
    assert!(batched.memory.worker_bytes > base.memory.worker_bytes);
    assert!(batched.required_bytes > base.required_bytes);
}

/// REV-051 RV-126: the prelude covers the preset path and `EncodeError` is a
/// standard error like the other facade errors.
mod prelude_surface {
    use dot_logger::prelude::*;

    fn standard_error<E: std::error::Error + Send + Sync + 'static>() {}

    #[test]
    fn f15_prelude_has_presets_and_encode_error_is_an_error() {
        let sizing: Sizing = Preset::control(1000);
        assert_eq!(sizing, Preset::control(1000));
        standard_error::<EncodeError>();
        assert_eq!(
            EncodeError.to_string(),
            "record does not fit the encode buffer"
        );
    }
}

/// TASK-023 S2b finding ③ (Human 2026-11-07): the IO batch never holds more
/// than half of a target's record budget, otherwise a writer with fewer slots
/// than the batch waits `batch_wait` for every slot-full of records.
#[test]
fn f15_batch_is_capped_at_half_the_record_budget() {
    let temp = Temp::new();
    let builder = || {
        declared(&temp).budget(
            Budget::new()
                .memory_bytes(64 * MIB)
                .sync_every(Duration::from_secs(1))
                .drop_deadline(Duration::from_secs(3))
                .writer_quota(WriterQuota::small(16, 64)),
        )
    };
    // Default batch 256 against 16 records: 8.
    assert_eq!(builder().plan().unwrap().batch_records, 8);
    // A smaller user value is kept.
    let small = builder()
        .budget(
            Budget::new()
                .memory_bytes(64 * MIB)
                .sync_every(Duration::from_secs(1))
                .drop_deadline(Duration::from_secs(3))
                .writer_quota(WriterQuota::small(16, 64))
                .batching(4, Duration::from_millis(10)),
        )
        .plan()
        .unwrap();
    assert_eq!(small.batch_records, 4);
    // 20k records through 16 slots: at one slot-full per 10 ms batch wait this
    // took about 14 s; with the cap it is bounded by the file writes.
    let logger = builder().start().unwrap();
    let pose = logger.channel::<Pose>("robot/pose").unwrap();
    let mut writer = logger.take_writer("control").unwrap();
    let started = std::time::Instant::now();
    for _ in 0..20_000 {
        while writer.try_log(pose, &Pose).is_err() {
            std::hint::spin_loop();
        }
    }
    let elapsed = started.elapsed();
    drop(writer);
    let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(30)).unwrap() else {
        panic!("shutdown must close");
    };
    assert_eq!(report.core.totals.dropped, 0, "{report:?}");
    // On Windows and macOS the host's short waits round up to the timer
    // resolution, so this small 16-slot quota takes about 24 s and 11-12 s
    // there (CI runs 45189880533, 45195150611); the bound checks the cap on
    // Linux, where waits are fine-grained.
    if !cfg!(target_os = "linux") {
        eprintln!("20k records through 16 slots: {elapsed:?}");
    } else {
        assert!(elapsed < Duration::from_secs(10), "{elapsed:?}");
    }
}
