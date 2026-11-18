//! Start a short-lived facade host that publishes JSON pose samples.
use dot_logger::{
    adapters::remote::{RemoteLimits, ServerConfig},
    logger::{EncodeError, Logger, Preset, Record, Retention, Schema},
};
use std::{fs, path::PathBuf, thread, time::Duration};

const TOPIC: &str = "robot/pose";
const DESCRIPTOR: &[u8] =
    br#"{"type":"object","properties":{"x":{"type":"integer"},"y":{"type":"integer"}}}"#;
const PAYLOAD: &[u8] = br#"{"x":51,"y":-7}"#;

struct Pose;

impl Record for Pose {
    const SCHEMA: Schema =
        Schema::new(0x504f_5345, 1, "json").descriptor("Pose", "jsonschema", DESCRIPTOR);

    fn encode(&self, output: &mut [u8]) -> Result<usize, EncodeError> {
        output
            .get_mut(..PAYLOAD.len())
            .ok_or(EncodeError)?
            .copy_from_slice(PAYLOAD);
        Ok(PAYLOAD.len())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: live_host <new-output-directory>")?;
    fs::create_dir(&directory)?;
    let logger = Logger::builder()
        .preset(Preset::control(100))
        .file_output(&directory, Retention::ring(64 << 20))
        .channel::<Pose>(TOPIC)
        .writer("demo")
        .remote_with(
            ServerConfig {
                enabled: true,
                ..ServerConfig::default()
            },
            RemoteLimits {
                connections: 4,
                subscriptions: 4,
                commands: 2,
                message_bytes: 32 * 1024,
                record_bytes: 64 * 1024,
                subscription_records: 4,
                subscription_bytes: 4 * 64 * 1024,
                max_age: Duration::from_secs(10),
                request_timeout: Duration::from_secs(2),
            },
        )
        .start()?;
    let pose = logger.channel::<Pose>(TOPIC)?;
    let metadata = pose.metadata(0, 0);
    let path = directory.join("viewer-catalog.json");
    let catalog = fs::File::create_new(&path)?;
    serde_json::to_writer_pretty(
        catalog,
        &serde_json::json!({
            "version": 1,
            "channels": [{
                "id": 1,
                "channel_id": metadata.channel_id,
                "schema_id": format!("{:032x}", metadata.schema_id),
                "schema_version": metadata.schema_version,
                "topic": TOPIC,
                "encoding": "json",
                "schema_name": "Pose",
                "schema": std::str::from_utf8(DESCRIPTOR)?,
            }]
        }),
    )?;
    let address = logger
        .remote_addr()
        .ok_or("upstream listener did not start")?;
    println!("http://{address}");
    println!("{}", path.display());
    let mut writer = logger.take_writer("demo")?;
    for _ in 0..600 {
        writer
            .try_log(pose, &Pose)
            .map_err(|error| format!("demo sample rejected: {error:?}"))?;
        thread::sleep(Duration::from_millis(100));
    }
    drop(writer);
    if logger.shutdown(Duration::from_secs(5))?.is_success() {
        Ok(())
    } else {
        Err("the demo file did not synchronize".into())
    }
}
