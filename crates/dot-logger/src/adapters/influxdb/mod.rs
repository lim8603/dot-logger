//! Explicit general-path InfluxDB v2 projection and finite HTTP writes.
//!
//! The host must give one writer exclusive ownership of a fresh measurement /
//! device / channel namespace for this session. A per-channel high-water mark
//! rejects equal and older real timestamps across all producers and batches.
//! Restarting a writer does not discover existing DB points. No raw payload,
//! file IO, spool, worker or exactly-once guarantee is provided here. Numeric
//! queries use fixed scope, finite response/row/time limits and no retries.
//!
//! # Writes
//!
//! `Writer::new(&QueryBudget, Config)` validates the fixed measurement, device,
//! channel, typed schema, original session and finite batch/event/retry/TTL limits
//! and reserves shared external memory. The host calls `write(&[Point], &AtomicBool)`
//! on its own Tokio runtime. Numbers become `integer`/`float`/`boolean` fields and
//! small events an `event` field; `session`/`producer`/`sequence` are fields, not
//! tags. A failed whole preflight keeps the high-water mark; an attempted batch
//! consumes its timestamps even when Unknown. HTTP 204 is `Acknowledged`; a 400/422
//! partial failure, a broken response or timeout and cancellation in flight are
//! `Unknown`; cancellation before the attempt is `NotSent`. Retries resend the same
//! encoded body. Only HTTPS or a literal loopback HTTP endpoint is accepted, with
//! redirects, proxies and implicit retries off; the token is a sensitive header and
//! server error bodies are not reported. The shared reservation lasts until HTTP
//! drops the last body, and a body still held makes the next batch `Busy`.
//! HTTP/TLS/runtime internals and RSS are separate headroom.
//!
//! # Numeric queries
//!
//! `Writer::query_numeric(NumericQuery, &AtomicBool)` reads `[from_ns, to_ns)` of a
//! fixed numeric channel with at most 1024 rows, a 1 MiB response, request timeout,
//! TTL and cancellation, keeping session, measurement, device, channel, receipt,
//! Unix ns and the typed value in `NumericPage`. Row/byte limits, damage, late
//! errors, partial responses and deadline expiry fail the whole query. The query
//! reservation lasts until the page and HTTP body are dropped. This bounds neither
//! the DB engine's CPU/RAM nor any remote rollback.
//!
//! # Routed deliveries
//!
//! `RoutedWriter::new(writer, completion, db_target, schemas)` fixes the session,
//! initial revision, DB completion contract and each channel's schema and
//! `I64Le`/`F64Le`/`BoolByte`/`Utf8Event` decoding. `write_delivery(delivery,
//! cancel)` is a synchronous one-record step for the host's general worker: it
//! keeps the original metadata, bytes and `Delivery` until local IO on its own
//! current-thread runtime ends, and returns the untried guard when called inside an
//! async runtime. `DeliveryAttempt::result()` gives the write result and
//! `publish()` reports `DatabaseAcknowledged`, failure, drop or Unknown for the
//! original receipt; on `Busy`, keep the attempt and call `publish` again - the
//! HTTP request is never resubmitted. `into_delivery()` recovers only an untried
//! guard; dropping an unpublished attempt is Unknown. File output runs in a
//! separate host worker. A real DB check: `python3 scripts/check-influxdb.py`
//! (needs Docker; `cargo test` ignores the four live tests).
//!
//! The `influxdb_write` example (`crates/dot-logger/examples/influxdb_write.rs`)
//! builds the session budget, writes a few points and reads them back; run it
//! against a throwaway DB with `python3 scripts/check-influxdb.py --example`.
mod numeric;
mod routed;
use crate::{
    capture::CaptureReceipt,
    query::{QueryBudget, Reservation},
};
pub use numeric::{
    NumericPage, NumericPoint, NumericQuery, NumericScope, NumericValue, QueryError,
};
use reqwest::{
    Client, Url,
    header::{AUTHORIZATION, CONTENT_TYPE, HeaderValue, RETRY_AFTER},
};
pub use routed::{
    DeliveryAttempt, ProjectionEncoding, ProjectionError, ProjectionSchema, RoutedWriter,
};
use std::{
    fmt::{self, Write as _},
    net::IpAddr,
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::time::Instant;

/// The conservative policy currently implemented; old points are rejected too.
#[derive(Clone, Copy, Debug)]
pub enum CollisionPolicy {
    /// Reject a timestamp no greater than this session's last attempted point.
    RejectNonIncreasing,
}
/// Fixed field schema, preventing type changes between batches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Signed numeric value.
    Integer,
    /// Finite floating-point value.
    Float,
    /// Boolean value.
    Boolean,
    /// Small UTF-8 event field, never an arbitrary raw record.
    Event,
}
/// One member of the finite startup channel set.
#[derive(Clone, Copy, Debug)]
pub struct Channel {
    /// Registered product channel, emitted as a bounded tag.
    pub id: u32,
    /// Immutable field type for this channel.
    pub kind: Kind,
}
/// Explicit projection, retry and credential configuration. Intentionally no Debug.
pub struct Config {
    /// HTTPS base URL, or literal loopback HTTP; no path, credentials or query.
    pub endpoint: String,
    /// Organization name or identifier, at most 128 UTF-8 bytes.
    pub organization: String,
    /// Exclusive test/application namespace, at most 128 UTF-8 bytes.
    pub bucket: String,
    /// Runtime secret reference resolved by the host; never serialized or logged.
    pub token: String,
    /// Original capture session to accept.
    pub session_id: u128,
    /// Fixed measurement, at most 128 bytes; no line protocol metacharacters.
    pub measurement: String,
    /// Fixed bounded device tag, at most 128 bytes.
    pub device: String,
    /// Unique channels (1..=128), never dynamic receipt or sequence tags.
    pub channels: Vec<Channel>,
    /// Required explicit collision behavior.
    pub collision_policy: CollisionPolicy,
    /// Maximum points per explicit call (1..=128).
    pub max_points: usize,
    /// Encoded body limit (256..=65536 bytes).
    pub max_batch_bytes: usize,
    /// Maximum small event bytes (1..=4096).
    pub max_event_bytes: usize,
    /// Total attempts including first (1..=8); retries reuse identical bytes.
    pub max_attempts: usize,
    /// Positive per-attempt timeout, at most 60 seconds.
    pub request_timeout: Duration,
    /// Positive retry cooling interval, at most 60 seconds.
    pub retry_delay: Duration,
    /// Positive whole-call admission/IO lifetime, at most one hour.
    pub ttl: Duration,
}
/// One explicitly mapped field. No implicit parsing of raw capture bytes.
#[derive(Clone, Copy, Debug)]
pub enum Value<'a> {
    /// Signed integer.
    Integer(i64),
    /// Finite float.
    Float(f64),
    /// Boolean.
    Boolean(bool),
    /// Small UTF-8 event.
    Event(&'a str),
}
/// Original identity and real timestamp; timestamps are never silently changed.
#[derive(Clone, Copy, Debug)]
pub struct Point<'a> {
    /// A configured bounded channel tag.
    pub channel: u32,
    /// Original accepted capture identity, emitted as fields.
    pub receipt: CaptureReceipt,
    /// Explicit Unix nanoseconds; source/mono clocks are not converted.
    pub real_ns: i64,
    /// Explicit typed projection.
    pub value: Value<'a>,
}
/// Redacted configuration/preflight error; no request URL, token or server body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// A previous HTTP body still owns this writer's finite buffer slot.
    Busy,
    /// Invalid endpoint, schema, identity, value or timestamp.
    InvalidInput,
    /// A declared body/event/shared reservation limit was exceeded.
    Limit,
    /// Equal or older timestamp for the same fixed series.
    Collision,
    /// HTTP client initialization failed; original credentials are not exposed.
    Transport,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InfluxDB projection: {self:?}")
    }
}
impl std::error::Error for Error {}
/// API write boundary, independent of filesystem durability or query validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// This explicit call attempted no HTTP request.
    NotSent,
    /// HTTP 204 confirmed the entire batch at the DB API boundary only.
    Acknowledged,
    /// Definitive whole-request rejection, without an earlier uncertain attempt.
    Rejected,
    /// Timeout, transport loss, partial write or otherwise unconfirmed result.
    Unknown,
}
/// Why the finite call stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// A terminal response was received.
    Response,
    /// Explicit cooperative cancellation, after active request future was dropped.
    Cancelled,
    /// Whole-call TTL reached.
    Deadline,
    /// Finite retry count exhausted.
    Attempts,
}
/// One bounded batch result; partial HTTP rejection never confirms every point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WriteReport {
    /// API-only outcome; Unknown applies to every unverified point in the batch.
    pub outcome: Outcome,
    /// Termination cause.
    pub stop: Stop,
    /// Request attempts initiated; zero means no request was sent.
    pub attempts: usize,
    /// Number of mapped points in this call.
    pub points: usize,
    /// Last observed HTTP status, without response body or credentials.
    pub status: Option<u16>,
}
struct Series {
    channel: Channel,
    last: Option<i64>,
}
/// Single-owner, general-path writer. Each `&mut self` call owns its IO/buffers.
/// Dropping a call does not claim remote cancellation; attempted timestamps stay
/// consumed. An HTTP-owned body retains the shared reservation after owner or
/// future Drop; a live previous body prevents another batch from allocating.
pub struct Writer {
    client: Client,
    url: Url,
    authorization: HeaderValue,
    config: Config,
    series: Vec<Series>,
    reservation: Arc<Reservation>,
    pending: Weak<BodyData>,
}
impl Writer {
    /// Validate and reserve before IO. No server connection or runtime is started.
    /// Requested-byte accounting allows two encoded bodies plus bounded schema
    /// scratch. HTTP/TLS/runtime allocations need separate host headroom; this
    /// reservation is not an RSS limit. Callers' input slices remain host-owned.
    pub fn new(budget: &QueryBudget, mut config: Config) -> Result<Self, Error> {
        if !valid_text(&config.endpoint, 2048) {
            return Err(Error::InvalidInput);
        }
        let mut url = Url::parse(&config.endpoint).map_err(|_| Error::InvalidInput)?;
        let loopback = url
            .host_str()
            .and_then(|s| s.trim_matches(['[', ']']).parse::<IpAddr>().ok())
            .is_some_and(|ip| ip.is_loopback());
        if !((url.scheme() == "https") || (url.scheme() == "http" && loopback))
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || !matches!(url.path(), "" | "/")
            || url.query().is_some()
            || url.fragment().is_some()
            || !valid_text(&config.organization, 128)
            || !valid_text(&config.bucket, 128)
            || !valid_text(&config.measurement, 128)
            || !config
                .measurement
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            || !valid_text(&config.device, 128)
            || config.device.contains('\\')
            || config.session_id == 0
            || config.channels.is_empty()
            || config.channels.len() > 128
            || !(1..=128).contains(&config.max_points)
            || !(256..=65536).contains(&config.max_batch_bytes)
            || !(1..=4096).contains(&config.max_event_bytes)
            || !(1..=8).contains(&config.max_attempts)
            || !duration_ok(config.request_timeout, 60)
            || !duration_ok(config.retry_delay, 60)
            || !duration_ok(config.ttl, 3600)
            || !valid_text(&config.token, 4096)
        {
            return Err(Error::InvalidInput);
        }
        for (i, channel) in config.channels.iter().enumerate() {
            if config.channels[..i].iter().any(|c| c.id == channel.id) {
                return Err(Error::InvalidInput);
            }
        }
        let mut authorization = HeaderValue::from_str(&format!("Token {}", config.token))
            .map_err(|_| Error::InvalidInput)?;
        authorization.set_sensitive(true);
        // Retain only the sensitive header; configuration never needs a token copy.
        config.token = String::new();
        url.set_path("/api/v2/write");
        url.query_pairs_mut()
            .append_pair("org", &config.organization)
            .append_pair("bucket", &config.bucket)
            .append_pair("precision", "ns");
        let bytes = config
            .max_batch_bytes
            .checked_mul(2)
            .and_then(|n| n.checked_add(32768))
            .ok_or(Error::Limit)?;
        let reservation = budget.reserve(bytes).map_err(|_| Error::Limit)?;
        let client = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(config.request_timeout)
            .pool_max_idle_per_host(0)
            .build()
            .map_err(|_| Error::Transport)?;
        let series = config
            .channels
            .iter()
            .map(|&channel| Series {
                channel,
                last: None,
            })
            .collect();
        // Remove excess capacity supplied by a host configuration.
        config.channels = config.channels.into_boxed_slice().into_vec();
        for text in [
            &mut config.endpoint,
            &mut config.organization,
            &mut config.bucket,
            &mut config.measurement,
            &mut config.device,
        ] {
            *text = std::mem::take(text).into_boxed_str().into_string();
        }
        Ok(Self {
            client,
            url,
            authorization,
            config,
            series,
            reservation: Arc::new(reservation),
            pending: Weak::new(),
        })
    }
    /// Preflight the complete batch, then explicitly write with bounded retries.
    /// No automatic queue, reconnection loop or file-backed retention is created.
    /// A 400/422 response may represent partial success and remains Unknown.
    /// Only 429/503 responses and transport failures are retry candidates; an
    /// unrepresentable Retry-After date stops rather than retrying prematurely.
    pub async fn write(
        &mut self,
        points: &[Point<'_>],
        cancel: &AtomicBool,
    ) -> Result<WriteReport, Error> {
        let deadline = Instant::now() + self.config.ttl;
        let mut report = WriteReport {
            outcome: Outcome::NotSent,
            stop: Stop::Cancelled,
            attempts: 0,
            points: points.len(),
            status: None,
        };
        if cancel.load(Ordering::Acquire) {
            return Ok(report);
        }
        if self.pending.upgrade().is_some() {
            return Err(Error::Busy);
        }
        let (body, next) = self.encode(points)?;
        let data = Arc::new(BodyData {
            text: body,
            _reservation: Arc::clone(&self.reservation),
        });
        self.pending = Arc::downgrade(&data);
        let body = bytes::Bytes::from_owner(BodyOwner(data));
        let mut uncertain = false;
        loop {
            if cancel.load(Ordering::Acquire) {
                report.stop = Stop::Cancelled;
                return Ok(report);
            }
            if Instant::now() >= deadline {
                report.stop = Stop::Deadline;
                return Ok(report);
            }
            // Before the first await, reserve every attempted series timestamp.
            // A dropped future or unknown response cannot reopen a collision.
            if report.attempts == 0 {
                for (series, last) in self.series.iter_mut().zip(&next) {
                    series.last = *last;
                }
            }
            report.attempts += 1;
            report.outcome = Outcome::Unknown;
            let request = self
                .client
                .post(self.url.clone())
                .header(AUTHORIZATION, self.authorization.clone())
                .header(CONTENT_TYPE, "text/plain; charset=utf-8")
                .body(body.clone())
                .send();
            // Poll cooperative cancellation without assuming remote rollback.
            let response =
                match cooperative(tokio::time::timeout_at(deadline, request), cancel).await {
                    Some(Ok(result)) => result,
                    Some(Err(_)) => {
                        report.stop = Stop::Deadline;
                        return Ok(report);
                    }
                    None => {
                        report.stop = Stop::Cancelled;
                        return Ok(report);
                    }
                };
            let mut delay = self.config.retry_delay;
            if let Ok(response) = response {
                let status = response.status().as_u16();
                report.status = Some(status);
                report.stop = Stop::Response;
                if status == 204 {
                    report.outcome = Outcome::Acknowledged;
                    return Ok(report);
                }
                if [401, 403, 404, 413].contains(&status) && !uncertain {
                    report.outcome = Outcome::Rejected;
                    return Ok(report);
                }
                if status != 429 && status != 503 {
                    return Ok(report);
                }
                uncertain |= status == 503;
                if let Some(value) = response.headers().get(RETRY_AFTER) {
                    match value.to_str().ok().and_then(|s| s.parse::<u64>().ok()) {
                        Some(seconds) => delay = delay.max(Duration::from_secs(seconds)),
                        None => return Ok(report),
                    }
                }
            } else {
                uncertain = true;
            }
            if report.attempts >= self.config.max_attempts {
                report.stop = Stop::Attempts;
                return Ok(report);
            }
            // Oversized Retry-After cannot overflow or extend the whole-call TTL.
            let wake = Instant::now()
                .checked_add(delay)
                .filter(|t| *t < deadline)
                .unwrap_or(deadline);
            if cooperative(tokio::time::sleep_until(wake), cancel)
                .await
                .is_none()
            {
                report.stop = Stop::Cancelled;
                return Ok(report);
            }
        }
    }
    fn encode(&self, points: &[Point<'_>]) -> Result<(String, Vec<Option<i64>>), Error> {
        if points.is_empty() || points.len() > self.config.max_points {
            return Err(Error::Limit);
        }
        let mut next: Vec<_> = self.series.iter().map(|s| s.last).collect();
        let mut body = Limited {
            text: String::with_capacity(self.config.max_batch_bytes),
            limit: self.config.max_batch_bytes,
        };
        for p in points {
            let index = self
                .series
                .iter()
                .position(|s| s.channel.id == p.channel)
                .ok_or(Error::InvalidInput)?;
            if p.receipt.session_id != self.config.session_id
                || p.receipt.accepted_seq == 0
                || p.real_ns <= i64::MIN + 1
                || p.real_ns == i64::MAX
            {
                return Err(Error::InvalidInput);
            }
            if next[index].is_some_and(|last| p.real_ns <= last) {
                return Err(Error::Collision);
            }
            next[index] = Some(p.real_ns);
            write!(
                body,
                "{},channel={},device=",
                self.config.measurement, p.channel
            )
            .map_err(|_| Error::Limit)?;
            for c in self.config.device.chars() {
                if matches!(c, ',' | '=' | ' ') {
                    body.write_char('\\').map_err(|_| Error::Limit)?;
                }
                body.write_char(c).map_err(|_| Error::Limit)?;
            }
            match (self.series[index].channel.kind, p.value) {
                (Kind::Integer, Value::Integer(v)) => write!(body, " integer={v}i"),
                (Kind::Float, Value::Float(v)) if v.is_finite() => write!(body, " float={v}"),
                (Kind::Boolean, Value::Boolean(v)) => write!(body, " boolean={v}"),
                (Kind::Event, Value::Event(v)) => {
                    if v.len() > self.config.max_event_bytes {
                        return Err(Error::Limit);
                    }
                    if v.chars().any(char::is_control) {
                        return Err(Error::InvalidInput);
                    }
                    body.write_str(" event=\"").map_err(|_| Error::Limit)?;
                    for c in v.chars() {
                        if matches!(c, '\\' | '"') {
                            body.write_char('\\').map_err(|_| Error::Limit)?;
                        }
                        body.write_char(c).map_err(|_| Error::Limit)?;
                    }
                    body.write_char('"')
                }
                _ => return Err(Error::InvalidInput),
            }
            .map_err(|_| Error::Limit)?;
            writeln!(
                body,
                ",session=\"{:032x}\",producer={}u,sequence={}u {}",
                p.receipt.session_id, p.receipt.producer_id, p.receipt.accepted_seq, p.real_ns
            )
            .map_err(|_| Error::Limit)?;
        }
        Ok((body.text, next))
    }
}
fn valid_text(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && !s.chars().any(char::is_control)
}
fn duration_ok(d: Duration, seconds: u64) -> bool {
    !d.is_zero() && d <= Duration::from_secs(seconds)
}
async fn cancelled(cancel: &AtomicBool) {
    while !cancel.load(Ordering::Acquire) {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
async fn cooperative<F: std::future::Future>(future: F, cancel: &AtomicBool) -> Option<F::Output> {
    use std::task::Poll;
    let mut future = std::pin::pin!(future);
    let mut cancellation = std::pin::pin!(cancelled(cancel));
    std::future::poll_fn(|cx| {
        if cancellation.as_mut().poll(cx).is_ready() {
            return Poll::Ready(None);
        }
        future.as_mut().poll(cx).map(Some)
    })
    .await
}
struct Limited {
    text: String,
    limit: usize,
}
struct BodyData {
    text: String,
    _reservation: Arc<Reservation>,
}
struct BodyOwner(Arc<BodyData>);
impl AsRef<[u8]> for BodyOwner {
    fn as_ref(&self) -> &[u8] {
        self.0.text.as_bytes()
    }
}
impl fmt::Write for Limited {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if s.len() > self.limit - self.text.len() {
            return Err(fmt::Error);
        }
        self.text.push_str(s);
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::config::*;
    #[test]
    fn cloned_http_body_holds_reservation_until_last_transport_owner_drops() {
        let plan = SessionBuilder::new(SessionConfig {
            memory: MemoryBudget {
                total_bytes: 1024 * 1024,
                input_bytes: 128 * 1024,
                raw_bytes: 128 * 1024,
                output_bytes: 128 * 1024,
                external_bytes: 256 * 1024,
                control_bytes: 128 * 1024,
                worker_bytes: 256 * 1024,
            },
            producers: vec![ProducerQuota {
                small: PoolQuota {
                    slots: 8,
                    slot_bytes: 16,
                    important_slots: 0,
                },
                raw: None,
            }],
            importance: ImportancePolicy::TreatEqually,
            workers: WorkerThreadingConfig {
                count: 1,
                stack_bytes: 4096,
                cpu_ids: vec![],
                affinity: crate::platform::Requirement::Optional,
            },
            capabilities: vec![],
        })
        .validate(&Default::default())
        .unwrap();
        let budget = QueryBudget::new(&plan);
        let reservation = Arc::new(budget.reserve(4096).unwrap());
        let data = Arc::new(BodyData {
            text: "original".into(),
            _reservation: Arc::clone(&reservation),
        });
        let pending = Arc::downgrade(&data);
        let local_body = bytes::Bytes::from_owner(BodyOwner(data));
        let transport_body = local_body.clone();
        drop((reservation, local_body));
        assert_eq!(budget.used_bytes(), 4096);
        assert!(pending.upgrade().is_some());
        assert_eq!(&transport_body[..], b"original");
        drop(transport_body);
        assert!(pending.upgrade().is_none());
        assert_eq!(budget.used_bytes(), 0);
    }
}
