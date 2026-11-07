//! TC-013 F16 (TASK-023 S2a): a channel can be routed to a subset of the file
//! outputs (Human 2026-11-07: outputs by declaration order).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use dot_logger::logger::{
    Budget, ChannelOptions, Logger, LoggerBuilder, Retention, Shutdown, StartError, WriterQuota,
};
use dot_logger::storage::decode_record;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MIB: usize = 1024 * 1024;

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "dot-logger-tc013-routing-{}-{nanos}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn sub(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        if !path.exists() {
            std::fs::create_dir(&path).unwrap();
        }
        path
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn budget() -> Budget {
    Budget::new()
        .memory_bytes(64 * MIB)
        .sync_every(Duration::from_millis(20))
        .drop_deadline(Duration::from_secs(3))
        .writer_quota(WriterQuota::small(64, 64))
}

/// Channel ID and payload of every record stored below one output.
fn stored(dir: &Path) -> Vec<(u32, Vec<u8>)> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("mcap"))
        .collect();
    files.sort();
    let mut records = Vec::new();
    for file in files {
        let bytes = std::fs::read(file).unwrap();
        for message in mcap::MessageStream::new(&bytes).unwrap() {
            let stored = decode_record(&message.unwrap().data, 64).unwrap();
            records.push((stored.record.metadata.channel_id, stored.payload.to_vec()));
        }
    }
    records
}

fn two_outputs(temp: &Temp) -> (LoggerBuilder, PathBuf, PathBuf) {
    let a = temp.sub("a");
    let b = temp.sub("b");
    let builder = Logger::builder()
        .budget(budget())
        .file_output(&a, Retention::ring(256 * MIB as u64))
        .file_output(&b, Retention::ring(256 * MIB as u64));
    (builder, a, b)
}

#[test]
fn f16_channels_reach_only_their_selected_outputs() {
    let temp = Temp::new();
    let (builder, _, _) = two_outputs(&temp);
    let logger = builder
        .channel_with::<[u8]>("first", ChannelOptions::new().outputs(&[0]))
        .channel_with::<[u8]>("second", ChannelOptions::new().outputs(&[1]))
        .channel::<[u8]>("both")
        .writer("w")
        .start()
        .unwrap();
    let first = logger.channel::<[u8]>("first").unwrap();
    let second = logger.channel::<[u8]>("second").unwrap();
    let both = logger.channel::<[u8]>("both").unwrap();
    let mut writer = logger.take_writer("w").unwrap();
    for (channel, byte) in [(first, 1u8), (second, 2), (both, 3)] {
        while writer.try_log(channel, &[byte; 4][..]).is_err() {
            std::thread::yield_now();
        }
    }
    drop(writer);
    let directories = logger.directories().to_vec();
    let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(10)).unwrap() else {
        panic!("shutdown must close");
    };
    assert!(report.is_success(), "{report:?}");
    // Channel IDs follow declaration order from 1.
    assert_eq!(
        stored(&directories[0]),
        vec![(1, vec![1; 4]), (3, vec![3; 4])]
    );
    assert_eq!(
        stored(&directories[1]),
        vec![(2, vec![2; 4]), (3, vec![3; 4])]
    );
}

#[test]
fn f16_empty_unknown_or_repeated_outputs_reject_start() {
    let temp = Temp::new();
    for outputs in [&[][..], &[2][..], &[1, 1][..]] {
        let (builder, _, _) = two_outputs(&temp);
        let result = builder
            .channel_with::<[u8]>("c", ChannelOptions::new().outputs(outputs))
            .writer("w")
            .plan();
        assert!(
            matches!(result, Err(StartError::InvalidOutputs)),
            "{outputs:?}: {result:?}"
        );
    }
    // Nothing was created by the rejected declarations.
    assert_eq!(std::fs::read_dir(temp.0.join("a")).unwrap().count(), 0);
}

#[test]
fn f16_default_options_route_to_every_output() {
    assert_eq!(ChannelOptions::new(), ChannelOptions::default());
    let temp = Temp::new();
    let (builder, _, _) = two_outputs(&temp);
    let logger = builder
        .channel_with::<[u8]>("c", ChannelOptions::new())
        .writer("w")
        .start()
        .unwrap();
    let c = logger.channel::<[u8]>("c").unwrap();
    let mut writer = logger.take_writer("w").unwrap();
    while writer.try_log(c, &[9u8; 2][..]).is_err() {
        std::thread::yield_now();
    }
    drop(writer);
    let directories = logger.directories().to_vec();
    let Shutdown::Closed(report) = logger.shutdown(Duration::from_secs(10)).unwrap() else {
        panic!("shutdown must close");
    };
    assert!(report.is_success(), "{report:?}");
    for dir in &directories {
        assert_eq!(stored(dir), vec![(1, vec![9; 2])]);
    }
}
