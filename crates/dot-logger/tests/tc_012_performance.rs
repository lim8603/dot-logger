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
        writes: Some(WriteCalls {
            calls: 800,
            largest: 4096,
            smallest: 16,
        }),
        periodic_syncs: Some(0),
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
    // TASK-023 S2b: a logger run whose report was not a success never gives
    // a rate, even with a final sync time.
    let mut lost = run(Tool::Logger, Some(3_000_000));
    lost.stopped = Some(Stop::Unsuccessful);
    assert_eq!(throughput(&lost), Err(Rejection::Unsuccessful));
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
    assert!(r.writes.unwrap().largest <= 4096);
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

/// REV-051 RV-127: a zero payload is a usage error, never a division panic.
#[test]
fn p0_record_count_rejects_a_zero_payload() {
    assert!(records_for(64 * 1024 * 1024, 0).is_err());
    assert_eq!(records_for(1000, 10).unwrap(), 100);
    assert_eq!(records_for(5, 10).unwrap(), 0);
}

/// TASK-023 S2b: call latency is judged at p99 ≤ 5 µs / p99.9 ≤ 20 µs, so
/// quantiles come from nanosecond log-linear buckets with ≤ 1/8 relative
/// error, an exact maximum and no answer for an empty run.
#[test]
fn latency_histogram_bounds_quantiles_within_an_eighth() {
    let mut empty = LatencyHistogram::default();
    assert_eq!(empty.quantile(0.99), None);
    for ns in [3, 3, 5] {
        empty.record(ns);
    }
    // Small values are exact.
    assert_eq!(empty.quantile(0.5), Some(3));
    assert_eq!(empty.quantile(1.0), Some(5));
    let mut h = LatencyHistogram::default();
    for ns in 1..=100_000_u64 {
        h.record(ns);
    }
    assert_eq!(h.count(), 100_000);
    assert_eq!(h.max(), 100_000);
    for (q, exact) in [(0.5, 50_000_u64), (0.99, 99_000), (0.999, 99_900)] {
        let got = h.quantile(q).unwrap();
        assert!(got >= exact && got <= exact + exact / 8, "{q}: {got}");
    }
    // Never above the observed maximum.
    assert_eq!(h.quantile(1.0), Some(100_000));
    let mut merged = LatencyHistogram::default();
    merged.merge(&h);
    merged.merge(&empty);
    assert_eq!(merged.count(), 100_003);
    assert_eq!(merged.max(), 100_000);
}

/// TASK-023 S2b: a logger record carries its sequence in the first eight
/// payload bytes and the P0 fill pattern after it, so read-back finds both
/// identity and content errors with the same payload size as P0.
#[test]
fn p1_payload_carries_its_sequence_and_detects_corruption() {
    let mut out = [0_u8; 64];
    assert_eq!(p1_payload(77, &mut out), Some(64));
    assert_eq!(payload_sequence(&out), Some(77));
    assert!(check_p1_payload(77, &out));
    assert!(!check_p1_payload(78, &out));
    out[40] ^= 1;
    assert!(!check_p1_payload(77, &out));
    // Eight bytes are the smallest payload that can carry a sequence.
    assert_eq!(p1_payload(1, &mut [0_u8; 7]), None);
}

/// TASK-023 S2b: with several producers per file the file order interleaves,
/// so the oracle demands each sequence exactly once, in file `seq % files`,
/// increasing per (producer, file), and nothing missing at the end.
#[test]
fn sequence_oracle_checks_placement_duplicates_order_and_gaps() {
    // 8 records, 2 files, 2 producers: producer = seq % 2, file = seq % 2.
    let mut o = SequenceOracle::new(8, 2, 2);
    for (file, seq) in [
        (0, 0),
        (1, 1),
        (0, 2),
        (1, 3),
        (0, 4),
        (1, 5),
        (0, 6),
        (1, 7),
    ] {
        o.see(file, seq).unwrap();
    }
    assert_eq!(o.finish().unwrap(), 8);
    let mut wrong_file = SequenceOracle::new(8, 2, 2);
    assert!(wrong_file.see(1, 0).is_err());
    let mut duplicate = SequenceOracle::new(8, 2, 2);
    duplicate.see(0, 0).unwrap();
    assert!(duplicate.see(0, 0).is_err());
    let mut reordered = SequenceOracle::new(8, 2, 2);
    reordered.see(0, 4).unwrap();
    assert!(reordered.see(0, 2).is_err());
    let mut outside = SequenceOracle::new(8, 2, 2);
    assert!(outside.see(0, 8).is_err());
    let mut missing = SequenceOracle::new(8, 2, 2);
    missing.see(0, 0).unwrap();
    assert!(missing.finish().is_err());
    // Producers 4, files 2: producers 0 and 2 both feed file 0 and may
    // interleave; each stays increasing on its own.
    let mut shared = SequenceOracle::new(8, 2, 4);
    for (file, seq) in [(0, 2), (0, 0), (0, 6), (0, 4)] {
        shared.see(file, seq).unwrap();
    }
}

/// TASK-023 S2b: the facade run shares the P0 workload — same records,
/// payload, files, rotation, sync period and IO-thread count — writes through
/// `Writer::try_log` from several producers into channels routed one file
/// each, passes the read-back oracle and forms a ratio against P0.
#[test]
fn logger_run_matches_the_p0_workload_and_passes_read_back() {
    const MIB: u64 = 1024 * 1024;
    let parent = std::env::temp_dir();
    let w = StorageWorkload {
        record_payload_bytes: 64,
        records: 20_000,
        files: 4,
        writers: 2,
        file_limit_bytes: 8 * MIB,
        block_bytes: 64 * 1024,
        sync_interval: Duration::from_millis(100),
        offered_bytes_per_second: None,
    };
    let load = LoggerLoad {
        producers: 4,
        channels: 8,
        slots: 1024,
        memory_bytes: 64 * MIB as usize,
        producer_cpus: Vec::new(),
        worker_cpus: Vec::new(),
    };
    let p0_dir = RunDir::create(&parent).unwrap();
    let p0 = write_p0(p0_dir.path(), w, 0, &[]).unwrap();
    let dir = RunDir::create(&parent).unwrap();
    let run = write_logger(dir.path(), w, &load, 0).unwrap();
    assert_eq!(run.storage.tool, Tool::Logger);
    assert_eq!(run.storage.workload, w);
    assert_eq!(run.storage.payload_bytes, 20_000 * 64);
    assert!(run.storage.stopped.is_none(), "{}", run.report);
    assert!(run.storage.durable_ns.unwrap() >= run.storage.accepted_ns);
    // The internal writer's requests and sync rounds are not observable.
    assert_eq!(run.storage.writes, None);
    assert_eq!(run.storage.periodic_syncs, None);
    // Every admitted call was timed once.
    assert_eq!(run.accepted.count(), 20_000);
    assert_eq!(run.rejected.count(), run.rejections);
    let v = verify_logger(&run).unwrap();
    assert_eq!(v.records, 20_000);
    assert_eq!(v.payload_bytes, 20_000 * 64);
    assert_eq!(v.files, run.storage.files_created);
    // Rotation follows the shared file limit.
    assert!(v.largest_file <= w.file_limit_bytes, "{v:?}");
    efficiency(&run.storage, Some(&p0)).unwrap();
    // The run directory is cleaned up with the logger outputs inside it.
    let path = dir.path().to_owned();
    drop(dir);
    assert!(!path.exists());
}

#[test]
fn logger_load_must_route_whole_files() {
    let w = StorageWorkload {
        record_payload_bytes: 64,
        records: 10,
        files: 4,
        writers: 2,
        file_limit_bytes: 8 * 1024 * 1024,
        block_bytes: 4096,
        sync_interval: Duration::from_millis(100),
        offered_bytes_per_second: None,
    };
    let load = LoggerLoad {
        producers: 1,
        channels: 6,
        slots: 16,
        memory_bytes: 64 << 20,
        producer_cpus: Vec::new(),
        worker_cpus: Vec::new(),
    };
    let dir = RunDir::create(&std::env::temp_dir()).unwrap();
    // Channel c goes to file c % files only when files divides channels.
    assert!(write_logger(dir.path(), w, &load, 0).is_err());
    // A payload must fit the eight-byte sequence.
    let tiny = StorageWorkload {
        record_payload_bytes: 4,
        ..w
    };
    let fine = LoggerLoad {
        channels: 8,
        ..load
    };
    assert!(write_logger(dir.path(), tiny, &fine, 0).is_err());
}

/// TASK-023 S2b / backlog T4: the call cost is split into `try_log` as a
/// whole, the encode alone, admission of a pre-encoded payload through
/// `producer().try_record_small`, a same-size copy and the clock pair itself,
/// with one sample per kind per round and a clean shutdown.
#[test]
fn call_cost_separates_encode_admission_copy_and_clock() {
    let dir = RunDir::create(&std::env::temp_dir()).unwrap();
    let cost = call_cost(dir.path(), 256, 2000).unwrap();
    for (name, h) in [
        ("try_log", &cost.try_log),
        ("encode", &cost.encode),
        ("record_small", &cost.record_small),
        ("copy", &cost.copy),
        ("clock", &cost.clock),
    ] {
        assert_eq!(h.count(), 2000, "{name}");
    }
    assert_eq!(cost.payload_bytes, 256);
    assert!(cost.success, "{}", cost.report);
}
