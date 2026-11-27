//! TASK-047: an application thread that writes to InfluxDB beside the soak
//! load, so a DB outage can be checked against RT capture and file output.
//!
//! Two channels at `HZ` each on a fixed time grid: channel 1 an integer equal
//! to the point index, channel 2 a float equal to half of it. Every `TICK` the
//! worker sends the points due since its last batch, at most `CAP` per channel;
//! points that fell due while a call was blocked are skipped and counted, never
//! queued, so memory stays bounded however long the DB is away. Each batch is
//! one JSON line (index range, outcome) in the batch log, for comparison with
//! the DB after recovery. Settings come from INFLUX_HOST, INFLUX_ORG,
//! INFLUX_BUCKET and INFLUX_TOKEN. Harness only, never a product API.
use dot_logger::{
    adapters::influxdb::{
        Channel, CollisionPolicy, Config, Error as WriteError, Kind, Outcome, Point, Value, Writer,
    },
    capture::CaptureReceipt,
    config::{
        ImportancePolicy, MemoryBudget, PoolQuota, ProducerQuota, SessionBuilder, SessionConfig,
        WorkerThreadingConfig,
    },
    platform::{PlatformCapabilities, Requirement},
    query::QueryBudget,
};
use serde_json::{Value as Json, json};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub const MEASUREMENT: &str = "soak_db";
pub const HZ: u64 = 100;
const PERIOD_NS: i64 = 1_000_000_000 / HZ as i64;
const TICK: Duration = Duration::from_millis(100);
const CAP: u64 = 50;
const SESSION: u128 = 0x47;
const KIB: usize = 1024;

/// The four settings, read before the logger starts so a missing one fails early.
pub struct Settings {
    endpoint: String,
    organization: String,
    bucket: String,
    token: String,
}
impl Settings {
    pub fn from_env() -> Result<Self, String> {
        let get = |name| std::env::var(name).map_err(|_| format!("--influx needs {name}"));
        Ok(Self {
            endpoint: get("INFLUX_HOST")?,
            organization: get("INFLUX_ORG")?,
            bucket: get("INFLUX_BUCKET")?,
            token: get("INFLUX_TOKEN")?,
        })
    }
}

/// Cumulative results; read by the sampler while the worker runs.
#[derive(Default)]
pub struct Stats {
    calls: AtomicU64,
    acknowledged: AtomicU64,
    unknown: AtomicU64,
    rejected: AtomicU64,
    not_sent: AtomicU64,
    busy: AtomicU64,
    errors: AtomicU64,
    points_acknowledged: AtomicU64,
    points_unknown: AtomicU64,
    points_sent: AtomicU64,
    points_skipped: AtomicU64,
    last_ack_unix_ms: AtomicU64,
    max_call_ms: AtomicU64,
    interval_max_call_ms: AtomicU64,
    in_call_since_unix_ms: AtomicU64,
}
impl Stats {
    pub fn sample(&self) -> Json {
        let get = |a: &AtomicU64| a.load(Ordering::Acquire);
        json!({"calls":get(&self.calls),"acknowledged":get(&self.acknowledged),
            "unknown":get(&self.unknown),"rejected":get(&self.rejected),
            "not_sent":get(&self.not_sent),"busy":get(&self.busy),"errors":get(&self.errors),
            "points_acknowledged":get(&self.points_acknowledged),
            "points_unknown":get(&self.points_unknown),"points_sent":get(&self.points_sent),
            "points_skipped":get(&self.points_skipped),
            "last_ack_unix_ms":get(&self.last_ack_unix_ms),"max_call_ms":get(&self.max_call_ms),
            "interval_max_call_ms":self.interval_max_call_ms.swap(0, Ordering::AcqRel),
            "in_call_since_unix_ms":get(&self.in_call_since_unix_ms)})
    }
}

fn unix_ns() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as i64)
}

pub fn spawn(
    settings: Settings,
    log: PathBuf,
    stats: Arc<Stats>,
    stop: Arc<AtomicBool>,
) -> std::io::Result<JoinHandle<Result<(), String>>> {
    thread::Builder::new()
        .name("soak-influx".into())
        .spawn(move || run(settings, log, &stats, &stop))
}

fn run(settings: Settings, log: PathBuf, stats: &Stats, stop: &AtomicBool) -> Result<(), String> {
    let error = |e: &dyn std::fmt::Debug| format!("influx worker: {e:?}");
    // The writer charges two bodies and its scratch to the external partition.
    let plan = SessionBuilder::new(SessionConfig {
        memory: MemoryBudget {
            total_bytes: 1024 * KIB,
            input_bytes: 64 * KIB,
            raw_bytes: 0,
            output_bytes: 64 * KIB,
            external_bytes: 512 * KIB,
            control_bytes: 64 * KIB,
            worker_bytes: 64 * KIB,
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
    .map_err(|e| error(&e))?;
    let budget = QueryBudget::new(&plan);
    let mut writer = Writer::new(
        &budget,
        Config {
            endpoint: settings.endpoint,
            organization: settings.organization,
            bucket: settings.bucket,
            token: settings.token,
            session_id: SESSION,
            measurement: MEASUREMENT.into(),
            device: "soak".into(),
            channels: vec![
                Channel {
                    id: 1,
                    kind: Kind::Integer,
                },
                Channel {
                    id: 2,
                    kind: Kind::Float,
                },
            ],
            collision_policy: CollisionPolicy::RejectNonIncreasing,
            max_points: (2 * CAP) as usize,
            max_batch_bytes: 32 * KIB,
            max_event_bytes: 1,
            max_attempts: 2,
            request_timeout: Duration::from_secs(2),
            retry_delay: Duration::from_millis(200),
            ttl: Duration::from_secs(5),
        },
    )
    .map_err(|e| error(&e))?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| error(&e))?;
    let mut out = BufWriter::new(File::create(&log).map_err(|e| error(&e))?);
    let cancel = AtomicBool::new(false);
    // Grid origin: point i is due at origin + i × period.
    let origin = unix_ns();
    let mut next = 0_u64;
    let mut points = Vec::with_capacity((2 * CAP) as usize);
    let mut tick = Instant::now() + TICK;
    while !stop.load(Ordering::Acquire) {
        thread::sleep(tick.saturating_duration_since(Instant::now()));
        tick = (tick + TICK).max(Instant::now());
        let now = unix_ns();
        // Points 0..due are due: their grid time is not after `now`.
        let due = ((now - origin) / PERIOD_NS) as u64 + 1;
        if due <= next {
            continue;
        }
        let from = next.max(due.saturating_sub(CAP));
        stats
            .points_skipped
            .fetch_add(2 * (from - next), Ordering::AcqRel);
        points.clear();
        for i in from..due {
            let t = origin + i as i64 * PERIOD_NS;
            for (channel, value) in [
                (1, Value::Integer(i as i64)),
                (2, Value::Float(i as f64 / 2.0)),
            ] {
                points.push(Point {
                    channel,
                    receipt: CaptureReceipt {
                        session_id: SESSION,
                        producer_id: 0,
                        accepted_seq: i * 2 + u64::from(channel),
                    },
                    real_ns: t,
                    value,
                });
            }
        }
        next = due;
        let n = points.len() as u64;
        let started = Instant::now();
        stats
            .in_call_since_unix_ms
            .store((now / 1_000_000) as u64, Ordering::Release);
        let result = runtime.block_on(writer.write(&points, &cancel));
        stats.in_call_since_unix_ms.store(0, Ordering::Release);
        let ms = started.elapsed().as_millis() as u64;
        stats.calls.fetch_add(1, Ordering::AcqRel);
        stats.max_call_ms.fetch_max(ms, Ordering::AcqRel);
        stats.interval_max_call_ms.fetch_max(ms, Ordering::AcqRel);
        let line = match result {
            Ok(report) => {
                let (batches, counted) = match report.outcome {
                    Outcome::Acknowledged => {
                        stats
                            .last_ack_unix_ms
                            .store((unix_ns() / 1_000_000) as u64, Ordering::Release);
                        (&stats.acknowledged, Some(&stats.points_acknowledged))
                    }
                    Outcome::Unknown => (&stats.unknown, Some(&stats.points_unknown)),
                    Outcome::Rejected => (&stats.rejected, None),
                    Outcome::NotSent => (&stats.not_sent, None),
                };
                batches.fetch_add(1, Ordering::AcqRel);
                if let Some(counted) = counted {
                    counted.fetch_add(n, Ordering::AcqRel);
                }
                if report.attempts > 0 {
                    stats.points_sent.fetch_add(n, Ordering::AcqRel);
                }
                json!({"from":from,"to":due,"outcome":format!("{:?}",report.outcome),
                    "stop":format!("{:?}",report.stop),"attempts":report.attempts,
                    "status":report.status,"ms":ms,"at_unix_ms":now / 1_000_000})
            }
            Err(e) => {
                if e == WriteError::Busy {
                    stats.busy.fetch_add(1, Ordering::AcqRel);
                } else {
                    stats.errors.fetch_add(1, Ordering::AcqRel);
                }
                json!({"from":from,"to":due,"error":format!("{e:?}"),"ms":ms,
                    "at_unix_ms":now / 1_000_000})
            }
        };
        writeln!(out, "{line}").map_err(|e| error(&e))?;
    }
    writeln!(
        out,
        "{}",
        json!({"end":true,"origin_unix_ns":origin,"period_ns":PERIOD_NS,"next":next})
    )
    .map_err(|e| error(&e))?;
    out.flush().map_err(|e| error(&e))
}
