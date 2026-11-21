//! Planning starts no workload; admission limits never certify measured results.
#![allow(clippy::unwrap_used)]
use dot_logger::{
    completion::Boundary,
    diagnostics::{
        CheckBudgets, CheckPlan, CheckSpec, NetworkPlan, PlanError, Progress, StopReason,
    },
};
use std::path::{Path, PathBuf};

fn spec() -> CheckSpec {
    CheckSpec {
        session_id: 42,
        revision: 7,
        environment_id: 11,
        workload_id: 22,
        directory: std::env::temp_dir().join("dot-logger-check-plan-only"),
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
fn progress() -> Progress {
    Progress {
        elapsed_ns: 0,
        written_bytes: 0,
        free_disk_bytes: 128 * 1024 * 1024,
        cancelled: false,
    }
}

#[test]
fn dry_run_preserves_provenance_and_operating_files_without_creating_a_directory() {
    let root = std::env::temp_dir().join(format!("dot-check-plan-{}", std::process::id()));
    let _cleanup = Cleanup(root.clone());
    std::fs::create_dir(&root).unwrap();
    let operating = root.join("operating.mcap");
    std::fs::write(&operating, b"operating-records").unwrap();
    let mut input = spec();
    input.directory = root.join("dedicated-check");
    let expected = input.clone();
    let plan = CheckPlan::build(input).unwrap();
    assert_eq!(plan.spec(), &expected);
    assert_eq!(plan.spec().network, NetworkPlan::UnavailableNoReceiver);
    assert!(!plan.spec().directory.exists());
    assert_eq!(std::fs::read(&operating).unwrap(), b"operating-records");
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    std::fs::remove_file(operating).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn invalid_budgets_memory_and_ambiguous_paths_are_rejected() {
    for field in 0..6 {
        let mut input = spec();
        match field {
            0 => input.budgets.duration_ns = 0,
            1 => input.budgets.write_bytes = 0,
            2 => input.budgets.memory_bytes = 0,
            3 => input.budgets.workers = 0,
            4 => input.required_memory_bytes = 0,
            _ => input.required_memory_bytes = input.budgets.memory_bytes + 1,
        }
        assert_eq!(
            CheckPlan::build(input).unwrap_err(),
            PlanError::InvalidBudget
        );
    }
    let temp = std::env::temp_dir();
    let root = temp.ancestors().last().unwrap().to_path_buf();
    for path in [
        PathBuf::new(),
        root,
        PathBuf::from("relative/check"),
        temp.join("..").join("operating"),
    ] {
        let mut input = spec();
        input.directory = path;
        assert_eq!(
            CheckPlan::build(input).unwrap_err(),
            PlanError::InvalidDirectory
        );
    }
    // Native path normalization of '.' is permitted in a declaration.
    let mut input = spec();
    input.directory = temp.join(Path::new(".")).join("check");
    assert!(CheckPlan::build(input).is_ok());
}

#[test]
fn time_and_physical_write_budgets_stop_new_admission_independently() {
    let plan = CheckPlan::build(spec()).unwrap();
    let mut p = progress();
    assert_eq!(plan.write_allowance(p), Ok(32 * 1024 * 1024));
    p.written_bytes = plan.spec().budgets.write_bytes - 1;
    assert_eq!(plan.write_allowance(p), Ok(1));
    p.written_bytes += 1;
    assert_eq!(plan.write_allowance(p), Err(StopReason::WriteBudget));
    p.written_bytes = 0;
    p.elapsed_ns = plan.spec().budgets.duration_ns - 1;
    assert!(plan.write_allowance(p).is_ok());
    p.elapsed_ns += 1;
    assert_eq!(plan.write_allowance(p), Err(StopReason::TimeBudget));
}

#[test]
fn disk_reserve_and_cancellation_prevent_new_work() {
    let plan = CheckPlan::build(spec()).unwrap();
    let mut p = progress();
    p.free_disk_bytes = plan.spec().budgets.disk_reserve_bytes + 3;
    assert_eq!(plan.write_allowance(p), Ok(3));
    p.free_disk_bytes -= 3;
    assert_eq!(plan.write_allowance(p), Err(StopReason::DiskReserve));
    p.free_disk_bytes = 0;
    assert_eq!(plan.write_allowance(p), Err(StopReason::DiskReserve));
    p.cancelled = true;
    assert_eq!(plan.write_allowance(p), Err(StopReason::Cancelled));
}

#[test]
fn extreme_counts_do_not_wrap_and_network_declarations_never_certify_capacity() {
    let mut input = spec();
    input.network = NetworkPlan::LoopbackOnly;
    let plan = CheckPlan::build(input.clone()).unwrap();
    assert_eq!(plan.spec().network, NetworkPlan::LoopbackOnly);
    input.network = NetworkPlan::RemoteUnmeasured { peer_id: 123 };
    input.budgets.write_bytes = u64::MAX;
    input.budgets.duration_ns = u64::MAX;
    input.budgets.disk_reserve_bytes = 0;
    let plan = CheckPlan::build(input).unwrap();
    let mut p = progress();
    p.free_disk_bytes = u64::MAX;
    p.written_bytes = u64::MAX - 1;
    assert_eq!(plan.write_allowance(p), Ok(1));
    p.written_bytes = u64::MAX;
    assert_eq!(plan.write_allowance(p), Err(StopReason::WriteBudget));
    assert_eq!(
        plan.spec().network,
        NetworkPlan::RemoteUnmeasured { peer_id: 123 }
    );
}

/// Removes a temporary path even when the test panics (REV-071 RV-188).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
struct Cleanup(std::path::PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0).or_else(|_| std::fs::remove_file(&self.0));
    }
}
