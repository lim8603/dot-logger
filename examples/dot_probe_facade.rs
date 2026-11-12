//! dot-probe style integration on the facade only (TASK-029 S4): `tracing`
//! logs, typed joint telemetry at 1 kHz and 30 Hz camera frames into files,
//! plus a loopback gRPC server that `dot-logger tail` can subscribe to.
//! No support module, `unsafe` or memory partition.
//!
//! `dot_probe_pipeline` (dot-logger-client) stays the advanced-path reference
//! for what the facade has no path for yet: an InfluxDB route and comparing
//! results before and after a configuration change.
//!
//! ```sh
//! cargo run -p dot-logger --features remote,tracing-adapter --example dot_probe_facade [DIR]
//! ```
use dot_logger::adapters::remote::ServerConfig;
use dot_logger::capture::Importance;
use dot_logger::logger::{
    EncodeError, Logger, Preset, Record, Retention, Schema, Shutdown, WriterQuota,
};
use std::time::Duration;
use tracing::Level;

/// Six joint positions and velocities, little-endian `f32`.
struct Joints {
    position: [f32; 6],
    velocity: [f32; 6],
}
impl Record for Joints {
    const SCHEMA: Schema = Schema::new(0x4a4f_494e, 1, "le-f32x12");
    fn encode(&self, out: &mut [u8]) -> Result<usize, EncodeError> {
        let out = out.get_mut(..48).ok_or(EncodeError)?;
        let values = self.position.iter().chain(&self.velocity);
        for (bytes, value) in out.chunks_exact_mut(4).zip(values) {
            bytes.copy_from_slice(&value.to_le_bytes());
        }
        Ok(48)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::args_os()
        .nth(1)
        .map_or_else(std::env::temp_dir, Into::into);
    let logger = Logger::builder()
        // 30 Hz frames up to 256 KiB and 1 kHz small records per writer.
        .preset(Preset::sensor(30, 256 << 10).small(1000, 256))
        .file_output(dir, Retention::ring(256 << 20))
        .channel::<Joints>("probe/joints")
        .channel::<[u8]>("probe/camera")
        .writer("control")
        .writer("camera")
        .tracing("probe/log", Level::INFO, WriterQuota::small(64, 1024))
        .remote(ServerConfig {
            enabled: true,
            ..ServerConfig::default()
        })
        .start()?;
    tracing::subscriber::set_global_default(logger.tracing()?)?;
    tracing::info!(addr = ?logger.remote_addr(), "dot-logger started; `dot-logger tail` can subscribe");
    let joints = logger.channel::<Joints>("probe/joints")?;
    let camera = logger.channel::<[u8]>("probe/camera")?;
    let mut control = logger.take_writer("control")?;
    let mut video = logger.take_writer("camera")?;
    let control_loop = std::thread::spawn(move || {
        let mut rejected = 0u32;
        for tick in 0..2000u16 {
            let angle = f32::from(tick) * 0.001;
            let state = Joints {
                position: [angle; 6],
                velocity: [1.0; 6],
            };
            // Never waits or allocates; count what the full queue refused.
            rejected += u32::from(control.try_log(joints, &state).is_err());
            std::thread::sleep(Duration::from_millis(1));
        }
        rejected
    });
    let camera_loop = std::thread::spawn(move || {
        let mut rejected = 0u32;
        for frame in 0..60u8 {
            let metadata = camera.metadata(video.now_ns(), 0);
            let mut producer = video.producer();
            let sent = producer
                .try_acquire(256 << 10, Importance::Normal)
                .map(|mut loan| {
                    loan.payload_mut().fill(frame);
                    producer.try_record_raw(metadata, loan).is_ok()
                });
            rejected += u32::from(!matches!(sent, Ok(true)));
            std::thread::sleep(Duration::from_millis(33));
        }
        rejected
    });
    let control_rejected = control_loop.join().map_err(|_| "control loop panicked")?;
    let camera_rejected = camera_loop.join().map_err(|_| "camera loop panicked")?;
    tracing::info!(control_rejected, camera_rejected, "loops finished");
    let shutdown = logger.shutdown(Duration::from_secs(5))?;
    if let Shutdown::Closed(report) = &shutdown {
        println!(
            "synchronized {} records; rejected control {control_rejected}, camera {camera_rejected}",
            report.core.required.complete
        );
    }
    if shutdown.is_success() {
        Ok(())
    } else {
        Err(format!("records not all synchronized: {shutdown:?}").into())
    }
}
