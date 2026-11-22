//! TASK-024 S1: native files exchanged between operating systems.
//!
//! `write <dir>` records a fixed, deterministic set on this OS through the
//! facade: two channels (one with a schema descriptor), Normal and Important
//! records at different levels, fixed clocks, and enough data to rotate into
//! several files. `verify <dir>...` reads every native file below each
//! directory, wherever it was written, and checks it record by record against
//! the same definition, plus the loss reports and a plain MCAP reader's view.
//!
//! CI runs `write` on Linux, Windows and macOS, then `verify` on each of them
//! over all three outputs. Verification harness only, never a product API.
use dot_logger::capture::{Importance, Level};
use dot_logger::logger::{EncodeError, Logged, Logger, Record, Retention, Schema, Sizing};
use dot_logger::storage::{LossReader, NativeReader};
use std::{
    collections::BTreeMap,
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    time::Duration,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const TICKS: u64 = 500;
const FIXED_PER_TICK: u64 = 60;
const BLOBS_PER_TICK: u64 = 20;
const FIXED: u64 = TICKS * FIXED_PER_TICK;
const BLOBS: u64 = TICKS * BLOBS_PER_TICK;
const FIXED_BYTES: usize = 256;
const MIN_BLOB: usize = 512;
const MAX_BLOB: usize = 4096;
const RING_BYTES: u64 = 256 << 20;
const MIN_FILES: usize = 2;
const MAX_PRODUCERS: usize = 4;
/// 2026-01-01T00:00:00Z; wall clocks are fixed so every OS writes the same.
const BASE_REAL_NS: i64 = 1_767_225_600_000_000_000;
const FIXED_TOPIC: &str = "exchange/fixed";
const BLOB_TOPIC: &str = "exchange/blob";
const DESCRIPTOR: &[u8] = br#"{"type":"string","contentEncoding":"base64"}"#;

/// Fixed-size record with a schema descriptor.
struct Fixed(u64);
impl Record for Fixed {
    const SCHEMA: Schema = Schema::new(0x4558_4348_0000_0001, 3, "exchange-pattern").descriptor(
        "exchange.Fixed",
        "jsonschema",
        DESCRIPTOR,
    );
    fn encode(&self, out: &mut [u8]) -> std::result::Result<usize, EncodeError> {
        fill(self.0, 31, out.get_mut(..FIXED_BYTES).ok_or(EncodeError)?);
        Ok(FIXED_BYTES)
    }
}
fn fill(index: u64, step: u64, out: &mut [u8]) {
    out[..8].copy_from_slice(&index.to_le_bytes());
    for (k, byte) in out.iter_mut().enumerate().skip(8) {
        *byte = index.wrapping_mul(step).wrapping_add(k as u64) as u8;
    }
}
fn blob_len(index: u64) -> usize {
    MIN_BLOB + (index * 977 % (MAX_BLOB - MIN_BLOB + 1) as u64) as usize
}
/// Global write order of a record; both clocks derive from it.
fn order(fixed: bool, index: u64) -> u64 {
    let per_tick = FIXED_PER_TICK + BLOBS_PER_TICK;
    if fixed {
        index / FIXED_PER_TICK * per_tick + index % FIXED_PER_TICK
    } else {
        index / BLOBS_PER_TICK * per_tick + FIXED_PER_TICK + index % BLOBS_PER_TICK
    }
}
fn clocks(fixed: bool, index: u64) -> (u64, i64) {
    let n = order(fixed, index);
    (1_000_000 + n * 1_000, BASE_REAL_NS + n as i64 * 1_000)
}

fn invalid(text: String) -> Box<dyn Error> {
    io::Error::other(text).into()
}

fn write(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    let logger = Logger::builder()
        .preset(Sizing::new(Duration::from_secs(1), Duration::from_secs(10)).small(8_000, MAX_BLOB))
        .file_output(dir, Retention::ring(RING_BYTES))
        .channel::<Fixed>(FIXED_TOPIC)
        .channel::<[u8]>(BLOB_TOPIC)
        .writer("exchange")
        .start()?;
    let fixed = logger.channel::<Fixed>(FIXED_TOPIC)?;
    let blob = logger
        .channel::<[u8]>(BLOB_TOPIC)?
        .at(Level::Warn)
        .important();
    let mut writer = logger.take_writer("exchange")?;
    let mut buffer = vec![0_u8; MAX_BLOB];
    let accept = |logged: std::result::Result<Logged, _>, what: &str| match logged {
        Ok(Logged::Accepted(_)) => Ok(()),
        other => Err(invalid(format!("{what} not accepted: {other:?}"))),
    };
    for tick in 0..TICKS {
        for k in 0..FIXED_PER_TICK {
            let index = tick * FIXED_PER_TICK + k;
            let (mono, real) = clocks(true, index);
            accept(writer.try_log_at(fixed, &Fixed(index), mono, real), "fixed")?;
        }
        for k in 0..BLOBS_PER_TICK {
            let index = tick * BLOBS_PER_TICK + k;
            let len = blob_len(index);
            fill(index, 7, &mut buffer[..len]);
            let (mono, real) = clocks(false, index);
            accept(writer.try_log_at(blob, &buffer[..len], mono, real), "blob")?;
        }
        // Paced well below the declared rate, so admission never rejects.
        std::thread::sleep(Duration::from_millis(10));
    }
    drop(writer);
    let shutdown = logger
        .shutdown(Duration::from_secs(30))
        .map_err(|e| invalid(format!("shutdown: {e:?}")))?;
    if !shutdown.is_success() {
        return Err(invalid(format!("shutdown not successful: {shutdown:?}")));
    }
    let files = mcap_files(dir)?;
    println!(
        "{} wrote {FIXED} fixed and {BLOBS} blob records into {} files below {}",
        std::env::consts::OS,
        files.len(),
        dir.display()
    );
    Ok(())
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "mcap") {
            out.push(path);
        }
    }
    Ok(())
}
fn mcap_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect(dir, &mut files)?;
    files.sort();
    Ok(files)
}

/// Check one OS's output; every mismatch is reported with its file.
fn verify(dir: &Path) -> Result<()> {
    let files = mcap_files(dir)?;
    if files.len() < MIN_FILES {
        return Err(invalid(format!(
            "{}: {} files, expected at least {MIN_FILES}",
            dir.display(),
            files.len()
        )));
    }
    let mut fixed_seen = vec![false; FIXED as usize];
    let mut blob_seen = vec![false; BLOBS as usize];
    let mut session = None;
    let mut loss_reports = 0_u64;
    let mut plain_messages = BTreeMap::<String, u64>::new();
    for path in &files {
        let bytes = fs::read(path)?;
        let fail = |what: String| invalid(format!("{}: {what}", path.display()));
        let reader = NativeReader::new(&bytes, MAX_BLOB, MAX_PRODUCERS)
            .map_err(|e| fail(format!("unreadable: {e:?}")))?;
        let info = reader.info();
        if *session.get_or_insert(info.session) != info.session {
            return Err(fail("a second session".into()));
        }
        if info.channels != 2 || info.loss_rows == 0 {
            return Err(fail(format!("unexpected file identity {info:?}")));
        }
        let mut topics = BTreeMap::new();
        for channel in reader.channels() {
            let id = channel.channel.channel_id;
            let schema = channel.schema.as_ref();
            let expected = match channel.topic.as_str() {
                FIXED_TOPIC => {
                    channel.channel.schema_id == Fixed::SCHEMA.id()
                        && channel.channel.schema_version == 3
                        && channel.payload_encoding == "exchange-pattern"
                        && schema.is_some_and(|s| {
                            s.encoding == "jsonschema"
                                && s.descriptor == DESCRIPTOR
                                && s.name.as_deref() == Some("exchange.Fixed")
                        })
                }
                BLOB_TOPIC => channel.payload_encoding == "opaque",
                _ => false,
            };
            if !expected {
                return Err(fail(format!("unexpected channel {}", channel.topic)));
            }
            topics.insert(id, channel.topic.clone());
        }
        for item in reader {
            let stored = item.map_err(|e| fail(format!("record error: {e:?}")))?;
            let meta = stored.record.metadata;
            let payload = stored.payload;
            let index = payload
                .get(..8)
                .and_then(|b| b.try_into().ok())
                .map(u64::from_le_bytes)
                .ok_or_else(|| fail("short payload".into()))?;
            let topic = topics.get(&meta.channel_id).map(String::as_str);
            let (is_fixed, seen, len, step, level, importance) = match topic {
                Some(FIXED_TOPIC) => (
                    true,
                    &mut fixed_seen,
                    FIXED_BYTES,
                    31,
                    Level::Info,
                    Importance::Normal,
                ),
                Some(BLOB_TOPIC) => (
                    false,
                    &mut blob_seen,
                    blob_len(index),
                    7,
                    Level::Warn,
                    Importance::Important,
                ),
                _ => return Err(fail(format!("record on channel {}", meta.channel_id))),
            };
            let mut expected = vec![0_u8; len];
            fill(index, step, &mut expected);
            let (mono, real) = clocks(is_fixed, index);
            let slot = usize::try_from(index)
                .ok()
                .and_then(|i| seen.get_mut(i))
                .ok_or_else(|| fail(format!("index {index} out of range")))?;
            if *slot
                || payload != expected.as_slice()
                || meta.mono_ns != mono
                || meta.real_ns != real
                || meta.level != level
                || meta.importance.unwrap_or_default() != importance
                || meta.source_time.is_some()
                || stored.record.receipt.session_id != info.session
            {
                return Err(fail(format!("{topic:?} record {index} differs: {meta:?}")));
            }
            *slot = true;
        }
        for report in LossReader::new(&bytes, MAX_BLOB, MAX_PRODUCERS)? {
            let report = report.map_err(|e| fail(format!("loss report: {e:?}")))?;
            loss_reports += 1;
            if let Some(row) = report.counts.iter().find(|c| c.count != 0) {
                return Err(fail(format!("loss row not zero: {row:?}")));
            }
        }
        // An ordinary MCAP reader sees the same messages (Lichtblick/Foxglove path).
        for message in mcap::MessageStream::new(&bytes)? {
            let message = message.map_err(|e| fail(format!("mcap: {e}")))?;
            *plain_messages
                .entry(message.channel.topic.clone())
                .or_default() += 1;
        }
    }
    let missing_fixed = fixed_seen.iter().filter(|s| !**s).count();
    let missing_blobs = blob_seen.iter().filter(|s| !**s).count();
    let plain_fixed = plain_messages.get(FIXED_TOPIC).copied().unwrap_or(0);
    let plain_blobs = plain_messages.get(BLOB_TOPIC).copied().unwrap_or(0);
    if missing_fixed + missing_blobs != 0
        || loss_reports == 0
        || plain_fixed != FIXED
        || plain_blobs != BLOBS
    {
        return Err(invalid(format!(
            "{}: missing fixed {missing_fixed}, blobs {missing_blobs}; loss reports \
             {loss_reports}; plain MCAP {plain_messages:?}",
            dir.display()
        )));
    }
    println!(
        "{} verified {}: {} files, {FIXED} fixed and {BLOBS} blob records, {loss_reports} \
         zero loss reports, plain MCAP topics {plain_messages:?}",
        std::env::consts::OS,
        dir.display(),
        files.len()
    );
    Ok(())
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.split_first() {
        Some((mode, dirs)) if mode == "write" && dirs.len() == 1 => write(Path::new(&dirs[0])),
        Some((mode, dirs)) if mode == "verify" && !dirs.is_empty() => {
            dirs.iter().try_for_each(|d| verify(Path::new(d)))
        }
        _ => Err(invalid(
            "usage: os_exchange write <dir> | verify <dir>...".into(),
        )),
    };
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("os_exchange: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_order_interleaves_ticks_and_never_repeats() {
        let mut orders: Vec<u64> = (0..FIXED)
            .map(|i| order(true, i))
            .chain((0..BLOBS).map(|i| order(false, i)))
            .collect();
        orders.sort_unstable();
        assert!(orders.iter().enumerate().all(|(n, o)| *o == n as u64));
        assert_eq!(order(false, 0), FIXED_PER_TICK);
        assert_eq!(order(true, FIXED_PER_TICK), FIXED_PER_TICK + BLOBS_PER_TICK);
    }

    #[test]
    fn blob_lengths_stay_within_the_slot() {
        assert!((0..BLOBS).all(|i| (MIN_BLOB..=MAX_BLOB).contains(&blob_len(i))));
    }
}
