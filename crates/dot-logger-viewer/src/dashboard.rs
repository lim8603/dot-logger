//! Read-only operations dashboard over loopback HTTP (ADR-016).
//!
//! One upstream worker owns the connection and sends one request at a time:
//! `GetStatus` at most once per second, and the page's read-only `Compare` and
//! `PlanCheck` lookups from a bounded queue. It keeps the last successful
//! status next to the latest failure. A small HTTP/1.1 responder answers
//! bounded GET requests for the embedded page and the JSON documents, one
//! request per connection. The upstream credential stays in this process.

use crate::websocket::local_origin;
use dot_logger::adapters::remote::{ManagementReply, ManagementRequest, RemoteError};
use dot_logger_client::{Cancellation, ClientError, RemoteClient, RemoteConfig};
use serde_json::{Value, json};
use std::{
    io,
    net::{Ipv4Addr, Ipv6Addr},
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot},
    time::{Instant, sleep_until, timeout},
};

/// Schema of the viewer's own status document.
pub const SCHEMA: &str = "dot_logger.viewer.dashboard.v1";
/// Schema of one read-only lookup reply.
pub const REQUEST_SCHEMA: &str = "dot_logger.viewer.request.v1";
const STATUS_SCHEMA: &str = "dot_logger.status.v1";
const COMPARE_SCHEMA: &str = "dot_logger.compare.v1";
const CHECK_SCHEMA: &str = "dot_logger.check.v1";
// Queued page lookups; a full queue answers viewer_busy.
const JOBS: usize = 4;
const POLL_INTERVAL: Duration = Duration::from_secs(1);
const HTTP_CLIENTS: usize = 8;
const HEAD_BYTES: usize = 8 * 1024;
const MAX_HEADERS: usize = 32;
const HEAD_DEADLINE: Duration = Duration::from_secs(2);
const WRITE_DEADLINE: Duration = Duration::from_secs(2);
const DRAIN_BYTES: usize = 64 * 1024;

const INDEX: &[u8] = include_bytes!("../assets/dashboard/index.html");
const SCRIPT: &[u8] = include_bytes!("../assets/dashboard/dashboard.js");
const STYLE: &[u8] = include_bytes!("../assets/dashboard/dashboard.css");
const POLICY: &str = "default-src 'none'; script-src 'self'; style-src 'self'; \
    connect-src 'self'; img-src 'self'; base-uri 'none'; form-action 'none'; \
    frame-ancestors 'none'";

/// A read-only lookup the page may ask for. Running a check or changing the
/// configuration is not offered (ADR-016).
#[derive(Clone, Copy)]
pub enum Query {
    /// `Compare` of two held snapshots (`dot_logger.compare.v1`).
    Compare,
    /// `PlanCheck` preview of a registered plan (`dot_logger.check.v1` plan).
    Plan,
}

impl Query {
    fn name(self) -> &'static str {
        match self {
            Self::Compare => "compare",
            Self::Plan => "plan",
        }
    }

    /// The `dot_logger.viewer.request.v1` document for one outcome at `at`
    /// (Unix milliseconds). The host report passes through unchanged; a
    /// failure carries its reason and no partial report.
    pub fn json(self, at: u64, outcome: Result<ManagementReply, ClientError>) -> Value {
        self.document(
            at,
            outcome.map_err(reason).and_then(|reply| self.report(reply)),
        )
    }

    fn document(self, at: u64, result: Result<Value, &'static str>) -> Value {
        let mut document = json!({
            "schema": REQUEST_SCHEMA, "request": self.name(), "at_unix_ms": at,
        });
        match result {
            Ok(report) => {
                document["outcome"] = json!("ok");
                document["report"] = report;
            }
            Err(reason) => {
                document["outcome"] = json!("failed");
                document["reason"] = json!(reason);
            }
        }
        document
    }

    fn report(self, reply: ManagementReply) -> Result<Value, &'static str> {
        let ManagementReply::Report(bytes) = reply else {
            return Err("corrupt");
        };
        let report: Value = serde_json::from_slice(&bytes).map_err(|_| "corrupt")?;
        let expected = match self {
            Self::Compare => report["schema"] == COMPARE_SCHEMA,
            Self::Plan => report["schema"] == CHECK_SCHEMA && report["kind"] == "plan",
        };
        if expected { Ok(report) } else { Err("corrupt") }
    }
}

struct Job {
    request: ManagementRequest,
    reply: oneshot::Sender<Result<ManagementReply, ClientError>>,
}

/// Last successful upstream status and the latest failure. They are kept
/// apart, so a failure never erases or zeroes the value it follows.
pub struct Status {
    upstream: String,
    attempts: u64,
    attempted_at: Option<u64>,
    last: Option<Received>,
    failure: Option<(&'static str, u64)>,
}

struct Received {
    at: u64,
    session: u128,
    revision: u64,
    metrics: Value,
}

impl Status {
    pub fn new(upstream: &str) -> Self {
        Self {
            upstream: upstream.to_owned(),
            attempts: 0,
            attempted_at: None,
            last: None,
            failure: None,
        }
    }

    /// Record one `GetStatus` outcome observed at `at` (Unix milliseconds).
    /// A non-status reply or a report without the status schema is corrupt.
    pub fn record(&mut self, at: u64, outcome: Result<ManagementReply, ClientError>) {
        self.attempts = self.attempts.saturating_add(1);
        self.attempted_at = Some(at);
        match outcome.map_err(reason).and_then(received) {
            Ok((session, revision, metrics)) => {
                self.last = Some(Received {
                    at,
                    session,
                    revision,
                    metrics,
                });
                self.failure = None;
            }
            Err(reason) => self.failure = Some((reason, at)),
        }
    }

    /// The `dot_logger.viewer.dashboard.v1` document. The upstream report is
    /// passed through unchanged; absent values stay absent.
    pub fn json(&self, now: u64) -> Value {
        let state = if self.failure.is_some() {
            "failed"
        } else if self.last.is_some() {
            "ok"
        } else {
            "pending"
        };
        json!({
            "schema": SCHEMA,
            "upstream": self.upstream,
            "now_unix_ms": now,
            "poll": {
                "interval_ms": POLL_INTERVAL.as_millis() as u64,
                "attempts": self.attempts,
                "state": state,
                "last_attempt_unix_ms": self.attempted_at,
                "failure": self.failure.map(|(reason, at)| json!({
                    "reason": reason, "at_unix_ms": at,
                })),
            },
            "last": self.last.as_ref().map(|last| json!({
                "received_at_unix_ms": last.at,
                "session": format!("{:032x}", last.session),
                "revision": last.revision.to_string(),
                "metrics": last.metrics,
            })),
        })
    }
}

fn received(reply: ManagementReply) -> Result<(u128, u64, Value), &'static str> {
    let ManagementReply::Status {
        session,
        revision,
        report,
    } = reply
    else {
        return Err("corrupt");
    };
    // An empty report means the host has no metrics backend.
    let metrics = if report.is_empty() {
        json!({"state": "Unavailable", "reason": "no_runtime_metrics_backend"})
    } else {
        let metrics: Value = serde_json::from_slice(&report).map_err(|_| "corrupt")?;
        if metrics["schema"] != STATUS_SCHEMA || !well_formed(&metrics) {
            return Err("corrupt");
        }
        metrics
    };
    Ok((session, revision, metrics))
}

// The fields the page consumes have the status.v1 types: observations are
// objects with a state, windows and snapshots are arrays of such entries.
fn well_formed(metrics: &Value) -> bool {
    let observation = |v: &Value| v.is_object() && v["state"].is_string();
    let window = |v: &Value| observation(v) && v["seconds"].is_u64();
    let snapshot = |v: &Value| {
        ["id", "revision", "epoch", "from_ns", "to_ns"]
            .iter()
            .all(|key| v[key].is_string())
    };
    observation(&metrics["latest"])
        && metrics["windows"]
            .as_array()
            .is_some_and(|w| w.iter().all(window))
        && metrics["snapshots"]
            .as_array()
            .is_some_and(|s| s.iter().all(snapshot))
        && (metrics["posted_revision"].is_null() || metrics["posted_revision"].is_string())
}

fn reason(error: ClientError) -> &'static str {
    match error {
        ClientError::Cancelled => "cancelled",
        ClientError::Disconnected => "disconnected",
        ClientError::Remote(error) => match error {
            RemoteError::InvalidInput => "invalid_input",
            RemoteError::Version => "version",
            RemoteError::Limit => "limit",
            RemoteError::PermissionDenied => "permission_denied",
            RemoteError::Disabled => "disabled",
            RemoteError::TimedOut => "timed_out",
            RemoteError::Transport => "transport",
            RemoteError::Gone => "gone",
            RemoteError::Unsupported => "unsupported",
            RemoteError::Corrupt => "corrupt",
            RemoteError::Conflict => "conflict",
            RemoteError::Busy => "busy",
            RemoteError::RestartRequired => "restart_required",
        },
        _ => "client",
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

fn lock(status: &Mutex<Status>) -> MutexGuard<'_, Status> {
    status
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Serve the dashboard on a loopback listener until `stop` completes.
/// `allow_origin` admits one exact extra Origin, as for the WebSocket.
pub async fn serve(
    listener: TcpListener,
    upstream: RemoteConfig,
    allow_origin: Option<String>,
    stop: impl Future<Output = ()>,
) -> io::Result<()> {
    // Only the checked scheme, host and port are ever shown (RV-166).
    let shown = upstream.checked_endpoint().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "upstream must be an http(s) URL with only a host and port",
        )
    })?;
    let port = listener.local_addr()?.port();
    let status = Arc::new(Mutex::new(Status::new(&shown)));
    // A lookup may wait behind a status request and the queued lookups.
    let deadline = upstream.timeout.saturating_mul(JOBS as u32 + 2);
    let (jobs, queue) = mpsc::channel(JOBS);
    let worker = tokio::spawn(work(status.clone(), upstream, queue));
    let shared = Arc::new(Shared {
        status,
        jobs,
        deadline,
        port,
        allow_origin,
    });
    let result = crate::listen(&listener, HTTP_CLIENTS, stop, |socket| {
        respond(socket, shared.clone())
    })
    .await;
    worker.abort();
    result
}

struct Shared {
    status: Arc<Mutex<Status>>,
    jobs: mpsc::Sender<Job>,
    deadline: Duration,
    port: u16,
    allow_origin: Option<String>,
}

// The only upstream user: one request at a time, queued lookups in between.
// The next GetStatus waits one interval after the previous one finished, so
// the host sees them at least an interval apart even when connecting or the
// network was slow (RV-170).
async fn work(status: Arc<Mutex<Status>>, upstream: RemoteConfig, mut queue: mpsc::Receiver<Job>) {
    let cancel = Cancellation::new();
    let mut client: Option<RemoteClient> = None;
    let mut due = Instant::now();
    loop {
        let (request, reply) = tokio::select! {
            () = sleep_until(due) => (ManagementRequest::GetStatus, None),
            Some(job) = queue.recv() => (job.request, Some(job.reply)),
        };
        let outcome = ask(&mut client, &upstream, &cancel, request).await;
        match reply {
            None => {
                due = Instant::now() + POLL_INTERVAL;
                lock(&status).record(now_ms(), outcome);
            }
            Some(reply) => {
                let _ = reply.send(outcome);
            }
        }
    }
}

// Connect when needed; a transport-level failure drops the connection so the
// next request reconnects. Host answers such as Gone keep it.
async fn ask(
    client: &mut Option<RemoteClient>,
    upstream: &RemoteConfig,
    cancel: &Cancellation,
    request: ManagementRequest,
) -> Result<ManagementReply, ClientError> {
    if client.is_none() {
        *client = Some(RemoteClient::connect(upstream.clone(), cancel).await?);
    }
    let connected = client.as_mut().ok_or(ClientError::Disconnected)?;
    let outcome = connected.request(request, cancel).await;
    if outcome.as_ref().is_err_and(broken) {
        if let Some(mut failed) = client.take() {
            failed.disconnect();
        }
    }
    outcome
}

fn broken(error: &ClientError) -> bool {
    !matches!(
        error,
        ClientError::Remote(
            RemoteError::InvalidInput
                | RemoteError::Limit
                | RemoteError::PermissionDenied
                | RemoteError::Disabled
                | RemoteError::Gone
                | RemoteError::Unsupported
                | RemoteError::Conflict
                | RemoteError::Busy
                | RemoteError::RestartRequired
        )
    )
}

enum Answer {
    Now(Vec<u8>),
    Ask(Query, ManagementRequest),
}

async fn respond(mut socket: TcpStream, shared: Arc<Shared>) {
    let response = match timeout(HEAD_DEADLINE, read_head(&mut socket)).await {
        Ok(Ok(head)) => match route(&head, &shared) {
            Answer::Now(response) => response,
            Answer::Ask(query, request) => lookup(&shared, query, request).await,
        },
        Ok(Err(code)) => rejection(code),
        Err(_) => return,
    };
    let _ = timeout(WRITE_DEADLINE, async {
        socket.write_all(&response).await?;
        socket.shutdown().await?;
        // Read what the client already sent, so closing does not reset the
        // connection before it has read the response.
        let mut sink = [0; 4096];
        let mut drained = 0;
        while drained < DRAIN_BYTES {
            match socket.read(&mut sink).await? {
                0 => break,
                n => drained += n,
            }
        }
        io::Result::Ok(())
    })
    .await;
}

// The request head up to and including the blank line, or a status code.
async fn read_head(socket: &mut TcpStream) -> Result<Vec<u8>, u16> {
    let mut head = Vec::with_capacity(1024);
    let mut chunk = [0; 1024];
    loop {
        let n = socket.read(&mut chunk).await.map_err(|_| 400u16)?;
        if n == 0 {
            return Err(400);
        }
        head.extend_from_slice(&chunk[..n]);
        if let Some(end) = head.windows(4).position(|w| w == b"\r\n\r\n") {
            if end + 4 > HEAD_BYTES {
                return Err(431);
            }
            head.truncate(end + 4);
            return Ok(head);
        }
        if head.len() > HEAD_BYTES {
            return Err(431);
        }
    }
}

async fn lookup(shared: &Shared, query: Query, request: ManagementRequest) -> Vec<u8> {
    let (reply, answer) = oneshot::channel();
    let document = if shared.jobs.try_send(Job { request, reply }).is_err() {
        query.document(now_ms(), Err("viewer_busy"))
    } else {
        match timeout(shared.deadline, answer).await {
            Ok(Ok(outcome)) => query.json(now_ms(), outcome),
            Ok(Err(_)) => query.document(now_ms(), Err("viewer_stopped")),
            Err(_) => query.document(now_ms(), Err("timed_out")),
        }
    };
    ok("application/json", document.to_string().as_bytes())
}

fn route(head: &[u8], shared: &Shared) -> Answer {
    let rejection = |code| Answer::Now(rejection(code));
    let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
    let mut request = httparse::Request::new(&mut headers);
    let Ok(httparse::Status::Complete(_)) = request.parse(head) else {
        return rejection(400);
    };
    let (Some(method), Some(target)) = (request.method, request.path) else {
        return rejection(400);
    };
    if method != "GET" {
        return rejection(405);
    }
    let values = |name: &'static str| header_values(request.headers, name);
    let hosts: Vec<_> = values("host").collect();
    let [Some(host)] = hosts.as_slice() else {
        return rejection(400);
    };
    match loopback_host(host, shared.port) {
        None => return rejection(400),
        Some(false) => return rejection(403),
        Some(true) => {}
    }
    let origins: Vec<_> = values("origin").collect();
    match origins.as_slice() {
        [] => {}
        [Some(origin)]
            if Some(*origin) == shared.allow_origin.as_deref() || local_origin(origin) => {}
        _ => return rejection(403),
    }
    if values("transfer-encoding").next().is_some()
        || values("content-length").any(|length| length.map(str::trim) != Some("0"))
    {
        return rejection(400);
    }
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let now = |content_type, body: &[u8]| Answer::Now(ok(content_type, body));
    match path {
        "/" => now("text/html; charset=utf-8", INDEX),
        "/dashboard.js" => now("text/javascript; charset=utf-8", SCRIPT),
        "/dashboard.css" => now("text/css; charset=utf-8", STYLE),
        "/api/status" => {
            let body = lock(&shared.status).json(now_ms()).to_string();
            now("application/json", body.as_bytes())
        }
        "/api/compare" => match ids(query, ["before", "after"]) {
            Some([before, after]) => {
                Answer::Ask(Query::Compare, ManagementRequest::Compare { before, after })
            }
            None => rejection(400),
        },
        "/api/plan" => match ids(query, ["id"]) {
            Some([plan]) => Answer::Ask(Query::Plan, ManagementRequest::PlanCheck { plan }),
            None => rejection(400),
        },
        _ => rejection(404),
    }
}

// Exactly the named decimal u64 parameters, each once, and nothing else.
fn ids<const N: usize>(query: &str, names: [&str; N]) -> Option<[u64; N]> {
    let mut found = [None; N];
    for pair in query.split('&') {
        let (name, value) = pair.split_once('=')?;
        let slot = found.get_mut(names.iter().position(|n| *n == name)?)?;
        if slot.is_some() || value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        *slot = Some(value.parse().ok()?);
    }
    let mut ids = [0; N];
    for (id, value) in ids.iter_mut().zip(found) {
        *id = value?;
    }
    Some(ids)
}

// Every value of one header; None for a value that is not UTF-8.
fn header_values<'a>(
    headers: &'a [httparse::Header<'a>],
    name: &'static str,
) -> impl Iterator<Item = Option<&'a str>> + 'a {
    headers
        .iter()
        .filter(move |h| h.name.eq_ignore_ascii_case(name))
        .map(|h| std::str::from_utf8(h.value).ok())
}

// Some(true) for a loopback name or address with this port, Some(false) for
// another well-formed host (DNS rebinding), None when malformed. A missing
// port is HTTP's default 80; an IPv6 address needs brackets (RV-168).
pub(crate) fn loopback_host(host: &str, port: u16) -> Option<bool> {
    let host = host.trim();
    let (loopback, rest) = match host.strip_prefix('[') {
        Some(bracketed) => {
            let (address, rest) = bracketed.split_once(']')?;
            (address.parse::<Ipv6Addr>().ok()?.is_loopback(), rest)
        }
        None => {
            let (name, rest) = host
                .find(':')
                .map_or((host, ""), |at| (&host[..at], &host[at..]));
            if name.is_empty() {
                return None;
            }
            let loopback = name.eq_ignore_ascii_case("localhost")
                || name.parse::<Ipv4Addr>().is_ok_and(|ip| ip.is_loopback());
            (loopback, rest)
        }
    };
    let host_port = match rest {
        "" => 80,
        _ => {
            let digits = rest.strip_prefix(':')?;
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            digits.parse::<u16>().ok()?
        }
    };
    Some(loopback && host_port == port)
}

fn ok(content_type: &str, body: &[u8]) -> Vec<u8> {
    response("200 OK", content_type, body, "")
}

fn rejection(code: u16) -> Vec<u8> {
    let (status, extra) = match code {
        403 => ("403 Forbidden", ""),
        404 => ("404 Not Found", ""),
        405 => ("405 Method Not Allowed", "Allow: GET\r\n"),
        431 => ("431 Request Header Fields Too Large", ""),
        _ => ("400 Bad Request", ""),
    };
    response(
        status,
        "text/plain; charset=utf-8",
        status.as_bytes(),
        extra,
    )
}

fn response(status: &str, content_type: &str, body: &[u8], extra: &str) -> Vec<u8> {
    let mut out = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n\
         Referrer-Policy: no-referrer\r\nContent-Security-Policy: {POLICY}\r\n\
         {extra}Connection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    out.extend_from_slice(body);
    out
}
