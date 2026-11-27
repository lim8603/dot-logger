//! Write a few points to InfluxDB v2 with the advanced-path `Writer`, then read
//! the numeric ones back.
//!
//! ```text
//! INFLUX_HOST=http://127.0.0.1:8086 INFLUX_ORG=my-org INFLUX_BUCKET=my-bucket \
//! INFLUX_TOKEN=... cargo run -p dot-logger --features influxdb --example influxdb_write
//! ```
//!
//! The endpoint must be HTTPS or a literal loopback HTTP address. The bucket
//! should be one this program owns: the writer rejects timestamps that are not
//! newer than its own last point per channel, but it does not look at what is
//! already stored. Exit codes: 0 read back everything, 1 failed, 2 usage.
use dot_logger::{
    adapters::influxdb::{
        Channel, CollisionPolicy, Config, Kind, NumericQuery, Outcome, Point, Value, Writer,
    },
    capture::CaptureReceipt,
    config::{
        ImportancePolicy, MemoryBudget, PoolQuota, ProducerQuota, SessionBuilder, SessionConfig,
        WorkerThreadingConfig,
    },
    platform::{PlatformCapabilities, Requirement},
    query::QueryBudget,
};
use std::{
    process::ExitCode,
    sync::atomic::AtomicBool,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const KIB: usize = 1024;
const SESSION: u128 = 0x2a;
const TEMPERATURE: u32 = 1;
const EVENTS: u32 = 2;

fn main() -> ExitCode {
    let settings = ["INFLUX_HOST", "INFLUX_ORG", "INFLUX_BUCKET", "INFLUX_TOKEN"]
        .map(|name| std::env::var(name).ok());
    let [
        Some(endpoint),
        Some(organization),
        Some(bucket),
        Some(token),
    ] = settings
    else {
        eprintln!("usage: set INFLUX_HOST, INFLUX_ORG, INFLUX_BUCKET and INFLUX_TOKEN");
        return ExitCode::from(2);
    };
    match run(endpoint, organization, bucket, token) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("influxdb_write: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(
    endpoint: String,
    organization: String,
    bucket: String,
    token: String,
) -> Result<(), Box<dyn std::error::Error>> {
    // The writer charges its request and response buffers to the session's
    // external memory partition, so it needs a validated session plan.
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
    .validate(&PlatformCapabilities::default())?;
    let budget = QueryBudget::new(&plan);
    let mut writer = Writer::new(
        &budget,
        Config {
            endpoint,
            organization,
            bucket,
            token,
            session_id: SESSION,
            measurement: "influxdb_write_example".into(),
            device: "robot_1".into(),
            channels: vec![
                Channel {
                    id: TEMPERATURE,
                    kind: Kind::Float,
                },
                Channel {
                    id: EVENTS,
                    kind: Kind::Event,
                },
            ],
            collision_policy: CollisionPolicy::RejectNonIncreasing,
            max_points: 8,
            max_batch_bytes: 4 * KIB,
            max_event_bytes: 64,
            max_attempts: 2,
            request_timeout: Duration::from_secs(10),
            retry_delay: Duration::from_millis(100),
            ttl: Duration::from_secs(30),
        },
    )?;

    // In an application the receipt comes from the capture that accepted the
    // record; here the sequence numbers are made up.
    let receipt = |seq| CaptureReceipt {
        session_id: SESSION,
        producer_id: 0,
        accepted_seq: seq,
    };
    let start: i64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos()
        .try_into()?;
    let points = [
        (TEMPERATURE, Value::Float(21.5)),
        (TEMPERATURE, Value::Float(21.75)),
        (EVENTS, Value::Event("door opened")),
        (TEMPERATURE, Value::Float(22.0)),
    ];
    let points: Vec<Point> = (0..)
        .zip(points)
        .map(|(i, (channel, value))| Point {
            channel,
            receipt: receipt(i as u64 + 1),
            real_ns: start + i * 1_000_000,
            value,
        })
        .collect();

    // The adapter starts no runtime of its own; the host supplies one.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let cancel = AtomicBool::new(false);
    let report = runtime.block_on(writer.write(&points, &cancel))?;
    println!(
        "write: {:?} after {} attempt(s), HTTP {:?}, {} points",
        report.outcome, report.attempts, report.status, report.points
    );
    if report.outcome != Outcome::Acknowledged {
        return Err("the database did not acknowledge the batch".into());
    }

    let page = runtime.block_on(writer.query_numeric(
        NumericQuery {
            channel: TEMPERATURE,
            from_ns: start,
            to_ns: start + 1_000_000_000,
            max_rows: 16,
            max_response_bytes: 16 * KIB,
        },
        &cancel,
    ))?;
    for point in page.points() {
        println!(
            "  t={} seq={} {:?}",
            point.real_ns, point.receipt.accepted_seq, point.value
        );
    }
    let written = points.iter().filter(|p| p.channel == TEMPERATURE).count();
    println!("read back {}/{written} numeric points", page.points().len());
    if page.points().len() != written {
        return Err("read back a different number of points than written".into());
    }
    Ok(())
}
