use base64::{Engine, engine::general_purpose::STANDARD};
use dot_logger_client::LiveRecord;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{self, Read},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub(crate) const MAX_CATALOG_BYTES: usize = 1024 * 1024;
const MAX_CHANNELS: usize = 128;
const MAX_SCHEMA_BYTES: usize = 256 * 1024;

pub(crate) struct Catalog {
    channels: HashMap<u32, Channel>,
    advertisement: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileCatalog {
    version: u8,
    channels: Vec<FileChannel>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileChannel {
    id: u32,
    channel_id: u32,
    schema_id: String,
    schema_version: u32,
    topic: String,
    encoding: Encoding,
    schema_name: String,
    schema: String,
    #[serde(default)]
    originals: bool,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Encoding {
    Json,
    Protobuf,
}

#[derive(Clone)]
pub(crate) struct Channel {
    pub(crate) native_id: u32,
    pub(crate) originals: bool,
    schema_id: u128,
    schema_version: u32,
    retired: Arc<AtomicBool>,
}

impl Channel {
    pub(crate) fn matches(&self, record: LiveRecord) -> bool {
        record.channel_id == self.native_id
            && record.schema_id == self.schema_id
            && record.schema_version == self.schema_version
    }

    pub(crate) fn retire(&self) {
        self.retired.store(true, Ordering::Release);
    }

    pub(crate) fn retired(&self) -> bool {
        self.retired.load(Ordering::Acquire)
    }
}

fn invalid(reason: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}

pub(crate) fn read_file(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
    let meta = fs::metadata(path)?;
    if !meta.is_file() || meta.len() > limit as u64 {
        return Err(invalid(
            "expected a regular file within the configured limit",
        ));
    }
    let mut file = fs::File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(invalid("expected a regular file"));
    }
    let mut bytes = Vec::new();
    (&mut file).take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(invalid("file exceeds the configured limit"));
    }
    Ok(bytes)
}

impl Catalog {
    pub(crate) fn parse(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() > MAX_CATALOG_BYTES {
            return Err(invalid("catalog exceeds 1 MiB"));
        }
        let catalog: FileCatalog =
            serde_json::from_slice(bytes).map_err(|_| invalid("invalid catalog JSON"))?;
        if catalog.version != 1 || !(1..=MAX_CHANNELS).contains(&catalog.channels.len()) {
            return Err(invalid("expected catalog version 1 with 1 to 128 channels"));
        }
        let mut channels = HashMap::new();
        let mut topics = HashSet::new();
        let mut native = HashSet::new();
        let mut advertised = Vec::new();
        for entry in &catalog.channels {
            let id = u128::from_str_radix(&entry.schema_id, 16)
                .ok()
                .filter(|_| {
                    entry.schema_id.len() == 32
                        && entry.schema_id.bytes().all(|byte| byte.is_ascii_hexdigit())
                })
                .ok_or_else(|| invalid("schema_id must be 32 hexadecimal digits"))?;
            if entry.id == 0
                || !valid_name(&entry.topic)
                || !valid_name(&entry.schema_name)
                || !topics.insert(&entry.topic)
                || !native.insert(entry.channel_id)
                || channels.contains_key(&entry.id)
            {
                return Err(invalid(
                    "duplicate or invalid channel, topic, schema name or advertisement ID",
                ));
            }
            let schema_encoding = match entry.encoding {
                Encoding::Json
                    if entry.schema.len() <= MAX_SCHEMA_BYTES
                        && (entry.schema.is_empty()
                            || serde_json::from_str::<serde_json::Value>(&entry.schema)
                                .is_ok_and(|schema| schema.is_object() || schema.is_boolean())) =>
                {
                    "jsonschema"
                }
                Encoding::Protobuf
                    if entry.schema.len() <= MAX_SCHEMA_BYTES.div_ceil(3) * 4
                        && STANDARD.decode(&entry.schema).is_ok_and(|bytes| {
                            !bytes.is_empty() && bytes.len() <= MAX_SCHEMA_BYTES
                        }) =>
                {
                    "protobuf"
                }
                _ => return Err(invalid("invalid or oversized schema definition")),
            };
            advertised.push(serde_json::json!({
                "id": entry.id, "topic": entry.topic, "encoding": entry.encoding,
                "schemaName": entry.schema_name, "schemaEncoding": schema_encoding,
                "schema": entry.schema,
            }));
            channels.insert(
                entry.id,
                Channel {
                    native_id: entry.channel_id,
                    originals: entry.originals,
                    schema_id: id,
                    schema_version: entry.schema_version,
                    retired: Arc::new(AtomicBool::new(false)),
                },
            );
        }
        let advertisement = serde_json::json!({"op":"advertise","channels":advertised}).to_string();
        if advertisement.len() > MAX_CATALOG_BYTES {
            return Err(invalid("advertisement exceeds 1 MiB"));
        }
        Ok(Self {
            channels,
            advertisement,
        })
    }

    pub(crate) fn channel(&self, id: u32) -> Option<&Channel> {
        self.channels.get(&id)
    }

    pub(crate) fn advertisement(&self) -> &str {
        &self.advertisement
    }

    pub(crate) fn retired_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.channels
            .iter()
            .filter_map(|(&id, channel)| channel.retired().then_some(id))
    }
}

fn valid_name(name: &str) -> bool {
    (1..=256).contains(&name.len()) && !name.chars().any(char::is_control)
}
