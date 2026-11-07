//! TC-013 F15 (TASK-028 S1): presets and sizing fill missing loss values
//! deterministically, never override user values, and explain the result.
#![allow(clippy::unwrap_used, clippy::expect_used)]

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
