//! Bounded host-owned, single-use check capabilities. No automatic worker.
#[cfg(feature = "remote")]
use super::HostCheckEvidence;
use super::{
    CheckPlan, CheckReport, CheckRuntime, CheckStartError, FileCheck, RoutedCheckBackend,
    RoutedCheckEvidence,
};
use crate::{
    storage::WriterBudget,
    telemetry::{capacity::Profile, report::Report},
};
use std::{io, path::Path};

/// Original runtime observation and optional external capacity evidence. A short
/// file check never creates or refreshes either measurement or validation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CheckSource {
    /// Unmodified runtime report, including original window and conditions.
    pub report: Report,
    /// Original external validation, with its own validity and conditions.
    pub profile: Option<Profile>,
}
/// Actual file results together with the original source evidence, if supplied.
#[derive(Debug)]
pub struct CheckEvidence {
    /// Result returned after actual IO settlement and cleanup attempts.
    pub report: CheckReport,
    /// Original source; never filled from synthetic file counters.
    pub source: Option<CheckSource>,
}
/// Capability/registration errors. Operational IO issues stay in CheckReport.
#[derive(Debug)]
pub enum HostError {
    /// Zero/too many slots, zero ID or inconsistent source/writer identity.
    InvalidInput,
    /// No unused slot or monotonically increasing capability ID remains.
    Limit,
    /// Unknown or already consumed capability; never automatically retried.
    Gone,
    /// Runner rejected the consumed plan before IO.
    Start(CheckStartError),
}
impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "check host: {self:?}")
    }
}
impl std::error::Error for HostError {}
struct Registered {
    id: u64,
    plan: CheckPlan,
    runner: Runner,
    source: Option<CheckSource>,
}
enum Runner {
    File {
        budget: WriterBudget,
        check: Box<FileCheck>,
    },
    Routed {
        target: u32,
        backend: Box<dyn RoutedCheckBackend>,
    },
}
/// Fixed registration slots owned and driven by a general-path host. Allocate
/// host metadata separately from each plan's execution budget. The host supplies
/// trusted directories and dedicated writer authorities; remote requests supply
/// only opaque IDs. IDs are local to this instance and never reused within it.
pub struct CheckHost {
    slots: Vec<Option<Registered>>,
    next: u64,
}
impl CheckHost {
    /// Allocate 1..=1024 slots; registration cannot grow the slot array.
    pub fn new(slots: usize) -> Result<Self, HostError> {
        if !(1..=1024).contains(&slots) {
            return Err(HostError::InvalidInput);
        }
        Ok(Self {
            slots: (0..slots).map(|_| None).collect(),
            next: 1,
        })
    }
    /// Register explicit host-owned declarations without IO or clock/probe work.
    /// Source reports must match revision/environment/workload, target and boundary.
    /// The source session/clock may differ from the dedicated check session; its
    /// original identity/times are retained without cross-clock arithmetic. An optional profile is retained verbatim, including
    /// expired/mismatched evidence; normal Report provenance rules still apply.
    pub fn register(
        &mut self,
        plan: CheckPlan,
        budget: WriterBudget,
        check: FileCheck,
        source: Option<CheckSource>,
    ) -> Result<u64, HostError> {
        let s = plan.spec();
        if s.session_id != budget.session || s.revision != budget.revision {
            return Err(HostError::InvalidInput);
        }
        validate_source(&plan, budget.target, source)?;
        let slot = self
            .slots
            .iter_mut()
            .find(|s| s.is_none())
            .ok_or(HostError::Limit)?;
        let id = self.next;
        self.next = id.checked_add(1).ok_or(HostError::Limit)?;
        *slot = Some(Registered {
            id,
            plan,
            runner: Runner::File {
                budget,
                check: Box::new(check),
            },
            source,
        });
        Ok(id)
    }
    /// Load-free bounded JSON preview; never creates/resolves/checks a path and
    /// never probes actual capability. No internal filesystem path is disclosed.
    pub fn preview(&self, id: u64) -> Result<Vec<u8>, HostError> {
        let r = self
            .slots
            .iter()
            .flatten()
            .find(|r| r.id == id)
            .ok_or(HostError::Gone)?;
        let (budget, check) = match &r.runner {
            Runner::File { budget, check } => (budget, check),
            Runner::Routed { target, .. } => {
                let s = r.plan.spec();
                return Ok(format!(concat!("{{\"schema\":\"dot_logger.check.v1\",\"kind\":\"plan\",\"plan\":\"{}\",",
                    "\"scope\":\"production_capture_loopback_snapshot\",\"session\":\"{:032x}\",\"revision\":\"{}\",",
                    "\"environment\":\"{:032x}\",\"workload\":\"{:032x}\",\"target\":{},\"boundary\":\"{:?}\",",
                    "\"network\":\"{:?}\",\"duration_ns\":\"{}\",\"write_bytes\":\"{}\",\"memory_bytes\":\"{}\",",
                    "\"required_memory_bytes\":\"{}\",\"disk_reserve_bytes\":\"{}\",\"workers\":{},",
                    "\"records\":1,\"capability\":\"Unprobed\",\"capacity\":\"Unavailable\",",
                    "\"network_measurement\":\"Unavailable\",\"source\":{}}}"),
                    id,s.session_id,s.revision,s.environment_id,s.workload_id,target,s.boundary,s.network,
                    s.budgets.duration_ns,s.budgets.write_bytes,s.budgets.memory_bytes,s.required_memory_bytes,
                    s.budgets.disk_reserve_bytes,s.budgets.workers,source_json(r.source))
                    .into_bytes().into_boxed_slice().into_vec());
            }
        };
        let s = r.plan.spec();
        let b = s.budgets;
        Ok(format!(concat!("{{\"schema\":\"dot_logger.check.v1\",\"kind\":\"plan\",\"plan\":\"{}\",",
            "\"session\":\"{:032x}\",\"revision\":\"{}\",\"environment\":\"{:032x}\",\"workload\":\"{:032x}\",",
            "\"target\":{},\"boundary\":\"{:?}\",\"network\":\"{:?}\",\"duration_ns\":\"{}\",",
            "\"write_bytes\":\"{}\",\"memory_bytes\":\"{}\",\"required_memory_bytes\":\"{}\",",
            "\"disk_reserve_bytes\":\"{}\",\"workers\":{},\"records\":{},\"payload_bytes\":\"{}\",",
            "\"interval_ns\":\"{}\",\"seed\":{},\"scope\":\"synthetic_file\",",
            "\"capability\":\"Unprobed\",\"network_measurement\":\"Unavailable\",\"capacity\":\"Unavailable\",",
            "\"source\":{}}}"),
            id,s.session_id,s.revision,s.environment_id,s.workload_id,budget.target,s.boundary,s.network,
            b.duration_ns,b.write_bytes,b.memory_bytes,s.required_memory_bytes,b.disk_reserve_bytes,b.workers,
            check.workload.records,check.workload.payload_bytes,check.workload.interval_ns,check.workload.seed,
            source_json(r.source)).into_bytes().into_boxed_slice().into_vec())
    }
    /// Consume the capability before preflight/IO. Even errors and cancellation
    /// require a newly registered explicit plan for any subsequent run.
    pub fn run<R: CheckRuntime>(
        &mut self,
        id: u64,
        runtime: &mut R,
    ) -> Result<CheckEvidence, HostError> {
        let slot = self
            .slots
            .iter_mut()
            .find(|s| s.as_ref().is_some_and(|r| r.id == id))
            .ok_or(HostError::Gone)?;
        if !matches!(slot.as_ref().map(|r| &r.runner), Some(Runner::File { .. })) {
            return Err(HostError::InvalidInput);
        }
        let r = slot.take().ok_or(HostError::Gone)?;
        let Runner::File { budget, check } = r.runner else {
            return Err(HostError::InvalidInput);
        };
        let report = r
            .plan
            .run_file(budget, *check, runtime)
            .map_err(HostError::Start)?;
        Ok(CheckEvidence {
            report,
            source: r.source,
        })
    }
    /// Register a trusted finite production backend without running it. Remote
    /// callers supply only this host's opaque capability, never input or paths.
    pub fn register_routed(
        &mut self,
        plan: CheckPlan,
        target: u32,
        backend: Box<dyn RoutedCheckBackend>,
        source: Option<CheckSource>,
    ) -> Result<u64, HostError> {
        let s = plan.spec();
        if s.session_id == 0
            || s.revision == 0
            || s.budgets.workers != 1
            || s.boundary != crate::completion::Boundary::ReceiverAcknowledged
            || s.network != super::NetworkPlan::LoopbackOnly
            || backend.target_id() != target
            || backend.required_memory_bytes() == 0
            || backend.required_memory_bytes() > s.required_memory_bytes
        {
            return Err(HostError::InvalidInput);
        }
        validate_source(&plan, target, source)?;
        let slot = self
            .slots
            .iter_mut()
            .find(|s| s.is_none())
            .ok_or(HostError::Limit)?;
        let id = self.next;
        self.next = id.checked_add(1).ok_or(HostError::Limit)?;
        *slot = Some(Registered {
            id,
            plan,
            runner: Runner::Routed { target, backend },
            source,
        });
        Ok(id)
    }
    /// Consume one routed capability before running; return only after backend
    /// IO settlement. Wrong-backend calls leave the capability untouched.
    pub fn run_routed<R: CheckRuntime>(
        &mut self,
        id: u64,
        runtime: &mut R,
    ) -> Result<RoutedCheckEvidence, HostError> {
        let slot = self
            .slots
            .iter_mut()
            .find(|s| s.as_ref().is_some_and(|r| r.id == id))
            .ok_or(HostError::Gone)?;
        if !matches!(
            slot.as_ref().map(|r| &r.runner),
            Some(Runner::Routed { .. })
        ) {
            return Err(HostError::InvalidInput);
        }
        let r = slot.take().ok_or(HostError::Gone)?;
        let Runner::Routed { target, backend } = r.runner else {
            return Err(HostError::InvalidInput);
        };
        let report = backend.run(&r.plan, runtime).map_err(HostError::Start)?;
        Ok(RoutedCheckEvidence {
            plan: r.plan,
            target,
            report,
            source: r.source,
        })
    }
}
fn validate_source(
    plan: &CheckPlan,
    target: u32,
    source: Option<CheckSource>,
) -> Result<(), HostError> {
    let s = plan.spec();
    if let Some(source) = source {
        let r = source.report;
        let c = r.conditions;
        if r.window.revision != s.revision
            || c.revision != s.revision
            || c.environment_id != s.environment_id
            || c.workload_id != s.workload_id
            || c.target_id != target
            || c.boundary != s.boundary
            || r.window != r.measurements.window
            || r.conditions != r.measurements.conditions
            || r != Report::build(r.measurements, source.profile, r.generated_at_ns)
        {
            return Err(HostError::InvalidInput);
        }
    }
    Ok(())
}
pub(super) fn source_json(source: Option<CheckSource>) -> String {
    match source {
        None => "null".into(),
        Some(s) => format!(concat!("{{\"session\":\"{:032x}\",\"revision\":\"{}\",",
            "\"epoch\":\"{}\",\"from_ns\":\"{}\",\"to_ns\":\"{}\",\"generated_at_ns\":\"{}\",",
            "\"conditions\":{},\"profile\":{}}}"),
            s.report.window.session_id,s.report.window.revision,s.report.window.epoch,
            s.report.window.from_ns,s.report.window.to_ns,s.report.generated_at_ns,conditions_json(s.report.conditions),
            s.profile.map_or_else(|| "null".into(), |p| format!(concat!("{{\"id\":\"{:032x}\",\"session\":\"{:032x}\",",
                "\"valid_from_ns\":\"{}\",\"valid_until_ns\":\"{}\",\"conditions\":{},",
                "\"completed_records\":\"{}\",\"elapsed_ns\":\"{}\",\"reused_as_check_capacity\":false}}"),
                p.id,p.session_id,p.valid_from_ns,p.valid_until_ns,conditions_json(p.conditions),
                p.completed_records.amount,p.completed_records.elapsed_ns))),
    }
}
fn conditions_json(c: crate::telemetry::capacity::Conditions) -> String {
    format!(
        concat!(
            "{{\"environment\":\"{:032x}\",\"workload\":\"{:032x}\",",
            "\"revision\":\"{}\",\"target\":{},\"boundary\":\"{:?}\"}}"
        ),
        c.environment_id, c.workload_id, c.revision, c.target_id, c.boundary
    )
}
impl CheckEvidence {
    /// Bounded host summary; native evidence retains complete source and original
    /// IO errors. No error strings, paths or inferred capacity cross transport.
    /// Return exact-length capacity so transport checks cannot reject spare
    /// formatter allocation capacity after the check has already run.
    pub fn summary(&self, id: u64) -> Vec<u8> {
        let r = &self.report;
        let s = r.plan.spec();
        format!(concat!("{{\"schema\":\"dot_logger.check.v1\",\"kind\":\"result\",\"plan\":\"{}\",",
            "\"scope\":\"synthetic_file\",\"outcome\":\"{:?}\",\"stop\":\"{:?}\",",
            "\"session\":\"{:032x}\",\"revision\":\"{}\",\"environment\":\"{:032x}\",\"workload\":\"{:032x}\",",
            "\"target\":{},\"boundary\":\"{:?}\",\"written_records\":{},\"synced_records\":{},",
            "\"duration_ns\":\"{}\",\"write_budget_bytes\":\"{}\",\"memory_budget_bytes\":\"{}\",",
            "\"disk_reserve_bytes\":\"{}\",\"workers\":{},\"requested_records\":{},\"payload_bytes\":\"{}\",",
            "\"interval_ns\":\"{}\",\"seed\":{},",
            "\"physical_write_bytes\":\"{}\",\"file_complete\":{},\"cleaned\":{},",
            "\"from_ns\":{},\"to_ns\":{},\"file_bytes\":{},",
            "\"primary_issue\":{},\"settlement_issue\":{},\"cleanup_issue\":{},",
            "\"capacity\":\"Unavailable\",\"network_measurement\":\"Unavailable\",\"source\":{}}}"),
            id,r.outcome,r.stop,s.session_id,s.revision,s.environment_id,s.workload_id,r.target_id,s.boundary,
            r.written_records,r.synced_records,s.budgets.duration_ns,s.budgets.write_bytes,s.budgets.memory_bytes,
            s.budgets.disk_reserve_bytes,s.budgets.workers,r.workload.records,r.workload.payload_bytes,r.workload.interval_ns,r.workload.seed,
            r.physical_write_bytes,r.file_complete,r.cleaned,
            optional_number(r.from_ns),optional_number(r.to_ns),optional_number(r.file_bytes),
            stage(&r.primary_issue),stage(&r.settlement_issue),stage(&r.cleanup_issue),source_json(self.source)).into_bytes().into_boxed_slice().into_vec()
    }
}
pub(super) fn optional_number(n: Option<u64>) -> String {
    n.map_or_else(|| "null".into(), |n| format!("\"{n}\""))
}
pub(super) fn stage(issue: &Option<super::CheckIssue>) -> String {
    issue.as_ref().map_or_else(
        || "null".into(),
        |i| {
            format!(
                "{{\"stage\":\"{:?}\",\"kind\":\"{:?}\",\"os_code\":{}}}",
                i.stage,
                i.error.kind(),
                i.error
                    .raw_os_error()
                    .map_or_else(|| "null".into(), |n| n.to_string())
            )
        },
    )
}

/// Fresh Linux filesystem space via a safe statvfs wrapper. This ordinary-path
/// syscall does no worker/process startup and retains original OS errors. Other
/// platforms return Unsupported, never zero or infinite capacity. Available
/// non-root blocks * fragment size is a snapshot, not a quota reservation.
pub fn linux_available_bytes(path: &Path) -> io::Result<u64> {
    #[cfg(target_os = "linux")]
    {
        let s = rustix::fs::statvfs(path)?;
        space_bytes(s.f_bavail, s.f_frsize, s.f_blocks)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = path;
        Err(io::ErrorKind::Unsupported.into())
    }
}
/// Fresh free space for an unprivileged writer below `path`, on every
/// supported OS: `statvfs` on Unix (Linux and macOS), `GetDiskFreeSpaceExW`
/// through `fs4` on Windows. Other platforms return Unsupported, never zero
/// or infinite capacity. A snapshot, not a quota reservation.
pub fn available_bytes(path: &Path) -> io::Result<u64> {
    #[cfg(unix)]
    {
        let s = rustix::fs::statvfs(path)?;
        space_bytes(s.f_bavail, s.f_frsize, s.f_blocks)
    }
    #[cfg(windows)]
    {
        fs4::available_space(path)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        Err(io::ErrorKind::Unsupported.into())
    }
}
#[cfg(unix)]
fn space_bytes(available: u64, fragment: u64, total: u64) -> io::Result<u64> {
    // Reject impossible/wrapped filesystem counters rather than treating them
    // as enormous free space. Fragment size is the statvfs block-count unit.
    if fragment == 0 || available > total {
        return Err(io::ErrorKind::InvalidData.into());
    }
    available
        .checked_mul(fragment)
        .ok_or_else(|| io::ErrorKind::InvalidData.into())
}
#[cfg(all(test, any(unix, windows)))]
#[allow(clippy::unwrap_used)]
mod portable_tests {
    #[test]
    fn available_bytes_reads_an_existing_directory_and_rejects_a_missing_one() {
        let dir = std::env::temp_dir();
        let free = super::available_bytes(&dir).unwrap();
        assert!(
            free > 0,
            "a writable temp directory reports some free space"
        );
        assert!(super::available_bytes(&dir.join("dot-logger-no-such-dir/x")).is_err());
        #[cfg(target_os = "linux")]
        {
            // Same statvfs source as the Linux-only probe, within ongoing writes.
            let linux = super::linux_available_bytes(&dir).unwrap();
            assert!(free.abs_diff(linux) < 1 << 30);
        }
    }
}
#[cfg(all(test, target_os = "linux"))]
#[allow(clippy::unwrap_used)]
mod tests {
    #[test]
    fn space_arithmetic_rejects_impossible_or_overflowing_counters() {
        assert_eq!(super::space_bytes(12, 4096, 100).unwrap(), 49152);
        assert_eq!(super::space_bytes(0, 4096, 100).unwrap(), 0);
        for (available, fragment, total) in [(1, 0, 1), (2, 1, 1), (u64::MAX, 4096, u64::MAX)] {
            assert!(super::space_bytes(available, fragment, total).is_err());
        }
    }
}

#[cfg(feature = "remote")]
impl CheckHost {
    /// Drive one admitted native management command synchronously on the host's
    /// general worker. Retain Command/permit until actual run settlement; remote
    /// waiter closure only stops new load. Return native evidence for host storage
    /// even when a late transport reply is discarded. Other operations are Unsupported.
    pub fn handle_command<R: CheckRuntime>(
        &mut self,
        command: crate::adapters::remote::Command,
        runtime: &mut R,
    ) -> Option<HostCheckEvidence> {
        use crate::adapters::remote::{ManagementReply, ManagementRequest, RemoteError};
        if command.response_capacity() < 4096 {
            command.respond(Err(RemoteError::Limit));
            return None;
        }
        let id = match command.request() {
            ManagementRequest::PlanCheck { plan } => {
                let reply = self
                    .preview(*plan)
                    .map(ManagementReply::Report)
                    .map_err(remote_error);
                command.respond(reply);
                return None;
            }
            ManagementRequest::RunCheck { plan } => *plan,
            _ => {
                command.respond(Err(RemoteError::Unsupported));
                return None;
            }
        };
        let routed = self
            .slots
            .iter()
            .flatten()
            .any(|r| r.id == id && matches!(r.runner, Runner::Routed { .. }));
        let mut runtime = CommandRuntime {
            command: &command,
            runtime,
        };
        let result = if routed {
            self.run_routed(id, &mut runtime)
                .map(HostCheckEvidence::Routed)
        } else {
            self.run(id, &mut runtime).map(HostCheckEvidence::File)
        };
        match result {
            Ok(evidence) => {
                command.respond(Ok(ManagementReply::Report(evidence.summary(id))));
                Some(evidence)
            }
            Err(e) => {
                command.respond(Err(remote_error(e)));
                None
            }
        }
    }
}
#[cfg(feature = "remote")]
fn remote_error(e: HostError) -> crate::adapters::remote::RemoteError {
    use crate::adapters::remote::RemoteError;
    match e {
        HostError::Gone => RemoteError::Gone,
        HostError::Limit => RemoteError::Limit,
        HostError::InvalidInput => RemoteError::InvalidInput,
        HostError::Start(
            CheckStartError::UnsupportedWorkers | CheckStartError::UnsupportedPlatform,
        ) => RemoteError::Unsupported,
        HostError::Start(CheckStartError::MemoryBudget { .. }) => RemoteError::Limit,
        HostError::Start(_) => RemoteError::InvalidInput,
    }
}
#[cfg(feature = "remote")]
struct CommandRuntime<'a, R> {
    command: &'a crate::adapters::remote::Command,
    runtime: &'a mut R,
}
#[cfg(feature = "remote")]
impl<R: CheckRuntime> CheckRuntime for CommandRuntime<'_, R> {
    fn now_ns(&mut self) -> io::Result<u64> {
        self.runtime.now_ns()
    }
    fn cancelled(&self) -> bool {
        self.command.cancelled() || self.runtime.cancelled()
    }
    fn available_bytes(&mut self, path: &Path) -> io::Result<u64> {
        self.runtime.available_bytes(path)
    }
    fn wait_until(&mut self, at: u64) -> io::Result<()> {
        // Underlying waits are split so remote closure is observed during pacing,
        // without interpreting cancellation as forced termination of OS IO.
        while !self.cancelled() {
            let now = self.runtime.now_ns()?;
            if now >= at {
                return Ok(());
            }
            let next = now.saturating_add(10_000_000).min(at);
            self.runtime.wait_until(next)?;
            if self.runtime.now_ns()? < next && !self.cancelled() {
                return Err(io::ErrorKind::InvalidData.into());
            }
        }
        Ok(())
    }
}
