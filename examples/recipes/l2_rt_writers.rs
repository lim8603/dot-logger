//! L2 recipe: two real-time writers - small typed records and
//! zero-copy camera frames from a loaned slot - and the shutdown report.
use dot_logger::capture::Importance;
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
        // 30 Hz camera frames up to 256 KiB and 1 kHz small records per writer
        .preset(Preset::sensor(30, 256 << 10).small(1000, 256))
        .file_output(std::env::temp_dir(), Retention::ring(256 << 20)) // an existing directory
        .channel::<Pose>("robot/pose")
        .channel::<[u8]>("camera/image")
        .writer("control")
        .writer("camera")
        .start()?;
    let pose = logger.channel::<Pose>("robot/pose")?;
    let image = logger.channel::<[u8]>("camera/image")?;
    let mut control = logger.take_writer("control")?;
    let mut camera = logger.take_writer("camera")?;
    let threads = [
        std::thread::spawn(move || {
            for x in (0..1000).map(|i| i as f32) {
                // Never waits or allocates; a full queue is a counted rejection.
                let _ = control.try_log(pose, &Pose { x, y: 0.0 });
                std::thread::sleep(Duration::from_millis(1));
            }
        }),
        std::thread::spawn(move || {
            for frame in 0..30u8 {
                let metadata = image.metadata(camera.now_ns(), 0);
                let mut producer = camera.producer();
                // Fill a pooled slot in place and hand it over without a copy.
                if let Ok(mut loan) = producer.try_acquire(256 << 10, Importance::Normal) {
                    loan.payload_mut().fill(frame);
                    let _ = producer.try_record_raw(metadata, loan);
                }
                std::thread::sleep(Duration::from_millis(33));
            }
        }),
    ];
    for thread in threads {
        thread.join().map_err(|_| "writer thread panicked")?;
    }
    match logger.shutdown(Duration::from_secs(5))? {
        shutdown if shutdown.is_success() => Ok(()),
        shutdown => Err(format!("records not all synchronized: {shutdown:?}").into()),
    }
}
