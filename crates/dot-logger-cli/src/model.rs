use crate::output::Failure;
use dot_logger::{
    capture::ChannelConfig,
    config::*,
    platform::*,
    query::QueryConfig,
    storage::{PayloadSchema, StorageChannel},
};
use dot_logger_client::{
    Cancellation, OfflineConfig, OfflineSource, open_loss_snapshot, open_snapshot,
};
use serde::Deserialize;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
const MAX_INPUT: usize = 4 * 1024 * 1024;
pub fn read(path: &Path) -> Result<Vec<u8>, Failure> {
    if !std::fs::metadata(path)?.is_file() {
        return Err(Failure::Input);
    }
    let f = File::open(path)?;
    if !f.metadata()?.is_file() || f.metadata()?.len() > MAX_INPUT as u64 {
        return Err(Failure::Input);
    }
    let mut data = Vec::new();
    f.take(MAX_INPUT as u64 + 1).read_to_end(&mut data)?;
    if data.len() > MAX_INPUT {
        return Err(Failure::Input);
    }
    Ok(data)
}
pub fn id(s: &str) -> Result<u128, Failure> {
    if s.len() != 32 || !s.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(Failure::Input);
    }
    u128::from_str_radix(s, 16).map_err(|_| Failure::Input)
}
fn unhex(s: &str) -> Result<Vec<u8>, Failure> {
    if s.len() % 2 != 0 || !s.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(Failure::Input);
    }
    s.as_bytes()
        .chunks_exact(2)
        .map(|b| {
            std::str::from_utf8(b)
                .map_err(|_| Failure::Input)
                .and_then(|s| u8::from_str_radix(s, 16).map_err(|_| Failure::Input))
        })
        .collect()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub session: String,
    pub producers: usize,
    pub external_bytes: usize,
    pub channels: Vec<Channel>,
    pub sources: Vec<Source>,
    pub limits: Limits,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    id: u32,
    schema: String,
    version: u32,
    topic: String,
    encoding: String,
    #[serde(default)]
    descriptor: Option<Descriptor>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Descriptor {
    encoding: String,
    data_hex: String,
    /// Type name for standard tools (API §21.1 ⑥), such as `my.Pose`.
    #[serde(default)]
    name: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    #[serde(default)]
    pub target: Option<u32>,
    pub revision: u64,
    pub path: PathBuf,
    pub sequence: u64,
    #[serde(default)]
    pub safe_offset: Option<u64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    #[serde(default)]
    max_loss_reports: usize,
    max_segments: usize,
    max_records: usize,
    max_record_bytes: usize,
    max_source_bytes: u64,
    max_payload_bytes: usize,
    max_metadata_bytes: usize,
    page_records: usize,
    page_bytes: usize,
    inline_bytes: usize,
    blob_read_bytes: usize,
    read_bytes: u64,
}
impl Manifest {
    pub fn load(path: &Path) -> Result<Self, Failure> {
        let manifest: Self = serde_json::from_slice(&read(path)?)?;
        manifest.validate()?;
        Ok(manifest)
    }
    pub fn channels(&self) -> Result<Vec<StorageChannel>, Failure> {
        self.channels
            .iter()
            .map(|c| {
                Ok(StorageChannel {
                    channel: ChannelConfig::new(c.id, id(&c.schema)?, c.version),
                    topic: c.topic.clone(),
                    payload_encoding: c.encoding.clone(),
                    schema: c
                        .descriptor
                        .as_ref()
                        .map(|d| {
                            Ok::<PayloadSchema, Failure>(PayloadSchema {
                                encoding: d.encoding.clone(),
                                descriptor: unhex(&d.data_hex)?,
                                name: d.name.clone(),
                            })
                        })
                        .transpose()?,
                })
            })
            .collect()
    }
    pub fn limits(&self) -> QueryConfig {
        let l = &self.limits;
        QueryConfig {
            max_loss_reports: l.max_loss_reports,
            max_segments: l.max_segments,
            max_records: l.max_records,
            max_record_bytes: l.max_record_bytes,
            max_source_bytes: l.max_source_bytes,
            max_payload_bytes: l.max_payload_bytes,
            max_metadata_bytes: l.max_metadata_bytes,
            producers: self.producers,
            page_records: l.page_records,
            page_bytes: l.page_bytes,
            inline_bytes: l.inline_bytes,
            blob_read_bytes: l.blob_read_bytes,
            read_bytes: l.read_bytes,
        }
    }
    pub fn validate(&self) -> Result<usize, Failure> {
        if self.schema_version != 2
            || self.producers == 0
            || self.producers > 1024
            || self.channels.is_empty()
            || self.channels.len() > 128
            || self.sources.is_empty()
            || self.sources.len() > 1024
            || self.sources.len() > self.limits.max_segments
            || self.external_bytes > 2 * 1024 * 1024 * 1024
            || self.limits.max_records > 1_000_000
            || self.limits.max_loss_reports > 1_000_000
            || self.limits.max_record_bytes > 128 * 1024 * 1024
            || self.limits.max_payload_bytes > 64 * 1024 * 1024
            || self.limits.page_bytes > 16 * 1024 * 1024
            || self.limits.max_metadata_bytes > MAX_INPUT
        {
            return Err(Failure::Input);
        }
        id(&self.session)?;
        for (i, s) in self.sources.iter().enumerate() {
            if self.sources[..i].iter().any(|p| {
                p.path == s.path
                    || p.sequence == s.sequence
                        && (p.target.is_none() || s.target.is_none() || p.target == s.target)
            }) {
                return Err(Failure::Input);
            }
        }
        let channels = self.channels()?;
        for (i, c) in channels.iter().enumerate() {
            if c.topic.is_empty()
                || c.topic.len() > 256
                || c.payload_encoding.is_empty()
                || c.payload_encoding.len() > 64
                || c.schema
                    .as_ref()
                    .is_some_and(|s| s.encoding.is_empty() || s.encoding.len() > 64)
                || channels[..i].iter().any(|p| {
                    p.channel.channel_id == c.channel.channel_id
                        || (p.channel.schema_id == c.channel.schema_id
                            && p.channel.schema_version == c.channel.schema_version
                            && p.schema != c.schema)
                })
            {
                return Err(Failure::Input);
            }
        }
        let reserved = self
            .limits()
            .reservation(&channels)
            .map_err(|_| Failure::Input)?;
        if reserved > self.external_bytes {
            return Err(Failure::Input);
        }
        self.plan(1024 * 1024)?;
        Ok(reserved)
    }
    pub fn plan(&self, output_bytes: usize) -> Result<ValidatedConfig, Failure> {
        let mib = 1024 * 1024;
        let total = self
            .external_bytes
            .checked_add(output_bytes)
            .and_then(|v| v.checked_add(3 * mib))
            .ok_or(Failure::Input)?;
        SessionBuilder::new(SessionConfig {
            memory: MemoryBudget {
                total_bytes: total,
                input_bytes: mib,
                raw_bytes: 0,
                output_bytes,
                external_bytes: self.external_bytes,
                control_bytes: mib,
                worker_bytes: mib,
            },
            producers: vec![
                ProducerQuota {
                    small: PoolQuota {
                        slots: 1,
                        slot_bytes: 1,
                        important_slots: 0
                    },
                    raw: None
                };
                self.producers
            ],
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
        .map_err(|_| Failure::Input)
    }
    pub fn sources(&self, manifest: &Path) -> Vec<OfflineSource> {
        let base = manifest.parent().unwrap_or_else(|| Path::new("."));
        self.sources
            .iter()
            .map(|s| OfflineSource {
                target: s.target,
                revision: s.revision,
                path: if s.path.is_absolute() {
                    s.path.clone()
                } else {
                    base.join(&s.path)
                },
                sequence: s.sequence,
                safe_offset: s.safe_offset,
            })
            .collect()
    }
    pub fn snapshot(
        &self,
        path: &Path,
        now: u64,
        deadline: u64,
        cancel: &Cancellation,
    ) -> Result<dot_logger::query::Snapshot<File>, Failure> {
        let budget = dot_logger::query::QueryBudget::new(&self.plan(1024 * 1024)?);
        open_snapshot(
            &budget,
            OfflineConfig {
                session: id(&self.session)?,
                channels: self.channels()?,
                limits: self.limits(),
                deadline_ns: deadline,
            },
            self.sources(path),
            now,
            cancel,
        )
        .map_err(Failure::from)
    }
    pub fn loss_snapshot(
        &self,
        path: &Path,
        now: u64,
        deadline: u64,
        cancel: &Cancellation,
    ) -> Result<dot_logger::query::LossSnapshot<File>, Failure> {
        let budget = dot_logger::query::QueryBudget::new(&self.plan(1024 * 1024)?);
        open_loss_snapshot(
            &budget,
            OfflineConfig {
                session: id(&self.session)?,
                channels: self.channels()?,
                limits: self.limits(),
                deadline_ns: deadline,
            },
            self.sources(path),
            now,
            cancel,
        )
        .map_err(Failure::from)
    }
}
