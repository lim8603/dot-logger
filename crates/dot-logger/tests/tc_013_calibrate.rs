//! TC-013 F19/F20 (TASK-028 S2, API §19.8): an explicit, bounded machine
//! calibration and its opt-in use by a preset. Measured values stay Observed,
//! the sustained rate is only an Estimated value, unreadable host values are
//! Unavailable, and user values are never overridden.
#![cfg(feature = "calibrate")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[cfg(target_os = "linux")]
use dot_logger::calibrate::Stop;
use dot_logger::calibrate::{CalibrateError, Calibration, Fact, Profile, ProfileError};
use dot_logger::logger::{
    EncodeError, Logger, LoggerBuilder, PlanWarning, Preset, Record, Retention, Schema, Shutdown,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MIB: u64 = 1024 * 1024;

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "dot-logger-tc013-calibrate-{}-{nanos}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn entries(&self) -> usize {
        std::fs::read_dir(&self.0).unwrap().count()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn go() -> AtomicBool {
    AtomicBool::new(false)
}

// Measuring needs the free-space check, which only Linux has (API §19.9).
#[cfg(target_os = "linux")]
#[test]
fn f19_write_limit_stops_the_run_and_leaves_no_file() {
    let temp = Temp::new();
    let profile = Calibration::new(&temp.0)
        .write_limit(3 * MIB)
        .time_limit(Duration::from_secs(60))
        .disk_reserve(0)
        .run(&go())
        .unwrap();
    let write = profile.write();
    assert_eq!(write.bytes, 3 * MIB);
    assert_eq!(write.stopped_by, Stop::WriteLimit);
    assert!(write.syncs >= 1, "the run ends with a data sync");
    assert!(write.elapsed > Duration::ZERO);
    assert_eq!(temp.entries(), 0, "the measurement directory is removed");
    // Observed burst rate; the sustained rate is a halved estimate with its
    // assumption spelled out.
    let burst = profile.burst_bytes_per_second();
    assert!(burst > 0);
    let sustained = profile.sustained_estimate();
    assert_eq!(sustained.bytes_per_second, burst / 2);
    assert!(sustained.assumption.contains("cache"));
    assert_eq!(profile.directory(), temp.0.as_path());
}

// Measuring needs the free-space check, which only Linux has (API §19.9).
#[cfg(target_os = "linux")]
#[test]
fn f19_time_limit_stops_after_the_first_block() {
    let temp = Temp::new();
    let profile = Calibration::new(&temp.0)
        .write_limit(64 * MIB)
        .time_limit(Duration::from_nanos(1))
        .disk_reserve(0)
        .run(&go())
        .unwrap();
    assert_eq!(profile.write().stopped_by, Stop::TimeLimit);
    assert_eq!(profile.write().bytes, 8 * MIB, "one 8 MiB block");
    assert_eq!(temp.entries(), 0);
}

// Measuring needs the free-space check, which only Linux has (API §19.9).
#[cfg(target_os = "linux")]
#[test]
fn f19_reserve_without_room_for_one_block_is_rejected_without_files() {
    let temp = Temp::new();
    let err = Calibration::new(&temp.0)
        .disk_reserve(u64::MAX)
        .run(&go())
        .unwrap_err();
    assert!(
        matches!(
            err,
            CalibrateError::NoSpace {
                reserve: u64::MAX,
                ..
            }
        ),
        "{err:?}"
    );
    assert_eq!(temp.entries(), 0);
}

// Measuring needs the free-space check, which only Linux has (API §19.9).
#[cfg(target_os = "linux")]
#[test]
fn f19_reserve_shrinks_the_write_limit() {
    let temp = Temp::new();
    let free = dot_logger::diagnostics::linux_available_bytes(&temp.0).unwrap();
    // Leave exactly 2 MiB above the reserve.
    let profile = Calibration::new(&temp.0)
        .write_limit(64 * MIB)
        .disk_reserve(free - 2 * MIB)
        .run(&go())
        .unwrap();
    assert!(profile.write().bytes <= 2 * MIB);
    assert_eq!(profile.write().stopped_by, Stop::DiskReserve);
    assert_eq!(temp.entries(), 0);
}

/// TASK-028 residual (EV-071): cancellation while blocks are being written,
/// not only before the run. A watcher cancels once the probe holds data; the
/// run must stop with `Cancelled` and remove the probe and its directory.
#[cfg(target_os = "linux")]
#[test]
fn f19_cancellation_during_measurement_stops_and_removes_the_probe() {
    fn probe_bytes(dir: &Path) -> u64 {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return 0;
        };
        entries
            .filter_map(Result::ok)
            .map(|e| {
                let path = e.path();
                if path.is_dir() {
                    probe_bytes(&path)
                } else {
                    e.metadata().map_or(0, |m| m.len())
                }
            })
            .sum()
    }
    let temp = Temp::new();
    let cancel = go();
    let done = AtomicBool::new(false);
    // At most 512 MiB if the watcher were never scheduled; it normally
    // cancels after the first 8 MiB block.
    let calibration = Calibration::new(&temp.0)
        .write_limit(512 * MIB)
        .time_limit(Duration::from_secs(60))
        .disk_reserve(0);
    let (result, seen) = std::thread::scope(|s| {
        let watcher = s.spawn(|| {
            while !done.load(Ordering::Acquire) {
                let seen = probe_bytes(&temp.0);
                if seen > 0 {
                    cancel.store(true, Ordering::Release);
                    return seen;
                }
                std::thread::sleep(Duration::from_micros(200));
            }
            0
        });
        let result = calibration.run(&cancel);
        done.store(true, Ordering::Release);
        (result, watcher.join().unwrap())
    });
    assert!(seen > 0, "the watcher must observe written data first");
    assert!(
        matches!(result, Err(CalibrateError::Cancelled)),
        "{result:?}"
    );
    assert_eq!(temp.entries(), 0, "probe file and directory are removed");
}

#[test]
fn f19_cancellation_and_invalid_input_leave_no_file() {
    let temp = Temp::new();
    let cancel = AtomicBool::new(true);
    let err = Calibration::new(&temp.0).run(&cancel).unwrap_err();
    assert!(matches!(err, CalibrateError::Cancelled), "{err:?}");
    assert_eq!(temp.entries(), 0);

    for bad in [
        Calibration::new(&temp.0).write_limit(0),
        Calibration::new(&temp.0).time_limit(Duration::ZERO),
        Calibration::new("relative/dir"),
    ] {
        let err = bad.run(&go()).unwrap_err();
        assert!(matches!(err, CalibrateError::InvalidInput), "{err:?}");
    }
    let err = Calibration::new(temp.0.join("missing"))
        .run(&go())
        .unwrap_err();
    let kind = if cfg!(target_os = "linux") {
        std::io::ErrorKind::NotFound
    } else {
        std::io::ErrorKind::Unsupported
    };
    assert!(
        matches!(&err, CalibrateError::Io(e) if e.kind() == kind),
        "{err:?}"
    );
    assert_eq!(temp.entries(), 0);
}

/// REV-053 RV-132: elsewhere the free-space check is `Unsupported` (API
/// §19.9), so a run fails before writing anything.
#[cfg(not(target_os = "linux"))]
#[test]
fn f19_other_platforms_fail_unsupported_without_files() {
    let temp = Temp::new();
    let err = Calibration::new(&temp.0)
        .disk_reserve(0)
        .run(&go())
        .unwrap_err();
    assert!(
        matches!(&err, CalibrateError::Io(e) if e.kind() == std::io::ErrorKind::Unsupported),
        "{err:?}"
    );
    assert_eq!(temp.entries(), 0);
}

#[cfg(target_os = "linux")]
#[test]
fn f19_linux_host_facts_are_observed() {
    let temp = Temp::new();
    let profile = Calibration::new(&temp.0)
        .write_limit(MIB)
        .disk_reserve(0)
        .run(&go())
        .unwrap();
    let host = profile.host();
    assert!(matches!(&host.kernel, Fact::Observed(k) if !k.is_empty()));
    assert!(matches!(host.preempt_rt, Fact::Observed(_)));
    assert!(matches!(&host.cpus_online, Fact::Observed(c) if !c.is_empty()));
    assert!(matches!(host.memory_total_bytes, Fact::Observed(n) if n > 0));
    assert!(matches!(&host.filesystem, Fact::Observed(f) if !f.is_empty()));
    assert!(matches!(host.free_bytes, Fact::Observed(n) if n > 0));
}

/// Format v1 fixture: 4 GiB in 2 s, a 600 ms stall, 1 MiB of memory left.
const FIXTURE: &str = r#"{
  "format": "dot-logger-calibration",
  "version": 1,
  "measured_unix_s": 1794400000,
  "directory": "/data/logs",
  "host": {
    "kernel": {"observed": "7.0.0-43-generic"},
    "preempt_rt": {"observed": false},
    "cpus_online": {"observed": [0, 1, 2, 3]},
    "cpus_isolated": {"observed": []},
    "governor": {"unavailable": "no cpufreq"},
    "memory_total_bytes": {"observed": 68719476736},
    "memory_available_bytes": {"observed": 1048576},
    "cgroup_memory_max_bytes": {"observed": null},
    "cgroup_memory_current_bytes": {"unavailable": "no cgroup v2"},
    "filesystem": {"observed": "ext4"},
    "free_bytes": {"observed": 900000000000}
  },
  "write": {
    "bytes": 4294967296,
    "elapsed_ns": 2000000000,
    "syncs": 3,
    "sync_max_ns": 600000000,
    "write_call_max_ns": 5000000,
    "stopped_by": "WriteLimit"
  }
}"#;

#[test]
fn f20_profile_json_round_trips_and_save_never_overwrites() {
    let temp = Temp::new();
    let profile = Profile::from_json(FIXTURE).unwrap();
    assert_eq!(profile.burst_bytes_per_second(), 2_147_483_648);
    assert_eq!(profile.stall(), Duration::from_millis(600));
    assert!(matches!(&profile.host().governor, Fact::Unavailable(r) if r == "no cpufreq"));
    assert_eq!(profile.host().cgroup_memory_max_bytes, Fact::Observed(None));

    let path = temp.0.join("machine.json");
    profile.save(&path).unwrap();
    assert_eq!(Profile::load(&path).unwrap(), profile);
    let err = profile.save(&path).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
    // Derived values are written for readers.
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("sustained_bytes_per_second"), "{text}");
}

#[test]
fn f20_malformed_profiles_are_rejected() {
    for (text, expected) in [
        ("not json", ProfileError::Syntax),
        (
            &FIXTURE.replace("dot-logger-calibration", "other") as &str,
            ProfileError::Format,
        ),
        (
            &FIXTURE.replace("\"version\": 1", "\"version\": 2"),
            ProfileError::Version(2),
        ),
        (
            &FIXTURE.replace("\"syncs\": 3,", ""),
            ProfileError::Field("write.syncs"),
        ),
    ] {
        assert_eq!(Profile::from_json(text).unwrap_err(), expected, "{text}");
    }
    assert!(matches!(
        Profile::load(Path::new("/nonexistent/machine.json")).unwrap_err(),
        ProfileError::Io(std::io::ErrorKind::NotFound)
    ));
}

struct Pose;
impl Record for Pose {
    const SCHEMA: Schema = Schema::new(0x504f_5345, 1, "le-i32x2");
    fn encode(&self, out: &mut [u8]) -> Result<usize, EncodeError> {
        out.get_mut(..8).ok_or(EncodeError)?.fill(1);
        Ok(8)
    }
}

fn declared(temp: &Temp) -> LoggerBuilder {
    Logger::builder()
        .file_output(&temp.0, Retention::ring(256 * MIB))
        .channel::<Pose>("robot/pose")
        .writer("control")
}

#[test]
fn f20_profile_stall_lengthens_the_absorb_but_a_user_absorb_wins() {
    let temp = Temp::new();
    let profile = Profile::from_json(FIXTURE).unwrap();
    // 600 ms stall × 2 = 1.2 s absorb: ⌈1000 × 1.2 × 1.25⌉ = 1500 slots.
    let plan = declared(&temp)
        .preset(Preset::control(1000).profile(&profile))
        .plan()
        .unwrap();
    assert_eq!(plan.writer_quota.unwrap().small_slots, 1500);
    // The user's absorb is kept, before or after the profile.
    for sizing in [
        Preset::control(1000)
            .absorb(Duration::from_secs(1))
            .profile(&profile),
        Preset::control(1000)
            .profile(&profile)
            .absorb(Duration::from_secs(1)),
    ] {
        let plan = declared(&temp).preset(sizing).plan().unwrap();
        assert_eq!(plan.writer_quota.unwrap().small_slots, 1250);
    }
    // A short stall never shortens the preset absorb (black_box: 2 s).
    let short = Profile::from_json(&FIXTURE.replace("600000000", "100000000")).unwrap();
    let plan = declared(&temp)
        .preset(Preset::black_box(1000).profile(&short))
        .plan()
        .unwrap();
    assert_eq!(plan.writer_quota.unwrap().small_slots, 2500);
}

#[test]
fn f20_plan_warns_on_memory_and_load_but_start_is_not_blocked() {
    let temp = Temp::new();
    // Sustained estimate = 2 bytes/s burst / 2 = 1 byte/s.
    let slow =
        Profile::from_json(&FIXTURE.replace("\"bytes\": 4294967296", "\"bytes\": 4")).unwrap();
    let builder = declared(&temp).preset(Preset::control(1000).profile(&slow));
    let plan = builder.plan().unwrap();
    let total = plan.memory.total_bytes;
    assert_eq!(
        plan.warnings,
        vec![
            PlanWarning::Memory {
                budget_bytes: total,
                available_bytes: 1_048_576,
            },
            PlanWarning::Load {
                offered_bytes_per_second: 256_000,
                limit_bytes_per_second: 0,
            },
        ]
    );
    let logger = builder.start().unwrap();
    let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(10)).unwrap() else {
        panic!("not closed");
    };
    assert!(report.is_success());

    // Without a profile the plan has no warnings.
    let plan = declared(&temp)
        .preset(Preset::control(1000))
        .plan()
        .unwrap();
    assert!(plan.warnings.is_empty());
}
