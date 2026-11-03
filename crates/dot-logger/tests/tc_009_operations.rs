//! TC-009 remote operations backend: explicit apply, observed status, compare.
#![cfg(feature = "remote")]
#![allow(clippy::unwrap_used)]
mod support;
use dot_logger::{
    adapters::remote::RemoteError,
    capture::*,
    completion::Boundary,
    telemetry::{
        Observation,
        capacity::Conditions,
        compare::Snapshot,
        operations::OperationsHost,
        report::{Report, ReportInput},
    },
};
use serde_json::Value;
use support::*;

const SECOND: u64 = 1_000_000_000;
fn owner() -> CaptureOwner {
    let mut o = CaptureOwner::new_tracked(
        &plan(),
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routing(),
        tracking(8),
    )
    .unwrap();
    o.enable_config_updates(&routing()).unwrap();
    o.enable_telemetry().unwrap();
    o
}
fn json(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}
const PATCH: &[u8] = br#"{"schema":"dot_logger.config_patch.v1","channels":[{"channel":7,"minimum_level":"warn","sample_every":2}],"routes":[{"channel":7,"targets":[10]}]}"#;

#[test]
fn apply_maps_conflict_busy_restart_and_invalid_patches_without_publishing() {
    let mut o = owner();
    o.scope(|mut s| {
        let host = OperationsHost::new(4).unwrap();
        let u = s.config_updates();
        for bad in [
            &b"not json"[..],
            br#"{"schema":"other","channels":[]}"#,
            br#"{"schema":"dot_logger.config_patch.v1","channels":[],"extra":1}"#,
            br#"{"schema":"dot_logger.config_patch.v1","channels":[{"channel":7,"minimum_level":"loud","sample_every":1}]}"#,
            br#"{"schema":"dot_logger.config_patch.v1","channels":[{"channel":7,"minimum_level":"info","sample_every":0}]}"#,
            br#"{"schema":"dot_logger.config_patch.v1"}"#,
        ] {
            assert_eq!(host.apply(u, 1, bad), Err(RemoteError::InvalidInput));
        }
        assert_eq!(
            host.apply(
                u,
                1,
                br#"{"schema":"dot_logger.config_patch.v1","memory":{"total_bytes":1}}"#
            ),
            Err(RemoteError::RestartRequired)
        );
        assert_eq!(host.apply(None, 1, PATCH), Err(RemoteError::Unsupported));
        assert_eq!(host.apply(u, 0, PATCH), Err(RemoteError::Conflict));
        assert_eq!(u.unwrap().posted_revision().unwrap(), 1);
        assert_eq!(host.apply(u, 1, PATCH), Ok(2));
        // The idle producer still holds revision 1 until it polls.
        let restore = br#"{"schema":"dot_logger.config_patch.v1","channels":[{"channel":7,"minimum_level":"trace","sample_every":1}]}"#;
        assert_eq!(host.apply(u, 2, restore), Err(RemoteError::Busy));
        assert_eq!(host.apply(u, 1, restore), Err(RemoteError::Conflict));
        let (mut p, _) = s.next().unwrap();
        assert_eq!(p.apply_pending_config(), 2);
        let mut t = [0; 2];
        assert_eq!(u.unwrap().route_targets(2, 7, &mut t), Ok(1));
        assert_eq!(t[0], 10);
    });
}

#[test]
fn status_reports_observed_stale_and_missing_metrics_without_zero_fill() {
    let mut o = owner();
    o.scope(|mut s| {
        let mut host = OperationsHost::new(2).unwrap();
        let telemetry = s.telemetry().unwrap();
        let u = s.config_updates();
        let empty = json(&host.status_report(telemetry, u, 0));
        assert_eq!(empty["schema"], "dot_logger.status.v1");
        assert_eq!(empty["posted_revision"], "1");
        assert_eq!(empty["latest"]["state"], "Unavailable");
        assert_eq!(empty["latest"]["reason"], "IncompleteWindow");
        assert_eq!(empty["windows"][0]["state"], "Unavailable");
        assert!(empty["latest"].get("capture").is_none());
        let (mut p, _) = s.next().unwrap();
        for at in 0..=10 {
            // Eight small slots and no consumer: stay within the pool.
            if at < 5 {
                accepted(p.try_record_small(meta(), &[1]).unwrap());
            }
            telemetry.sample(at * SECOND, None).unwrap();
        }
        let now = 10 * SECOND;
        let fresh = json(&host.status_report(telemetry, u, now));
        assert_eq!(fresh["latest"]["state"], "Observed");
        assert_eq!(fresh["latest"]["at_ns"], (10 * SECOND).to_string());
        assert_eq!(fresh["latest"]["capture"]["accepted"], "5");
        assert_eq!(fresh["latest"]["outputs"]["state"], "Unavailable");
        assert_eq!(fresh["windows"][0]["seconds"], 10);
        assert_eq!(fresh["windows"][0]["state"], "Observed");
        // The first record precedes the baseline sample, so the window delta excludes it.
        assert_eq!(fresh["windows"][0]["counts"]["accepted"], "4");
        assert_eq!(fresh["windows"][1]["state"], "Unavailable");
        assert_eq!(fresh["snapshots"].as_array().unwrap().len(), 0);
        let stale = json(&host.status_report(telemetry, u, now + 3 * SECOND));
        assert_eq!(stale["latest"]["state"], "Stale");
        assert_eq!(stale["latest"]["capture"]["accepted"], "5");
        let Observation::Observed(window) = telemetry.window(10) else {
            panic!("window")
        };
        let id = host.register_snapshot(snapshot(window)).unwrap();
        let listed = json(&host.status_report(telemetry, u, now));
        assert_eq!(listed["snapshots"][0]["id"], id.to_string());
        assert_eq!(listed["snapshots"][0]["revision"], "1");
        assert_eq!(listed["snapshots"][0]["to_ns"], now.to_string());
        let no_config = json(&host.status_report(telemetry, None, now));
        assert_eq!(no_config["posted_revision"], Value::Null);
    });
}

#[test]
fn compare_preserves_differences_and_rejects_unknown_or_exhausted_ids() {
    let mut o = owner();
    o.scope(|mut s| {
        let mut host = OperationsHost::new(2).unwrap();
        let telemetry = s.telemetry().unwrap();
        let (mut p, _) = s.next().unwrap();
        for at in 0..=20 {
            if at < 4 || at == 12 || at == 16 {
                accepted(p.try_record_small(meta(), &[1]).unwrap());
            }
            telemetry.sample(at * SECOND, None).unwrap();
            if at == 10 {
                let Observation::Observed(w) = telemetry.window(10) else {
                    panic!("before")
                };
                host.register_snapshot(snapshot(w)).unwrap();
            }
        }
        let Observation::Observed(w) = telemetry.window(10) else {
            panic!("after")
        };
        let after = host.register_snapshot(snapshot(w)).unwrap();
        assert_eq!(host.register_snapshot(snapshot(w)), Err(RemoteError::Limit));
        let report = json(&host.compare(1, after).unwrap());
        assert_eq!(report["schema"], "dot_logger.compare.v1");
        assert_eq!(report["before"]["id"], "1");
        assert_eq!(report["after"]["id"], after.to_string());
        assert_eq!(report["comparable"], true);
        assert_eq!(report["changes"]["accepted_rate"]["state"], "Observed");
        assert_eq!(report["changes"]["accepted_rate"]["direction"], "Decrease");
        assert_eq!(report["changes"]["cpu_rate"]["state"], "Unavailable");
        assert_eq!(
            report["changes"]["cpu_rate"]["reason"],
            "MissingMeasurement"
        );
        assert_eq!(host.compare(1, 99), Err(RemoteError::Gone));
        assert_eq!(
            host.compare(after, 1)
                .map(|r| json(&r)["comparable"].clone()),
            Ok(Value::Bool(false))
        );
        host.forget_snapshot(1).unwrap();
        assert_eq!(host.compare(1, after), Err(RemoteError::Gone));
        assert_eq!(host.forget_snapshot(1), Err(RemoteError::Gone));
        // A freed slot gets a new ID; old IDs never come back.
        assert_eq!(host.register_snapshot(snapshot(w)), Ok(after + 1));
        // The reused first slot must not reorder the listing: IDs ascend.
        let listed = json(&host.status_report(telemetry, None, 20 * SECOND));
        let ids: Vec<_> = listed["snapshots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(ids, [after.to_string(), (after + 1).to_string()]);
    });
}

#[test]
fn host_slots_are_bounded() {
    assert!(OperationsHost::new(0).is_err());
    assert!(OperationsHost::new(65).is_err());
    let host = OperationsHost::new(64).unwrap();
    assert!(format!("{host:?}").starts_with("OperationsHost"));
}

fn snapshot(window: dot_logger::telemetry::Window) -> Snapshot {
    let input = ReportInput::from_window(
        window,
        Conditions {
            environment_id: 1,
            workload_id: 1,
            revision: window.revision,
            target_id: 10,
            boundary: Boundary::Written,
        },
    );
    Snapshot::new(Report::build(input, None, window.to_ns), 1)
}
