//! Read-only operations dashboard over loopback HTTP (ADR-016).
//!
//! One poller asks the upstream for `GetStatus` at most once per second, one
//! request at a time, and keeps the last successful report next to the latest
//! failure. A small HTTP/1.1 responder answers bounded GET requests for the
//! embedded page and the status JSON, one request per connection. The upstream
//! credential stays in this process.

use crate::websocket::local_origin;
use dot_logger::adapters::remote::{ManagementReply, ManagementRequest, RemoteError};
use dot_logger_client::{Cancellation, ClientError, RemoteClient, RemoteConfig};
use serde_json::{Value, json};
use std::{
    io,
    net::IpAddr,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::{MissedTickBehavior, timeout},
};

/// Schema of the viewer's own status document.
pub const SCHEMA: &str = "dot_logger.viewer.dashboard.v1";
const STATUS_SCHEMA: &str = "dot_logger.status.v1";
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
        if metrics["schema"] != STATUS_SCHEMA {
            return Err("corrupt");
        }
        metrics
    };
    Ok((session, revision, metrics))
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
    let port = listener.local_addr()?.port();
    let status = Arc::new(Mutex::new(Status::new(&upstream.endpoint)));
    let poller = tokio::spawn(poll(status.clone(), upstream));
    let allow_origin = Arc::new(allow_origin);
    let result = crate::listen(&listener, HTTP_CLIENTS, stop, |socket| {
        respond(socket, status.clone(), port, allow_origin.clone())
    })
    .await;
    poller.abort();
    result
}

// One GetStatus at a time, at most once per interval. Any failure drops the
// connection; the next attempt reconnects.
async fn poll(status: Arc<Mutex<Status>>, upstream: RemoteConfig) {
    let cancel = Cancellation::new();
    let mut client: Option<RemoteClient> = None;
    let mut tick = tokio::time::interval(POLL_INTERVAL);
    tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        if client.is_none() {
            match RemoteClient::connect(upstream.clone(), &cancel).await {
                Ok(connected) => client = Some(connected),
                Err(error) => {
                    lock(&status).record(now_ms(), Err(error));
                    continue;
                }
            }
        }
        let Some(connected) = client.as_mut() else {
            continue;
        };
        let outcome = connected
            .request(ManagementRequest::GetStatus, &cancel)
            .await;
        if outcome.is_err() {
            if let Some(mut failed) = client.take() {
                failed.disconnect();
            }
        }
        lock(&status).record(now_ms(), outcome);
    }
}

async fn respond(
    mut socket: TcpStream,
    status: Arc<Mutex<Status>>,
    port: u16,
    allow_origin: Arc<Option<String>>,
) {
    let response = match timeout(HEAD_DEADLINE, read_head(&mut socket)).await {
        Ok(Ok(head)) => route(&head, &status, port, allow_origin.as_deref()),
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

fn route(head: &[u8], status: &Mutex<Status>, port: u16, allow_origin: Option<&str>) -> Vec<u8> {
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
    match loopback_host(host, port) {
        None => return rejection(400),
        Some(false) => return rejection(403),
        Some(true) => {}
    }
    let origins: Vec<_> = values("origin").collect();
    match origins.as_slice() {
        [] => {}
        [Some(origin)] if Some(*origin) == allow_origin || local_origin(origin) => {}
        _ => return rejection(403),
    }
    if values("transfer-encoding").next().is_some()
        || values("content-length").any(|length| length.map(str::trim) != Some("0"))
    {
        return rejection(400);
    }
    let path = target.split_once('?').map_or(target, |(path, _)| path);
    match path {
        "/" => ok("text/html; charset=utf-8", INDEX),
        "/dashboard.js" => ok("text/javascript; charset=utf-8", SCRIPT),
        "/dashboard.css" => ok("text/css; charset=utf-8", STYLE),
        "/api/status" => {
            let body = lock(status).json(now_ms()).to_string();
            ok("application/json", body.as_bytes())
        }
        _ => rejection(404),
    }
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
// another well-formed host (DNS rebinding), None when malformed.
fn loopback_host(host: &str, port: u16) -> Option<bool> {
    let (name, host_port) = host.trim().rsplit_once(':')?;
    let host_port: u16 = host_port.parse().ok()?;
    if name.is_empty() {
        return None;
    }
    let loopback = name.eq_ignore_ascii_case("localhost")
        || name
            .strip_prefix('[')
            .and_then(|v6| v6.strip_suffix(']'))
            .unwrap_or(name)
            .parse::<IpAddr>()
            .is_ok_and(|ip| ip.is_loopback());
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
