//! Finite general-path wiring example; no dot-probe source/app modification.
//! Session mono time is a declared virtual clock. HTTP/ACK/deadlines use real
//! Instant time. Virtual report rates are functional examples, not throughput.
// CLI and fault tests consume different portions of this shared example module.
#![allow(dead_code)]
use dot_logger::{
    adapters::{influxdb as db, tracing::log_record},
    capture::*,
    completion::*,
    config::*,
    control::config_update::{ChannelPatch, ConfigPatch},
    platform::*,
    pool::RawRecordOutcome,
    query::QueryBudget,
    routing::*,
    storage::*,
    telemetry::{
        Observation,
        capacity::Conditions,
        compare::{Comparison, Snapshot},
        report::{OutputDelta, Report, ReportInput},
    },
};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
mod remote;
use remote::Remote;
/// Original ordinary-path error, without database credentials.
pub type Error = Box<dyn std::error::Error + Send + Sync>;
const FILE: u32 = 10;
const DATABASE: u32 = 20;
const RECEIVER: u32 = 30;
const SECOND: u64 = 1_000_000_000;
const TICKS: u64 = 10;
/// Receiving queue behavior for the finite example/failure scenario.
#[derive(Clone, Copy, Default)]
pub enum ReceiverMode {
    /// Admit each complete record to a one-slot consumer queue.
    #[default]
    Accept,
    /// Fill the queue before the last original; refuse its handoff and do not ACK.
    FullOnLast,
}
/// Explicit finite example options, not product defaults or device capacity.
pub struct Options {
    /// Total real-time admission budget, at most 30 seconds.
    pub duration: Duration,
    /// Query and compare the original ten numeric DB points after phase one.
    pub verify_query: bool,
    /// Bounded consumer policy.
    pub receiver: ReceiverMode,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(30),
            verify_query: false,
            receiver: ReceiverMode::Accept,
        }
    }
}
/// Quiescent interval counts; separate target assignments from ingress records.
#[derive(Debug)]
pub struct Phase {
    /// Original capture revision of this virtual-clock interval.
    pub revision: u64,
    /// Actual accepted inputs in this phase.
    pub accepted: u64,
    /// Intentional level/sampling omissions.
    pub skipped: u64,
    /// Native selected-target completion delta; unselected is not loss.
    pub file: CompletionTotals,
    /// Native DB completion delta (phase two has no DB assignments).
    pub database: CompletionTotals,
    /// Native receiver completion delta.
    pub receiver: CompletionTotals,
    snapshot: Snapshot,
}
/// Same-receipt functional evidence; no CPU, RSS, RT or device capacity claim.
#[derive(Debug)]
pub struct PipelineReport {
    /// Before filter/sampling/route changes.
    pub before: Phase,
    /// After explicit validate/apply and producer acknowledgment.
    pub after: Phase,
    /// Full original metadata/bytes verified from closed MCAP files.
    pub file_verified: usize,
    /// Original metadata/bytes admitted and read from the bounded receiver queue.
    pub remote_verified: usize,
    /// Actual raw slots reclaimed after delivery release.
    pub raw_reclaimed: usize,
    /// Actual DB query points matched to original receipt/time/value, or unmeasured.
    pub numeric_verified: Option<usize>,
    /// Retains workload change; no causal improvement inferred across fanout.
    pub comparison: Comparison,
    /// Shared external reservation after actual owner/runtime close.
    pub external_after_close: usize,
}
impl PipelineReport {
    /// Whether this exact functional profile completed every selected boundary.
    /// A successful function return can still contain DB/receiver failures.
    pub fn is_complete(&self) -> bool {
        let clean = |t: CompletionTotals| {
            t.pending == 0
                && t.failed == 0
                && t.dropped == 0
                && t.unknown == 0
                && t.busy == 0
                && !t.overflowed
        };
        self.before.accepted == 30
            && self.before.skipped == 0
            && self.after.accepted == 15
            && self.after.skipped == 15
            && self.before.file.complete == 30
            && self.after.file.complete == 15
            && self.before.database.complete == 20
            && self.after.database.assigned == 0
            && self.before.receiver.complete == 30
            && self.after.receiver.complete == 15
            && [
                self.before.file,
                self.after.file,
                self.before.database,
                self.after.database,
                self.before.receiver,
                self.after.receiver,
            ]
            .into_iter()
            .all(clean)
            && self.file_verified == 45
            && self.remote_verified == 45
            && self.raw_reclaimed == 20
            && self.numeric_verified == Some(10)
            && self.external_after_close == 0
    }
    /// Stable machine-readable functional result with explicit missing evidence.
    pub fn json(&self) -> serde_json::Value {
        let phase = |p: &Phase| {
            serde_json::json!({"session_id":format!("{:032x}",p.snapshot.report.window.session_id),
            "epoch":p.snapshot.report.window.epoch,"revision":p.revision,
            "application_input_id":p.snapshot.application_input_id,
            "conditions":{"environment_id":p.snapshot.report.conditions.environment_id,
                "workload_id":p.snapshot.report.conditions.workload_id,
                "target_id":p.snapshot.report.conditions.target_id,
                "boundary":format!("{:?}",p.snapshot.report.conditions.boundary)},
            "offered":30,"accepted":p.accepted,"skipped":p.skipped,
            "file":totals_json(p.file),"database":totals_json(p.database),"receiver":totals_json(p.receiver),
            "from_ns":p.snapshot.report.window.from_ns,"to_ns":p.snapshot.report.window.to_ns})
        };
        serde_json::json!({"functional_pass":self.is_complete(),"scope":"SyntheticFunctionalExample","clock":"VirtualSessionMono; rates are not measured throughput",
            "before":phase(&self.before),"after":phase(&self.after),"file_verified":self.file_verified,
            "remote_verified":self.remote_verified,"raw_reclaimed":self.raw_reclaimed,"numeric_verified":self.numeric_verified,
            "external_after_close":self.external_after_close,"comparable":self.comparison.differences.comparable(),
            "workload_changed":self.comparison.differences.workload,"accepted_rate_change":format!("{:?}",self.comparison.accepted_rate),
            "cpu_rate_change":format!("{:?}",self.comparison.cpu_rate),"capacity":"Unavailable","rss":"Unavailable",
            "app_integration":"NotRun","native":"NotRun"})
    }
}
fn totals_json(t: CompletionTotals) -> serde_json::Value {
    serde_json::json!({"assigned":t.assigned,"complete":t.complete,"failed":t.failed,"dropped":t.dropped,"unknown":t.unknown,"pending":t.pending})
}
/// Pure no-IO plan, without reading environment variables or probing paths.
pub fn plan_description() -> serde_json::Value {
    serde_json::json!({"scope":"SyntheticFunctionalExample","phases":2,"offered_per_phase":30,
        "session_clock":"VirtualSessionMono","virtual_interval_ns":10*SECOND,"real_admission_seconds":30,
        "max_payload_bytes":512,"raw_bytes":64,"consumer_queue_slots":1,"logger_budget_bytes":32*1024*1024,
        "routes_before":"file+receiver; log/numeric also DB","routes_after":"file+receiver; DB off",
        "after":"Info logs filtered; Normal numeric every 2; raw unchanged",
        "comparison":"fanout/workload differs; causal rate change unavailable","performance":"NotMeasured"})
}
pub(super) fn invalid(message: &'static str) -> Error {
    io::Error::new(io::ErrorKind::InvalidData, message).into()
}
/// Explicit bounded DB settings. Caller owns a fresh exclusive measurement.
pub fn database_config(
    endpoint: String,
    organization: String,
    bucket: String,
    token: String,
    measurement: String,
    session_id: u128,
) -> db::Config {
    db::Config {
        endpoint,
        organization,
        bucket,
        token,
        measurement,
        session_id,
        device: "dot_probe_synthetic".into(),
        channels: vec![
            db::Channel {
                id: 7,
                kind: db::Kind::Event,
            },
            db::Channel {
                id: 8,
                kind: db::Kind::Integer,
            },
        ],
        collision_policy: db::CollisionPolicy::RejectNonIncreasing,
        max_points: 1,
        max_batch_bytes: 4096,
        max_event_bytes: 512,
        max_attempts: 1,
        request_timeout: Duration::from_secs(1),
        retry_delay: Duration::from_millis(1),
        ttl: Duration::from_secs(2),
    }
}
/// Read the owned harness/example DB environment, without printing secrets.
pub fn database_from_env(measurement: &str) -> Result<db::Config, Error> {
    let env = |name| {
        std::env::var(name).map_err(|_| invalid("missing DOT_LOGGER_TEST_INFLUX DB environment"))
    };
    let session = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        | (u128::from(std::process::id()) << 96);
    Ok(database_config(
        env("DOT_LOGGER_TEST_INFLUX_URL")?,
        env("DOT_LOGGER_TEST_INFLUX_ORG")?,
        env("DOT_LOGGER_TEST_INFLUX_BUCKET")?,
        env("DOT_LOGGER_TEST_INFLUX_TOKEN")?,
        measurement.into(),
        session,
    ))
}
fn config() -> Result<ValidatedConfig, Error> {
    let mib = 1024 * 1024;
    Ok(SessionBuilder::new(SessionConfig {
        memory: MemoryBudget {
            total_bytes: 32 * mib,
            input_bytes: mib,
            raw_bytes: mib,
            output_bytes: 8 * mib,
            external_bytes: 16 * mib,
            control_bytes: 2 * mib,
            worker_bytes: 4 * mib,
        },
        producers: vec![ProducerQuota {
            small: PoolQuota {
                slots: 4,
                slot_bytes: 512,
                important_slots: 0,
            },
            raw: Some(PoolQuota {
                slots: 1,
                slot_bytes: 64,
                important_slots: 0,
            }),
        }],
        importance: ImportancePolicy::TreatEqually,
        workers: WorkerThreadingConfig {
            count: 1,
            stack_bytes: mib,
            cpu_ids: vec![],
            affinity: Requirement::Optional,
        },
        capabilities: vec![],
    })
    .validate(&PlatformCapabilities::default())?)
}
fn routes() -> RoutingConfig {
    RoutingConfig {
        targets: [FILE, DATABASE, RECEIVER]
            .into_iter()
            .map(|id| TargetConfig {
                id,
                required: id == FILE,
                queue_slots: 4,
                pending_slots: 2,
                retained_bytes: 4096,
                max_attempts: 1,
                max_age: Duration::from_secs(5),
            })
            .collect(),
        routes: vec![
            Route {
                channel_id: 7,
                targets: vec![FILE, DATABASE, RECEIVER],
            },
            Route {
                channel_id: 8,
                targets: vec![FILE, DATABASE, RECEIVER],
            },
            Route {
                channel_id: 9,
                targets: vec![FILE, RECEIVER],
            },
        ],
        shared_records: 4,
        small_copy_bytes: 2048,
        schedule: Schedule {
            record_quantum: 1,
            byte_quantum: 512,
            records_per_turn: 1,
            bytes_per_turn: 512,
            retries_per_target: 1,
            reclaim_slots_per_producer: 1,
        },
    }
}
fn tracking() -> CompletionConfig {
    CompletionConfig {
        history_retention: Duration::from_secs(60),
        window_records: 64,
        max_queries: 1,
        page_items: 4,
        feedback_events: 16,
        updates_per_turn: 16,
        reconcile_per_turn: 16,
        targets: vec![
            (FILE, Boundaries::FILE, Boundary::Written),
            (
                DATABASE,
                Boundaries::DATABASE,
                Boundary::DatabaseAcknowledged,
            ),
            (RECEIVER, Boundaries::REMOTE, Boundary::ReceiverAcknowledged),
        ]
        .into_iter()
        .map(|(id, supported, completion)| TargetContract {
            id,
            required: id == FILE,
            supported,
            completion,
        })
        .collect(),
    }
}
fn channels() -> Vec<StorageChannel> {
    (7..=9)
        .map(|id| StorageChannel {
            channel: ChannelConfig::new(id, 92 + u128::from(id), 1),
            topic: format!("synthetic/{id}"),
            payload_encoding: if id == 7 { "json" } else { "opaque" }.into(),
            schema: None,
        })
        .collect()
}
fn patch() -> ConfigPatch {
    ConfigPatch {
        channels: vec![
            ChannelPatch {
                channel_id: 7,
                minimum_level: Level::Warn,
                sample_every: 1,
            },
            ChannelPatch {
                channel_id: 8,
                minimum_level: Level::Trace,
                sample_every: 2,
            },
        ],
        routes: vec![
            Route {
                channel_id: 7,
                targets: vec![FILE, RECEIVER],
            },
            Route {
                channel_id: 8,
                targets: vec![FILE, RECEIVER],
            },
        ],
        memory: None,
    }
}
fn delta(old: CompletionTotals, new: CompletionTotals) -> Result<CompletionTotals, Error> {
    if old.overflowed || new.overflowed || old.busy != 0 || new.busy != 0 {
        return Err(invalid("invalid completion endpoints"));
    }
    let diff = |a: u64, b: u64| {
        a.checked_sub(b)
            .ok_or_else(|| invalid("decreasing completion totals"))
    };
    Ok(CompletionTotals {
        records: diff(new.records, old.records)?,
        expired_records: diff(new.expired_records, old.expired_records)?,
        assigned: diff(new.assigned, old.assigned)?,
        complete: diff(new.complete, old.complete)?,
        failed: diff(new.failed, old.failed)?,
        dropped: diff(new.dropped, old.dropped)?,
        unknown: diff(new.unknown, old.unknown)?,
        pending: new.pending,
        ..Default::default()
    })
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Result<Self, Error> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "dot-logger-pipeline-{}-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }
    fn cleanup(&self) -> Result<(), Error> {
        fs::remove_dir_all(&self.0)?;
        Ok(())
    }
    fn file(&self, phase: usize) -> Result<(PathBuf, File), Error> {
        let path = self.0.join(format!("revision-{}.mcap", phase + 1));
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)?;
        Ok((path, file))
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn verify_file(path: &Path, expected: &[(CapturedRecord, Vec<u8>)]) -> Result<usize, Error> {
    let file = File::open(path)?;
    if file.metadata()?.len() > 2 * 1024 * 1024 {
        return Err(invalid("example file byte limit"));
    }
    let mut bytes = Vec::new();
    file.take(2 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err(invalid("example file grew"));
    }
    let mut count = 0;
    for message in NativeReader::new(&bytes, 512, 4096)? {
        let decoded = message?;
        let original = expected
            .get(count)
            .ok_or_else(|| invalid("extra file record"))?;
        if decoded.record != original.0 || decoded.payload != original.1 {
            return Err(invalid("file metadata/payload changed"));
        }
        count += 1;
    }
    if count != expected.len() {
        return Err(invalid("missing file records"));
    }
    Ok(count)
}
fn admit(
    p: &mut Producer<'_>,
    metadata: RecordMetadata,
    tick: u64,
) -> Result<RecordOutcome, Error> {
    match metadata.channel_id {
        7 => {
            let mut scratch = [0; 512];
            let encoded = log_record(
                &mut scratch,
                &log::Record::builder()
                    .target("dot_probe.host")
                    .level(log::Level::Info)
                    .args(format_args!("tick={tick}"))
                    .build(),
            )?;
            encoded
                .capture(p, metadata)
                .map_err(|_| invalid("log admission failed"))
        }
        8 => p
            .try_record_small(metadata, &(tick as i64).to_le_bytes())
            .map_err(|_| invalid("numeric admission failed")),
        _ => {
            let mut loan = p
                .try_acquire(64, Importance::Normal)
                .map_err(|_| invalid("raw acquisition failed"))?;
            loan.payload_mut().fill(tick as u8);
            match p
                .try_record_raw(metadata, loan)
                .map_err(|_| invalid("raw admission failed"))?
            {
                RawRecordOutcome::Accepted(r) => Ok(RecordOutcome::Accepted(r)),
                RawRecordOutcome::Skipped(loan) => {
                    drop(loan);
                    Ok(RecordOutcome::Skipped)
                }
            }
        }
    }
}
fn next<'a>(output: &mut TargetConsumer<'_, 'a>) -> Result<Delivery<'a>, Error> {
    output
        .try_next(Instant::now())
        .ok_or_else(|| invalid("output missing"))?
        .map_err(|_| invalid("output dropped"))
}
/// Run two finite synthetic phases, preserving original identity at each boundary.
/// Requires an ordinary worker outside Tokio and a fresh exclusive DB namespace.
pub fn run(
    database: db::Config,
    options: Options,
    cancelled: &AtomicBool,
) -> Result<PipelineReport, Error> {
    if cancelled.load(Ordering::Acquire)
        || options.duration.is_zero()
        || options.duration > Duration::from_secs(30)
    {
        return Err(invalid("cancelled or invalid admission budget"));
    }
    if tokio::runtime::Handle::try_current().is_ok() {
        return Err(invalid("run on an ordinary worker outside Tokio"));
    }
    let deadline = Instant::now() + options.duration;
    let plan = config()?;
    let budget = QueryBudget::new(&plan);
    let session = database.session_id;
    // Validate and reserve the DB writer before files or listener side effects.
    let writer = db::Writer::new(&budget, database)?;
    let path = Temp::new()?;
    let route = routes();
    let mut owner = CaptureOwner::new_tracked(
        &plan,
        session,
        1,
        &channels().iter().map(|c| c.channel).collect::<Vec<_>>(),
        &route,
        tracking(),
    )?;
    owner.enable_config_updates(&route)?;
    owner.enable_telemetry()?;
    let mut remote = Remote::new(&budget)?;
    let mut file_verified = 0;
    let mut remote_verified = 0;
    let mut raw_reclaimed = 0;
    let mut numeric_verified = None;
    let real_base: i64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos()
        .try_into()?;
    let phases = owner.scope(|mut scope| -> Result<Vec<Phase>, Error> {
        let completion = scope
            .completion()
            .ok_or_else(|| invalid("completion missing"))?;
        let telemetry = scope
            .telemetry()
            .ok_or_else(|| invalid("telemetry missing"))?;
        let updates = scope
            .config_updates()
            .ok_or_else(|| invalid("config updates missing"))?;
        let control = scope.control();
        let (mut producer, consumer) = scope.next().ok_or_else(|| invalid("producer missing"))?;
        let mut router = Router::new(&plan, vec![consumer], route)?;
        let mut db = Some(db::RoutedWriter::new(
            writer,
            completion,
            DATABASE,
            vec![
                db::ProjectionSchema {
                    channel_id: 7,
                    schema_id: 99,
                    schema_version: 1,
                    encoding: db::ProjectionEncoding::Utf8Event,
                },
                db::ProjectionSchema {
                    channel_id: 8,
                    schema_id: 100,
                    schema_version: 1,
                    encoding: db::ProjectionEncoding::I64Le,
                },
            ],
        )?);
        let mut phases = Vec::with_capacity(2);
        let mut numeric_expected = Vec::with_capacity(10);
        for phase in 0..2 {
            let revision = (phase + 1) as u64;
            let start = phase as u64 * 11 * SECOND;
            if phase == 1 {
                let valid = updates.validate_config(1, &patch())?;
                if !valid.information_loss || valid.restart_required {
                    return Err(invalid("invalid config preview"));
                }
                if updates.apply_config(1, &patch())? != 2 || producer.apply_pending_config() != 2 {
                    return Err(invalid("revision adoption failed"));
                }
            }
            let baseline = telemetry.sample(start, Some(completion))?;
            if baseline.revision != revision
                || !matches!(baseline.capture, Observation::Observed(_))
            {
                return Err(invalid("unstable baseline"));
            }
            let before = [FILE, DATABASE, RECEIVER]
                .map(|id| {
                    completion
                        .target_totals(id)
                        .ok_or_else(|| invalid("missing target totals"))
                })
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?;
            let (file_path, file) = path.file(phase)?;
            let mut file = SegmentWriter::new(
                router.writer_budget_for_revision(FILE, revision)?,
                WriterConfig {
                    max_loss_reports: 0,
                    max_payload_bytes: 512,
                    write_buffer_bytes: 4096,
                    chunk_target_bytes: 4096,
                    max_records: 32,
                    max_chunks: 32,
                    max_metadata_bytes: 4096,
                },
                channels(),
                file,
            )?;
            let mut expected = Vec::with_capacity(30);
            router.scope(|mut dispatch, mut outputs| -> Result<(), Error> {
                for tick in 1..=TICKS {
                    for channel in 7..=9 {
                        if cancelled.load(Ordering::Acquire) || Instant::now() >= deadline {
                            return Err(invalid("admission cancelled or deadline expired"));
                        }
                        let metadata = RecordMetadata {
                            channel_id: channel,
                            schema_id: 92 + u128::from(channel),
                            schema_version: 1,
                            mono_ns: start + tick * SECOND - 100 + u64::from(channel),
                            real_ns: real_base
                                + (phase as i64 * 100)
                                + (tick as i64 * 3)
                                + i64::from(channel),
                            source_time: None,
                            level: Level::Info,
                            importance: Some(Importance::Normal),
                        };
                        let RecordOutcome::Accepted(receipt) =
                            admit(&mut producer, metadata, tick)?
                        else {
                            continue;
                        };
                        dispatch.turn(Instant::now(), &mut [None; 16])?;
                        let original = next(&mut outputs[0])?;
                        if original.record().receipt != receipt {
                            return Err(invalid("file receipt changed"));
                        }
                        expected.push((original.record(), original.payload().to_vec()));
                        file.write_delivery(&original)?;
                        drop(original);
                        if phase == 0 && channel != 9 {
                            let original = next(&mut outputs[1])?;
                            if original.record().receipt != receipt {
                                return Err(invalid("DB receipt changed"));
                            }
                            let attempt = db
                                .as_mut()
                                .ok_or_else(|| invalid("DB writer missing"))?
                                .write_delivery(original, cancelled);
                            if channel == 8 {
                                numeric_expected.push(db::NumericPoint {
                                    receipt,
                                    real_ns: metadata.real_ns,
                                    value: db::NumericValue::Integer(tick as i64),
                                });
                            }
                            attempt.publish()?;
                            drop(attempt);
                        }
                        let full = phase == 1
                            && tick == TICKS
                            && channel == 9
                            && matches!(options.receiver, ReceiverMode::FullOnLast);
                        remote_verified +=
                            usize::from(remote.receive(next(&mut outputs[2])?, full)?);
                        raw_reclaimed += dispatch.turn(Instant::now(), &mut [None; 16])?.reclaimed;
                    }
                    telemetry.sample(start + tick * SECOND, Some(completion))?;
                }
                Ok(())
            })?;
            let finished = file.finish()?;
            let mut file = finished.into_inner();
            // This example reports Written, not a synthetic Synced completion.
            std::io::Write::flush(&mut file)?;
            file.sync_all()?;
            drop(file);
            file_verified += verify_file(&file_path, &expected)?;
            let Observation::Observed(window) = telemetry.window(10) else {
                return Err(invalid("complete stable window missing"));
            };
            if window.revision != revision
                || window.from_ns != start
                || window.to_ns != start + 10 * SECOND
            {
                return Err(invalid("window endpoints changed"));
            }
            let after = [FILE, DATABASE, RECEIVER]
                .map(|id| {
                    completion
                        .target_totals(id)
                        .ok_or_else(|| invalid("missing target totals"))
                })
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?;
            let totals = before
                .into_iter()
                .zip(after)
                .map(|(a, b)| delta(a, b))
                .collect::<Result<Vec<_>, _>>()?;
            let mut input = ReportInput::from_window(
                window,
                Conditions {
                    environment_id: 1,
                    workload_id: u128::from(revision),
                    revision,
                    target_id: FILE,
                    boundary: Boundary::Written,
                },
            );
            let t = totals[0];
            input.output = Observation::Observed(OutputDelta {
                complete: t.complete,
                pending: t.pending,
                failed: t.failed,
                dropped: t.dropped,
                unknown: t.unknown,
            });
            phases.push(Phase {
                revision,
                accepted: window.counts.accepted,
                skipped: window.counts.skipped,
                file: totals[0],
                database: totals[1],
                receiver: totals[2],
                snapshot: Snapshot::new(Report::build(input, None, window.to_ns), 1),
            });
            if phase == 0 {
                let mut writer = db
                    .take()
                    .ok_or_else(|| invalid("DB writer missing"))?
                    .into_writer();
                if options.verify_query && totals[1].complete == 20 {
                    let page = remote.query(
                        &mut writer,
                        db::NumericQuery {
                            channel: 8,
                            from_ns: real_base,
                            to_ns: real_base + 100,
                            max_rows: 16,
                            max_response_bytes: 32768,
                        },
                        cancelled,
                    )?;
                    if page.points() != numeric_expected {
                        return Err(invalid("DB numeric receipt/time/value changed"));
                    }
                    numeric_verified = Some(page.points().len());
                    drop(page);
                }
                drop(writer);
            }
        }
        control.close();
        Ok(phases)
    });
    // Always join/close actual server IO, even after a capture/file/query failure.
    let closed = remote.close();
    drop(remote);
    drop(owner);
    let cleaned = path.cleanup();
    drop(path);
    let external_after_close = budget.used_bytes();
    if external_after_close != 0 {
        return Err(invalid("external reservations still owned"));
    }
    let mut phases = phases?.into_iter();
    closed?;
    cleaned?;
    let before = phases
        .next()
        .ok_or_else(|| invalid("before phase missing"))?;
    let after = phases
        .next()
        .ok_or_else(|| invalid("after phase missing"))?;
    let comparison = Comparison::build(before.snapshot, after.snapshot);
    Ok(PipelineReport {
        before,
        after,
        file_verified,
        remote_verified,
        raw_reclaimed,
        numeric_verified,
        comparison,
        external_after_close,
    })
}
