//! General-path extension point for a trusted, host-registered routing check.
use super::{CheckOutcome, CheckPlan, CheckRuntime, CheckSource, CheckStartError, CheckStop};
use crate::capture::CaptureReceipt;

/// Trusted host backend, never selected or constructed from remote input.
/// Implementations must account for retained input, buffers, runtime and queue
/// memory, stop new admission on deadline/cancellation, and close actual IO before
/// returning. A receiver ACK must refer to the original production receipt.
pub trait RoutedCheckBackend: Send {
    /// Fixed target actually exercised by this backend.
    fn target_id(&self) -> u32;
    /// Conservative memory requirement including retained host input/metadata.
    fn required_memory_bytes(&self) -> usize;
    /// Explicitly execute once on a general host worker, settling even on errors.
    fn run(
        self: Box<Self>,
        plan: &CheckPlan,
        runtime: &mut dyn CheckRuntime,
    ) -> Result<RoutedCheckReport, CheckStartError>;
}
/// Actual production capture/loopback handoff evidence; never a capacity profile.
#[derive(Debug)]
pub struct RoutedCheckReport {
    /// Functional result after IO close and reservation release.
    pub outcome: CheckOutcome,
    /// Original operational error after actual backend settlement, if any.
    pub issue: Option<super::CheckIssue>,
    /// Reason for stopping admission.
    pub stop: CheckStop,
    /// Original production ingress receipt, absent when no record was admitted.
    pub receipt: Option<CaptureReceipt>,
    /// Original receipt completion after settlement; absent if unobserved.
    pub completion: Option<crate::completion::QueryOutcome>,
    /// Original session-monotonic observation before execution.
    pub from_ns: Option<u64>,
    /// Original session-monotonic observation after actual IO closure.
    pub to_ns: Option<u64>,
    /// Records admitted by the actual capture producer.
    pub accepted: u64,
    /// Original receipts confirmed at ReceiverAcknowledged.
    pub acknowledged: u64,
    /// Raw slots actually reclaimed after delivery settlement.
    pub raw_reclaimed: usize,
    /// External reservation still outstanding after IO close.
    pub external_bytes_after_close: usize,
}
/// Routed result together with unchanged host plan and runtime evidence.
#[derive(Debug)]
pub struct RoutedCheckEvidence {
    /// Original host declarations.
    pub plan: CheckPlan,
    /// Host-owned target identity.
    pub target: u32,
    /// Actual execution result.
    pub report: RoutedCheckReport,
    /// Original runtime Report/profile, never synthesized from this check.
    pub source: Option<CheckSource>,
}
/// Result of a management command; host retains native evidence after reply loss.
#[derive(Debug)]
pub enum HostCheckEvidence {
    /// Dedicated synthetic file IO, sync and cleanup.
    File(super::CheckEvidence),
    /// Actual production capture, routing and receiver handoff.
    Routed(RoutedCheckEvidence),
}
impl HostCheckEvidence {
    /// Bounded summary after actual settlement, with explicit scope.
    pub fn summary(&self, id: u64) -> Vec<u8> {
        match self {
            Self::File(e) => e.summary(id),
            Self::Routed(e) => e.summary(id),
        }
    }
}
impl RoutedCheckEvidence {
    /// Fixed-shape transport summary. Paths/error strings and capacity guesses
    /// are excluded; original source evidence is preserved by the native value.
    pub fn summary(&self, id: u64) -> Vec<u8> {
        let r = &self.report;
        let s = self.plan.spec();
        let receipt = r.receipt.map_or_else(
            || "null".into(),
            |r| {
                format!(
                    "{{\"session\":\"{:032x}\",\"producer\":{},\"sequence\":\"{}\"}}",
                    r.session_id, r.producer_id, r.accepted_seq
                )
            },
        );
        format!(concat!("{{\"schema\":\"dot_logger.check.v1\",\"kind\":\"result\",\"plan\":\"{}\",",
            "\"scope\":\"production_capture_loopback_snapshot\",\"outcome\":\"{:?}\",\"stop\":\"{:?}\",",
            "\"session\":\"{:032x}\",\"revision\":\"{}\",\"environment\":\"{:032x}\",\"workload\":\"{:032x}\",",
            "\"target\":{},\"boundary\":\"{:?}\",\"network\":\"{:?}\",\"duration_ns\":\"{}\",",
            "\"memory_budget_bytes\":\"{}\",\"write_budget_bytes\":\"{}\",\"disk_reserve_bytes\":\"{}\",\"workers\":{},",
            "\"from_ns\":{},\"to_ns\":{},\"receipt\":{},\"completion\":{},\"accepted\":{},\"acknowledged\":{},",
            "\"raw_reclaimed\":{},\"external_bytes_after_close\":{},\"physical_write_bytes\":0,",
            "\"source_freshness\":\"Unavailable\",\"capacity\":\"Unavailable\",\"network_measurement\":\"Unavailable\",",
            "\"rss_measurement\":\"Unavailable\",\"primary_issue\":{},\"source\":{}}}"),
            id,r.outcome,r.stop,s.session_id,s.revision,s.environment_id,s.workload_id,self.target,s.boundary,s.network,
            s.budgets.duration_ns,s.budgets.memory_bytes,s.budgets.write_bytes,s.budgets.disk_reserve_bytes,s.budgets.workers,
            super::host::optional_number(r.from_ns),super::host::optional_number(r.to_ns),receipt,r.completion.map_or_else(|| "null".into(), |c| format!("\"{c:?}\"")),r.accepted,r.acknowledged,r.raw_reclaimed,r.external_bytes_after_close,
            super::host::stage(&r.issue), super::host::source_json(self.source)).into_bytes().into_boxed_slice().into_vec()
    }
}
