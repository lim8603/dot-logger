//! TC-013 F11 (TASK-027 S3): worker CPU placement. A separate test binary so
//! `/proc/self/task` lists only this logger's worker threads.
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use dot_logger::logger::{
    Budget, EncodeError, Logger, Record, Retention, Schema, Shutdown, WriterQuota,
};
use std::time::Duration;

const MIB: usize = 1024 * 1024;

struct Pose;
impl Record for Pose {
    const SCHEMA: Schema = Schema::new(0x504f_5345, 1, "le-i32x2");
    fn encode(&self, _: &mut [u8]) -> Result<usize, EncodeError> {
        Ok(0)
    }
}

fn budget() -> Budget {
    Budget::new()
        .memory_bytes(64 * MIB)
        .sync_every(Duration::from_millis(20))
        .drop_deadline(Duration::from_secs(3))
        .writer_quota(WriterQuota::small(64, 64))
}

fn worker_cpu_lists() -> Vec<(String, String)> {
    let mut lists = Vec::new();
    for task in std::fs::read_dir("/proc/self/task").unwrap() {
        let task = task.unwrap().path();
        let Ok(comm) = std::fs::read_to_string(task.join("comm")) else {
            continue;
        };
        if !comm.starts_with("dot-logger-") {
            continue;
        }
        let Ok(status) = std::fs::read_to_string(task.join("status")) else {
            continue;
        };
        let cpus = status
            .lines()
            .find_map(|l| l.strip_prefix("Cpus_allowed_list:"))
            .unwrap()
            .trim()
            .to_string();
        lists.push((comm.trim().to_string(), cpus));
    }
    lists
}

#[test]
fn f11_required_worker_cpus_pin_every_worker_thread() {
    use dot_logger::platform::Requirement;
    let allowed = std::fs::read_to_string("/proc/self/status").unwrap();
    let allowed = allowed
        .lines()
        .find_map(|l| l.strip_prefix("Cpus_allowed_list:"))
        .unwrap()
        .trim()
        .to_string();
    // The first CPU this process may use, e.g. "0-15" or "3,5".
    let cpu: usize = allowed.split([',', '-']).next().unwrap().parse().unwrap();
    let temp = std::env::temp_dir().join(format!("dot-logger-tc013-f11-{}", std::process::id()));
    std::fs::create_dir_all(&temp).unwrap();
    let logger = Logger::builder()
        .budget(budget().worker_cpus(&[cpu], Requirement::Required))
        .file_output(&temp, Retention::ring(256 * MIB as u64))
        .channel::<Pose>("robot/pose")
        .writer("control")
        .start()
        .unwrap();
    assert_eq!(logger.health().affinity, Some(true));
    let workers = worker_cpu_lists();
    assert!(
        workers.iter().any(|(n, _)| n == "dot-logger-host")
            && workers.iter().any(|(n, _)| n == "dot-logger-io"),
        "{workers:?}"
    );
    for (name, cpus) in &workers {
        assert_eq!(cpus, &cpu.to_string(), "{name} is not pinned");
    }
    let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(10)).unwrap() else {
        panic!("shutdown did not close");
    };
    assert!(report.is_success(), "{report:?}");
    std::fs::remove_dir_all(&temp).unwrap();
}
