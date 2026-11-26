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
        file_record_limit: u64::MAX,
        block_bytes: 4096,
        sync_interval: Duration::from_millis(1000),
        offered_bytes_per_second: None,
        large: None,
        warmup: Duration::ZERO,
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
        start_ns: 0,
        accepted_ns: 1_000_000,
        durable_ns,
        stopped: None,
        sync_overdue: false,
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
            fifo_priority: None,
        },
        kernel: "7.0.0-42-generic".into(),
        policy: ThreadPolicy::default(),
        origin_ns: 0,
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
    let r = write_p0(owned.path(), workload(), 0, &[], &Gate::new(2, 0)).unwrap();
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
    let r = write_p0(owned.path(), workload(), u64::MAX, &[], &Gate::new(2, 0)).unwrap();
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
    let r = write_p0(owned.path(), w, 0, &[], &Gate::new(w.writers, 0)).unwrap();
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
        fifo_priority: None,
    };
    let r = run_control(cfg, 50, &Gate::new(0, 1), Duration::ZERO).unwrap();
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
/// quantiles come from nanosecond log-linear buckets with an exact maximum
/// and no answer for an empty run. TASK-042 publishes p50 of about 160 ns,
/// so the relative error is ≤ 1/32 (5 ns there), not 1/8 (20 ns).
#[test]
fn latency_histogram_bounds_quantiles_within_a_thirty_second() {
    let mut empty = LatencyHistogram::default();
    assert_eq!(empty.quantile(0.99), None);
    for ns in [3, 3, 5] {
        empty.record(ns);
    }
    // Small values are exact.
    assert_eq!(empty.quantile(0.5), Some(3));
    assert_eq!(empty.quantile(1.0), Some(5));
    let mut small = LatencyHistogram::default();
    small.record(31);
    small.record(1_000_000);
    assert_eq!(small.quantile(0.5), Some(31));
    let mut h = LatencyHistogram::default();
    for ns in 1..=100_000_u64 {
        h.record(ns);
    }
    assert_eq!(h.count(), 100_000);
    assert_eq!(h.max(), 100_000);
    for (q, exact) in [(0.5, 50_000_u64), (0.99, 99_000), (0.999, 99_900)] {
        let got = h.quantile(q).unwrap();
        assert!(got >= exact && got <= exact + exact / 32, "{q}: {got}");
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
        file_record_limit: FACADE_SEGMENT_RECORDS,
        block_bytes: 64 * 1024,
        sync_interval: Duration::from_millis(100),
        offered_bytes_per_second: None,
        large: None,
        warmup: Duration::ZERO,
    };
    let load = LoggerLoad {
        producers: 4,
        channels: 8,
        slots: 1024,
        raw_slots: 0,
        // Native loss reports additionally reserve physical message indexes.
        memory_bytes: 80 * MIB as usize,
        producer_cpus: Vec::new(),
        worker_cpus: Vec::new(),
        telemetry: false,
    };
    let p0_dir = RunDir::create(&parent).unwrap();
    let p0 = write_p0(p0_dir.path(), w, 0, &[], &Gate::new(w.writers, 0)).unwrap();
    let dir = RunDir::create(&parent).unwrap();
    let run = write_logger(dir.path(), w, &load, 0, &Gate::new(load.producers, 0)).unwrap();
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
        file_record_limit: FACADE_SEGMENT_RECORDS,
        block_bytes: 4096,
        sync_interval: Duration::from_millis(100),
        offered_bytes_per_second: None,
        large: None,
        warmup: Duration::ZERO,
    };
    let load = LoggerLoad {
        producers: 1,
        channels: 6,
        slots: 16,
        raw_slots: 0,
        memory_bytes: 64 << 20,
        producer_cpus: Vec::new(),
        worker_cpus: Vec::new(),
        telemetry: false,
    };
    let dir = RunDir::create(&std::env::temp_dir()).unwrap();
    // Channel c goes to file c % files only when files divides channels.
    assert!(write_logger(dir.path(), w, &load, 0, &Gate::new(load.producers, 0)).is_err());
    // A payload must fit the eight-byte sequence.
    let tiny = StorageWorkload {
        record_payload_bytes: 4,
        ..w
    };
    let fine = LoggerLoad {
        channels: 8,
        ..load
    };
    assert!(write_logger(dir.path(), tiny, &fine, 0, &Gate::new(fine.producers, 0)).is_err());
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

/// TASK-023 S2b finding ⑤ (Human 2026-11-07): the facade also rotates after
/// a fixed number of records per segment, so P0 rotates at the same count and
/// the logger run refuses any other count.
#[test]
fn p0_rotates_at_the_shared_record_limit_and_the_logger_requires_it() {
    let parent = std::env::temp_dir();
    let w = StorageWorkload {
        file_record_limit: 50,
        records: 1000,
        files: 1,
        writers: 1,
        ..workload()
    };
    let dir = RunDir::create(&parent).unwrap();
    let r = write_p0(dir.path(), w, 0, &[], &Gate::new(w.writers, 0)).unwrap();
    // 50 frames of 1016 B stay below the 64 KiB byte limit (64 frames), so
    // the record count rotates first: 1000 records make 20 files.
    assert_eq!(r.files_created, 20);
    let v = verify_p0(dir.path(), &r).unwrap();
    assert_eq!(v.records, 1000);
    assert_eq!(v.largest_file_records, 50);
    let logger_dir = RunDir::create(&parent).unwrap();
    let load = LoggerLoad {
        producers: 1,
        channels: 1,
        slots: 64,
        raw_slots: 0,
        memory_bytes: 64 << 20,
        producer_cpus: Vec::new(),
        worker_cpus: Vec::new(),
        telemetry: false,
    };
    let w = StorageWorkload {
        record_payload_bytes: 64,
        file_limit_bytes: 8 << 20,
        ..w
    };
    assert!(
        write_logger(
            logger_dir.path(),
            w,
            &load,
            0,
            &Gate::new(load.producers, 0)
        )
        .is_err()
    );
    assert_eq!(FACADE_SEGMENT_RECORDS, 16_384);
}

/// REV-052 RV-130: every party finishes its setup before one shared start,
/// so a slow setup never shortens another party's measured interval.
#[test]
fn gate_starts_every_party_after_the_slowest_setup() {
    let gate = Gate::new(2, 1);
    let (ready, passed): (Vec<u64>, Vec<u64>) = std::thread::scope(|scope| {
        let handles: Vec<_> = [0_u64, 0, 60]
            .into_iter()
            .map(|delay| {
                let gate = &gate;
                scope.spawn(move || {
                    std::thread::sleep(Duration::from_millis(delay));
                    let ready = monotonic_ns();
                    let start = gate.pass().unwrap();
                    assert!(monotonic_ns() >= start);
                    (ready, start)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).unzip()
    });
    assert!(passed.iter().all(|&s| s == passed[0]));
    assert!(passed[0] > *ready.iter().max().unwrap());
    assert_eq!(gate.started(), Some(passed[0]));
}
#[test]
fn gate_releases_every_party_when_one_setup_fails() {
    let gate = Gate::new(1, 1);
    std::thread::scope(|scope| {
        let waiting = scope.spawn(|| gate.pass());
        std::thread::sleep(Duration::from_millis(20));
        gate.abort();
        assert!(waiting.join().unwrap().is_err());
    });
    assert_eq!(gate.started(), None);
    // A load whose party count differs from the gate never waits forever.
    let owned = RunDir::create(&std::env::temp_dir()).unwrap();
    assert!(write_p0(owned.path(), workload(), 0, &[], &Gate::new(3, 0)).is_err());
}
/// RV-130: a control result describes the load only when both counted from
/// the same start and the load's admission covered the control window; the
/// final durability tail is reported apart from the load interval.
#[test]
fn load_window_needs_a_shared_start_and_coverage_of_the_control_window() {
    let ms = 1_000_000;
    let mut c = control(0, None);
    c.origin_ns = 5_000 * ms;
    // 100 releases of 10 ms: a 1 s control window.
    let mut r = run(Tool::P0SimpleFile, Some(1_200 * ms));
    r.start_ns = 5_000 * ms;
    r.accepted_ns = 1_000 * ms;
    let w = load_window(&c, &r);
    assert_eq!(w.control_ns, 1_000 * ms);
    assert_eq!(w.load_ns, 1_000 * ms);
    assert_eq!(w.covered_ns, 1_000 * ms);
    assert_eq!(w.tail_ns, Some(200 * ms));
    assert_eq!(w.judge(), Ok(()));
    // Up to 1 % of the window may end early (the paced last record).
    r.accepted_ns = 990 * ms;
    assert_eq!(load_window(&c, &r).judge(), Ok(()));
    r.accepted_ns = 989 * ms;
    assert_eq!(load_window(&c, &r).judge(), Err(Rejection::NotConcurrent));
    // Different starts are not one concurrent run, however long the load.
    r.accepted_ns = 2_000 * ms;
    r.start_ns = 5_001 * ms;
    let w = load_window(&c, &r);
    assert_eq!(w.skew_ns, ms);
    assert_eq!(w.judge(), Err(Rejection::NotConcurrent));
}
/// RV-130: an artificial setup delay on the control side moves the shared
/// start, not the load's or the control's measured intervals.
#[test]
fn a_slow_setup_shifts_neither_the_load_nor_the_control_window() {
    let owned = RunDir::create(&std::env::temp_dir()).unwrap();
    let mut w = workload();
    w.offered_bytes_per_second = Some(10_000_000);
    let gate = Gate::new(w.writers, 1);
    let cfg = ControlConfig {
        cpu: None,
        period: Duration::from_millis(2),
        deadline: Duration::from_millis(2),
        work_iterations: calibrate(Duration::from_micros(20)),
        fifo_priority: None,
    };
    let (c, r) = std::thread::scope(|scope| {
        let c = scope.spawn(|| {
            std::thread::sleep(Duration::from_millis(100));
            run_control(cfg, 125, &gate, Duration::ZERO)
        });
        let r = write_p0(owned.path(), w, 0, &[], &gate);
        (c.join().unwrap().unwrap(), r.unwrap())
    });
    assert_eq!(c.origin_ns, r.start_ns);
    assert_eq!(gate.started(), Some(r.start_ns));
    // 3 MB at 10 MB/s: ~300 ms of load from the start, setup excluded.
    assert!(r.accepted_ns >= 250_000_000, "{}", r.accepted_ns);
    assert!(r.accepted_ns < 2_000_000_000, "{}", r.accepted_ns);
    let window = load_window(&c, &r);
    assert_eq!(window.control_ns, 250_000_000);
    assert_eq!(window.judge(), Ok(()), "{window:?}");
}
#[test]
fn a_load_that_ends_early_is_not_concurrent_with_the_control_window() {
    let owned = RunDir::create(&std::env::temp_dir()).unwrap();
    let w = workload();
    let gate = Gate::new(w.writers, 1);
    let cfg = ControlConfig {
        cpu: None,
        period: Duration::from_millis(10),
        deadline: Duration::from_millis(10),
        work_iterations: 1,
        fifo_priority: None,
    };
    let (c, r) = std::thread::scope(|scope| {
        let c = scope.spawn(|| run_control(cfg, 200, &gate, Duration::ZERO));
        let r = write_p0(owned.path(), w, 0, &[], &gate);
        (c.join().unwrap().unwrap(), r.unwrap())
    });
    // Unpaced 3 MB is admitted long before the 2 s control window ends.
    assert!(r.accepted_ns < 1_900_000_000, "{}", r.accepted_ns);
    assert_eq!(load_window(&c, &r).judge(), Err(Rejection::NotConcurrent));
}

/// TASK-023 S2c: a mixed workload (P3) puts every `every`-th record at the
/// large size on one of `streams` dedicated files; both tools share its
/// placement, sizes and paced schedule.
#[test]
fn mixed_workload_places_sizes_and_schedule_identically_for_both_tools() {
    let mut w = workload();
    w.records = 10;
    w.record_payload_bytes = 100;
    w.large = Some(LargeRecords {
        bytes: 1_000,
        every: 4,
        streams: 2,
    });
    w.offered_bytes_per_second = Some(1_000_000);
    // Records 0, 4, 8 are large: streams 0, 1, 0 on files 0, 2, 0.
    assert_eq!(w.payload_of(0), 1_000);
    assert_eq!(w.payload_of(1), 100);
    assert_eq!(w.stream_of(4), Some(1));
    assert_eq!(w.stream_of(5), None);
    assert_eq!(w.file_of(0), 0);
    assert_eq!(w.file_of(4), 2);
    assert_eq!(w.file_of(8), 0);
    assert_eq!(w.file_of(5), 1);
    assert_eq!(w.max_payload(), 1_000);
    // 7 small + 3 large.
    assert_eq!(w.total_payload_bytes(), 7 * 100 + 3 * 1_000);
    // Record 5 is due after 0..5: 3 small + 2 large = 2,300 B at 1 MB/s.
    assert_eq!(w.due_ns(5), Some(2_300_000));
    assert_eq!(w.due_ns(0), Some(0));
    w.offered_bytes_per_second = None;
    assert_eq!(w.due_ns(5), None);
    // Large streams need whole groups of files.
    w.large = Some(LargeRecords {
        bytes: 1_000,
        every: 4,
        streams: 3,
    });
    assert!(w.validate().is_err());
}
fn mixed() -> StorageWorkload {
    StorageWorkload {
        record_payload_bytes: 64,
        records: 3_000,
        files: 4,
        writers: 2,
        file_limit_bytes: 8 * 1024 * 1024,
        file_record_limit: FACADE_SEGMENT_RECORDS,
        block_bytes: 64 * 1024,
        sync_interval: Duration::from_millis(100),
        offered_bytes_per_second: None,
        large: Some(LargeRecords {
            bytes: 256 * 1024,
            every: 100,
            streams: 2,
        }),
        warmup: Duration::ZERO,
    }
}
#[test]
fn p0_mixed_run_matches_the_oracle() {
    let w = mixed();
    let owned = RunDir::create(&std::env::temp_dir()).unwrap();
    let r = write_p0(owned.path(), w, 0, &[], &Gate::new(w.writers, 0)).unwrap();
    assert_eq!(r.payload_bytes, w.total_payload_bytes());
    let v = verify_p0(owned.path(), &r).unwrap();
    assert_eq!(v.records, 3_000);
    assert_eq!(v.payload_bytes, w.total_payload_bytes());
    throughput(&r).unwrap();
}
fn mixed_load(producers: usize) -> LoggerLoad {
    LoggerLoad {
        producers,
        channels: 8,
        slots: 256,
        raw_slots: 4,
        memory_bytes: 80 << 20,
        producer_cpus: Vec::new(),
        worker_cpus: Vec::new(),
        telemetry: false,
    }
}
/// S2c P3: large records go through `Writer::producer()` raw loans on their
/// stream's producer, small ones through `try_log`; read-back and the P0
/// ratio use the same workload.
#[test]
fn logger_mixed_run_sends_large_records_as_raw_loans_and_matches_p0() {
    let w = mixed();
    let load = mixed_load(4);
    let p0_dir = RunDir::create(&std::env::temp_dir()).unwrap();
    let p0 = write_p0(p0_dir.path(), w, 0, &[], &Gate::new(w.writers, 0)).unwrap();
    let dir = RunDir::create(&std::env::temp_dir()).unwrap();
    let run = write_logger(dir.path(), w, &load, 0, &Gate::new(load.producers, 0)).unwrap();
    assert!(run.storage.stopped.is_none(), "{}", run.report);
    assert_eq!(run.storage.payload_bytes, w.total_payload_bytes());
    // 30 large records (0, 100, ..) as raw loans, the rest through try_log.
    assert_eq!(run.raw_accepted.count(), 30);
    assert_eq!(run.accepted.count(), 2_970);
    let v = verify_logger(&run).unwrap();
    assert_eq!(v.records, 3_000);
    assert_eq!(v.payload_bytes, w.total_payload_bytes());
    efficiency(&run.storage, Some(&p0)).unwrap();
}
/// S2c P2: every record large, one raw producer per stream.
#[test]
fn logger_all_large_run_uses_only_raw_producers() {
    let mut w = mixed();
    w.records = 40;
    w.large = Some(LargeRecords {
        bytes: 512 * 1024,
        every: 1,
        streams: 2,
    });
    let mut load = mixed_load(2);
    // Native user/meta pairs keep two MCAP message index entries.
    load.memory_bytes = 80 << 20;
    let dir = RunDir::create(&std::env::temp_dir()).unwrap();
    let run = write_logger(dir.path(), w, &load, 0, &Gate::new(load.producers, 0)).unwrap();
    assert!(run.storage.stopped.is_none(), "{}", run.report);
    assert_eq!(run.raw_accepted.count(), 40);
    assert_eq!(run.accepted.count(), 0);
    assert_eq!(verify_logger(&run).unwrap().payload_bytes, 40 * 512 * 1024);
    // Small records need a producer besides the raw streams.
    let mut mixed_w = mixed();
    mixed_w.records = 10;
    assert!(write_logger(dir.path(), mixed_w, &mixed_load(2), 0, &Gate::new(2, 0)).is_err());
}

/// TASK-023 S3: a paced run starts with a warm-up the results leave out —
/// its records are offered and stored, but throughput counts the payload
/// after it over warm-up end → final sync.
#[test]
fn warmup_moves_the_measured_window_past_the_first_records() {
    let mut w = workload();
    w.records = 30;
    w.record_payload_bytes = 100;
    w.offered_bytes_per_second = Some(1_000_000);
    w.warmup = Duration::from_millis(1);
    // Record s is due at s × 100 µs: records 0..10 are the warm-up.
    assert_eq!(w.first_measured(), 10);
    w.validate().unwrap();
    let mut r = run(Tool::P0SimpleFile, Some(5_000_000));
    r.workload = w;
    r.payload_bytes = 3_000;
    let t = throughput(&r).unwrap();
    assert_eq!(t.payload_bytes, 2_000);
    assert_eq!(t.durable_ns, 4_000_000);
    // A warm-up needs a paced schedule to end on.
    w.offered_bytes_per_second = None;
    assert!(w.validate().is_err());
}
/// S3: the control loops run only in the measured window, so their shared
/// start is the load start plus the warm-up.
#[test]
fn control_loops_start_after_the_warmup_of_their_load() {
    let ms = 1_000_000;
    let mut c = control(0, None);
    let mut r = run(Tool::P0SimpleFile, Some(1_300 * ms));
    r.workload.offered_bytes_per_second = Some(1_000_000);
    r.workload.warmup = Duration::from_millis(100);
    r.start_ns = 5_000 * ms;
    r.accepted_ns = 1_100 * ms;
    c.origin_ns = 5_100 * ms;
    assert_eq!(load_window(&c, &r).judge(), Ok(()));
    c.origin_ns = 5_000 * ms;
    assert_eq!(load_window(&c, &r).judge(), Err(Rejection::NotConcurrent));
    // The loop itself waits the given delay past the gate's start.
    let gate = Gate::new(0, 1);
    let cfg = ControlConfig {
        cpu: None,
        period: Duration::from_millis(2),
        deadline: Duration::from_millis(2),
        work_iterations: 1,
        fifo_priority: None,
    };
    let c = run_control(cfg, 5, &gate, Duration::from_millis(20)).unwrap();
    assert_eq!(c.origin_ns, gate.started().unwrap() + 20 * ms);
}
/// S3: call latencies come from measured records only.
#[test]
fn logger_latencies_leave_out_the_warmup_records() {
    let mut w = mixed();
    w.large = None;
    w.records = 2_000;
    w.offered_bytes_per_second = Some(64 * 2_000 * 5);
    w.warmup = Duration::from_millis(50);
    assert_eq!(w.first_measured(), 500);
    let load = mixed_load(4);
    let dir = RunDir::create(&std::env::temp_dir()).unwrap();
    let run = write_logger(dir.path(), w, &load, 0, &Gate::new(load.producers, 0)).unwrap();
    assert!(run.storage.stopped.is_none(), "{}", run.report);
    assert_eq!(run.accepted.count(), 1_500);
    assert_eq!(verify_logger(&run).unwrap().records, 2_000);
}

/// Human 2026-11-08 (long-run rule): a periodic sync that missed its deadline
/// fails a paced normal-load run, while an unpaced efficiency run that lost
/// nothing keeps its rate and reports the overdue sync.
#[test]
fn an_overdue_sync_fails_only_the_paced_normal_load() {
    let mut paced = run(Tool::Logger, Some(3_000_000));
    paced.workload.offered_bytes_per_second = Some(1_000_000_000);
    paced.sync_overdue = true;
    assert_eq!(throughput(&paced), Err(Rejection::Unsuccessful));
    let mut unpaced = run(Tool::Logger, Some(3_000_000));
    unpaced.sync_overdue = true;
    assert!(throughput(&unpaced).is_ok());
}
/// Long-run rule: S is the stable rate after the SLC cache ran out, never the
/// cached burst.
#[test]
fn sustained_rate_is_the_stable_window_after_the_cache_drop() {
    let mut series = vec![1_000_u64; 40];
    series.extend(std::iter::repeat_n(500, 200));
    let s = sustained_rate(&series, 120);
    assert_eq!(s.peak_bytes_per_second, 1_000);
    let drop = s.drop_at.unwrap();
    assert!((40..=50).contains(&drop), "{drop}");
    assert_eq!(s.bytes_per_second, 500);
    assert!(s.stable);
    // No drop yet: the rate is reported but not taken as sustained.
    let flat = vec![900_u64; 100];
    let s = sustained_rate(&flat, 30);
    assert_eq!(s.drop_at, None);
    assert!(!s.stable);
    assert_eq!(s.bytes_per_second, 900);
}
#[test]
fn sustained_write_samples_every_second_and_cleans_up() {
    let owned = RunDir::create(&std::env::temp_dir()).unwrap();
    let run = measure_sustained(owned.path(), 2, 2, 64 * 1024, 32 << 20, 2, &[]).unwrap();
    assert!(run.total_bytes >= 32 << 20);
    assert!(!run.series.is_empty());
    assert_eq!(run.series.iter().sum::<u64>(), run.total_bytes);
    assert_eq!(std::fs::read_dir(owned.path()).unwrap().count(), 0);
}

// RT request (TASK-022 rule, 2026-11-11): RT rows run the loops as
// SCHED_FIFO (control 70 / safety 80). The applied policy is read back from
// /proc, never assumed, and a refused priority is an error, not a fallback.
#[test]
fn proc_stat_policy_reads_rt_priority_and_policy_after_the_command_name() {
    let mut fields: Vec<String> = (3..=52).map(|n| n.to_string()).collect();
    fields[40 - 3] = "70".into();
    fields[41 - 3] = "1".into();
    let stat = format!("1234 (a) b (c)) {}", fields.join(" "));
    assert_eq!(
        thread_policy(&stat),
        Some(ThreadPolicy {
            policy: SCHED_FIFO,
            priority: 70
        })
    );
    assert_eq!(thread_policy("1234 (truncated"), None);
}

#[test]
fn fifo_control_loop_applies_and_reports_its_policy() {
    let limit = rtprio_limit().unwrap();
    if limit == 0 {
        eprintln!("RLIMIT_RTPRIO is 0: SCHED_FIFO is not permitted here; skipped");
        return;
    }
    let work_iterations = calibrate(Duration::from_micros(50));
    // A fresh thread per run: a policy stays on the thread that took it.
    let run = move |fifo_priority| {
        std::thread::spawn(move || {
            let cfg = ControlConfig {
                cpu: None,
                period: Duration::from_millis(2),
                deadline: Duration::from_millis(2),
                work_iterations,
                fifo_priority,
            };
            run_control(cfg, 5, &Gate::new(0, 1), Duration::ZERO)
        })
        .join()
        .unwrap()
    };
    let fifo = run(Some(1)).unwrap();
    assert_eq!(
        fifo.policy,
        ThreadPolicy {
            policy: SCHED_FIFO,
            priority: 1
        }
    );
    assert_eq!(run(None).unwrap().policy.policy, 0);
    if limit < 99 {
        assert!(
            run(Some(99)).is_err(),
            "a refused priority must not fall back"
        );
    }
}

/// TASK-042: latency rows write only to a tmpfs ring with room for twice
/// the ring, checked from the writing process (2026-11-23 fill incident).
#[test]
fn latency_rows_refuse_an_output_that_is_not_tmpfs_or_too_small() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let err = require_tmpfs(Target {
        dir: repo,
        ring: 1 << 20,
    })
    .unwrap_err();
    assert!(err.to_string().contains("is not tmpfs"), "{err}");
    let shm = std::path::Path::new("/dev/shm");
    let err = require_tmpfs(Target {
        dir: shm,
        ring: u64::MAX / 4,
    })
    .unwrap_err();
    assert!(err.to_string().contains("free"), "{err}");
    require_tmpfs(Target {
        dir: shm,
        ring: 64 << 20,
    })
    .unwrap();
}

/// TASK-042: every row times exactly the calls it was asked for, sorts them
/// by outcome, measures as many clock pairs and shuts down cleanly.
#[test]
fn latency_rows_time_every_requested_call_by_outcome() {
    let run = RunDir::create(std::path::Path::new("/dev/shm")).unwrap();
    let out = run.path().join("out-000");
    std::fs::create_dir(&out).unwrap();
    let target = Target {
        dir: &out,
        ring: 64 << 20,
    };
    let rt = Rt { cpu: 0, fifo: None };
    let h = warm(target, 16, 3000, rt).unwrap();
    assert_eq!((h.accepted.count(), h.clock.count()), (3000, 3000));
    assert_eq!((h.rejected.count(), h.skipped.count()), (0, 0));
    assert!(h.success);
    // C1 sizes its sweep from sysfs; a runner without cache data cannot run it.
    match llc_bytes(0) {
        Ok(llc) => {
            let (c, swept) = cold(target, 64, 3, rt).unwrap();
            assert_eq!(c.accepted.count(), 3);
            assert_eq!(swept, 2 * llc);
        }
        Err(e) => eprintln!("C1 not checked: {e}"),
    }
    let sparse = sparse(target, 64, Duration::from_millis(1), 50, 4096, rt).unwrap();
    assert_eq!(sparse.accepted.count() + sparse.capacity, 50);
    let r = reject(target, 64, 200, rt).unwrap();
    assert!(
        r.skipped.count() >= 200 && r.rejected.count() >= 200,
        "{r:?}"
    );
    assert_eq!(r.clock.count(), 200);
    assert!(r.success);
    let j = jitter(
        target,
        64,
        Duration::from_millis(2),
        20,
        Duration::from_micros(100),
        5,
        None,
        // A zero threshold records every batch in the diagnostic list.
        Some(Duration::ZERO),
        rt,
    )
    .unwrap();
    assert_eq!(
        (j.wake.count(), j.response.count(), j.calls.count()),
        (20, 20, 20)
    );
    assert_eq!((j.slow.len(), j.slow_dropped), (20, 0));
    assert!(j.slow.iter().enumerate().all(|(k, b)| {
        b.release == k as u64
            && b.seq == 5 * k as u64
            && b.max_call_index < 5
            && b.max_call_ns <= b.batch_ns
    }));
    assert!(j.success);
    let m = multi(target, 16, 1000, &[0, 1], rt).unwrap();
    assert_eq!(m.len(), 2);
    assert!(m.iter().all(|w| w.accepted.count() == 1000 && w.success));
}

/// Runs `f` on its own thread and fails the test if it has not returned
/// within `limit`: a row that hangs must not hang the test run.
fn within<T: Send + 'static>(limit: Duration, f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    match rx.recv_timeout(limit) {
        Ok(value) => value,
        Err(e) => panic!("latency row did not end in time: {e}"),
    }
}

/// REV-073 RV-207: a measuring thread that fails its setup (M: a CPU that
/// is not online) or panics (J: a CPU past the CPU set, with the IO load on)
/// ends the row with an error in bounded time.
#[test]
fn latency_rows_end_when_a_measuring_thread_fails() {
    let rt = Rt { cpu: 0, fifo: None };
    let m = within(Duration::from_secs(60), move || {
        let run = RunDir::create(std::path::Path::new("/dev/shm")).unwrap();
        let out = run.path().join("out-000");
        std::fs::create_dir(&out).unwrap();
        let target = Target {
            dir: &out,
            ring: 64 << 20,
        };
        multi(target, 16, 1000, &[0, 1000], rt)
            .map(|_| ())
            .map_err(|e| e.to_string())
    });
    assert!(m.is_err(), "{m:?}");
    let j = within(Duration::from_secs(60), move || {
        let run = RunDir::create(std::path::Path::new("/dev/shm")).unwrap();
        let out = run.path().join("out-000");
        let load = run.path().join("load-000");
        std::fs::create_dir(&out).unwrap();
        std::fs::create_dir(&load).unwrap();
        let target = Target {
            dir: &out,
            ring: 64 << 20,
        };
        let io = IoLoad {
            dir: &load,
            bytes: 64,
            per_second: 1000,
        };
        let panics = Rt {
            cpu: 100_000,
            fifo: None,
        };
        jitter(
            target,
            64,
            Duration::from_millis(2),
            20,
            Duration::from_micros(100),
            5,
            Some(io),
            None,
            panics,
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
    });
    assert!(j.unwrap_err().contains("panicked"));
}
