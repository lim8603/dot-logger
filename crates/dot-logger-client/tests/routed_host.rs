//! Fixed RunCheck capabilities use real production routing and preserve provenance.
#![cfg(feature = "remote")]
#![allow(clippy::unwrap_used)]
use dot_logger::{completion::Boundary, diagnostics::*};
#[path = "../examples/support/dot_probe.rs"]
mod dot_probe;
use dot_probe::SnapshotCheck;
use std::{path::PathBuf, sync::atomic::AtomicBool, time::Instant};

const SNAPSHOT: &[u8] = br#"{"threads":[{"name":"ControlThread","interval_us":10000,"ticks":2}]}"#;
fn plan(memory: usize) -> CheckPlan {
    CheckPlan::build(CheckSpec {
        session_id: 991,
        revision: 1,
        environment_id: 11,
        workload_id: 22,
        directory: PathBuf::from("/nonexistent/routed-check-must-not-create"),
        budgets: CheckBudgets {
            duration_ns: 2_000_000_000,
            write_bytes: 1,
            memory_bytes: memory,
            disk_reserve_bytes: 0,
            workers: 1,
        },
        required_memory_bytes: memory,
        boundary: Boundary::ReceiverAcknowledged,
        network: NetworkPlan::LoopbackOnly,
    })
    .unwrap()
}
#[test]
fn fixed_routed_capability_previews_without_io_then_runs_once_with_original_receipt() {
    let mut host = CheckHost::new(1).unwrap();
    let id = host
        .register_routed(
            plan(32 * 1024 * 1024),
            10,
            Box::new(SnapshotCheck::new(SNAPSHOT.to_vec(), true).unwrap()),
            None,
        )
        .unwrap();
    let preview: serde_json::Value = serde_json::from_slice(&host.preview(id).unwrap()).unwrap();
    assert_eq!(preview["scope"], "production_capture_loopback_snapshot");
    assert_eq!(preview["capability"], "Unprobed");
    assert!(!std::path::Path::new("/nonexistent/routed-check-must-not-create").exists());
    let cancel = AtomicBool::new(false);
    let mut runtime = FileCheckRuntime::new(Instant::now(), &cancel, |_| panic!("no disk IO"));
    let evidence = host.run_routed(id, &mut runtime).unwrap();
    assert_eq!(evidence.report.outcome, CheckOutcome::Pass);
    assert_eq!(evidence.report.receipt.unwrap().session_id, 991);
    assert_eq!(
        (evidence.report.accepted, evidence.report.acknowledged),
        (1, 1)
    );
    assert_eq!(evidence.report.external_bytes_after_close, 0);
    assert_eq!(evidence.report.raw_reclaimed, 1);
    assert!(host.run_routed(id, &mut runtime).is_err());
    assert!(host.preview(id).is_err());
}
#[test]
fn insufficient_memory_rejects_registration_without_consuming_a_slot() {
    let mut host = CheckHost::new(1).unwrap();
    assert!(
        host.register_routed(
            plan(1024),
            10,
            Box::new(SnapshotCheck::new(SNAPSHOT.to_vec(), true).unwrap()),
            None
        )
        .is_err()
    );
    assert!(
        host.register_routed(
            plan(32 * 1024 * 1024),
            10,
            Box::new(SnapshotCheck::new(SNAPSHOT.to_vec(), true).unwrap()),
            None
        )
        .is_ok()
    );
}
#[test]
fn cancellation_and_missing_receiver_never_admit_raw_work() {
    for receiver in [true, false] {
        let mut host = CheckHost::new(1).unwrap();
        let id = host
            .register_routed(
                plan(32 * 1024 * 1024),
                10,
                Box::new(SnapshotCheck::new(SNAPSHOT.to_vec(), receiver).unwrap()),
                None,
            )
            .unwrap();
        let cancel = AtomicBool::new(receiver);
        let mut runtime = FileCheckRuntime::new(Instant::now(), &cancel, |_| panic!("no disk IO"));
        let e = host.run_routed(id, &mut runtime).unwrap();
        assert_eq!(e.report.accepted, 0);
        assert_eq!(
            e.report.outcome,
            if receiver {
                CheckOutcome::Aborted
            } else {
                CheckOutcome::Inconclusive
            }
        );
    }
}

fn source() -> CheckSource {
    use dot_logger::telemetry::{
        CaptureCounts, Window,
        capacity::Conditions,
        report::{Report, ReportInput},
    };
    let conditions = Conditions {
        environment_id: 11,
        workload_id: 22,
        revision: 1,
        target_id: 10,
        boundary: Boundary::ReceiverAcknowledged,
    };
    let window = Window {
        session_id: 7766,
        revision: 1,
        epoch: 8,
        from_ns: 12,
        to_ns: 20,
        counts: CaptureCounts::default(),
    };
    CheckSource {
        report: Report::build(ReportInput::from_window(window, conditions), None, 30),
        profile: None,
    }
}
#[test]
fn original_runtime_report_and_scope_survive_preview_run_and_serialization() {
    let original = source();
    let mut host = CheckHost::new(1).unwrap();
    let id = host
        .register_routed(
            plan(32 * 1024 * 1024),
            10,
            Box::new(SnapshotCheck::new(SNAPSHOT.to_vec(), true).unwrap()),
            Some(original),
        )
        .unwrap();
    let preview: serde_json::Value = serde_json::from_slice(&host.preview(id).unwrap()).unwrap();
    assert_eq!(preview["source"]["session"], format!("{:032x}", 7766));
    let cancel = AtomicBool::new(false);
    let mut runtime = FileCheckRuntime::new(Instant::now(), &cancel, |_| panic!("no disk IO"));
    assert!(matches!(
        host.run(id, &mut runtime),
        Err(HostError::InvalidInput)
    ));
    let e = host.run_routed(id, &mut runtime).unwrap();
    assert_eq!(e.source, Some(original));
    let result: serde_json::Value = serde_json::from_slice(&e.summary(id)).unwrap();
    assert_eq!(result["source"], preview["source"]);
    assert_eq!(result["receipt"]["session"], format!("{:032x}", 991));
    assert_eq!(result["capacity"], "Unavailable");
    assert_eq!(result["physical_write_bytes"], 0);
    assert!(e.summary(id).len() <= 4096);
}
#[test]
fn foreign_provenance_target_network_and_worker_declarations_are_rejected() {
    for case in 0..5 {
        let mut spec = plan(32 * 1024 * 1024).spec().clone();
        let mut src = source();
        match case {
            0 => spec.revision = 2,
            1 => spec.network = NetworkPlan::RemoteUnmeasured { peer_id: 7 },
            2 => spec.budgets.workers = 2,
            3 => spec.boundary = Boundary::Synced,
            _ => src.report.generated_at_ns = 99, // unchanged rebuilt report is still valid; use foreign conditions
        }
        if case == 4 {
            spec.environment_id = 999;
        }
        let mut host = CheckHost::new(1).unwrap();
        assert!(
            host.register_routed(
                CheckPlan::build(spec).unwrap(),
                if case == 0 { 11 } else { 10 },
                Box::new(SnapshotCheck::new(SNAPSHOT.to_vec(), true).unwrap()),
                Some(src)
            )
            .is_err()
        );
    }
}

#[test]
fn expired_host_clock_stops_admission_before_capture() {
    struct Clock(u64);
    impl CheckRuntime for Clock {
        fn now_ns(&mut self) -> std::io::Result<u64> {
            self.0 += 2_000_000_000;
            Ok(self.0)
        }
        fn cancelled(&self) -> bool {
            false
        }
        fn available_bytes(&mut self, _: &std::path::Path) -> std::io::Result<u64> {
            panic!("no probe")
        }
        fn wait_until(&mut self, _: u64) -> std::io::Result<()> {
            panic!("no pacing")
        }
    }
    let mut host = CheckHost::new(1).unwrap();
    let id = host
        .register_routed(
            plan(32 * 1024 * 1024),
            10,
            Box::new(SnapshotCheck::new(SNAPSHOT.to_vec(), true).unwrap()),
            None,
        )
        .unwrap();
    let e = host.run_routed(id, &mut Clock(0)).unwrap();
    assert_eq!(e.report.accepted, 0);
    assert_eq!(e.report.outcome, CheckOutcome::Inconclusive);
    assert_eq!(e.report.stop, CheckStop::Limit(StopReason::TimeBudget));
}

#[test]
fn an_initial_clock_failure_retains_the_original_os_error_without_claiming_a_time_window() {
    struct BrokenClock;
    impl CheckRuntime for BrokenClock {
        fn now_ns(&mut self) -> std::io::Result<u64> {
            Err(std::io::Error::from_raw_os_error(13))
        }
        fn cancelled(&self) -> bool {
            false
        }
        fn available_bytes(&mut self, _: &std::path::Path) -> std::io::Result<u64> {
            panic!("no IO")
        }
        fn wait_until(&mut self, _: u64) -> std::io::Result<()> {
            panic!("no IO")
        }
    }
    let mut host = CheckHost::new(1).unwrap();
    let id = host
        .register_routed(
            plan(32 * 1024 * 1024),
            10,
            Box::new(SnapshotCheck::new(SNAPSHOT.to_vec(), true).unwrap()),
            None,
        )
        .unwrap();
    let e = host.run_routed(id, &mut BrokenClock).unwrap();
    assert_eq!(e.report.outcome, CheckOutcome::Fail);
    assert_eq!(
        e.report.issue.as_ref().unwrap().error.raw_os_error(),
        Some(13)
    );
    assert_eq!(e.report.from_ns, None);
    assert_eq!(e.report.to_ns, None);
    assert_eq!(e.report.accepted, 0);
}
