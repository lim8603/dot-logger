use dot_logger::{adapters::remote::RemoteError, query::QueryError, replay::ReplayError};
use dot_logger_client::ClientError;
use serde_json::{Value, json};
use std::io::Write;
#[derive(Clone, Copy, Debug)]
pub enum Failure {
    Input,
    Io,
    Corrupt,
    Limit,
    Cancelled,
    Deadline,
    Unsupported,
    Unknown,
    Gone,
    PermissionDenied,
    Version,
}
impl Failure {
    pub fn code(self) -> i32 {
        match self {
            Self::Input | Self::Unsupported | Self::PermissionDenied => 2,
            Self::Limit | Self::Cancelled | Self::Deadline | Self::Unknown | Self::Gone => 3,
            Self::Io | Self::Corrupt | Self::Version => 4,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Input => "invalid_input",
            Self::Io => "io_or_transport",
            Self::Corrupt => "invalid_data",
            Self::Limit => "limit",
            Self::Cancelled => "cancelled",
            Self::Deadline => "deadline",
            Self::Unsupported => "unsupported",
            Self::Unknown => "unknown",
            Self::Gone => "gone_or_expired",
            Self::PermissionDenied => "permission_denied",
            Self::Version => "incompatible_protocol",
        }
    }
}
impl From<std::io::Error> for Failure {
    fn from(_: std::io::Error) -> Self {
        Self::Io
    }
}
impl From<serde_json::Error> for Failure {
    fn from(_: serde_json::Error) -> Self {
        Self::Input
    }
}
impl From<QueryError> for Failure {
    fn from(e: QueryError) -> Self {
        match e {
            QueryError::Cancelled => Self::Cancelled,
            QueryError::Expired => Self::Deadline,
            QueryError::Gone => Self::Gone,
            QueryError::Limit => Self::Limit,
            QueryError::Io(_) => Self::Io,
            QueryError::CorruptData => Self::Corrupt,
            QueryError::Unsupported => Self::Unsupported,
            _ => Self::Input,
        }
    }
}
impl From<ReplayError> for Failure {
    fn from(e: ReplayError) -> Self {
        match e {
            ReplayError::Query(q) => q.into(),
            ReplayError::Limit | ReplayError::StaleGeneration => Self::Limit,
            _ => Self::Input,
        }
    }
}
impl From<ClientError> for Failure {
    fn from(e: ClientError) -> Self {
        match e {
            ClientError::Cancelled => Self::Cancelled,
            ClientError::AcknowledgementUnknown => Self::Unknown,
            ClientError::Remote(RemoteError::TimedOut) => Self::Deadline,
            ClientError::Remote(RemoteError::Limit) => Self::Limit,
            ClientError::Remote(RemoteError::Unsupported) => Self::Unsupported,
            ClientError::Remote(RemoteError::Corrupt) => Self::Corrupt,
            ClientError::Remote(RemoteError::Version) => Self::Version,
            ClientError::Remote(RemoteError::Gone) => Self::Gone,
            ClientError::Remote(RemoteError::PermissionDenied) => Self::PermissionDenied,
            ClientError::Remote(RemoteError::InvalidInput | RemoteError::Disabled) => Self::Input,
            ClientError::StaleConnection => Self::Gone,
            ClientError::AlreadyConnected => Self::Input,
            ClientError::ConsumerRejected
            | ClientError::Disconnected
            | ClientError::Remote(RemoteError::Transport) => Self::Io,
        }
    }
}
/// JSON lines and human text use the same versioned native result representation.
pub struct Output {
    pub remote_io: bool,
    pub mutation_requested: bool,
    pub json: bool,
    pub command: String,
    pub records: usize,
    pub acknowledged: usize,
    cap: usize,
    used: usize,
    writer: std::io::Stdout,
}
impl Output {
    pub fn new(json: bool, command: String, cap: usize) -> Self {
        Self {
            remote_io: false,
            mutation_requested: false,
            json,
            command,
            cap,
            used: 0,
            records: 0,
            acknowledged: 0,
            writer: std::io::stdout(),
        }
    }
    pub fn payload_capacity(&self) -> usize {
        self.cap.saturating_sub(self.used).saturating_sub(1024) / 2
    }
    pub fn emit(&mut self, kind: &str, data: Value) -> Result<(), Failure> {
        self.write(kind, data, false)
    }
    fn write(&mut self, kind: &str, data: Value, terminal: bool) -> Result<(), Failure> {
        let value = json!({"schema_version":1,"command":self.command,"kind":kind,"data":data});
        let mut line = if self.json {
            serde_json::to_vec(&value)?
        } else {
            format!("{} {}: {}", self.command, kind, value["data"]).into_bytes()
        };
        line.push(b'\n');
        let reserve = if terminal { 0 } else { 512 };
        if self.used.saturating_add(line.len()).saturating_add(reserve) > self.cap {
            return Err(Failure::Limit);
        }
        self.writer.write_all(&line)?;
        self.writer.flush()?;
        self.used += line.len();
        Ok(())
    }
    pub fn finish(&mut self, result: Result<Value, Failure>, timed_out: bool) -> i32 {
        match result {
            Ok(mut data) => {
                if let Some(data) = data.as_object_mut() {
                    data.insert("records".into(), json!(self.records));
                    data.insert("acknowledged".into(), json!(self.acknowledged));
                    data.insert("outcome".into(), json!("complete"));
                }
                if self.write("summary", data, true).is_err() {
                    4
                } else {
                    0
                }
            }
            Err(failure) => {
                let failure = if timed_out && matches!(failure, Failure::Cancelled) {
                    Failure::Deadline
                } else {
                    failure
                };
                let mut data = json!({"outcome":failure.name(),"records":self.records,"acknowledged":self.acknowledged,"exit_code":failure.code()});
                if self.remote_io
                    && (matches!(
                        failure,
                        Failure::Io
                            | Failure::Corrupt
                            | Failure::Cancelled
                            | Failure::Deadline
                            | Failure::Unknown
                    ) || (self.mutation_requested && matches!(failure, Failure::Gone)))
                {
                    data["posted_host_io"] = json!("completion_unconfirmed");
                }
                let kind = if failure.code() == 3 {
                    "summary"
                } else {
                    "error"
                };
                if self.write(kind, data, true).is_err() {
                    4
                } else {
                    failure.code()
                }
            }
        }
    }
}
pub fn hex(bytes: &[u8]) -> String {
    const H: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(H[(b >> 4) as usize] as char);
        s.push(H[(b & 15) as usize] as char);
    }
    s
}
pub fn record(r: dot_logger::capture::CapturedRecord, payload: Option<&[u8]>) -> Value {
    json!({"receipt":{"session":format!("{:032x}",r.receipt.session_id),"producer":r.receipt.producer_id,"sequence":r.receipt.accepted_seq.to_string()},"channel":r.metadata.channel_id,"schema":format!("{:032x}",r.metadata.schema_id),"schema_version":r.metadata.schema_version,"revision":r.revision.to_string(),"mono_ns":r.metadata.mono_ns.to_string(),"real_ns":r.metadata.real_ns.to_string(),"source_time":r.metadata.source_time.map(|s|json!({"ticks":s.ticks.to_string(),"clock_domain":s.clock_domain.to_string()})),"level":format!("{:?}",r.metadata.level),"importance":r.metadata.importance.map(|v|format!("{v:?}")),"payload_bytes":r.payload_len,"data_hex":payload.map(hex)})
}
