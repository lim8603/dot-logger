use crate::{
    model::{self, Manifest},
    output::{self, Failure, Output},
};
use clap::ArgMatches;
use dot_logger::{
    adapters::remote::*, capture::*, completion::Boundary, query::*, replay::*, storage::*,
};
use dot_logger_client::{Cancellation, ReceiverConfig, RemoteClient, RemoteConfig, RemoteEvent};
use serde_json::{Value, json};
use std::{
    fs::{File, OpenOptions},
    path::Path,
    time::{Duration, Instant},
};
#[derive(Clone)]
pub struct Context {
    pub args: ArgMatches,
    pub cancel: Cancellation,
    pub started: Instant,
    pub deadline_ns: u64,
}
impl Context {
    pub fn validate(&self) -> Result<(), Failure> {
        let (name, sub) = self.command()?;
        let remote = self.optional("endpoint").is_some();
        match (name, sub) {
            ("query", _) | ("replay", _) => {
                let query = self.query()?;
                self.limit()?;
                if name == "replay" {
                    self.speed()?;
                    self.action()?;
                    if remote {
                        self.number::<u64>("from")?;
                        self.number::<u64>("handle")?;
                        self.number::<u64>("generation")?;
                    } else if query.basis != TimeBasis::Mono
                        || !matches!(self.action()?, ReplayAction::Start)
                    {
                        return Err(Failure::Unsupported);
                    }
                } else if remote {
                    self.number::<u64>("snapshot")?;
                    self.number::<u64>("cursor")?;
                }
            }
            ("tail", _) if remote => {
                self.limit()?;
                self.level()?;
                let records = self.number::<usize>("records")?;
                let bytes = self.number::<usize>("record-bytes")?;
                let external = self.number::<usize>("external-bytes")?;
                let age = self.number::<u64>("age-ms")?;
                if records == 0
                    || records > 1024
                    || bytes == 0
                    || bytes > 64 * 1024 * 1024
                    || external > 2 * 1024 * 1024 * 1024
                    || !(1..=86_400_000).contains(&age)
                    || bytes
                        .checked_add(1024)
                        .and_then(|b| b.checked_mul(records))
                        .is_none_or(|b| b > external)
                {
                    return Err(Failure::Input);
                }
                tail_plan(external)?;
                if let Some(channel) = self.optional("channel") {
                    channel.parse::<u32>().map_err(|_| Failure::Input)?;
                }
            }
            ("blob", _) if remote => {
                self.number::<u64>("snapshot")?;
                self.number::<u64>("blob")?;
                let offset = self.number::<u64>("offset")?;
                let length = self.number::<u32>("length")?;
                if length == 0 || offset.checked_add(length as u64).is_none() {
                    return Err(Failure::Input);
                }
            }
            ("completion", _) if remote => {
                model::id(self.text("session")?)?;
                self.number::<u32>("producer")?;
                self.number::<u32>("target")?;
                if self.number::<u64>("sequence")? == 0
                    || !matches!(
                        self.text("boundary")?,
                        "enqueued" | "written" | "synced" | "database" | "receiver"
                    )
                {
                    return Err(Failure::Input);
                }
            }
            ("config", Some("validate" | "diff")) if !remote => {}
            ("config", Some("apply")) if remote => {
                self.text("patch")?;
                self.number::<u64>("base-revision")?;
            }
            ("check", Some("plan" | "run")) if remote => {
                self.number::<u64>("plan")?;
            }
            ("compare", _) if remote => {
                self.number::<u64>("before")?;
                self.number::<u64>("after")?;
            }
            ("status", _) => {}
            ("recover", _) if !remote => {
                self.text("output")?;
            }
            ("calibrate", _) if !remote => {
                self.text("directory")?;
                self.text("output")?;
                self.number::<u64>("write-limit")?;
                self.number::<u64>("time-limit-ms")?;
                self.number::<u64>("disk-reserve")?;
                return Ok(());
            }
            _ => return Err(Failure::Unsupported),
        }
        if !(remote || name == "config" && sub == Some("diff")) {
            self.text("manifest")?;
        }
        if name == "config" && sub == Some("diff") {
            self.text("before")?;
            self.text("after")?;
        }
        Ok(())
    }
    pub fn text(&self, key: &str) -> Result<&str, Failure> {
        self.args
            .get_one::<String>(key)
            .map(String::as_str)
            .ok_or(Failure::Input)
    }
    pub fn optional(&self, key: &str) -> Option<&str> {
        self.args.get_one::<String>(key).map(String::as_str)
    }
    pub fn number<T: std::str::FromStr>(&self, key: &str) -> Result<T, Failure> {
        self.text(key)?.parse().map_err(|_| Failure::Input)
    }
    pub fn now(&self) -> u64 {
        self.started.elapsed().as_nanos().min(u64::MAX as u128) as u64
    }
    pub fn check(&self) -> Result<(), Failure> {
        if self.cancel.is_cancelled() {
            Err(Failure::Cancelled)
        } else if self.now() >= self.deadline_ns {
            Err(Failure::Deadline)
        } else {
            Ok(())
        }
    }
    pub fn speed(&self) -> Result<Speed, Failure> {
        let (n, d) = self.text("speed")?.split_once('/').ok_or(Failure::Input)?;
        Speed::new(
            n.parse().map_err(|_| Failure::Input)?,
            d.parse().map_err(|_| Failure::Input)?,
        )
        .map_err(Failure::from)
    }
    pub fn query(&self) -> Result<Query, Failure> {
        let basis = match self.text("basis")? {
            "mono" => TimeBasis::Mono,
            "real" => TimeBasis::Real,
            "source" => {
                let domain = self.number("clock-domain")?;
                if domain == 0 {
                    return Err(Failure::Input);
                }
                TimeBasis::Source(domain)
            }
            _ => return Err(Failure::Input),
        };
        let query = Query {
            basis,
            from: self.number("from")?,
            to: self.number("to")?,
            channel: self
                .optional("channel")
                .map(|v| v.parse().map_err(|_| Failure::Input))
                .transpose()?,
            minimum_level: self.level()?,
        };
        if query.from >= query.to {
            return Err(Failure::Input);
        }
        Ok(query)
    }
    pub fn level(&self) -> Result<Level, Failure> {
        Ok(match self.text("level")? {
            "trace" => Level::Trace,
            "debug" => Level::Debug,
            "info" => Level::Info,
            "warn" => Level::Warn,
            "error" => Level::Error,
            "fatal" => Level::Fatal,
            _ => return Err(Failure::Input),
        })
    }
    pub fn limit(&self) -> Result<usize, Failure> {
        let n = self.number("limit")?;
        if !(1..=1_000_000).contains(&n) {
            return Err(Failure::Input);
        }
        Ok(n)
    }
    fn action(&self) -> Result<ReplayAction, Failure> {
        Ok(match self.text("action")? {
            "start" => ReplayAction::Start,
            "pause" => ReplayAction::Pause,
            "resume" => ReplayAction::Resume,
            "seek" => ReplayAction::Seek,
            "stop" => ReplayAction::Stop,
            "speed" => ReplayAction::Speed,
            _ => return Err(Failure::Input),
        })
    }
    fn command(&self) -> Result<(&str, Option<&str>), Failure> {
        let (name, sub) = self.args.subcommand().ok_or(Failure::Input)?;
        Ok((name, sub.subcommand_name()))
    }
}
fn emit_record(
    out: &mut Output,
    record: CapturedRecord,
    payload: Option<&[u8]>,
    extra: Value,
) -> Result<(), Failure> {
    if payload.is_some_and(|p| p.len() > out.payload_capacity()) {
        return Err(Failure::Limit);
    }
    let mut value = output::record(record, payload);
    if let (Some(value), Some(extra)) = (value.as_object_mut(), extra.as_object()) {
        value.extend(extra.iter().map(|(k, v)| (k.clone(), v.clone())));
    }
    out.emit("record", value)?;
    out.records += 1;
    Ok(())
}
pub fn local(ctx: &Context, out: &mut Output) -> Result<Value, Failure> {
    ctx.check()?;
    let (command, sub) = ctx.command()?;
    if command == "calibrate" {
        return calibrate(ctx, out);
    }
    if command == "config" && sub == Some("diff") {
        let before: Value = serde_json::from_slice(&model::read(Path::new(ctx.text("before")?))?)?;
        let after: Value = serde_json::from_slice(&model::read(Path::new(ctx.text("after")?))?)?;
        out.emit("diff", json!({"changed":before!=after,"values":"redacted"}))?;
        return Ok(json!({"applied":false}));
    }
    let path = Path::new(ctx.text("manifest")?);
    let manifest = Manifest::load(path)?;
    if command == "config" && sub == Some("validate") {
        out.emit("validation",json!({"valid":true,"scope":"offline_session_declaration","reservation_bytes":manifest.validate()?,"external_bytes":manifest.external_bytes,"sources":manifest.sources.len(),"source_content":"not_scanned","runtime_capability":"Unavailable"}))?;
        return Ok(json!({"scope":"offline_session_declaration"}));
    }
    if command == "status" {
        out.emit("status",json!({"session":manifest.session,"revision":manifest.revision.to_string(),"availability":"Observed","metrics":{"state":"Unavailable","reason":"no_runtime_metrics_backend"}}))?;
        return Ok(json!({}));
    }
    if command == "recover" {
        return recover(ctx, out, &manifest, path);
    }
    let mut snapshot = manifest.snapshot(path, ctx.now(), ctx.deadline_ns, &ctx.cancel)?;
    let query = ctx.query()?;
    let include = ctx.args.get_flag("include-payload");
    let limit = ctx.limit()?;
    match command {
        "query" => {
            let mut cursor = snapshot.start(query, ctx.now(), ctx.cancel.as_atomic())?;
            loop {
                ctx.check()?;
                let page = snapshot.page(&cursor, ctx.now(), ctx.cancel.as_atomic())?;
                let next = page.next.clone();
                let rows: Vec<_> = page
                    .records
                    .iter()
                    .map(|r| {
                        (
                            r.clone(),
                            if include {
                                page.inline(r).map(|p| p.to_vec())
                            } else {
                                None
                            },
                        )
                    })
                    .collect();
                let row_count = rows.len();
                for (i, (r, inline)) in rows.into_iter().enumerate() {
                    let payload = if include {
                        match &r.payload {
                            Payload::Inline { .. } => inline,
                            Payload::Blob(handle) => {
                                if r.record.payload_len > out.payload_capacity() {
                                    return Err(Failure::Limit);
                                }
                                let mut bytes = Vec::with_capacity(r.record.payload_len);
                                while bytes.len() < r.record.payload_len {
                                    ctx.check()?;
                                    let n = manifest
                                        .limits()
                                        .blob_read_bytes
                                        .min(r.record.payload_len - bytes.len());
                                    bytes.extend_from_slice(snapshot.read_blob(
                                        handle,
                                        bytes.len() as u64,
                                        n,
                                        ctx.now(),
                                        ctx.cancel.as_atomic(),
                                    )?);
                                }
                                Some(bytes)
                            }
                        }
                    } else {
                        None
                    };
                    emit_record(
                        out,
                        r.record,
                        payload.as_deref(),
                        json!({"payload_storage":if matches!(r.payload,Payload::Blob(_)){"blob"}else{"inline"}}),
                    )?;
                    if out.records == limit && (i + 1 < row_count || next.is_some()) {
                        return Err(Failure::Limit);
                    }
                }
                if let Some(next) = next {
                    cursor = next;
                } else {
                    break;
                }
            }
            drop(snapshot);
            Ok(json!({"readers_closed":true,"source":"immutable_files"}))
        }
        "replay" => {
            if !matches!(ctx.action()?, ReplayAction::Start) {
                return Err(Failure::Unsupported);
            }
            let mut replay = Replay::new(
                snapshot,
                query,
                ctx.speed()?,
                ctx.now(),
                ctx.cancel.as_atomic(),
            )?;
            loop {
                ctx.check()?;
                match replay.poll(ctx.now(), out.records < limit, ctx.cancel.as_atomic())? {
                    ReplayPoll::Ready(frame) => {
                        let record = *frame.record;
                        let generation = frame.generation;
                        let late = frame.late_ns;
                        let (mut bytes, blob) = match frame.payload {
                            ReplayPayload::Inline(bytes) => {
                                (if include { Some(bytes.to_vec()) } else { None }, None)
                            }
                            ReplayPayload::Blob(handle) => (None, Some(handle.clone())),
                        };
                        if include {
                            if let Some(blob) = blob {
                                if record.payload_len > out.payload_capacity() {
                                    return Err(Failure::Limit);
                                }
                                let mut data = Vec::with_capacity(record.payload_len);
                                while data.len() < record.payload_len {
                                    ctx.check()?;
                                    let n = manifest
                                        .limits()
                                        .blob_read_bytes
                                        .min(record.payload_len - data.len());
                                    data.extend_from_slice(replay.read_blob(
                                        generation,
                                        &blob,
                                        data.len() as u64,
                                        n,
                                        ctx.now(),
                                        ctx.cancel.as_atomic(),
                                    )?);
                                }
                                bytes = Some(data);
                            }
                        }
                        emit_record(
                            out,
                            record,
                            bytes.as_deref(),
                            json!({"generation":generation.to_string(),"late_ns":late.to_string()}),
                        )?;
                    }
                    ReplayPoll::Waiting { due_ns, .. } => {
                        if out.records == limit {
                            replay.stop();
                            return Err(Failure::Limit);
                        }
                        let wait = due_ns.saturating_sub(ctx.now()).min(10_000_000);
                        if wait > 0 {
                            std::thread::sleep(Duration::from_nanos(wait));
                        }
                    }
                    ReplayPoll::End | ReplayPoll::Stopped => {
                        replay.stop();
                        break;
                    }
                    ReplayPoll::Paused => return Err(Failure::Input),
                }
            }
            Ok(json!({"readers_closed":true,"source":"immutable_files"}))
        }
        _ => Err(Failure::Unsupported),
    }
}
fn recover(ctx: &Context, out: &mut Output, m: &Manifest, path: &Path) -> Result<Value, Failure> {
    use dot_logger::routing::{Router, RoutingConfig, Schedule, TargetConfig};
    if m.sources.len() != 1 {
        return Err(Failure::Input);
    }
    let source = m.sources(path).remove(0);
    let metadata = std::fs::metadata(&source.path)?;
    if !metadata.is_file() {
        return Err(Failure::Input);
    }
    if source
        .safe_offset
        .is_some_and(|safe| safe != metadata.len())
    {
        return Err(Failure::Unsupported);
    }
    let mut file = File::open(&source.path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(Failure::Input);
    }
    if source
        .safe_offset
        .is_some_and(|safe| safe != metadata.len())
    {
        return Err(Failure::Unsupported);
    }
    // create_new prevents overwriting the source or an existing output; failures
    // preserve the independent partial file for investigation, without retries.
    let output = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(ctx.text("output")?)?;
    let plan = m.plan(128 * 1024 * 1024)?;
    let channels = m.channels()?;
    let mut owner = CaptureOwner::new(
        &plan,
        model::id(&m.session)?,
        m.revision,
        &channels.iter().map(|c| c.channel).collect::<Vec<_>>(),
    )
    .map_err(|_| Failure::Input)?;
    let report=owner.scope(|s|->Result<RecoveryReport,Failure> {
        let router=Router::new(&plan,s.map(|(_,r)|r).collect(),RoutingConfig {targets:vec![TargetConfig {id:1,required:true,queue_slots:1,pending_slots:0,retained_bytes:1,max_attempts:1,max_age:Duration::from_secs(1)}],routes:channels.iter().map(|c|dot_logger::routing::Route {channel_id:c.channel.channel_id,targets:vec![1]}).collect(),shared_records:1,small_copy_bytes:1,schedule:Schedule {record_quantum:1,byte_quantum:1,records_per_turn:1,bytes_per_turn:1,retries_per_target:1,reclaim_slots_per_producer:1}}).map_err(|_|Failure::Input)?;
        let limits=m.limits();
        let options=RecoveryOptions {writer:WriterConfig {max_payload_bytes:limits.max_payload_bytes,write_buffer_bytes:0,chunk_target_bytes:limits.max_record_bytes,max_records:limits.max_records,max_chunks:1024,max_metadata_bytes:limits.max_metadata_bytes},limits:RecoveryConfig {max_source_bytes:limits.max_source_bytes,max_record_bytes:limits.max_record_bytes,max_chunks:1024}};
        match recover_segment(&mut file,router.writer_budget(1).map_err(|_|Failure::Input)?,channels,options,output,ctx.cancel.as_atomic()) {
            Ok(segment)=>Ok(segment.report),
            Err(error)=>{
                let r=error.report;out.emit("recovery",json!({"records":r.records,"chunks":r.chunks,"verified_end":r.verified_end,"excluded_from":r.excluded.0,"excluded_to":r.excluded.1,"stop":format!("{:?}",r.stop),"published":false}))?;
                Err(if r.stop==RecoveryStop::Cancelled{Failure::Cancelled}else{Failure::Io})
            }
        }
    })?;
    out.emit("recovery",json!({"records":report.records,"chunks":report.chunks,"verified_end":report.verified_end,"excluded_from":report.excluded.0,"excluded_to":report.excluded.1,"stop":format!("{:?}",report.stop),"file_synced":true,"namespace_synced":false}))?;
    if !matches!(
        report.stop,
        RecoveryStop::DataEnd | RecoveryStop::EndOfInput
    ) {
        return Err(Failure::Limit);
    }
    Ok(json!({"file_synced":true,"namespace_synced":false}))
}
async fn remote_config(ctx: &Context) -> Result<RemoteConfig, Failure> {
    let mut config = RemoteConfig::new(ctx.text("endpoint")?);
    config.timeout = Duration::from_millis(ctx.number("timeout-ms")?);
    config.message_bytes = ctx.number("message-bytes")?;
    let env = ctx.text("token-env")?;
    if env.is_empty()
        || env.len() > 128
        || !env.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
    {
        return Err(Failure::Input);
    }
    config.token = match std::env::var(env) {
        Ok(token) => Some(token),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(_)) => return Err(Failure::Input),
    };
    config.ca_certificate = ctx
        .optional("ca")
        .map(|p| model::read(Path::new(p)))
        .transpose()?;
    config.tls_domain = ctx.optional("tls-domain").map(str::to_owned);
    Ok(config)
}
pub async fn remote(ctx: &Context, out: &mut Output) -> Result<Value, Failure> {
    let (name, sub) = ctx.command()?;
    let mut client = RemoteClient::connect(remote_config(ctx).await?, &ctx.cancel).await?;
    out.remote_io = true;
    let include = ctx.args.get_flag("include-payload");
    match name {
        "query" => {
            let query = ctx.query()?;
            let limit = ctx.limit()?;
            let mut snapshot = ctx.number("snapshot")?;
            let mut cursor = ctx.number("cursor")?;
            loop {
                ctx.check()?;
                let ManagementReply::Page {
                    snapshot: current,
                    cursor: next,
                    items,
                } = client
                    .request(
                        ManagementRequest::Query {
                            snapshot,
                            cursor,
                            query,
                        },
                        &ctx.cancel,
                    )
                    .await?
                else {
                    return Err(Failure::Corrupt);
                };
                if next != 0 && next == cursor {
                    return Err(Failure::Corrupt);
                }
                snapshot = current;
                let n = items.len();
                for (i, item) in items.into_iter().enumerate() {
                    let mut bytes = item.inline;
                    if include && item.blob != 0 {
                        if item.record.payload_len > out.payload_capacity() {
                            return Err(Failure::Limit);
                        }
                        bytes = Vec::with_capacity(item.record.payload_len);
                        while bytes.len() < item.record.payload_len {
                            let length = (item.record.payload_len - bytes.len())
                                .min(ctx.number::<usize>("message-bytes")?.saturating_sub(1024));
                            let ManagementReply::Blob(chunk) = client
                                .request(
                                    ManagementRequest::ReadBlob {
                                        snapshot,
                                        blob: item.blob,
                                        offset: bytes.len() as u64,
                                        length: u32::try_from(length)
                                            .map_err(|_| Failure::Input)?,
                                    },
                                    &ctx.cancel,
                                )
                                .await?
                            else {
                                return Err(Failure::Corrupt);
                            };
                            if chunk.is_empty() {
                                return Err(Failure::Corrupt);
                            }
                            bytes.extend_from_slice(&chunk);
                        }
                    }
                    emit_record(
                        out,
                        item.record,
                        if include { Some(&bytes) } else { None },
                        json!({"snapshot":snapshot.to_string(),"blob":item.blob.to_string()}),
                    )?;
                    if out.records == limit && (i + 1 < n || next != 0) {
                        return Err(Failure::Limit);
                    }
                }
                cursor = next;
                if cursor == 0 {
                    break;
                }
            }
            Ok(json!({"snapshot":snapshot.to_string(),"cursor":cursor.to_string()}))
        }
        "tail" => {
            let records = ctx.number::<usize>("records")?;
            let bytes = ctx.number::<usize>("record-bytes")?;
            let external = ctx.number::<usize>("external-bytes")?;
            if records == 0
                || records > 1024
                || bytes == 0
                || bytes > 64 * 1024 * 1024
                || external > 2 * 1024 * 1024 * 1024
            {
                return Err(Failure::Input);
            }
            let plan = tail_plan(external)?;
            let budget = QueryBudget::new(&plan);
            let filter = SubscriptionFilter {
                channel: ctx
                    .optional("channel")
                    .map(|s| s.parse().map_err(|_| Failure::Input))
                    .transpose()?,
                minimum_level: ctx.level()?,
                originals: ctx.args.get_flag("originals"),
                overflow: if ctx.args.get_flag("disconnect-on-overflow") {
                    OverflowPolicy::Disconnect
                } else {
                    OverflowPolicy::Gap
                },
            };
            let mut stream = client
                .subscribe(
                    &budget,
                    filter,
                    ReceiverConfig {
                        records,
                        record_bytes: bytes,
                        ttl: Duration::from_millis(ctx.number("age-ms")?),
                    },
                    &ctx.cancel,
                )
                .await?;
            let limit = ctx.limit()?;
            let mut gaps = 0;
            while let Some(event) = stream.next(&ctx.cancel).await? {
                match event {
                    RemoteEvent::Gap { first, last } => {
                        gaps += 1;
                        out.emit("gap",json!({"first":first.to_string(),"last":last.to_string(),"reason":"overflow"}))?;
                    }
                    RemoteEvent::Record(record) => {
                        let mut failure = None;
                        let accepted = stream
                            .handoff(
                                record,
                                |r, bytes| match emit_record(
                                    out,
                                    *r,
                                    if include { Some(bytes) } else { None },
                                    json!({"ack":"pending"}),
                                ) {
                                    Ok(()) => true,
                                    Err(error) => {
                                        failure = Some(error);
                                        false
                                    }
                                },
                                &ctx.cancel,
                            )
                            .await;
                        if let Some(failure) = failure {
                            return Err(failure);
                        }
                        accepted?;
                        out.acknowledged += 1;
                        if out.records == limit {
                            return Ok(json!({"gaps":gaps,"stopped":"record_limit"}));
                        }
                    }
                }
            }
            Err(Failure::Unknown)
        }
        _ => {
            let request = match (name, sub) {
                ("status", _) => ManagementRequest::GetStatus,
                ("blob", _) => ManagementRequest::ReadBlob {
                    snapshot: ctx.number("snapshot")?,
                    blob: ctx.number("blob")?,
                    offset: ctx.number("offset")?,
                    length: ctx.number("length")?,
                },
                ("replay", _) => ManagementRequest::Replay {
                    handle: ctx.number("handle")?,
                    action: ctx.action()?,
                    generation: ctx.number("generation")?,
                    from: ctx.number("from")?,
                    speed: ctx.speed()?,
                },
                ("completion", _) => ManagementRequest::GetCompletion {
                    receipt: CaptureReceipt {
                        session_id: model::id(ctx.text("session")?)?,
                        producer_id: ctx.number("producer")?,
                        accepted_seq: ctx.number("sequence")?,
                    },
                    target: ctx.number("target")?,
                    boundary: match ctx.text("boundary")? {
                        "enqueued" => Boundary::Enqueued,
                        "written" => Boundary::Written,
                        "synced" => Boundary::Synced,
                        "database" => Boundary::DatabaseAcknowledged,
                        "receiver" => Boundary::ReceiverAcknowledged,
                        _ => return Err(Failure::Input),
                    },
                },
                ("config", Some("apply")) => ManagementRequest::ApplyConfig {
                    base_revision: ctx.number("base-revision")?,
                    patch: model::read(Path::new(ctx.text("patch")?))?,
                },
                ("check", Some("plan")) => ManagementRequest::PlanCheck {
                    plan: ctx.number("plan")?,
                },
                ("check", Some("run")) => ManagementRequest::RunCheck {
                    plan: ctx.number("plan")?,
                },
                ("compare", _) => ManagementRequest::Compare {
                    before: ctx.number("before")?,
                    after: ctx.number("after")?,
                },
                _ => return Err(Failure::Unsupported),
            };
            out.mutation_requested = matches!(
                request,
                ManagementRequest::ApplyConfig { .. }
                    | ManagementRequest::RunCheck { .. }
                    | ManagementRequest::Replay { .. }
            );
            let reply = client.request(request, &ctx.cancel).await?;
            let (kind, data) = match reply {
                ManagementReply::Status {
                    session,
                    revision,
                    report,
                } => {
                    // An empty report means the host has no metrics backend.
                    let metrics = if report.is_empty() {
                        json!({"state":"Unavailable","reason":"no_runtime_metrics_backend"})
                    } else {
                        host_report(&report, "dot_logger.status.v1")?
                    };
                    (
                        "status",
                        json!({"session":format!("{session:032x}"),"revision":revision.to_string(),"availability":"Observed","metrics":metrics}),
                    )
                }
                ManagementReply::Blob(bytes) => {
                    if bytes.len() > out.payload_capacity() {
                        return Err(Failure::Limit);
                    }
                    (
                        "blob",
                        json!({"bytes":bytes.len(),"data_hex":output::hex(&bytes)}),
                    )
                }
                ManagementReply::Replay { generation, state } => (
                    "replay",
                    json!({"generation":generation.to_string(),"state":format!("{state:?}")}),
                ),
                ManagementReply::Completion(status) => {
                    let confirmed = [
                        Boundary::Enqueued,
                        Boundary::Written,
                        Boundary::Synced,
                        Boundary::DatabaseAcknowledged,
                        Boundary::ReceiverAcknowledged,
                    ]
                    .into_iter()
                    .filter(|b| status.confirmed.contains(*b))
                    .map(|b| format!("{b:?}"))
                    .collect::<Vec<_>>();
                    (
                        "completion",
                        json!({"outcome":format!("{:?}",status.outcome),"confirmed":confirmed}),
                    )
                }
                ManagementReply::Applied(revision) => {
                    ("applied", json!({"revision":revision.to_string()}))
                }
                ManagementReply::Report(bytes) => {
                    if bytes.len() > out.payload_capacity() {
                        return Err(Failure::Limit);
                    }
                    if name == "check" {
                        let data: Value =
                            serde_json::from_slice(&bytes).map_err(|_| Failure::Corrupt)?;
                        let expected = if sub == Some("plan") {
                            "plan"
                        } else {
                            "result"
                        };
                        if data["schema"] != "dot_logger.check.v1" || data["kind"] != expected {
                            return Err(Failure::Corrupt);
                        }
                        ("check", data)
                    } else if name == "compare" {
                        ("compare", host_report(&bytes, "dot_logger.compare.v1")?)
                    } else {
                        (
                            "report",
                            json!({"encoding":"host_owned_bytes","data_hex":output::hex(&bytes)}),
                        )
                    }
                }
                ManagementReply::Page { .. } => return Err(Failure::Corrupt),
            };
            let check_failure = if name == "check" && sub == Some("run") {
                match data["outcome"].as_str() {
                    Some("Pass") => None,
                    Some("Fail") => Some(Failure::CheckFailed),
                    Some("Aborted") => Some(Failure::CheckAborted),
                    Some("Inconclusive") => Some(Failure::CheckInconclusive),
                    _ => return Err(Failure::Corrupt),
                }
            } else {
                None
            };
            out.emit(kind, data)?;
            if let Some(failure) = check_failure {
                return Err(failure);
            }
            Ok(json!({}))
        }
    }
}
/// Decode a bounded host JSON report; an unknown schema is invalid data.
fn host_report(bytes: &[u8], schema: &str) -> Result<Value, Failure> {
    let data: Value = serde_json::from_slice(bytes).map_err(|_| Failure::Corrupt)?;
    if data["schema"] != schema {
        return Err(Failure::Corrupt);
    }
    Ok(data)
}
fn tail_plan(external: usize) -> Result<dot_logger::config::ValidatedConfig, Failure> {
    use dot_logger::{config::*, platform::*};
    let mib = 1024 * 1024;
    SessionBuilder::new(SessionConfig {
        memory: MemoryBudget {
            total_bytes: external.checked_add(4 * mib).ok_or(Failure::Input)?,
            input_bytes: mib,
            raw_bytes: 0,
            output_bytes: mib,
            external_bytes: external,
            control_bytes: mib,
            worker_bytes: mib,
        },
        producers: vec![ProducerQuota {
            small: PoolQuota {
                slots: 1,
                slot_bytes: 1,
                important_slots: 0,
            },
            raw: None,
        }],
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

/// TASK-028 S2 (API §19.8): one bounded measurement saved as a new profile.
/// An existing profile is reported before anything is written.
fn calibrate(ctx: &Context, out: &mut Output) -> Result<Value, Failure> {
    use dot_logger::calibrate::{CalibrateError, Calibration};
    let output = Path::new(ctx.text("output")?);
    if output.exists() {
        return Err(Failure::Conflict);
    }
    let profile = Calibration::new(ctx.text("directory")?)
        .write_limit(ctx.number("write-limit")?)
        .time_limit(Duration::from_millis(ctx.number("time-limit-ms")?))
        .disk_reserve(ctx.number("disk-reserve")?)
        .run(ctx.cancel.as_atomic())
        .map_err(|e| match e {
            CalibrateError::InvalidInput => Failure::Input,
            CalibrateError::NoSpace { .. } => Failure::Limit,
            CalibrateError::Cancelled => Failure::Cancelled,
            CalibrateError::Io(_) => Failure::Io,
        })?;
    out.emit("calibration", serde_json::from_str(&profile.to_json())?)?;
    profile.save(output).map_err(|e| {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            Failure::Conflict
        } else {
            Failure::Io
        }
    })?;
    Ok(json!({"profile": output.to_string_lossy()}))
}
