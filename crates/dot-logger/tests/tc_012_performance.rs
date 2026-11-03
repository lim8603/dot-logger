//! TC-012 P0: result judgment, simple-file baseline and synthetic control loop.
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]
#[path = "../examples/support/perf/mod.rs"]
mod perf;
use perf::*;
use std::time::Duration;

fn workload() -> StorageWorkload {
    StorageWorkload {
        record_payload_bytes: 1000,
        records: 3000,
        files: 4,
        writers: 2,
        file_limit_bytes: 64 * 1024,
        block_bytes: 4096,
        sync_interval: Duration::from_millis(1000),
        offered_bytes_per_second: None,
    }
}
fn run(tool: Tool, durable_ns: Option<u64>) -> StorageRun {
    StorageRun {
        tool,
        workload: workload(),
        payload_bytes: 3_000_000,
        framing_bytes: 48_000,
        files_created: 52,
        write_calls: 800,
        largest_write: 4096,
        smallest_write: 16,
        periodic_syncs: 0,
        accepted_ns: 1_000_000,
        durable_ns,
        stopped: None,
    }
}

#[test]
fn throughput_uses_final_sync_and_rejects_ram_acceptance_only() {
    assert_eq!(
        throughput(&run(Tool::P0SimpleFile, None)),
        Err(Rejection::NoFinalSync)
    );
    let t = throughput(&run(Tool::P0SimpleFile, Some(3_000_000))).unwrap();
    // 3 MB over 3 ms to durability, not over the 1 ms memory acceptance.
    assert_eq!(t.payload_bytes, 3_000_000);
    assert_eq!(t.durable_ns, 3_000_000);
    assert_eq!(t.bytes_per_second(), 1_000_000_000.0);
    let mut zero = run(Tool::P0SimpleFile, Some(0));
    zero.accepted_ns = 0;
    assert_eq!(throughput(&zero), Err(Rejection::ZeroDuration));
    let mut stopped = run(Tool::P0SimpleFile, Some(3_000_000));
    stopped.stopped = Some(Stop::Reserve);
    assert_eq!(throughput(&stopped), Err(Rejection::Incomplete));
}

#[test]
fn efficiency_requires_a_matching_p0_baseline() {
    let logger = run(Tool::Logger, Some(4_000_000));
    let p0 = run(Tool::P0SimpleFile, Some(3_000_000));
    assert_eq!(efficiency(&logger, None), Err(Rejection::MissingBaseline));
    assert_eq!(
        efficiency(&logger, Some(&logger)),
        Err(Rejection::BaselineNotP0)
    );
    let mut other = p0.clone();
    other.workload.files = 16;
    assert_eq!(
        efficiency(&logger, Some(&other)),
        Err(Rejection::WorkloadMismatch)
    );
    let mut fewer = p0.clone();
    fewer.payload_bytes -= 1;
    assert_eq!(
        efficiency(&logger, Some(&fewer)),
        Err(Rejection::PayloadMismatch)
    );
    assert_eq!(
        efficiency(&logger, Some(&run(Tool::P0SimpleFile, None))),
        Err(Rejection::NoFinalSync)
    );
    let e = efficiency(&logger, Some(&p0)).unwrap();
    assert_eq!(e.percent(), 75.0);
}

#[test]
fn skipped_releases_count_as_misses() {
    let ms = 1_000_000;
    // Finished inside the deadline: next release, nothing skipped.
    assert_eq!(
        account(0, 10 * ms, 10 * ms, 3 * ms),
        Cycle {
            missed: false,
            skipped: 0,
            next: 1
        }
    );
    // Release 0 finished at 25 ms: it missed, release 1 (deadline 20) is
    // skipped, release 2 (deadline 30) may still start late.
    assert_eq!(
        account(0, 10 * ms, 10 * ms, 25 * ms),
        Cycle {
            missed: true,
            skipped: 1,
            next: 2
        }
    );
    // Exactly at the deadline is on time.
    assert!(!account(3, 10 * ms, 10 * ms, 40 * ms).missed);
}

fn control(misses: u64, cpu: Option<usize>) -> ControlRun {
    ControlRun {
        config: ControlConfig {
            cpu,
            period: Duration::from_millis(10),
            deadline: Duration::from_millis(10),
            work_iterations: 1000,
        },
        kernel: "7.0.0-42-generic".into(),
        releases: 100,
        completed: 100 - misses,
        misses,
        skipped: 0,
        off_cpu: 0,
        wake: Histogram::default(),
        response: Histogram::default(),
    }
}

#[test]
fn additional_misses_need_three_paired_runs_under_identical_conditions() {
    let off = [
        control(0, Some(2)),
        control(1, Some(2)),
        control(0, Some(2)),
    ];
    let on = [
        control(1, Some(2)),
        control(1, Some(2)),
        control(0, Some(2)),
    ];
    assert_eq!(
        additional_misses(&off[..2], &on),
        Err(Rejection::InsufficientRepetitions)
    );
    let moved = [
        control(0, Some(2)),
        control(1, Some(3)),
        control(0, Some(2)),
    ];
    assert_eq!(
        additional_misses(&off, &moved),
        Err(Rejection::ConditionMismatch)
    );
    let mut kernel = on.clone();
    kernel[0].kernel = "7.0.0-38-realtime".into();
    assert_eq!(
        additional_misses(&off, &kernel),
        Err(Rejection::ConditionMismatch)
    );
    let a = additional_misses(&off, &on).unwrap();
    assert_eq!(a.paired, vec![1, 0, 0]);
    assert_eq!(a.total, 1);
}

#[test]
fn histogram_keeps_exact_max_and_power_of_two_buckets() {
    let mut h = Histogram::default();
    for ns in [0, 999, 1_000, 1_999, 2_000, 1_000_000] {
        h.record(ns);
    }
    assert_eq!(h.count, 6);
    assert_eq!(h.max_ns, 1_000_000);
    // Bucket k holds [2^(k-1), 2^k) microseconds; bucket 0 holds < 1 µs.
    assert_eq!(h.buckets[0], 2);
    assert_eq!(h.buckets[1], 2);
    assert_eq!(h.buckets[2], 1);
    assert_eq!(h.buckets[10], 1);
}

#[test]
fn p0_writer_rotates_syncs_and_matches_the_oracle() {
    let parent = std::env::temp_dir();
    let owned = RunDir::create(&parent).unwrap();
    let r = write_p0(owned.path(), workload(), 0, &[]).unwrap();
    assert_eq!(r.tool, Tool::P0SimpleFile);
    assert_eq!(r.payload_bytes, 3_000_000);
    assert_eq!(r.framing_bytes, 3000 * FRAME_HEADER as u64);
    assert!(r.durable_ns.unwrap() >= r.accepted_ns);
    assert!(r.stopped.is_none());
    // 64 KiB files hold 64 records of 1016 B: 3000 records need > 4 files.
    assert!(r.files_created > 4);
    assert!(r.largest_write <= 4096);
    let v = verify_p0(owned.path(), &r).unwrap();
    assert_eq!(v.records, 3000);
    assert_eq!(v.payload_bytes, 3_000_000);
    assert_eq!(v.files, r.files_created);
    throughput(&r).unwrap();
    let path = owned.path().to_owned();
    drop(owned);
    assert!(!path.exists());
}

#[test]
fn p0_writer_stops_admission_before_the_free_space_reserve() {
    let owned = RunDir::create(&std::env::temp_dir()).unwrap();
    let r = write_p0(owned.path(), workload(), u64::MAX, &[]).unwrap();
    assert_eq!(r.stopped, Some(Stop::Reserve));
    assert_eq!(throughput(&r), Err(Rejection::Incomplete));
    // What was admitted is still synced and verifiable.
    assert!(r.durable_ns.is_some());
    let v = verify_p0(owned.path(), &r).unwrap();
    assert_eq!(v.payload_bytes, r.payload_bytes);
}

#[test]
fn paced_p0_writer_does_not_exceed_the_offered_rate() {
    let owned = RunDir::create(&std::env::temp_dir()).unwrap();
    let mut w = workload();
    w.offered_bytes_per_second = Some(10_000_000);
    let r = write_p0(owned.path(), w, 0, &[]).unwrap();
    // 3 MB at 10 MB/s cannot be accepted in under ~300 ms.
    assert!(r.accepted_ns >= 250_000_000, "{}", r.accepted_ns);
}

#[test]
fn control_loop_counts_every_release_on_the_monotonic_clock() {
    let cfg = ControlConfig {
        cpu: None,
        period: Duration::from_millis(2),
        deadline: Duration::from_millis(2),
        work_iterations: calibrate(Duration::from_micros(50)),
    };
    let r = run_control(cfg, 50).unwrap();
    assert_eq!(r.releases, 50);
    assert_eq!(r.completed + r.misses, r.releases);
    assert!(r.skipped <= r.misses);
    assert_eq!(r.wake.count, r.completed + r.misses - r.skipped);
    assert!(!r.kernel.is_empty());
}

#[test]
fn periodic_calibration_converges_in_the_release_cycle() {
    let c = calibrate_periodic(Duration::from_micros(200), Duration::from_millis(1), 50).unwrap();
    assert!(c.iterations > 0);
    assert!((1..=8).contains(&c.rounds));
    assert!(c.median_ns > 0);
}
