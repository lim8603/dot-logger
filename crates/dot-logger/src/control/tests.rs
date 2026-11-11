//! Deterministic public-API schedules around sync admission and shutdown.
#![allow(clippy::unwrap_used)]
use super::*;
use crate::{
    capture::{CaptureOwner, ChannelConfig},
    completion::{Boundaries, CompletionConfig, TargetContract},
    config::{
        ImportancePolicy, MemoryBudget, PoolQuota, ProducerQuota, SessionBuilder, SessionConfig,
        WorkerThreadingConfig,
    },
    platform::{PlatformCapabilities, Requirement},
    routing::{Route, Router, RoutingConfig, Schedule, TargetConfig},
};
use std::cell::RefCell;
use std::sync::{Arc, Barrier};
use std::time::Duration;

struct SubmissionPause {
    entered: Arc<Barrier>,
    resume: Arc<Barrier>,
}
thread_local! {
    static SUBMISSION_PAUSE: RefCell<Option<SubmissionPause>> = const { RefCell::new(None) };
    static REPORT_PAUSE: RefCell<Option<SubmissionPause>> = const { RefCell::new(None) };
}
pub(super) fn pause_emergency_report() {
    if let Some(pause) = REPORT_PAUSE.with(|pause| pause.borrow_mut().take()) {
        pause.entered.wait();
        pause.resume.wait();
    }
}

#[test]
fn earlier_alarm_during_report_cannot_leave_protected_complete_using_old_deadline() {
    use super::emergency::*;
    use crate::platform::power::PowerAlarm;
    let (mut owner, plan, route) = setup();
    owner.scope(|mut scope| {
        let start = Instant::now();
        let mut control = ControlOwner::new(
            &plan,
            scope.control(),
            ControlConfig {
                sync_slots: 1,
                work_per_poll: 64,
            },
            start,
        )
        .unwrap();
        control
            .enable_emergency(
                &plan,
                EmergencyConfig {
                    files: vec![EmergencyFile {
                        target: 10,
                        rank: 1,
                        protected_bytes: 64,
                        estimated_drain_sync: Duration::ZERO,
                    }],
                    safety_margin: Duration::from_millis(200),
                },
            )
            .unwrap();
        let (_, input) = scope.next().unwrap();
        let mut router = Router::new(&plan, vec![input], route).unwrap();
        router.scope(|mut d, _| {
            control
                .notify_power(
                    PowerAlarm::new(1, start, Duration::from_secs(3)).unwrap(),
                    start,
                )
                .unwrap();
            control.poll_emergency(start, &mut d).unwrap();
            control.observe_protected_bytes(10, 0).unwrap();
            let token = control.shutdown_sync().unwrap();
            let mut ranges = [ProducerRange::default(); 1];
            let now = start + Duration::from_secs(1);
            control
                .next_sync(token, 10, now, &mut d, &mut ranges)
                .unwrap()
                .unwrap()
                .complete(SyncResult::Synced);
            assert!(
                control
                    .poll_emergency(now, &mut d)
                    .unwrap()
                    .unwrap()
                    .protected_complete
            );
            let entered = Arc::new(Barrier::new(2));
            let resume = Arc::new(Barrier::new(2));
            REPORT_PAUSE.with(|p| {
                *p.borrow_mut() = Some(SubmissionPause {
                    entered: entered.clone(),
                    resume: resume.clone(),
                })
            });
            std::thread::scope(|threads| {
                let control = &control;
                threads.spawn(move || {
                    entered.wait();
                    control
                        .notify_power(
                            PowerAlarm::new(2, start, Duration::from_millis(500)).unwrap(),
                            now,
                        )
                        .unwrap();
                    resume.wait();
                });
                let report = control.poll_emergency(now, &mut d).unwrap().unwrap();
                assert_eq!(report.poweroff_deadline, start + Duration::from_millis(500));
                assert!(!report.protected_complete);
            });
        });
    });
}

// Test-only scheduler: the real request method performs every state transition.
// Thread-local registration prevents unrelated parallel tests from pausing here.
pub(super) fn pause_sync_submission() {
    if let Some(pause) = SUBMISSION_PAUSE.with(|pause| pause.borrow_mut().take()) {
        pause.entered.wait();
        pause.resume.wait();
    }
}

fn setup() -> (CaptureOwner, ValidatedConfig, RoutingConfig) {
    let plan = SessionBuilder::new(SessionConfig {
        memory: MemoryBudget {
            total_bytes: 1024 * 1024,
            input_bytes: 128 * 1024,
            raw_bytes: 0,
            output_bytes: 256 * 1024,
            external_bytes: 0,
            control_bytes: 256 * 1024,
            worker_bytes: 256 * 1024,
        },
        producers: vec![ProducerQuota {
            small: PoolQuota {
                slots: 8,
                slot_bytes: 16,
                important_slots: 0,
            },
            raw: None,
        }],
        importance: ImportancePolicy::TreatEqually,
        workers: WorkerThreadingConfig {
            count: 1,
            stack_bytes: 4096,
            cpu_ids: vec![],
            affinity: Requirement::Optional,
        },
        capabilities: vec![],
    })
    .validate(&PlatformCapabilities::default())
    .unwrap();
    let routing = RoutingConfig {
        targets: vec![TargetConfig {
            id: 10,
            required: true,
            queue_slots: 4,
            pending_slots: 1,
            retained_bytes: 128,
            max_attempts: 3,
            max_age: Duration::from_secs(1),
        }],
        routes: vec![Route {
            channel_id: 7,
            targets: vec![10],
        }],
        shared_records: 4,
        small_copy_bytes: 64,
        schedule: Schedule {
            record_quantum: 1,
            byte_quantum: 64,
            records_per_turn: 1,
            bytes_per_turn: 64,
            retries_per_target: 1,
            reclaim_slots_per_producer: 1,
        },
    };
    let owner = CaptureOwner::new_tracked(
        &plan,
        42,
        1,
        &[ChannelConfig::new(7, 99, 1)],
        &routing,
        CompletionConfig {
            history_retention: Duration::from_secs(60),
            window_records: 2,
            max_queries: 1,
            page_items: 2,
            feedback_events: 1,
            updates_per_turn: 1,
            reconcile_per_turn: 1,
            targets: vec![TargetContract {
                id: 10,
                required: true,
                supported: Boundaries::FILE,
                completion: Boundary::Written,
            }],
        },
    )
    .unwrap();
    (owner, plan, routing)
}

fn overlapping_submission(timeout: bool, target: u32) {
    let (mut owner, plan, route) = setup();
    owner.scope(|scope| {
        let completion = scope.completion().unwrap();
        let now = Instant::now();
        let end = now + Duration::from_secs(1);
        let control = ControlOwner::new(
            &plan,
            scope.control(),
            ControlConfig { sync_slots: 2, work_per_poll: 8 },
            now,
        ).unwrap();
        let inputs = scope.map(|(_, reader)| reader).collect();
        let mut router = Router::new(&plan, inputs, route).unwrap();
        let mut ranges = [ProducerRange::default(); 1];
        let frozen = completion.capture_scope(&mut ranges).unwrap();
        router.scope(|mut dispatcher, _| {
            std::thread::scope(|threads| {
                let entered = Arc::new(Barrier::new(2));
                let resume = Arc::new(Barrier::new(2));
                let pause = SubmissionPause {
                    entered: Arc::clone(&entered),
                    resume: Arc::clone(&resume),
                };
                let request = threads.spawn(|| {
                    SUBMISSION_PAUSE.with(|slot| *slot.borrow_mut() = Some(pause));
                    control.request_sync(frozen, &[target], now, end)
                });
                entered.wait();
                let concurrent = control.request_sync(frozen, &[10], now, end);
                let initial = control.begin_shutdown(
                    ShutdownPlan { sync_files: false }, now, end, &mut dispatcher,
                );
                let closed_admission = control.request_sync(frozen, &[10], now, end);
                let expired = timeout.then(|| control.poll_shutdown(end, &mut dispatcher));
                // Always release the submitter before asserting, even on an error.
                resume.wait();
                let result = request.join().unwrap();
                assert_eq!(concurrent, Err(ControlError::Busy));
                assert_eq!(closed_admission, Err(ControlError::Closing));
                assert!(
                    matches!(initial.unwrap(), ShutdownPoll::Closing(_)),
                    "shutdown closed before an admitted sync request was published",
                );
                if target != 10 {
                    assert_eq!(result, Err(ControlError::UnsupportedBoundary));
                    assert!(matches!(
                        control.poll_shutdown(now, &mut dispatcher).unwrap(),
                        ShutdownPoll::Closed(report) if report.is_success(),
                    ));
                    return;
                }
                let token = result.unwrap();
                let mut cutoffs = [ProducerRange::default(); 1];
                if let Some(expired) = expired {
                    assert!(matches!(expired.unwrap(), ShutdownPoll::Incomplete(_)));
                    assert!(
                        control.next_sync(token, 10, end, &mut dispatcher, &mut cutoffs)
                            .unwrap().is_none(),
                    );
                    assert!(matches!(
                        control.poll_shutdown(end, &mut dispatcher).unwrap(),
                        ShutdownPoll::Closed(report)
                            if report.deadline_elapsed && report.sync_failed && !report.is_success(),
                    ));
                } else {
                    let work = control.next_sync(token, 10, now, &mut dispatcher, &mut cutoffs)
                        .unwrap().unwrap();
                    assert!(matches!(
                        control.poll_shutdown(now, &mut dispatcher).unwrap(),
                        ShutdownPoll::Closing(report) if report.sync_in_flight == 1,
                    ));
                    work.complete(SyncResult::Synced);
                    assert!(matches!(
                        control.poll_shutdown(now, &mut dispatcher).unwrap(),
                        ShutdownPoll::Closed(report) if report.is_success() && report.sync_in_flight == 0,
                    ));
                }
                assert_eq!(
                    control.request_sync(frozen, &[10], if timeout { end } else { now },
                        end + Duration::from_secs(1)),
                    Err(ControlError::Closing),
                );
            });
        });
    });
}

#[test]
fn admitted_sync_submission_blocks_closed_until_its_io_finishes() {
    overlapping_submission(false, 10);
}

#[test]
fn admitted_sync_submission_keeps_timeout_incomplete_until_it_is_published() {
    overlapping_submission(true, 10);
}

#[test]
fn rejected_sync_submission_releases_shutdown_without_starting_io() {
    overlapping_submission(false, 99);
}

fn small_metadata() -> crate::capture::RecordMetadata {
    crate::capture::RecordMetadata {
        channel_id: 7,
        schema_id: 99,
        schema_version: 1,
        mono_ns: 1,
        real_ns: 2,
        source_time: None,
        level: crate::capture::Level::Info,
        importance: None,
    }
}

// T5 ⑫ (EV-066): the facade host polls shutdown every turn while it drains.
// A full completion-ledger count (producers × window × targets) on each of
// those polls starved dispatch, so the host's poll counts only when the
// result can matter: closure is possible, or an Incomplete report is due.
#[test]
fn host_shutdown_poll_counts_the_ledger_only_when_closure_or_a_report_is_due() {
    let (mut owner, plan, route) = setup();
    owner.scope(|mut scope| {
        let now = Instant::now();
        let end = now + Duration::from_secs(1);
        let control = ControlOwner::new(
            &plan,
            scope.control(),
            ControlConfig {
                sync_slots: 1,
                work_per_poll: 8,
            },
            now,
        )
        .unwrap();
        let (mut producer, reader) = scope.next().unwrap();
        let mut router = Router::new(&plan, vec![reader], route).unwrap();
        router.scope(|mut d, _| {
            producer.try_record_small(small_metadata(), &[1]).unwrap();
            assert!(matches!(
                control
                    .begin_shutdown(ShutdownPlan { sync_files: false }, now, end, &mut d)
                    .unwrap(),
                ShutdownPoll::Closing(_)
            ));
            let scans = crate::completion::totals_scans();
            assert!(
                control
                    .poll_shutdown_when_due(now, &mut d, true)
                    .unwrap()
                    .is_none(),
                "an undrained record keeps shutdown closing",
            );
            assert_eq!(
                crate::completion::totals_scans(),
                scans,
                "no count while draining"
            );
            // The first report after the deadline still carries fresh totals.
            assert!(matches!(
                control.poll_shutdown_when_due(end, &mut d, true).unwrap(),
                Some(ShutdownPoll::Incomplete(report)) if report.totals.records == 1,
            ));
            let scans = crate::completion::totals_scans();
            assert!(
                control
                    .poll_shutdown_when_due(end, &mut d, false)
                    .unwrap()
                    .is_none()
            );
            assert_eq!(
                crate::completion::totals_scans(),
                scans,
                "report already sent"
            );
            // The public poll keeps counting on every call.
            assert!(matches!(
                control.poll_shutdown(end, &mut d).unwrap(),
                ShutdownPoll::Incomplete(report) if report.totals.records == 1,
            ));
            assert!(crate::completion::totals_scans() > scans);
        });
    });
}

#[test]
fn host_shutdown_poll_still_closes_with_counted_totals() {
    let (mut owner, plan, route) = setup();
    owner.scope(|scope| {
        let now = Instant::now();
        let end = now + Duration::from_secs(1);
        let control = ControlOwner::new(
            &plan,
            scope.control(),
            ControlConfig {
                sync_slots: 1,
                work_per_poll: 8,
            },
            now,
        )
        .unwrap();
        let inputs = scope.map(|(_, reader)| reader).collect();
        let mut router = Router::new(&plan, inputs, route).unwrap();
        router.scope(|mut d, _| {
            control
                .begin_shutdown(ShutdownPlan { sync_files: false }, now, end, &mut d)
                .unwrap();
            let scans = crate::completion::totals_scans();
            assert!(matches!(
                control.poll_shutdown_when_due(now, &mut d, true).unwrap(),
                Some(ShutdownPoll::Closed(report)) if report.is_success(),
            ));
            assert!(crate::completion::totals_scans() > scans);
        });
    });
}
