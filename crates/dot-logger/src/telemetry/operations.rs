//! Host-driven remote operations backend for status, explicit apply and compare.
//!
//! The host owns the instance, drives every command on its general worker and
//! keeps sampling `Telemetry` once per second itself; no worker or timer starts
//! here. Remote callers supply only a revision precondition, a bounded patch and
//! opaque snapshot IDs. Snapshots are host-built reports; this module never
//! creates measurements, capacity or estimates.
use super::{
    CaptureCounts, Observation, Sample, Telemetry, Unavailable, Window,
    capacity::{Basis, Value},
    compare::{Comparison, Direction, RateChange, Snapshot},
};
use crate::{
    adapters::remote::{Command, ManagementReply, ManagementRequest, RemoteError},
    capture::Level,
    completion::CompletionTotals,
    control::config_update::{ChannelPatch, ConfigPatch, ConfigUpdateError, ConfigUpdates},
    routing::Route,
};
use serde_json::{Map, Value as Json, json};

/// Host-owned schema accepted by ApplyConfig.
pub const PATCH_SCHEMA: &str = "dot_logger.config_patch.v1";
/// Host-owned schema returned in the Status report.
pub const STATUS_SCHEMA: &str = "dot_logger.status.v1";
/// Host-owned schema returned by Compare.
pub const COMPARE_SCHEMA: &str = "dot_logger.compare.v1";
const WINDOWS: [u16; 3] = [10, 60, 300];
const MAX_SLOTS: usize = 64;

/// Fixed snapshot slots and the local capability counter. IDs are never reused
/// within one instance; a replacement host starts a new, unrelated ID space.
#[derive(Debug)]
pub struct OperationsHost {
    slots: Vec<Option<(u64, Snapshot)>>,
    next: u64,
}
impl OperationsHost {
    /// Allocate 1..=64 snapshot slots; registration never grows the array.
    ///
    /// # Errors
    /// `InvalidInput` when `slots` is 0 or above 64.
    pub fn new(slots: usize) -> Result<Self, RemoteError> {
        if !(1..=MAX_SLOTS).contains(&slots) {
            return Err(RemoteError::InvalidInput);
        }
        Ok(Self {
            slots: vec![None; slots],
            next: 1,
        })
    }
    /// Retain a host-built stable-window snapshot under a new opaque ID.
    /// A full host returns Limit; nothing is evicted implicitly.
    ///
    /// # Errors
    /// `Limit` when every slot is held or the instance's ID space is exhausted.
    pub fn register_snapshot(&mut self, snapshot: Snapshot) -> Result<u64, RemoteError> {
        let slot = self
            .slots
            .iter_mut()
            .find(|s| s.is_none())
            .ok_or(RemoteError::Limit)?;
        let id = self.next;
        self.next = id.checked_add(1).ok_or(RemoteError::Limit)?;
        *slot = Some((id, snapshot));
        Ok(id)
    }
    /// Release one snapshot slot; the ID stays retired.
    ///
    /// # Errors
    /// `Gone` when `id` is not currently held, including a forgotten ID.
    pub fn forget_snapshot(&mut self, id: u64) -> Result<(), RemoteError> {
        let slot = self
            .slots
            .iter_mut()
            .find(|s| s.as_ref().is_some_and(|(i, _)| *i == id))
            .ok_or(RemoteError::Gone)?;
        *slot = None;
        Ok(())
    }
    /// Parse a `dot_logger.config_patch.v1` JSON patch and post it explicitly.
    /// Producer adoption stays per producer; the result is the posted revision.
    /// Memory changes are reported as RestartRequired and never applied live.
    ///
    /// # Errors
    /// `Unsupported` without config updates or tracking; `InvalidInput` for a
    /// malformed or effectively empty patch, or routes that do not fit;
    /// `RestartRequired` for memory changes; `Conflict` for a stale
    /// `base_revision`; `Busy` while a previous revision drains; `Limit` for
    /// revision exhaustion or metadata beyond the control reservation; `Gone`
    /// while closing; `Transport` when management state is unavailable.
    pub fn apply(
        &self,
        updates: Option<ConfigUpdates<'_>>,
        base_revision: u64,
        patch: &[u8],
    ) -> Result<u64, RemoteError> {
        let updates = updates.ok_or(RemoteError::Unsupported)?;
        let patch = parse_patch(patch)?;
        updates
            .apply_config(base_revision, &patch)
            .map_err(|e| match e {
                ConfigUpdateError::Conflict => RemoteError::Conflict,
                ConfigUpdateError::RevisionBusy => RemoteError::Busy,
                ConfigUpdateError::RestartRequired => RemoteError::RestartRequired,
                ConfigUpdateError::InvalidPatch | ConfigUpdateError::BufferTooSmall => {
                    RemoteError::InvalidInput
                }
                ConfigUpdateError::Untracked => RemoteError::Unsupported,
                ConfigUpdateError::RevisionExhausted | ConfigUpdateError::Config(_) => {
                    RemoteError::Limit
                }
                ConfigUpdateError::Closing => RemoteError::Gone,
                ConfigUpdateError::Unavailable => RemoteError::Transport,
            })
    }
    /// Bounded `dot_logger.status.v1` JSON from the host's retained history.
    /// Missing samples or windows stay Unavailable; stale values keep their time.
    /// `snapshots` lists held snapshots in ascending ID order.
    pub fn status_report(
        &self,
        telemetry: Telemetry<'_>,
        updates: Option<ConfigUpdates<'_>>,
        now_ns: u64,
    ) -> Vec<u8> {
        self.status(telemetry, posted_revision(updates), now_ns)
    }
    fn status(&self, telemetry: Telemetry<'_>, posted: Option<u64>, now_ns: u64) -> Vec<u8> {
        let posted = posted.map_or(Json::Null, |r| Json::String(r.to_string()));
        let windows: Vec<Json> = WINDOWS
            .iter()
            .map(|&seconds| {
                let mut w = observation(telemetry.window(seconds), window_json);
                w.insert("seconds".into(), json!(seconds));
                Json::Object(w)
            })
            .collect();
        // Slot reuse reorders storage; list by ID, which is registration order.
        let mut held: Vec<_> = self.slots.iter().flatten().collect();
        held.sort_unstable_by_key(|(id, _)| *id);
        let snapshots: Vec<Json> = held
            .into_iter()
            .map(|(id, s)| {
                let w = s.report.window;
                json!({"id":id.to_string(),"revision":w.revision.to_string(),
                    "epoch":w.epoch.to_string(),"from_ns":w.from_ns.to_string(),
                    "to_ns":w.to_ns.to_string()})
            })
            .collect();
        json!({"schema":STATUS_SCHEMA,"session":format!("{:032x}",telemetry.session),
            "posted_revision":posted,"latest":observation(telemetry.latest(now_ns), sample_json),
            "windows":windows,"snapshots":snapshots})
        .to_string()
        .into_bytes()
    }
    /// Bounded `dot_logger.compare.v1` JSON preserving every condition difference.
    ///
    /// # Errors
    /// `Gone` when `before` or `after` is not currently held.
    pub fn compare(&self, before: u64, after: u64) -> Result<Vec<u8>, RemoteError> {
        let find = |id| {
            self.slots
                .iter()
                .flatten()
                .find(|(i, _)| *i == id)
                .map(|(_, s)| *s)
                .ok_or(RemoteError::Gone)
        };
        let c = Comparison::build(find(before)?, find(after)?);
        let d = c.differences;
        let side = |id: u64, s: &Snapshot| {
            let (w, k) = (s.report.window, s.report.conditions);
            json!({"id":id.to_string(),"session":format!("{:032x}",w.session_id),
                "revision":w.revision.to_string(),"epoch":w.epoch.to_string(),
                "from_ns":w.from_ns.to_string(),"to_ns":w.to_ns.to_string(),
                "application_input_id":format!("{:032x}",s.application_input_id),
                "conditions":{"environment_id":format!("{:032x}",k.environment_id),
                    "workload_id":format!("{:032x}",k.workload_id),"revision":k.revision.to_string(),
                    "target_id":k.target_id,"boundary":format!("{:?}",k.boundary)}})
        };
        Ok(json!({"schema":COMPARE_SCHEMA,"before":side(before,&c.before),"after":side(after,&c.after),
            "comparable":d.comparable(),
            "differences":{"session_or_epoch":d.session_or_epoch,"environment":d.environment,
                "workload":d.workload,"application_input":d.application_input,"target":d.target,
                "boundary":d.boundary,"measurement_scope":d.measurement_scope,"invalid_window":d.invalid_window},
            "changes":{"accepted_rate":change_json(c.accepted_rate),
                "accepted_bytes_rate":change_json(c.accepted_bytes_rate),
                "completed_rate":change_json(c.completed_rate),
                "intentional_skip_rate":change_json(c.intentional_skip_rate),
                "capacity_rejection_rate":change_json(c.capacity_rejection_rate),
                "cpu_rate":change_json(c.cpu_rate),"latency_ns":change_json(c.latency_ns),
                "memory_bytes":change_json(c.memory_bytes)}})
        .to_string()
        .into_bytes())
    }
    /// Answer GetStatus/ApplyConfig/Compare on the host's general worker; other
    /// operations are Unsupported so a separate host may serve them. Reports
    /// that do not fit the response budget fail as Limit without being cut.
    pub fn handle_command(
        &self,
        command: Command,
        telemetry: Telemetry<'_>,
        updates: Option<ConfigUpdates<'_>>,
        now_ns: u64,
    ) {
        let fit = |data: Vec<u8>| {
            if data.len() > command.response_capacity() {
                Err(RemoteError::Limit)
            } else {
                Ok(data)
            }
        };
        let reply = match command.request() {
            ManagementRequest::GetStatus => {
                // One revision read keeps the reply and its report consistent.
                let posted = posted_revision(updates);
                fit(self.status(telemetry, posted, now_ns)).map(|report| ManagementReply::Status {
                    session: telemetry.session,
                    revision: posted.unwrap_or(telemetry.revision),
                    report,
                })
            }
            ManagementRequest::ApplyConfig {
                base_revision,
                patch,
            } => self
                .apply(updates, *base_revision, patch)
                .map(ManagementReply::Applied),
            ManagementRequest::Compare { before, after } => self
                .compare(*before, *after)
                .and_then(fit)
                .map(ManagementReply::Report),
            _ => Err(RemoteError::Unsupported),
        };
        command.respond(reply);
    }
}

fn posted_revision(updates: Option<ConfigUpdates<'_>>) -> Option<u64> {
    updates.and_then(|u| u.posted_revision().ok())
}
fn parse_patch(bytes: &[u8]) -> Result<ConfigPatch, RemoteError> {
    let bad = RemoteError::InvalidInput;
    let Ok(Json::Object(root)) = serde_json::from_slice::<Json>(bytes) else {
        return Err(bad);
    };
    only(&root, &["schema", "channels", "routes", "memory"])?;
    if root.get("schema").and_then(Json::as_str) != Some(PATCH_SCHEMA) {
        return Err(bad);
    }
    if root.get("memory").is_some_and(|m| !m.is_null()) {
        return Err(RemoteError::RestartRequired);
    }
    let list = |key| match root.get(key) {
        None => Ok(&[][..]),
        Some(Json::Array(items)) => Ok(items.as_slice()),
        Some(_) => Err(bad),
    };
    let channels = list("channels")?
        .iter()
        .map(|c| {
            let c = c.as_object().ok_or(bad)?;
            only(c, &["channel", "minimum_level", "sample_every"])?;
            Ok(ChannelPatch {
                channel_id: u32_field(c, "channel")?,
                minimum_level: match c.get("minimum_level").and_then(Json::as_str) {
                    Some("trace") => Level::Trace,
                    Some("debug") => Level::Debug,
                    Some("info") => Level::Info,
                    Some("warn") => Level::Warn,
                    Some("error") => Level::Error,
                    _ => return Err(bad),
                },
                sample_every: u32_field(c, "sample_every")?,
            })
        })
        .collect::<Result<_, _>>()?;
    let routes = list("routes")?
        .iter()
        .map(|r| {
            let r = r.as_object().ok_or(bad)?;
            only(r, &["channel", "targets"])?;
            let targets = r
                .get("targets")
                .and_then(Json::as_array)
                .ok_or(bad)?
                .iter()
                .map(|t| t.as_u64().and_then(|t| u32::try_from(t).ok()).ok_or(bad))
                .collect::<Result<_, _>>()?;
            Ok(Route {
                channel_id: u32_field(r, "channel")?,
                targets,
            })
        })
        .collect::<Result<_, _>>()?;
    Ok(ConfigPatch {
        channels,
        routes,
        memory: None,
    })
}
fn only(object: &Map<String, Json>, keys: &[&str]) -> Result<(), RemoteError> {
    if object.keys().all(|k| keys.contains(&k.as_str())) {
        Ok(())
    } else {
        Err(RemoteError::InvalidInput)
    }
}
fn u32_field(object: &Map<String, Json>, key: &str) -> Result<u32, RemoteError> {
    object
        .get(key)
        .and_then(Json::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .ok_or(RemoteError::InvalidInput)
}
fn observation<T>(o: Observation<T>, f: fn(T) -> Map<String, Json>) -> Map<String, Json> {
    let (state, value) = match o {
        Observation::Observed(v) => ("Observed", f(v)),
        Observation::Stale(v) => ("Stale", f(v)),
        Observation::Unavailable(r) => {
            let mut m = Map::new();
            m.insert("reason".into(), json!(unavailable(r)));
            ("Unavailable", m)
        }
    };
    let mut m = value;
    m.insert("state".into(), json!(state));
    m
}
fn unavailable(r: Unavailable) -> String {
    format!("{r:?}")
}
fn sample_json(s: Sample) -> Map<String, Json> {
    let mut m = Map::new();
    m.insert("at_ns".into(), json!(s.at_ns.to_string()));
    m.insert("revision".into(), json!(s.revision.to_string()));
    m.insert("epoch".into(), json!(s.epoch.to_string()));
    m.insert(
        "capture".into(),
        Json::Object(observation(s.capture, counts_json)),
    );
    m.insert(
        "outputs".into(),
        Json::Object(observation(s.outputs, totals_json)),
    );
    m.insert(
        "required_outputs".into(),
        Json::Object(observation(s.required_outputs, totals_json)),
    );
    m
}
fn window_json(w: Window) -> Map<String, Json> {
    let mut m = Map::new();
    m.insert("revision".into(), json!(w.revision.to_string()));
    m.insert("epoch".into(), json!(w.epoch.to_string()));
    m.insert("from_ns".into(), json!(w.from_ns.to_string()));
    m.insert("to_ns".into(), json!(w.to_ns.to_string()));
    m.insert("counts".into(), Json::Object(counts_json(w.counts)));
    m
}
fn counts_json(c: CaptureCounts) -> Map<String, Json> {
    let pairs = [
        ("accepted", c.accepted),
        ("accepted_bytes", c.accepted_bytes),
        ("skipped", c.skipped),
        ("rejected", c.rejected),
        ("rejected_capacity", c.rejected_capacity),
        ("rejected_contract", c.rejected_contract),
        ("rejected_lifecycle", c.rejected_lifecycle),
        ("acquire_rejected", c.acquire_rejected),
        ("acquire_rejected_capacity", c.acquire_rejected_capacity),
        ("acquire_rejected_contract", c.acquire_rejected_contract),
        ("acquire_rejected_lifecycle", c.acquire_rejected_lifecycle),
    ];
    let mut m: Map<String, Json> = pairs
        .into_iter()
        .map(|(k, v)| (k.to_owned(), json!(v.to_string())))
        .collect();
    m.insert("overflowed".into(), json!(c.overflowed));
    m
}
fn totals_json(t: CompletionTotals) -> Map<String, Json> {
    let pairs = [
        ("records", t.records),
        ("assigned", t.assigned),
        ("complete", t.complete),
        ("failed", t.failed),
        ("dropped", t.dropped),
        ("unknown", t.unknown),
        ("pending", t.pending),
        ("busy", t.busy),
        ("expired_records", t.expired_records),
    ];
    let mut m: Map<String, Json> = pairs
        .into_iter()
        .map(|(k, v)| (k.to_owned(), json!(v.to_string())))
        .collect();
    m.insert("overflowed".into(), json!(t.overflowed));
    m
}
fn change_json(v: Value<RateChange>) -> Json {
    let rate = |c: RateChange| {
        json!({"direction":match c.direction {
            Direction::Increase => "Increase",
            Direction::Equal => "Equal",
            Direction::Decrease => "Decrease",
        },"numerator":c.numerator.to_string(),"denominator":c.denominator.to_string()})
    };
    let with = |state: &str, c: RateChange, extra: Option<(&str, String)>| {
        let mut m = rate(c);
        m["state"] = json!(state);
        if let Some((k, v)) = extra {
            m[k] = json!(v);
        }
        m
    };
    match v {
        Value::Observed(c) => with("Observed", c, None),
        Value::Validated { value, profile_id } => with(
            "Validated",
            value,
            Some(("profile_id", format!("{profile_id:032x}"))),
        ),
        Value::Estimated { value, assumptions } => with(
            "Estimated",
            value,
            Some(("assumptions", format!("{assumptions:?}"))),
        ),
        Value::Stale { value, basis, .. } => with(
            "Stale",
            value,
            Some((
                "basis",
                match basis {
                    Basis::Observed => "Observed".into(),
                    Basis::Validated(id) => format!("Validated:{id:032x}"),
                    Basis::Estimated(a) => format!("Estimated:{a:?}"),
                },
            )),
        ),
        Value::Unavailable(r) => json!({"state":"Unavailable","reason":format!("{r:?}")}),
    }
}
