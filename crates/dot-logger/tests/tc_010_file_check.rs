//! Real temporary-file check lifecycle; fake clocks/probes are functional fixtures.
#![cfg(unix)]
#![allow(clippy::unwrap_used)]
mod support;
use dot_logger::{
    capture::{CaptureOwner, ChannelConfig},
    completion::Boundary,
    diagnostics::{
        CheckBudgets, CheckOutcome, CheckPlan, CheckRuntime, CheckSpec, CheckStartError, CheckStop,
        FileCheck, FileWorkload, NetworkPlan, StopReason,
    },
    routing::Router,
    storage::StorageChannel,
};
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "dot-check-run-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::write(path.join("operating.mcap"), b"keep-existing-records").unwrap();
        Self(path)
    }
    fn spec(&self) -> CheckSpec {
        CheckSpec {
            session_id: 42,
            revision: 7,
            environment_id: 11,
            workload_id: 22,
            directory: self.0.join("check"),
            budgets: CheckBudgets {
                duration_ns: 10_000_000_000,
                write_bytes: 32 * 1024 * 1024,
                memory_bytes: 16 * 1024 * 1024,
                disk_reserve_bytes: 64 * 1024 * 1024,
                workers: 1,
            },
            required_memory_bytes: 8 * 1024 * 1024,
            boundary: Boundary::Synced,
            network: NetworkPlan::UnavailableNoReceiver,
        }
    }
    fn unchanged_and_clean(&self) {
        assert_eq!(
            fs::read(self.0.join("operating.mcap")).unwrap(),
            b"keep-existing-records"
        );
        assert!(!self.0.join("check").exists());
        assert_eq!(fs::read_dir(&self.0).unwrap().count(), 1);
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[derive(Default)]
struct Runtime {
    now: u64,
    cancel_at: Option<u64>,
    free_calls: usize,
    advance_probe_at: Option<usize>,
    advance_after_wait: bool,
    fail_probe_at: Option<usize>,
    low_space_at: Option<usize>,
    inspect: Option<PathBuf>,
    messages: usize,
    fail_after_wait: bool,
    reverse_on_wait: bool,
    unexpected_file_on_wait: bool,
    own_directory: Option<PathBuf>,
    replace_file_on_wait: bool,
    stop_space_after_wait: bool,
}
impl CheckRuntime for Runtime {
    fn now_ns(&mut self) -> io::Result<u64> {
        Ok(self.now)
    }
    fn cancelled(&self) -> bool {
        self.cancel_at.is_some_and(|t| self.now >= t)
    }
    fn available_bytes(&mut self, _: &Path) -> io::Result<u64> {
        self.free_calls += 1;
        if self.advance_probe_at == Some(self.free_calls) {
            self.now += 10_000_000_000;
        }
        if self.fail_probe_at == Some(self.free_calls) {
            return Err(io::Error::from_raw_os_error(5));
        }
        if self.low_space_at.is_some_and(|n| self.free_calls >= n) {
            return Ok(64 * 1024 * 1024);
        }
        Ok(1024 * 1024 * 1024)
    }
    fn wait_until(&mut self, deadline: u64) -> io::Result<()> {
        // Observe the actual complete-chunk prefix before cleanup.
        if let Some(dir) = &self.inspect {
            let bytes = fs::read(dir.join("check.mcap"))?;
            self.messages = dot_logger::storage::NativeReader::new(&bytes, 64, 4096)
                .map_err(io::Error::other)?
                .filter_map(Result::ok)
                .inspect(|m| {
                    let decoded = m;
                    assert_eq!(decoded.payload, vec![17; 64]);
                    assert_eq!(decoded.record.revision, 7);
                })
                .count();
        }
        if self.unexpected_file_on_wait {
            fs::write(
                self.own_directory.as_ref().unwrap().join("foreign"),
                b"keep-foreign",
            )?;
        }
        if self.replace_file_on_wait {
            self.replace_file_on_wait = false;
            let dir = self.own_directory.as_ref().unwrap();
            fs::rename(dir.join("check.mcap"), dir.join("moved.mcap"))?;
            fs::write(dir.join("check.mcap"), b"foreign-owner")?;
        }
        if self.stop_space_after_wait {
            self.low_space_at = Some(self.free_calls + 1);
        }
        if self.advance_after_wait {
            self.advance_probe_at = Some(self.free_calls + 1);
        }
        if self.fail_after_wait {
            self.fail_probe_at = Some(self.free_calls + 1);
        }
        self.now = if self.reverse_on_wait { 9 } else { deadline };
        Ok(())
    }
}
fn workload() -> FileWorkload {
    FileWorkload {
        records: 4,
        payload_bytes: 64,
        interval_ns: 1_000_000,
        seed: 17,
    }
}
fn run<R: CheckRuntime>(
    spec: CheckSpec,
    w: FileWorkload,
    r: &mut R,
) -> Result<dot_logger::diagnostics::CheckReport, CheckStartError> {
    let plan = support::plan();
    let routing = support::routing();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        42,
        7,
        &[ChannelConfig::new(7, 99, 1)],
        &routing,
        support::tracking(8),
    )
    .unwrap();
    owner.scope(|mut scope| {
        let (_, a) = scope.next().unwrap();
        let (_, b) = scope.next().unwrap();
        let router = Router::new(&plan, vec![a, b], routing).unwrap();
        let budget = router.writer_budget(10).unwrap();
        let probe = budget.clone();
        let before = probe.used_bytes();
        let result = CheckPlan::build(spec).unwrap().run_file(
            budget,
            FileCheck {
                workload: w,
                channel: StorageChannel {
                    channel: ChannelConfig::new(7, 99, 1),
                    topic: "check".into(),
                    payload_encoding: "opaque".into(),
                    schema: None,
                },
            },
            r,
        );
        assert_eq!(probe.used_bytes(), before, "writer reservation leaked");
        result
    })
}
#[test]
fn real_mcap_is_finished_synced_closed_and_removed_without_touching_operating_records() {
    let temp = Temp::new();
    let mut runtime = Runtime {
        inspect: Some(temp.spec().directory),
        ..Default::default()
    };
    let report = run(temp.spec(), workload(), &mut runtime).unwrap();
    assert_eq!(report.outcome, CheckOutcome::Pass, "{report:?}");
    assert_eq!(report.stop, CheckStop::Completed);
    assert_eq!(report.written_records, 4);
    assert_eq!(report.synced_records, 4);
    assert!(report.file_complete && report.cleaned);
    assert!(report.physical_write_bytes > 4 * 64);
    assert!(report.physical_write_bytes <= report.plan.spec().budgets.write_bytes);
    assert_eq!(runtime.messages, 3);
    assert!(report.primary_issue.is_none() && report.cleanup_issue.is_none());
    temp.unchanged_and_clean();
}
#[test]
fn time_budget_stops_new_load_but_finishes_and_syncs_the_written_prefix() {
    let temp = Temp::new();
    let mut spec = temp.spec();
    spec.budgets.duration_ns = 1_000_000;
    let report = run(spec, workload(), &mut Runtime::default()).unwrap();
    assert_eq!(report.outcome, CheckOutcome::Inconclusive, "{report:?}");
    assert_eq!(report.stop, CheckStop::Limit(StopReason::TimeBudget));
    assert_eq!(report.written_records, 1);
    assert_eq!(report.synced_records, 1);
    assert!(report.file_complete && report.cleaned);
    temp.unchanged_and_clean();
}
#[test]
fn physical_write_budget_leaves_footer_room_and_returns_inconclusive() {
    let temp = Temp::new();
    let mut spec = temp.spec();
    spec.budgets.write_bytes = 100_000;
    let mut w = workload();
    w.records = 100;
    w.payload_bytes = 512;
    w.interval_ns = 0;
    let report = run(spec, w, &mut Runtime::default()).unwrap();
    assert_eq!(report.outcome, CheckOutcome::Inconclusive, "{report:?}");
    assert_eq!(report.stop, CheckStop::Limit(StopReason::WriteBudget));
    assert!(report.written_records < 100);
    assert!(report.file_complete && report.cleaned);
    assert!(report.physical_write_bytes <= 100_000);
    temp.unchanged_and_clean();
}
#[test]
fn cancellation_after_progress_drains_syncs_and_cleans_before_returning_aborted() {
    let temp = Temp::new();
    let report = run(
        temp.spec(),
        workload(),
        &mut Runtime {
            cancel_at: Some(1_000_000),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(report.outcome, CheckOutcome::Aborted, "{report:?}");
    assert_eq!(report.stop, CheckStop::Limit(StopReason::Cancelled));
    assert_eq!(report.written_records, 1);
    assert_eq!(report.synced_records, 1);
    assert!(report.file_complete && report.cleaned);
    temp.unchanged_and_clean();
}
#[test]
fn pre_cancel_and_missing_receiver_create_no_test_directory() {
    let temp = Temp::new();
    let r = run(
        temp.spec(),
        workload(),
        &mut Runtime {
            cancel_at: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(r.outcome, CheckOutcome::Aborted);
    assert_eq!(r.physical_write_bytes, 0);
    let mut spec = temp.spec();
    spec.boundary = Boundary::ReceiverAcknowledged;
    let r = run(spec, workload(), &mut Runtime::default()).unwrap();
    assert_eq!(r.outcome, CheckOutcome::Inconclusive);
    assert_eq!(r.stop, CheckStop::UnsupportedBoundary);
    assert_eq!(r.written_records, 0);
    assert_eq!(r.plan.spec().network, NetworkPlan::UnavailableNoReceiver);
    temp.unchanged_and_clean();
}
#[test]
fn initial_reserve_shortage_and_probe_failure_never_start_new_load() {
    let temp = Temp::new();
    let r = run(
        temp.spec(),
        workload(),
        &mut Runtime {
            low_space_at: Some(1),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(r.stop, CheckStop::Limit(StopReason::DiskReserve));
    assert_eq!(r.physical_write_bytes, 0);
    let r = run(
        temp.spec(),
        workload(),
        &mut Runtime {
            fail_probe_at: Some(1),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(r.outcome, CheckOutcome::Fail);
    assert_eq!(r.primary_issue.unwrap().error.raw_os_error(), Some(5));
    temp.unchanged_and_clean();
}
#[test]
fn existing_directory_and_symlink_are_preserved() {
    let temp = Temp::new();
    let dir = temp.0.join("check");
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("owner-data"), b"existing").unwrap();
    let r = run(temp.spec(), workload(), &mut Runtime::default()).unwrap();
    assert_eq!(r.outcome, CheckOutcome::Fail);
    assert_eq!(fs::read(dir.join("owner-data")).unwrap(), b"existing");
    fs::remove_file(dir.join("owner-data")).unwrap();
    fs::remove_dir(&dir).unwrap();
    std::os::unix::fs::symlink(temp.0.join("operating.mcap"), &dir).unwrap();
    let r = run(temp.spec(), workload(), &mut Runtime::default()).unwrap();
    assert_eq!(r.outcome, CheckOutcome::Fail);
    assert!(fs::symlink_metadata(&dir).unwrap().file_type().is_symlink());
    fs::remove_file(dir).unwrap();
    temp.unchanged_and_clean();
}
#[test]
fn memory_identity_and_worker_mismatches_are_rejected_before_creating_files() {
    let temp = Temp::new();
    let mut spec = temp.spec();
    spec.required_memory_bytes = 1;
    assert!(matches!(
        run(spec, workload(), &mut Runtime::default()),
        Err(CheckStartError::MemoryBudget { .. })
    ));
    let mut spec = temp.spec();
    spec.revision = 8;
    assert!(matches!(
        run(spec, workload(), &mut Runtime::default()),
        Err(CheckStartError::Identity)
    ));
    let mut spec = temp.spec();
    spec.budgets.workers = 2;
    assert!(matches!(
        run(spec, workload(), &mut Runtime::default()),
        Err(CheckStartError::UnsupportedWorkers)
    ));
    temp.unchanged_and_clean();
}

#[test]
fn observation_failure_after_a_written_chunk_still_finishes_and_syncs_the_prefix() {
    let temp = Temp::new();
    let report = run(
        temp.spec(),
        workload(),
        &mut Runtime {
            fail_after_wait: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(report.outcome, CheckOutcome::Fail);
    assert_eq!(
        report.primary_issue.as_ref().unwrap().error.raw_os_error(),
        Some(5)
    );
    assert!(report.file_complete, "{report:?}");
    assert_eq!(report.written_records, 1);
    assert_eq!(report.synced_records, 1);
    assert!(report.cleaned);
    temp.unchanged_and_clean();
}
#[test]
fn reversed_clock_stops_load_and_still_settles_the_existing_file() {
    let temp = Temp::new();
    let report = run(
        temp.spec(),
        workload(),
        &mut Runtime {
            now: 10,
            reverse_on_wait: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(report.outcome, CheckOutcome::Fail);
    assert_eq!(
        report.primary_issue.as_ref().unwrap().error.kind(),
        io::ErrorKind::InvalidData
    );
    assert!(report.file_complete, "{report:?}");
    assert_eq!(report.synced_records, 1);
    temp.unchanged_and_clean();
}
#[test]
fn cleanup_does_not_recursively_remove_an_unexpected_file() {
    let temp = Temp::new();
    let report = run(
        temp.spec(),
        workload(),
        &mut Runtime {
            unexpected_file_on_wait: true,
            own_directory: Some(temp.spec().directory),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(report.outcome, CheckOutcome::Fail);
    assert_eq!(report.stop, CheckStop::Completed);
    assert!(report.cleanup_issue.is_some());
    assert!(!report.cleaned);
    assert_eq!(
        fs::read(temp.0.join("check/foreign")).unwrap(),
        b"keep-foreign"
    );
    fs::remove_file(temp.0.join("check/foreign")).unwrap();
    fs::remove_dir(temp.0.join("check")).unwrap();
    temp.unchanged_and_clean();
}

#[test]
fn replaced_output_identity_is_preserved_and_never_claimed_synced() {
    let temp = Temp::new();
    let report = run(
        temp.spec(),
        workload(),
        &mut Runtime {
            replace_file_on_wait: true,
            own_directory: Some(temp.spec().directory),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(report.outcome, CheckOutcome::Fail);
    assert_eq!(report.synced_records, 0);
    assert!(report.settlement_issue.is_some() && report.cleanup_issue.is_some());
    assert_eq!(
        fs::read(temp.0.join("check/check.mcap")).unwrap(),
        b"foreign-owner"
    );
    let bytes = fs::read(temp.0.join("check/moved.mcap")).unwrap();
    assert_eq!(
        dot_logger::storage::NativeReader::new(&bytes, 64, 4096)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .len(),
        4
    );
}
#[test]
fn space_loss_during_run_prevents_completion_and_still_cleans_owned_resources() {
    let temp = Temp::new();
    let report = run(
        temp.spec(),
        workload(),
        &mut Runtime {
            stop_space_after_wait: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(report.stop, CheckStop::Limit(StopReason::DiskReserve));
    assert_eq!(report.outcome, CheckOutcome::Fail);
    assert!(!report.file_complete);
    assert_eq!(report.written_records, 1);
    assert_eq!(report.synced_records, 0);
    assert!(report.settlement_issue.is_some());
    assert!(report.cleaned);
    temp.unchanged_and_clean();
}
#[test]
fn real_monotonic_runtime_executes_the_same_bounded_file_lifecycle() {
    use dot_logger::diagnostics::FileCheckRuntime;
    use std::{sync::atomic::AtomicBool, time::Instant};
    let temp = Temp::new();
    let cancel = AtomicBool::new(false);
    // Space is a controlled fixture, not a device-capacity measurement.
    let mut runtime =
        FileCheckRuntime::new(Instant::now(), &cancel, |_: &Path| Ok(1024 * 1024 * 1024));
    let report = run(temp.spec(), workload(), &mut runtime).unwrap();
    assert_eq!(report.outcome, CheckOutcome::Pass);
    assert!(report.to_ns.unwrap() >= report.from_ns.unwrap() + 3_000_000);
    temp.unchanged_and_clean();
}

#[test]
fn unsupported_network_and_invalid_workloads_do_not_start_file_load() {
    let temp = Temp::new();
    for network in [
        NetworkPlan::LoopbackOnly,
        NetworkPlan::RemoteUnmeasured { peer_id: 123 },
    ] {
        let mut spec = temp.spec();
        spec.network = network;
        let r = run(spec, workload(), &mut Runtime::default()).unwrap();
        assert_eq!(r.outcome, CheckOutcome::Inconclusive);
        assert_eq!(r.stop, CheckStop::UnsupportedNetwork);
        assert_eq!(r.physical_write_bytes, 0);
    }
    for w in [
        FileWorkload {
            records: 0,
            ..workload()
        },
        FileWorkload {
            payload_bytes: 0,
            ..workload()
        },
        FileWorkload {
            records: 3,
            interval_ns: u64::MAX,
            ..workload()
        },
    ] {
        assert!(matches!(
            run(temp.spec(), w, &mut Runtime::default()),
            Err(CheckStartError::InvalidWorkload)
        ));
    }
    temp.unchanged_and_clean();
}

#[test]
fn slow_space_probes_cannot_admit_load_after_time_budget_or_cancellation() {
    for initial in [true, false] {
        for cancel in [false, true] {
            let temp = Temp::new();
            let mut runtime = Runtime {
                advance_probe_at: initial.then_some(1),
                advance_after_wait: !initial,
                cancel_at: cancel.then_some(10_000_000_000),
                ..Default::default()
            };
            let report = run(temp.spec(), workload(), &mut runtime).unwrap();
            assert_eq!(
                report.written_records,
                if initial { 0 } else { 1 },
                "{report:?}"
            );
            assert_eq!(
                report.stop,
                CheckStop::Limit(if cancel {
                    StopReason::Cancelled
                } else {
                    StopReason::TimeBudget
                })
            );
            assert!(report.cleaned);
            if initial {
                assert_eq!(report.physical_write_bytes, 0);
            }
            temp.unchanged_and_clean();
        }
    }
}
