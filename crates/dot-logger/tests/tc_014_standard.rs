//! TC-014 TASK-030 S1: standard MCAP export (API §21) — user schema channels
//! without the product envelope, plus one metadata channel.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use dot_logger::logger::{
    Budget, EncodeError, Logged, Logger, Preset, Record, Retention, Schema, WriterQuota,
};
use dot_logger::storage::decode_record;
use dot_logger::storage::standard::{ExportError, ExportLimits, META_TOPIC, export};
use prost::Message;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MIB: usize = 1024 * 1024;
const POSE_FDS: &[u8] = include_bytes!("fixtures/standard/pose.fds");

/// `test.Pose { float x = 1; float y = 2; }` encoded by hand.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Pose {
    x: f32,
    y: f32,
}
impl Record for Pose {
    const SCHEMA: Schema =
        Schema::new(0x504f_5345, 1, "protobuf").descriptor("test.Pose", "protobuf", POSE_FDS);
    fn encode(&self, out: &mut [u8]) -> Result<usize, EncodeError> {
        let out = out.get_mut(..10).ok_or(EncodeError)?;
        out[0] = 0x0d;
        out[1..5].copy_from_slice(&self.x.to_le_bytes());
        out[5] = 0x15;
        out[6..].copy_from_slice(&self.y.to_le_bytes());
        Ok(10)
    }
}

/// An application-specific encoding without a descriptor.
struct Blob(u8, usize);
impl Record for Blob {
    const SCHEMA: Schema = Schema::new(0x424c_4f42, 1, "vendor-blob");
    fn encode(&self, out: &mut [u8]) -> Result<usize, EncodeError> {
        let out = out.get_mut(..self.1).ok_or(EncodeError)?;
        out.fill(self.0);
        Ok(self.1)
    }
}

/// Mirror of `dot_logger.RecordMeta` written independently from the
/// product code (API §21.2).
#[derive(Clone, PartialEq, Message)]
struct RecordSource {
    #[prost(sint64, tag = "1")]
    ticks: i64,
    #[prost(uint64, tag = "2")]
    clock_domain: u64,
}
#[derive(Clone, PartialEq, Message)]
struct RecordMeta {
    #[prost(uint32, tag = "1")]
    channel: u32,
    #[prost(uint32, tag = "2")]
    producer: u32,
    #[prost(uint64, tag = "3")]
    sequence: u64,
    #[prost(uint64, tag = "4")]
    mono: u64,
    #[prost(sint64, tag = "5")]
    real: i64,
    #[prost(uint64, tag = "6")]
    revision: u64,
    #[prost(uint32, tag = "7")]
    level: u32,
    #[prost(uint32, tag = "8")]
    importance: u32,
    #[prost(message, optional, tag = "9")]
    source: Option<RecordSource>,
}

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "dot-logger-tc014-{}-{nanos}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Closed segments anywhere below `dir` (`<session>/<target>/N.mcap`).
fn closed_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(closed_files(&path));
        } else if path.extension().is_some_and(|e| e == "mcap") {
            files.push(path);
        }
    }
    files.sort();
    files
}

fn log<T: Record>(
    writer: &mut dot_logger::logger::Writer,
    channel: dot_logger::logger::Channel<T>,
    msg: &T,
) {
    loop {
        match writer.try_log(channel, msg) {
            Ok(Logged::Accepted(_)) => return,
            Ok(Logged::Skipped) => panic!("default Info channel never skips"),
            Err(e) if e.is_capacity() => std::thread::yield_now(),
            Err(e) => panic!("unexpected rejection {e:?}"),
        }
    }
}

/// Records `poses` poses and `blobs` blobs of `blob_len` bytes; returns the
/// closed segment files.
fn record_file(temp: &Temp, poses: usize, blobs: usize, blob_len: usize) -> Vec<PathBuf> {
    let logger = Logger::builder()
        .budget(
            Budget::new()
                .memory_bytes(64 * MIB)
                .sync_every(Duration::from_millis(20))
                .drop_deadline(Duration::from_secs(3))
                .writer_quota(WriterQuota::small(64, blob_len.max(64))),
        )
        .file_output(&temp.0, Retention::ring(256 * MIB as u64))
        .channel::<Pose>("robot/pose")
        .channel::<Blob>("robot/blob")
        .writer("control")
        .start()
        .unwrap();
    let pose = logger.channel::<Pose>("robot/pose").unwrap();
    let blob = logger.channel::<Blob>("robot/blob").unwrap();
    let mut writer = logger.take_writer("control").unwrap();
    for i in 0..poses.max(blobs) {
        if i < poses {
            log(
                &mut writer,
                pose,
                &Pose {
                    x: i as f32,
                    y: -(i as f32),
                },
            );
        }
        if i < blobs {
            log(&mut writer, blob, &Blob(i as u8, blob_len));
        }
    }
    drop(writer);
    assert!(
        logger
            .shutdown(Duration::from_secs(5))
            .unwrap()
            .is_success()
    );
    closed_files(&temp.0)
}

fn export_bytes(input: &[u8]) -> (Vec<u8>, dot_logger::storage::standard::ExportReport) {
    let mut out = Cursor::new(Vec::new());
    let report = export(
        input,
        &mut out,
        ExportLimits::new(16 * MIB),
        &AtomicBool::new(false),
    )
    .unwrap();
    (out.into_inner(), report)
}

#[test]
fn f1_export_decodes_user_payloads_and_keeps_envelope_fields_in_meta() {
    let temp = Temp::new();
    let files = record_file(&temp, 100, 10, 16);
    assert_eq!(files.len(), 1);
    let input = std::fs::read(&files[0]).unwrap();
    let originals: Vec<_> = mcap::MessageStream::new(&input)
        .unwrap()
        .map(|m| {
            let m = m.unwrap();
            (m.channel.topic.clone(), decode_record(&m.data, 64).unwrap())
        })
        .collect();
    assert_eq!(originals.len(), 110);

    let (output, report) = export_bytes(&input);
    assert_eq!(report.messages, 110);
    assert!(report.truncated.is_none());

    let summary = mcap::Summary::read(&output).unwrap().unwrap();
    let messages: Vec<_> = mcap::MessageStream::new(&output)
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let (user, meta): (Vec<_>, Vec<_>) =
        messages.iter().partition(|m| m.channel.topic != META_TOPIC);
    assert_eq!(user.len(), 110);
    assert_eq!(meta.len(), 110);

    // User channels: the application's schema and encoding, never the envelope.
    let pose = user
        .iter()
        .find(|m| m.channel.topic == "robot/pose")
        .unwrap();
    assert_eq!(pose.channel.message_encoding, "protobuf");
    let schema = pose.channel.schema.as_ref().unwrap();
    assert_eq!(schema.name, "test.Pose");
    assert_eq!(schema.encoding, "protobuf");
    assert_eq!(&schema.data[..], POSE_FDS);
    let blob = user
        .iter()
        .find(|m| m.channel.topic == "robot/blob")
        .unwrap();
    assert_eq!(blob.channel.message_encoding, "vendor-blob");
    assert!(blob.channel.schema.is_none(), "no descriptor -> schemaless");
    assert!(
        summary
            .schemas
            .values()
            .all(|s| s.name != "dot_logger.Envelope")
    );

    let meta_schema = meta[0].channel.schema.as_ref().unwrap();
    assert_eq!(meta_schema.name, "dot_logger.RecordMeta");
    assert_eq!(meta_schema.encoding, "protobuf");
    assert_eq!(meta[0].channel.message_encoding, "protobuf");

    // Every user message: payload verbatim, sequence low 32 bits, real time;
    // the following meta message keeps the rest (API §21.1).
    for (i, (topic, stored)) in originals.iter().enumerate() {
        let m = &messages[2 * i];
        let info = RecordMeta::decode(&messages[2 * i + 1].data[..]).unwrap();
        assert_eq!(&m.channel.topic, topic);
        assert_eq!(&m.data[..], &stored.payload[..]);
        let seq = stored.record.receipt.accepted_seq;
        assert_eq!(m.sequence, seq as u32);
        assert_eq!(m.log_time, stored.record.metadata.real_ns as u64);
        assert_eq!(m.publish_time, m.log_time);
        assert_eq!(messages[2 * i + 1].log_time, m.log_time);
        assert_eq!(info.channel, u32::from(m.channel.id));
        assert_eq!(info.producer, stored.record.receipt.producer_id);
        assert_eq!(info.sequence, seq);
        assert_eq!(info.mono, stored.record.metadata.mono_ns);
        assert_eq!(info.real, stored.record.metadata.real_ns);
        assert_eq!(info.revision, stored.record.revision);
        assert_eq!(info.level, stored.record.metadata.level as u32);
        assert_eq!(info.importance, 0, "facade default importance is Normal");
        assert_eq!(info.source, None);
    }
    let pose_report = report
        .channels
        .iter()
        .find(|c| c.topic == "robot/pose")
        .unwrap();
    assert_eq!(pose_report.schema_name.as_deref(), Some("test.Pose"));
    assert_eq!(pose_report.messages, 100);
    let blob_report = report
        .channels
        .iter()
        .find(|c| c.topic == "robot/blob")
        .unwrap();
    assert_eq!(blob_report.schema_name, None);
    assert_eq!(blob_report.messages, 10);
}

#[test]
fn f2_truncated_input_exports_the_complete_prefix_and_reports_it() {
    let temp = Temp::new();
    let files = record_file(&temp, 0, 1000, 1000);
    let input = std::fs::read(&files[0]).unwrap();
    let total = mcap::MessageStream::new(&input).unwrap().count() as u64;
    let cut = &input[..input.len() * 2 / 3];
    let (output, report) = export_bytes(cut);
    assert!(report.truncated.is_some());
    assert!(
        report.messages > 0 && report.messages < total,
        "{} of {total}",
        report.messages
    );
    let exported = mcap::MessageStream::new(&output)
        .unwrap()
        .filter(|m| m.as_ref().unwrap().channel.topic != META_TOPIC)
        .count() as u64;
    assert_eq!(exported, report.messages);
}

#[test]
fn f3_a_file_that_is_not_dot_logger_is_rejected() {
    let mut plain = Cursor::new(Vec::new());
    {
        let mut w = mcap::Writer::new(&mut plain).unwrap();
        let id = w
            .add_channel(0, "plain", "json", &Default::default())
            .unwrap();
        w.write_to_known_channel(
            &mcap::records::MessageHeader {
                channel_id: id,
                sequence: 1,
                log_time: 1,
                publish_time: 1,
            },
            b"{}",
        )
        .unwrap();
        w.finish().unwrap();
    }
    let mut out = Cursor::new(Vec::new());
    let err = export(
        &plain.into_inner()[..],
        &mut out,
        ExportLimits::new(MIB),
        &AtomicBool::new(false),
    )
    .unwrap_err();
    assert!(matches!(err, ExportError::NotDotLogger), "{err:?}");
}

#[test]
fn f4_a_user_topic_equal_to_the_meta_topic_is_rejected() {
    let temp = Temp::new();
    let logger = Logger::builder()
        .preset(Preset::control(1000))
        .file_output(&temp.0, Retention::ring(256 * MIB as u64))
        .channel::<Pose>(META_TOPIC)
        .writer("control")
        .start()
        .unwrap();
    let channel = logger.channel::<Pose>(META_TOPIC).unwrap();
    let mut writer = logger.take_writer("control").unwrap();
    log(&mut writer, channel, &Pose { x: 1.0, y: 2.0 });
    drop(writer);
    assert!(
        logger
            .shutdown(Duration::from_secs(5))
            .unwrap()
            .is_success()
    );
    let input = std::fs::read(&closed_files(&temp.0)[0]).unwrap();
    let mut out = Cursor::new(Vec::new());
    let err = export(
        &input[..],
        &mut out,
        ExportLimits::new(MIB),
        &AtomicBool::new(false),
    )
    .unwrap_err();
    assert!(matches!(err, ExportError::TopicConflict), "{err:?}");
}

#[test]
fn f5_records_larger_than_the_limit_fail_instead_of_allocating() {
    let temp = Temp::new();
    let files = record_file(&temp, 0, 4, 4096);
    let input = std::fs::read(&files[0]).unwrap();
    let mut out = Cursor::new(Vec::new());
    let result = export(
        &input[..],
        &mut out,
        ExportLimits::new(1024),
        &AtomicBool::new(false),
    );
    assert!(
        matches!(result, Err(ExportError::RecordTooLarge)),
        "{result:?}"
    );
}

#[test]
fn f6_cancellation_stops_the_export() {
    let temp = Temp::new();
    let files = record_file(&temp, 10, 0, 16);
    let input = std::fs::read(&files[0]).unwrap();
    let mut out = Cursor::new(Vec::new());
    let result = export(
        &input[..],
        &mut out,
        ExportLimits::new(MIB),
        &AtomicBool::new(true),
    );
    assert!(matches!(result, Err(ExportError::Cancelled)), "{result:?}");
}

/// TASK-033 S3 (UC-03·UC-11): an Important record with a source time, sent
/// through the facade's advanced path, keeps both in the meta channel. F1
/// covers only the facade default (Normal, no source time).
#[test]
fn f7_important_records_and_source_time_survive_in_meta() {
    use dot_logger::capture::{Importance, Level, RecordOutcome, SourceTime};
    let temp = Temp::new();
    let logger = Logger::builder()
        .budget(
            Budget::new()
                .memory_bytes(64 * MIB)
                .sync_every(Duration::from_millis(20))
                .drop_deadline(Duration::from_secs(3))
                .writer_quota(WriterQuota::small(64, 64)),
        )
        .file_output(&temp.0, Retention::ring(256 * MIB as u64))
        .channel::<Pose>("robot/pose")
        .writer("control")
        .start()
        .unwrap();
    let pose = logger.channel::<Pose>("robot/pose").unwrap();
    let mut writer = logger.take_writer("control").unwrap();
    let mut payload = [0_u8; 10];
    Pose { x: 1.0, y: 2.0 }.encode(&mut payload).unwrap();
    let mut metadata = pose
        .at(Level::Error)
        .metadata(writer.now_ns(), 1_700_000_000_000_000_000);
    metadata.importance = Some(Importance::Important);
    metadata.source_time = Some(SourceTime {
        ticks: -42,
        clock_domain: 7,
    });
    let outcome = writer.producer().try_record_small(metadata, &payload);
    assert!(
        matches!(outcome, Ok(RecordOutcome::Accepted(_))),
        "{outcome:?}"
    );
    log(&mut writer, pose, &Pose { x: 3.0, y: 4.0 });
    drop(writer);
    assert!(
        logger
            .shutdown(Duration::from_secs(5))
            .unwrap()
            .is_success()
    );
    let files = closed_files(&temp.0);
    let (output, report) = export_bytes(&std::fs::read(&files[0]).unwrap());
    assert_eq!(report.messages, 2);
    let metas: Vec<RecordMeta> = mcap::MessageStream::new(&output)
        .unwrap()
        .map(|m| m.unwrap())
        .filter(|m| m.channel.topic == META_TOPIC)
        .map(|m| RecordMeta::decode(&m.data[..]).unwrap())
        .collect();
    assert_eq!(metas.len(), 2);
    assert_eq!(metas[0].importance, 1, "Important is kept");
    assert_eq!(metas[0].level, Level::Error as u32);
    assert_eq!(
        metas[0].source,
        Some(RecordSource {
            ticks: -42,
            clock_domain: 7
        })
    );
    assert_eq!(metas[1].importance, 0);
    assert_eq!(metas[1].source, None);
}

/// Hand-built `dot-logger/1` records for damaged inputs the writer cannot
/// produce (MCAP spec: opcode, u64 length, little-endian fields).
mod raw {
    pub const ENVELOPE: &str = "dot_logger.Envelope";
    fn string(out: &mut Vec<u8>, s: &str) {
        out.extend_from_slice(&(s.len() as u32).to_le_bytes());
        out.extend_from_slice(s.as_bytes());
    }
    fn record(out: &mut Vec<u8>, opcode: u8, body: &[u8]) {
        out.push(opcode);
        out.extend_from_slice(&(body.len() as u64).to_le_bytes());
        out.extend_from_slice(body);
    }
    /// Magic and a `dot-logger/1` header.
    pub fn file() -> Vec<u8> {
        let mut out = mcap::MAGIC.to_vec();
        let mut body = Vec::new();
        string(&mut body, "dot-logger/1");
        string(&mut body, "hand-built");
        record(&mut out, mcap::records::op::HEADER, &body);
        out
    }
    pub fn schema(out: &mut Vec<u8>, id: u16, name: &str, encoding: &str, data: &[u8]) {
        let mut body = id.to_le_bytes().to_vec();
        string(&mut body, name);
        string(&mut body, encoding);
        body.extend_from_slice(&(data.len() as u32).to_le_bytes());
        body.extend_from_slice(data);
        record(out, mcap::records::op::SCHEMA, &body);
    }
    /// A channel with the `payload_encoding` metadata the export requires.
    pub fn channel(out: &mut Vec<u8>, id: u16, schema_id: u16, topic: &str, encoding: &str) {
        let mut body = id.to_le_bytes().to_vec();
        body.extend_from_slice(&schema_id.to_le_bytes());
        string(&mut body, topic);
        string(&mut body, encoding);
        let mut map = Vec::new();
        string(&mut map, "payload_encoding");
        string(&mut map, "json");
        body.extend_from_slice(&(map.len() as u32).to_le_bytes());
        body.extend_from_slice(&map);
        record(out, mcap::records::op::CHANNEL, &body);
    }
    /// A schema record whose body is too short to parse.
    pub fn broken_schema(out: &mut Vec<u8>) {
        record(out, mcap::records::op::SCHEMA, &[1]);
    }
}

fn export_err(input: &[u8]) -> ExportError {
    export(
        input,
        &mut Cursor::new(Vec::new()),
        ExportLimits::new(MIB),
        &AtomicBool::new(false),
    )
    .unwrap_err()
}

/// TASK-033 S3: one schema id defined twice with different content is not a
/// `dot-logger/1` file the export can trust.
#[test]
fn f8_a_schema_id_redefined_with_other_content_is_invalid() {
    let mut input = raw::file();
    raw::schema(&mut input, 1, raw::ENVELOPE, "protobuf", b"a");
    raw::schema(&mut input, 1, raw::ENVELOPE, "protobuf", b"a");
    raw::schema(&mut input, 1, raw::ENVELOPE, "protobuf", b"b");
    let err = export_err(&input);
    assert!(matches!(err, ExportError::InvalidRecord), "{err:?}");
}

/// TASK-033 S3: a channel id repeated with another topic is invalid; the
/// same channel repeated (summary section) is accepted once.
#[test]
fn f9_a_channel_id_redefined_with_another_topic_is_invalid() {
    let mut input = raw::file();
    raw::schema(&mut input, 1, raw::ENVELOPE, "protobuf", b"a");
    raw::channel(&mut input, 1, 1, "robot/a", "protobuf");
    raw::channel(&mut input, 1, 1, "robot/a", "protobuf");
    raw::channel(&mut input, 1, 1, "robot/b", "protobuf");
    let err = export_err(&input);
    assert!(matches!(err, ExportError::InvalidRecord), "{err:?}");
}

/// TASK-033 S3: a channel that does not carry `dot_logger.Envelope` as
/// protobuf means the file is not `dot-logger/1`, even with the header.
#[test]
fn f10_a_channel_without_the_protobuf_envelope_is_not_dot_logger() {
    for (name, schema_encoding, schema_id, message_encoding) in [
        ("other.Type", "protobuf", 1, "protobuf"),
        (raw::ENVELOPE, "jsonschema", 1, "protobuf"),
        (raw::ENVELOPE, "protobuf", 1, "json"),
        (raw::ENVELOPE, "protobuf", 0, "protobuf"),
    ] {
        let mut input = raw::file();
        raw::schema(&mut input, 1, name, schema_encoding, b"a");
        raw::channel(&mut input, 1, schema_id, "robot/a", message_encoding);
        let err = export_err(&input);
        assert!(
            matches!(err, ExportError::NotDotLogger),
            "{name} {schema_encoding} {schema_id} {message_encoding}: {err:?}"
        );
    }
}

/// TASK-033 S3: a record that cannot be parsed after the output started
/// ends the export with the records before it and a truncation reason.
#[test]
fn f11_an_unparsable_record_after_the_header_truncates_the_export() {
    let mut input = raw::file();
    raw::schema(&mut input, 1, raw::ENVELOPE, "protobuf", b"a");
    raw::channel(&mut input, 1, 1, "robot/a", "protobuf");
    raw::broken_schema(&mut input);
    raw::channel(&mut input, 2, 1, "robot/after", "protobuf");
    let (output, report) = export_bytes(&input);
    assert!(report.truncated.is_some(), "{report:?}");
    assert_eq!(report.messages, 0);
    let topics: Vec<_> = report.channels.iter().map(|c| c.topic.as_str()).collect();
    assert_eq!(topics, ["robot/a"]);
    let summary = mcap::Summary::read(&output).unwrap().unwrap();
    let mut out_topics: Vec<_> = summary.channels.values().map(|c| c.topic.clone()).collect();
    out_topics.sort();
    assert_eq!(out_topics, [META_TOPIC, "robot/a"]);
}

/// TASK-033 S3: input metadata and attachments are not exported but are
/// counted, so the caller can say what was left out.
#[test]
fn f12_metadata_and_attachments_are_skipped_and_counted() {
    let mut input = Cursor::new(Vec::new());
    {
        let mut w = mcap::WriteOptions::new()
            .profile("dot-logger/1")
            .create(&mut input)
            .unwrap();
        let schema = w.add_schema(raw::ENVELOPE, "protobuf", b"a").unwrap();
        let metadata = [("payload_encoding".to_string(), "json".to_string())].into();
        w.add_channel(schema, "robot/a", "protobuf", &metadata)
            .unwrap();
        w.write_metadata(&mcap::records::Metadata {
            name: "session".into(),
            metadata: [("k".to_string(), "v".to_string())].into(),
        })
        .unwrap();
        w.attach(&mcap::Attachment {
            log_time: 1,
            create_time: 1,
            name: "notes.txt".into(),
            media_type: "text/plain".into(),
            data: std::borrow::Cow::Borrowed(b"hello"),
        })
        .unwrap();
        w.finish().unwrap();
    }
    let (output, report) = export_bytes(&input.into_inner());
    assert_eq!(report.skipped_records, 2, "{report:?}");
    assert!(report.truncated.is_none(), "{report:?}");
    assert_eq!(report.channels.len(), 1);
    let summary = mcap::Summary::read(&output).unwrap().unwrap();
    assert!(summary.attachment_indexes.is_empty());
    assert!(summary.metadata_indexes.is_empty());
}
