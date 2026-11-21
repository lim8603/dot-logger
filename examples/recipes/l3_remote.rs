//! L3 recipe: L1 plus a loopback gRPC server that streams copies of
//! the written records to subscribers (`dot-logger tail`).
use dot_logger::adapters::remote::ServerConfig;
use dot_logger::logger::{EncodeError, Logger, Preset, Record, Retention, Schema};
use std::time::Duration;

/// Your message type and its encoding (not counted as integration code).
struct Pose {
    x: f32,
    y: f32,
}
impl Record for Pose {
    const SCHEMA: Schema = Schema::new(0x504f_5345, 1, "le-f32x2");
    fn encode(&self, out: &mut [u8]) -> Result<usize, EncodeError> {
        let out = out.get_mut(..8).ok_or(EncodeError)?;
        out[..4].copy_from_slice(&self.x.to_le_bytes());
        out[4..].copy_from_slice(&self.y.to_le_bytes());
        Ok(8)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logger = Logger::builder()
        .preset(Preset::control(1000)) // 1 kHz control loop: fills memory, sync and drop limits
        .file_output(std::env::temp_dir(), Retention::ring(64 << 20)) // an existing directory
        .channel::<Pose>("robot/pose")
        .writer("control")
        .remote(ServerConfig {
            enabled: true,
            ..ServerConfig::default()
        }) // loopback, anonymous read
        .start()?;
    println!("http://{}", logger.remote_addr().ok_or("no listener")?);
    let pose = logger.channel::<Pose>("robot/pose")?;
    let mut writer = logger.take_writer("control")?;
    let rt = std::thread::spawn(move || {
        for x in (0..1000).map(|i| i as f32) {
            // Never waits or allocates; a full queue is a counted rejection.
            let _ = writer.try_log(pose, &Pose { x, y: 0.0 });
        }
    });
    rt.join().map_err(|_| "control thread panicked")?;
    match logger.shutdown(Duration::from_secs(5))? {
        shutdown if shutdown.is_success() => Ok(()),
        shutdown => Err(format!("records not all synchronized: {shutdown:?}").into()),
    }
}
