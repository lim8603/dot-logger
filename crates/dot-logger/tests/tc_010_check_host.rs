//! Host capability lifecycle and actual Linux filesystem observations.
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]
mod support;
use dot_logger::{
    capture::{CaptureOwner, ChannelConfig},
    completion::Boundary,
    diagnostics::{
        CheckBudgets, CheckHost, CheckPlan, CheckSpec, FileCheck, FileWorkload, HostError,
        NetworkPlan, linux_available_bytes,
    },
    routing::Router,
    storage::StorageChannel,
};
use std::{
    fs,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Instant,
};
#[test]
fn plan_is_load_free_run_is_single_use_and_actual_probe_preserves_operating_files() {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "dot-check-host-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _cleanup = Cleanup(path.clone());
    fs::create_dir(&path).unwrap();
    fs::write(path.join("operating"), b"keep").unwrap();
    let plan = support::plan();
    let routing = support::routing();
    let channel = ChannelConfig::new(7, 99, 1);
    let mut owner =
        CaptureOwner::new_tracked(&plan, 42, 7, &[channel], &routing, support::tracking(8))
            .unwrap();
    owner.scope(|scope| {
        let router = Router::new(&plan, scope.map(|(_, r)| r).collect(), routing).unwrap();
        let spec = CheckSpec {
            session_id: 42,
            revision: 7,
            environment_id: 11,
            workload_id: 22,
            directory: path.join("check"),
            budgets: CheckBudgets {
                duration_ns: 10_000_000_000,
                write_bytes: 32 * 1024 * 1024,
                memory_bytes: 16 * 1024 * 1024,
                disk_reserve_bytes: 0,
                workers: 1,
            },
            required_memory_bytes: 8 * 1024 * 1024,
            boundary: Boundary::Synced,
            network: NetworkPlan::UnavailableNoReceiver,
        };
        let mut host = CheckHost::new(1).unwrap();
        let check = FileCheck {
            workload: FileWorkload {
                records: 2,
                payload_bytes: 64,
                interval_ns: 0,
                seed: 17,
            },
            channel: StorageChannel {
                channel,
                topic: "check".into(),
                payload_encoding: "opaque".into(),
                schema: None,
            },
        };
        let id = host
            .register(
                CheckPlan::build(spec).unwrap(),
                router.writer_budget(10).unwrap(),
                check,
                None,
            )
            .unwrap();
        let preview = host.preview(id).unwrap();
        assert!(
            std::str::from_utf8(&preview)
                .unwrap()
                .contains("\"kind\":\"plan\"")
        );
        assert!(!path.join("check").exists());
        assert_eq!(fs::read_dir(&path).unwrap().count(), 1);
        assert!(matches!(host.preview(id + 1), Err(HostError::Gone)));
        let cancel = AtomicBool::new(false);
        let mut runtime = dot_logger::diagnostics::FileCheckRuntime::new(
            Instant::now(),
            &cancel,
            linux_available_bytes,
        );
        let evidence = host.run(id, &mut runtime).unwrap();
        assert_eq!(
            evidence.report.outcome,
            dot_logger::diagnostics::CheckOutcome::Pass,
            "{:?}",
            evidence.report
        );
        assert!(evidence.source.is_none());
        assert!(evidence.report.cleaned);
        assert!(matches!(host.preview(id), Err(HostError::Gone)));
        assert!(matches!(host.run(id, &mut runtime), Err(HostError::Gone)));
        assert_eq!(fs::read(path.join("operating")).unwrap(), b"keep");
        assert_eq!(fs::read_dir(&path).unwrap().count(), 1);
    });
    fs::remove_file(path.join("operating")).unwrap();
    fs::remove_dir(&path).unwrap();
}
#[test]
fn actual_probe_errors_and_host_limits_are_explicit() {
    assert!(CheckHost::new(0).is_err());
    assert!(CheckHost::new(1025).is_err());
    assert!(linux_available_bytes(std::path::Path::new("/nonexistent-dot-check-probe")).is_err());
    #[cfg(target_os = "linux")]
    assert!(linux_available_bytes(std::path::Path::new("/tmp")).is_ok());
}

fn source() -> dot_logger::diagnostics::CheckSource {
    use dot_logger::telemetry::{CaptureCounts, Window, capacity::*, report::*};
    let conditions = Conditions {
        environment_id: 11,
        workload_id: 22,
        revision: 7,
        target_id: 10,
        boundary: Boundary::Synced,
    };
    let window = Window {
        session_id: 4242,
        revision: 7,
        epoch: 3,
        from_ns: 1,
        to_ns: 2,
        counts: CaptureCounts::default(),
    };
    let profile = Profile {
        session_id: 4242,
        id: 99,
        conditions,
        valid_from_ns: 0,
        valid_until_ns: 1,
        completed_records: Rate {
            amount: 1,
            elapsed_ns: 1,
        },
    };
    dot_logger::diagnostics::CheckSource {
        report: Report::build(
            ReportInput::from_window(window, conditions),
            Some(profile),
            10,
        ),
        profile: Some(profile),
    }
}
#[test]
fn source_is_preserved_expired_profiles_are_not_refreshed_and_slots_do_not_alias() {
    use dot_logger::diagnostics::{CheckOutcome, CheckSource, CheckStop, StopReason};
    let plan = support::plan();
    let routing = support::routing();
    let channel = ChannelConfig::new(7, 99, 1);
    let mut owner =
        CaptureOwner::new_tracked(&plan, 42, 7, &[channel], &routing, support::tracking(8))
            .unwrap();
    owner.scope(|scope| {
        let router = Router::new(&plan, scope.map(|(_, r)| r).collect(), routing).unwrap();
        let mut host = CheckHost::new(1).unwrap();
        let register = |host: &mut CheckHost, source: Option<CheckSource>| {
            host.register(
                CheckPlan::build(CheckSpec {
                    session_id: 42,
                    revision: 7,
                    environment_id: 11,
                    workload_id: 22,
                    directory: std::env::temp_dir().join("dot-host-no-io"),
                    budgets: CheckBudgets {
                        duration_ns: 1,
                        write_bytes: 1,
                        memory_bytes: 16 * 1024 * 1024,
                        disk_reserve_bytes: 0,
                        workers: 1,
                    },
                    required_memory_bytes: 8 * 1024 * 1024,
                    boundary: Boundary::Synced,
                    network: NetworkPlan::UnavailableNoReceiver,
                })
                .unwrap(),
                router.writer_budget(10).unwrap(),
                FileCheck {
                    workload: FileWorkload {
                        records: 1,
                        payload_bytes: 1,
                        interval_ns: 0,
                        seed: 1,
                    },
                    channel: StorageChannel {
                        channel,
                        topic: "check".into(),
                        payload_encoding: "opaque".into(),
                        schema: None,
                    },
                },
                source,
            )
        };
        for field in 0..7 {
            let mut invalid = source();
            match field {
                0 => invalid.report.window.session_id += 1,
                1 => invalid.report.conditions.environment_id += 1,
                2 => invalid.report.conditions.workload_id += 1,
                3 => invalid.report.conditions.target_id += 1,
                4 => invalid.report.conditions.boundary = Boundary::Written,
                5 => invalid.report.window.revision += 1,
                _ => invalid.report.measurements.window.epoch += 1,
            }
            assert!(matches!(
                register(&mut host, Some(invalid)),
                Err(HostError::InvalidInput)
            ));
        }
        let original = source();
        let id = register(&mut host, Some(original)).unwrap();
        assert!(matches!(register(&mut host, None), Err(HostError::Limit)));
        let preview = host.preview(id).unwrap();
        assert!(preview.len() < 4096);
        let cancel = AtomicBool::new(true);
        let mut runtime = dot_logger::diagnostics::FileCheckRuntime::new(
            Instant::now(),
            &cancel,
            |_: &std::path::Path| -> std::io::Result<u64> {
                panic!("cancelled run must not probe")
            },
        );
        let evidence = host.run(id, &mut runtime).unwrap();
        assert_eq!(evidence.report.outcome, CheckOutcome::Aborted);
        assert_eq!(
            evidence.report.stop,
            CheckStop::Limit(StopReason::Cancelled)
        );
        assert_eq!(evidence.source, Some(original));
        assert_eq!(evidence.source.unwrap().report.window.session_id, 4242);
        assert_eq!(evidence.report.plan.spec().session_id, 42);
        let summary = evidence.summary(id);
        assert!(summary.len() < 4096);
        assert!(
            String::from_utf8(summary)
                .unwrap()
                .contains("\"capacity\":\"Unavailable\"")
        );
        let next = register(&mut host, None).unwrap();
        assert!(next > id);
        assert!(matches!(host.preview(id), Err(HostError::Gone)));
        assert!(host.preview(next).is_ok());
    });
}

/// Removes a temporary path even when the test panics (REV-071 RV-188).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
struct Cleanup(std::path::PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0).or_else(|_| std::fs::remove_file(&self.0));
    }
}
