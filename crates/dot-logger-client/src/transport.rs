use crate::{
    Acknowledgement, Cancellation, ClientError, ClientReceiver, ConnectionId, PendingRecord,
    ReceiverConfig, codec, wire,
};
use dot_logger::{adapters::remote::*, capture::CapturedRecord, query::QueryBudget};
use std::{
    io,
    net::IpAddr,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::TcpStream,
};
use tonic::{
    client::Grpc,
    codegen::http::{Uri, uri::PathAndQuery},
    metadata::{Ascii, MetadataValue},
    transport::{Certificate, Channel, ClientTlsConfig, Endpoint},
};

/// Explicit remote endpoint and finite operation/codec limits. Deliberately not
/// Debug: the bearer token and CA bytes must not appear in diagnostics.
#[derive(Clone)]
pub struct RemoteConfig {
    /// Absolute HTTP(S) authority, with no userinfo, query or RPC path.
    pub endpoint: String,
    /// Optional 16..256 printable bearer credential; never logged.
    pub token: Option<String>,
    /// Explicit trusted CA PEM for HTTPS (mandatory for HTTPS).
    pub ca_certificate: Option<Vec<u8>>,
    /// Optional TLS certificate DNS identity, e.g. localhost for a fixture IP.
    pub tls_domain: Option<String>,
    /// Finite connect/RPC/next-call wait, up to one hour. A local `next` timeout
    /// leaves the subscription open; server lease expiry remains terminal.
    pub timeout: Duration,
    /// Encoded request/reply cap: 2048..=16 MiB; decode/runtime headroom is separate.
    pub message_bytes: usize,
}
impl RemoteConfig {
    /// The endpoint as `scheme://host:port` after the syntax check that
    /// [`RemoteClient::connect`] applies: http or https, a host, no userinfo,
    /// path or query. Safe to display. The loopback-only plaintext and TLS
    /// requirements are still checked when connecting.
    ///
    /// # Errors
    /// `InvalidInput` for an endpoint `connect` would reject by syntax.
    pub fn checked_endpoint(&self) -> Result<String, ClientError> {
        let (uri, https) = parse_endpoint(&self.endpoint)?;
        let host = uri
            .host()
            .ok_or(ClientError::Remote(RemoteError::InvalidInput))?;
        let host = if host.contains(':') && !host.starts_with('[') {
            format!("[{host}]")
        } else {
            host.to_owned()
        };
        let port = uri.port_u16().unwrap_or(if https { 443 } else { 80 });
        Ok(format!(
            "{}://{host}:{port}",
            if https { "https" } else { "http" }
        ))
    }
    /// Bounded defaults; plaintext is permitted only for literal loopback IPs.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            token: None,
            ca_certificate: None,
            tls_domain: None,
            timeout: Duration::from_secs(5),
            message_bytes: MAX_CHUNK_BYTES + 1024,
        }
    }
}
// Syntax shared by connect and checked_endpoint: an absolute http(s) URI with
// a host and optional port, no userinfo, and no path beyond "/" or query.
// Returns the URI and whether it is https.
fn parse_endpoint(endpoint: &str) -> Result<(Uri, bool), ClientError> {
    let invalid = || ClientError::Remote(RemoteError::InvalidInput);
    if endpoint.len() > 2048 {
        return Err(invalid());
    }
    let uri: Uri = endpoint.parse().map_err(|_| invalid())?;
    let https = match uri.scheme_str() {
        Some("http") => false,
        Some("https") => true,
        _ => return Err(invalid()),
    };
    let (Some(host), Some(authority)) = (uri.host(), uri.authority()) else {
        return Err(invalid());
    };
    // An out-of-range port parses as no port; check what follows the host.
    let port = authority.as_str().strip_prefix(host).ok_or_else(invalid)?;
    let port_ok = port.is_empty()
        || port.strip_prefix(':').is_some_and(|digits| {
            !digits.is_empty()
                && digits.bytes().all(|b| b.is_ascii_digit())
                && digits.parse::<u16>().is_ok()
        });
    if !port_ok
        || authority.as_str().contains('@')
        || uri
            .path_and_query()
            .is_some_and(|p| !matches!(p.as_str(), "" | "/"))
    {
        return Err(invalid());
    }
    Ok((uri, https))
}
struct ConnectionState {
    closed: AtomicBool,
    read: Mutex<Option<Waker>>,
    write: Mutex<Option<Waker>>,
}
impl ConnectionState {
    fn new() -> Self {
        Self {
            closed: AtomicBool::new(false),
            read: Mutex::new(None),
            write: Mutex::new(None),
        }
    }
    fn close(&self) {
        self.closed.store(true, Ordering::Release);
        for slot in [&self.read, &self.write] {
            let wake = slot.lock().ok().and_then(|mut w| w.take());
            if let Some(w) = wake {
                w.wake();
            }
        }
    }
    fn register(&self, cx: &Context<'_>, writing: bool) -> io::Result<()> {
        let slot = if writing { &self.write } else { &self.read };
        *slot
            .lock()
            .map_err(|_| io::Error::other("connection state unavailable"))? =
            Some(cx.waker().clone());
        if self.closed.load(Ordering::Acquire) {
            Err(io::Error::new(
                io::ErrorKind::ConnectionAborted,
                "connection closed",
            ))
        } else {
            Ok(())
        }
    }
}
struct ConnectionIo {
    stream: TcpStream,
    state: Arc<ConnectionState>,
}
impl Drop for ConnectionIo {
    fn drop(&mut self) {
        self.state.close();
    }
}
impl AsyncRead for ConnectionIo {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if let Err(e) = self.state.register(cx, false) {
            return Poll::Ready(Err(e));
        }
        let before = buf.filled().len();
        let result = Pin::new(&mut self.stream).poll_read(cx, buf);
        if matches!(&result, Poll::Ready(Err(_)))
            || (matches!(&result, Poll::Ready(Ok(()))) && buf.filled().len() == before)
        {
            self.state.close();
        }
        result
    }
}
impl AsyncWrite for ConnectionIo {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        if let Err(e) = self.state.register(cx, true) {
            return Poll::Ready(Err(e));
        }
        let result = Pin::new(&mut self.stream).poll_write(cx, buf);
        if matches!(&result, Poll::Ready(Err(_))) {
            self.state.close();
        }
        result
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if let Err(e) = self.state.register(cx, true) {
            return Poll::Ready(Err(e));
        }
        Pin::new(&mut self.stream).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}
fn rpc_error(status: tonic::Status) -> ClientError {
    let reason = status
        .metadata()
        .get(dot_logger::adapters::remote::REASON_KEY)
        .and_then(|v| v.to_str().ok());
    ClientError::Remote(match (status.code(), reason) {
        (tonic::Code::Unavailable, Some("busy")) => RemoteError::Busy,
        (tonic::Code::FailedPrecondition, Some("restart_required")) => RemoteError::RestartRequired,
        (code, _) => code_error(code),
    })
}
fn code_error(code: tonic::Code) -> RemoteError {
    match code {
        tonic::Code::InvalidArgument => RemoteError::InvalidInput,
        tonic::Code::PermissionDenied | tonic::Code::Unauthenticated => {
            RemoteError::PermissionDenied
        }
        tonic::Code::ResourceExhausted => RemoteError::Limit,
        tonic::Code::OutOfRange => RemoteError::Limit,
        tonic::Code::DataLoss => RemoteError::Corrupt,
        tonic::Code::FailedPrecondition => RemoteError::Version,
        tonic::Code::DeadlineExceeded => RemoteError::TimedOut,
        tonic::Code::NotFound => RemoteError::Gone,
        tonic::Code::Unimplemented => RemoteError::Unsupported,
        tonic::Code::Aborted => RemoteError::Conflict,
        _ => RemoteError::Transport,
    }
}
fn auth<T>(body: T, token: &Option<MetadataValue<Ascii>>, timeout: Duration) -> tonic::Request<T> {
    let mut request = tonic::Request::new(body);
    if let Some(token) = token {
        request
            .metadata_mut()
            .insert("authorization", token.clone());
    }
    request.set_timeout(timeout);
    request
}
fn cap(grpc: Grpc<Channel>, max: usize) -> Grpc<Channel> {
    grpc.max_decoding_message_size(max)
        .max_encoding_message_size(max)
}
async fn unary(
    grpc: &mut Grpc<Channel>,
    token: &Option<MetadataValue<Ascii>>,
    timeout: Duration,
    path: &'static str,
    body: wire::Request,
) -> Result<wire::Reply, ClientError> {
    grpc.ready()
        .await
        .map_err(|_| ClientError::Remote(RemoteError::Transport))?;
    let reply: tonic::Response<wire::Reply> = grpc
        .unary(
            auth(body, token, timeout),
            PathAndQuery::from_static(path),
            tonic::codec::ProstCodec::default(),
        )
        .await
        .map_err(rpc_error)?;
    Ok(reply.into_inner())
}
/// Native client over one actual TCP/TLS connection. No automatic reconnect or
/// retry is permitted: create a fresh client after failure. The caller owns Tokio.
pub struct RemoteClient {
    grpc: Grpc<Channel>,
    token: Option<MetadataValue<Ascii>>,
    state: Arc<ConnectionState>,
    timeout: Duration,
    max: usize,
}
impl Drop for RemoteClient {
    fn drop(&mut self) {
        self.state.close();
    }
}
impl RemoteClient {
    /// Validate security/limits and connect explicitly. Diagnostic errors never
    /// echo endpoint text, credentials, TLS bytes or arbitrary server messages.
    pub async fn connect(config: RemoteConfig, cancel: &Cancellation) -> Result<Self, ClientError> {
        if cancel.is_cancelled() {
            return Err(ClientError::Cancelled);
        }
        let invalid = || ClientError::Remote(RemoteError::InvalidInput);
        if config.endpoint.len() > 2048
            || config.timeout.is_zero()
            || config.timeout > Duration::from_secs(3600)
            || !(2048..=16 * 1024 * 1024).contains(&config.message_bytes)
        {
            return Err(invalid());
        }
        let (uri, https) = parse_endpoint(&config.endpoint)?;
        let host = uri
            .host()
            .ok_or_else(invalid)?
            .trim_matches(['[', ']'])
            .to_owned();
        let scheme = if https { "https" } else { "http" };
        let loopback = host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback());
        if !loopback && (scheme == "http" || config.token.is_none()) {
            return Err(ClientError::Remote(RemoteError::PermissionDenied));
        }
        let token = match config.token {
            None => None,
            Some(t) => {
                if !(16..=256).contains(&t.len()) || !t.bytes().all(|b| b.is_ascii_graphic()) {
                    return Err(invalid());
                }
                let mut value: MetadataValue<Ascii> =
                    format!("Bearer {t}").parse().map_err(|_| invalid())?;
                value.set_sensitive(true);
                Some(value)
            }
        };
        let port = uri.port_u16().unwrap_or(if https { 443 } else { 80 });
        let mut endpoint = Endpoint::from(uri)
            .connect_timeout(config.timeout)
            .buffer_size(1)
            .http2_adaptive_window(false);
        if https {
            let ca = config.ca_certificate.ok_or_else(invalid)?;
            if ca.is_empty() || ca.len() > 65536 {
                return Err(invalid());
            }
            let mut tls = ClientTlsConfig::new().ca_certificate(Certificate::from_pem(ca));
            if let Some(domain) = config.tls_domain {
                if domain.is_empty() || domain.len() > 253 {
                    return Err(invalid());
                }
                tls = tls.domain_name(domain);
            }
            endpoint = endpoint.tls_config(tls).map_err(|_| invalid())?;
        } else if config.ca_certificate.is_some() || config.tls_domain.is_some() {
            return Err(invalid());
        }
        let state = Arc::new(ConnectionState::new());
        let state_io = state.clone();
        let channel = cancel
            .guard(config.timeout, async move {
                let stream = TcpStream::connect((host.as_str(), port))
                    .await
                    .map_err(|_| ClientError::Remote(RemoteError::Transport))?;
                stream
                    .set_nodelay(true)
                    .map_err(|_| ClientError::Remote(RemoteError::Transport))?;
                let io = Arc::new(Mutex::new(Some(ConnectionIo {
                    stream,
                    state: state_io,
                })));
                let connector = tower::service_fn(move |_uri: Uri| {
                    let next = io.lock().ok().and_then(|mut io| io.take());
                    async move {
                        next.map(hyper_util::rt::TokioIo::new).ok_or_else(|| {
                            io::Error::new(
                                io::ErrorKind::ConnectionAborted,
                                "explicit reconnect required",
                            )
                        })
                    }
                });
                endpoint
                    .connect_with_connector(connector)
                    .await
                    .map_err(|_| ClientError::Remote(RemoteError::Transport))
            })
            .await?;
        Ok(Self {
            grpc: cap(Grpc::new(channel), config.message_bytes),
            token,
            state,
            timeout: config.timeout,
            max: config.message_bytes,
        })
    }
    /// Close the actual connection, including subscription IO; pending remote work
    /// may remain host-owned. No reconnect is started.
    pub fn disconnect(&mut self) {
        self.state.close();
    }
    /// Execute one bounded native host request. Timeouts/cancellation end this wait,
    /// not actual host IO; mutating command completion may therefore be unknown.
    pub async fn request(
        &mut self,
        request: ManagementRequest,
        cancel: &Cancellation,
    ) -> Result<ManagementReply, ClientError> {
        if self.state.closed.load(Ordering::Acquire) {
            return Err(ClientError::Disconnected);
        }
        let (method, path) = codec::method(&request);
        let boundary = if let ManagementRequest::GetCompletion { boundary, .. } = &request {
            Some(*boundary)
        } else {
            None
        };
        let replay = if let ManagementRequest::Replay {
            action, generation, ..
        } = &request
        {
            Some((*action, *generation))
        } else {
            None
        };
        let blob_limit = if let ManagementRequest::ReadBlob { length, .. } = &request {
            Some(*length)
        } else {
            None
        };
        let request = codec::request(request, self.max).map_err(ClientError::Remote)?;
        let reply = cancel
            .guard(
                self.timeout,
                unary(&mut self.grpc, &self.token, self.timeout, path, request),
            )
            .await?;
        let reply =
            codec::reply(reply, method, self.max, blob_limit).map_err(ClientError::Remote)?;
        if let (Some(boundary), ManagementReply::Completion(status)) = (boundary, &reply) {
            if status.outcome == dot_logger::completion::QueryOutcome::Complete
                && !status.confirmed.contains(boundary)
            {
                return Err(ClientError::Remote(RemoteError::Corrupt));
            }
        }
        if let (Some((action, requested)), ManagementReply::Replay { generation, .. }) =
            (replay, &reply)
        {
            if requested > 0 {
                let expected = if action == ReplayAction::Seek {
                    requested
                        .checked_add(1)
                        .ok_or(ClientError::Remote(RemoteError::Limit))?
                } else {
                    requested
                };
                if *generation != expected {
                    return Err(ClientError::Remote(RemoteError::Corrupt));
                }
            }
        }
        Ok(reply)
    }
    /// Open a bounded subscription on this exact TCP connection. Originals are
    /// controlled by the explicit native filter; no receiver ACK is sent here.
    pub async fn subscribe(
        &mut self,
        budget: &QueryBudget,
        filter: SubscriptionFilter,
        limits: ReceiverConfig,
        cancel: &Cancellation,
    ) -> Result<RemoteSubscription, ClientError> {
        if self.state.closed.load(Ordering::Acquire) {
            return Err(ClientError::Disconnected);
        }
        let records =
            u32::try_from(limits.records).map_err(|_| ClientError::Remote(RemoteError::Limit))?;
        let bytes = limits
            .records
            .checked_mul(limits.record_bytes)
            .ok_or(ClientError::Remote(RemoteError::Limit))?;
        let age_ms = u64::try_from(limits.ttl.as_millis())
            .map_err(|_| ClientError::Remote(RemoteError::Limit))?;
        if records == 0 || records > 1024 || age_ms == 0 {
            return Err(ClientError::Remote(RemoteError::InvalidInput));
        }
        let mut receiver = ClientReceiver::new();
        let id = receiver.begin_connection(budget, limits)?;
        let request = wire::SubscribeRequest {
            major: PROTOCOL_MAJOR,
            channel: filter.channel,
            minimum_level: filter.minimum_level as u32,
            originals: filter.originals,
            disconnect: filter.overflow == OverflowPolicy::Disconnect,
            records,
            bytes: bytes as u64,
            age_ms,
        };
        let mut stream = cancel
            .guard(self.timeout, async {
                self.grpc
                    .ready()
                    .await
                    .map_err(|_| ClientError::Remote(RemoteError::Transport))?;
                let stream: tonic::Response<tonic::Streaming<wire::Frame>> = self
                    .grpc
                    .server_streaming(
                        auth(request, &self.token, self.timeout),
                        PathAndQuery::from_static("/dot_logger.remote.v1.Logger/Subscribe"),
                        tonic::codec::ProstCodec::default(),
                    )
                    .await
                    .map_err(rpc_error)?;
                Ok(stream.into_inner())
            })
            .await?;
        let first = cancel
            .guard(self.timeout, async {
                stream
                    .message()
                    .await
                    .map_err(rpc_error)?
                    .ok_or(ClientError::Disconnected)
            })
            .await?;
        codec::version(first.major).map_err(ClientError::Remote)?;
        if first.kind != 0 || first.subscription == 0 {
            return Err(ClientError::Remote(RemoteError::Corrupt));
        }
        Ok(RemoteSubscription {
            stream: Some(stream),
            grpc: self.grpc.clone(),
            token: self.token.clone(),
            state: self.state.clone(),
            timeout: self.timeout,
            receiver,
            id,
            subscription: first.subscription,
            acks: Vec::new(),
            oldest: None,
            in_flight: std::collections::VecDeque::new(),
            batch: ACK_BATCH.min(limits.records),
            lost: false,
            confirmed: Arc::new(AtomicU64::new(0)),
        })
    }
}
/// One bounded native subscription event; transport/wire types remain private.
// Keep the bounded record owner inline rather than allocate a box per handoff.
#[allow(clippy::large_enum_variant)]
pub enum RemoteEvent {
    /// Verified complete record, without an acknowledgement until consumer handoff.
    Record(PendingRecord),
    /// Inclusive missing transfer positions due to the bounded overflow policy.
    Gap {
        /// First discarded position.
        first: u64,
        /// Last discarded position.
        last: u64,
    },
}
// ADR-015 ACK pipeline: at most this many queued ACKs per RPC, the longest a
// queued ACK waits while `next` blocks, and the concurrent ACK RPCs.
const ACK_BATCH: usize = 64;
const ACK_DELAY: Duration = Duration::from_millis(5);
const ACK_IN_FLIGHT: usize = 4;
type AckRpc = tokio::task::JoinHandle<Result<(), ClientError>>;
/// Stream/receiver/ACK owner bound to the originating actual connection.
/// Drop closes only this stream; dropping its parent client closes all connection IO.
pub struct RemoteSubscription {
    stream: Option<tonic::Streaming<wire::Frame>>,
    grpc: Grpc<Channel>,
    token: Option<MetadataValue<Ascii>>,
    state: Arc<ConnectionState>,
    timeout: Duration,
    receiver: ClientReceiver,
    id: ConnectionId,
    subscription: u64,
    acks: Vec<wire::AckItem>,
    oldest: Option<Instant>,
    in_flight: std::collections::VecDeque<AckRpc>,
    batch: usize,
    // Sticky: some handed-off record's ACK failed or was never sent.
    lost: bool,
    // Records in ACK RPCs the server confirmed, counted by each RPC task.
    confirmed: Arc<AtomicU64>,
}
impl Drop for RemoteSubscription {
    fn drop(&mut self) {
        self.close();
    }
}
impl RemoteSubscription {
    fn close(&mut self) {
        self.stream = None;
        // Unsent ACKs are never claimed; the server settles them as Unknown,
        // and every later flush reports AcknowledgementUnknown.
        self.lost |= !self.acks.is_empty();
        self.acks.clear();
        self.oldest = None;
        let _ = self.receiver.end_connection(&self.id);
    }
    // Any failed ACK RPC makes its records' outcome unknown and ends the stream.
    fn unknown(&mut self) -> ClientError {
        self.lost = true;
        self.close();
        ClientError::AcknowledgementUnknown
    }
    // Whether handed-off ACKs are not yet settled (or one already failed).
    // These outrank a stream or connection error (REV-071 RV-177, RV-191).
    fn unsettled(&mut self) -> bool {
        self.reap().is_err() || !self.acks.is_empty() || !self.in_flight.is_empty()
    }
    // Check finished ACK RPCs without waiting; the first failure is reported.
    fn reap(&mut self) -> Result<(), ClientError> {
        while self.in_flight.front().is_some_and(|rpc| rpc.is_finished()) {
            let Some(mut rpc) = self.in_flight.pop_front() else {
                break;
            };
            // A finished JoinHandle is ready; a no-op waker is never stored.
            let mut cx = Context::from_waker(Waker::noop());
            match std::future::Future::poll(Pin::new(&mut rpc), &mut cx) {
                Poll::Ready(Ok(Ok(()))) => {}
                _ => return Err(self.unknown()),
            }
        }
        Ok(())
    }
    // Wait for the oldest ACK RPC. It stays queued until it finishes, so a
    // dropped wait (a local `next` timeout) keeps it and its outcome.
    async fn settle_one(&mut self) -> Result<(), ClientError> {
        let Some(rpc) = self.in_flight.front_mut() else {
            return Ok(());
        };
        let result = rpc.await;
        self.in_flight.pop_front();
        match result {
            Ok(Ok(())) => Ok(()),
            _ => Err(self.unknown()),
        }
    }
    // Send every queued ACK as one RPC, after making room in the bounded window.
    async fn send_acks(&mut self) -> Result<(), ClientError> {
        self.reap()?;
        if self.acks.is_empty() {
            return Ok(());
        }
        while self.in_flight.len() >= ACK_IN_FLIGHT {
            self.settle_one().await?;
        }
        if self.state.closed.load(Ordering::Acquire) {
            return Err(self.unknown());
        }
        let records = self.acks.len() as u64;
        let request = codec::acks(self.subscription, std::mem::take(&mut self.acks));
        self.oldest = None;
        let mut grpc = self.grpc.clone();
        let request = auth(request, &self.token, self.timeout);
        let timeout = self.timeout;
        let confirmed = self.confirmed.clone();
        let rpc = tokio::spawn(async move {
            tokio::time::timeout(timeout, async move {
                grpc.ready()
                    .await
                    .map_err(|_| ClientError::Remote(RemoteError::Transport))?;
                let reply: tonic::Response<wire::Reply> = grpc
                    .unary(
                        request,
                        PathAndQuery::from_static("/dot_logger.remote.v1.Logger/Ack"),
                        tonic::codec::ProstCodec::default(),
                    )
                    .await
                    .map_err(rpc_error)?;
                let reply = reply.into_inner();
                codec::version(reply.major).map_err(ClientError::Remote)?;
                if reply.kind != 0 {
                    return Err(ClientError::Remote(RemoteError::Corrupt));
                }
                confirmed.fetch_add(records, Ordering::Relaxed);
                Ok(())
            })
            .await
            .map_err(|_| ClientError::Remote(RemoteError::TimedOut))?
        });
        self.in_flight.push_back(rpc);
        Ok(())
    }
    /// Send queued ACKs and wait for every ACK RPC of this subscription. Ok means
    /// the server confirmed every record handed off so far. A failure closes the
    /// stream with AcknowledgementUnknown; no ACK is retried. Once an
    /// ACK failed or a queued ACK was dropped by the stream closing, every
    /// later flush reports AcknowledgementUnknown too.
    pub async fn flush(&mut self, cancel: &Cancellation) -> Result<(), ClientError> {
        if self.lost {
            return Err(self.unknown());
        }
        let result = cancel
            .guard(self.timeout, async {
                self.send_acks().await?;
                while !self.in_flight.is_empty() {
                    self.settle_one().await?;
                }
                Ok(())
            })
            .await;
        if result.is_err() {
            return Err(self.unknown());
        }
        Ok(())
    }
    /// Records handed off on this subscription whose ACK the server has
    /// confirmed so far, including RPCs no later call has collected yet. After
    /// a successful `flush` it equals every handoff; a record whose outcome is
    /// unknown is never counted.
    pub fn acknowledged(&self) -> u64 {
        self.confirmed.load(Ordering::Relaxed)
    }
    /// Invalidate queued/ready fragments before accepting a newer replay generation.
    /// Host replay/stream generation must be advanced separately by its owner.
    pub fn discard_generations(&mut self, generation: u64) -> Result<(), ClientError> {
        self.receiver
            .discard_generations(&self.id, self.subscription, generation)
    }
    /// Await one complete record or explicit Gap. No ACK is sent by reading.
    /// A local wait timeout returns TimedOut without closing the stream. Retry on
    /// this same subscription explicitly; no reconnect is performed. Server errors
    /// (including its idle lease timeout) and cancellation close the stream.
    pub async fn next(
        &mut self,
        cancel: &Cancellation,
    ) -> Result<Option<RemoteEvent>, ClientError> {
        // An earlier ACK failure stays reported, never as a clean end (RV-177).
        if self.lost {
            return Err(self.unknown());
        }
        if self.state.closed.load(Ordering::Acquire) {
            if self.unsettled() {
                return Err(self.unknown());
            }
            self.close();
            return Err(ClientError::Disconnected);
        }
        // Keep server errors inside Ok so they cannot be confused with the local
        // timer. Streaming::message's decoder state and our partial records are
        // owned by self, not by the dropped wait future (tonic 0.13.1).
        match cancel
            .guard(self.timeout, async { Ok(self.next_inner().await) })
            .await
        {
            Ok(Ok(event)) => Ok(event),
            Err(ClientError::Remote(RemoteError::TimedOut)) => {
                Err(ClientError::Remote(RemoteError::TimedOut))
            }
            Ok(Err(error)) | Err(error) => {
                // Unsettled handoffs outrank the stream's own error: report
                // that their ACKs are unknown (REV-071 RV-177).
                if self.unsettled() {
                    return Err(self.unknown());
                }
                self.close();
                Err(error)
            }
        }
    }
    async fn next_inner(&mut self) -> Result<Option<RemoteEvent>, ClientError> {
        loop {
            self.reap()?;
            // Send queued ACKs once the pipeline is idle or the oldest is due, so
            // a stream that keeps delivering frames cannot starve them.
            if self.oldest.is_some_and(|t| t.elapsed() >= ACK_DELAY)
                || (!self.acks.is_empty() && self.in_flight.is_empty())
            {
                self.send_acks().await?;
            }
            let deadline = self
                .oldest
                .map(|t| tokio::time::Instant::from_std(t + ACK_DELAY));
            let Some(stream) = self.stream.as_mut() else {
                return Ok(None);
            };
            // Streaming::message keeps its decoder state in the stream, so the
            // ACK timer may drop the wait future (tonic 0.13.1). While an ACK
            // RPC is outstanding, wait for it too so its failure surfaces now,
            // not after the next frame or the local timeout (REV-071 RV-177).
            let mut settled = None;
            let message = match (deadline, self.in_flight.front_mut()) {
                (Some(deadline), _) => tokio::select! {
                    biased;
                    message = stream.message() => Some(message),
                    () = tokio::time::sleep_until(deadline) => None,
                },
                (None, Some(rpc)) => tokio::select! {
                    biased;
                    message = stream.message() => Some(message),
                    result = rpc => {
                        settled = Some(result);
                        None
                    }
                },
                (None, None) => Some(stream.message().await),
            };
            if let Some(result) = settled {
                self.in_flight.pop_front();
                if !matches!(result, Ok(Ok(()))) {
                    return Err(self.unknown());
                }
                continue;
            }
            let Some(message) = message else {
                self.send_acks().await?;
                continue;
            };
            let Some(frame) = message.map_err(rpc_error)? else {
                // The server ended the stream: settle what was handed off, so
                // an unconfirmed ACK is reported instead of a clean end.
                self.send_acks().await?;
                while !self.in_flight.is_empty() {
                    self.settle_one().await?;
                }
                self.close();
                return Ok(None);
            };
            codec::version(frame.major).map_err(ClientError::Remote)?;
            if frame.subscription != self.subscription {
                return Err(ClientError::Remote(RemoteError::Corrupt));
            }
            match frame.kind {
                1 => {
                    if let Some(record) = self.receiver.push(
                        &self.id,
                        codec::chunk(frame).map_err(ClientError::Remote)?,
                        Instant::now(),
                    )? {
                        return Ok(Some(RemoteEvent::Record(record)));
                    }
                }
                2 if frame.gap_reason == 1
                    && frame.gap_first > 0
                    && frame.gap_last >= frame.gap_first =>
                {
                    return Ok(Some(RemoteEvent::Gap {
                        first: frame.gap_first,
                        last: frame.gap_last,
                    }));
                }
                _ => return Err(ClientError::Remote(RemoteError::Corrupt)),
            }
        }
    }
    /// Synchronously hand off data, then queue its ACK for the same
    /// non-reconnecting socket and return without waiting for the server.
    /// Consumer rejection produces no ACK. Cancellation after
    /// acceptance, or a failure of an earlier ACK RPC, reports
    /// AcknowledgementUnknown, never successful receiving-application completion.
    /// Use [`Self::flush`] to learn that the server confirmed the handoffs.
    pub async fn handoff(
        &mut self,
        record: PendingRecord,
        consumer: impl FnOnce(&CapturedRecord, &[u8]) -> bool,
        cancel: &Cancellation,
    ) -> Result<(), ClientError> {
        if cancel.is_cancelled() {
            return Err(ClientError::Cancelled);
        }
        if self.lost {
            return Err(self.unknown());
        }
        if self.stream.is_none() || self.state.closed.load(Ordering::Acquire) {
            if self.unsettled() {
                return Err(self.unknown());
            }
            return Err(ClientError::Disconnected);
        }
        let mut called = false;
        let receipt: Acknowledgement = self
            .receiver
            .handoff(record, |record, bytes| {
                called = true;
                consumer(record, bytes)
            })
            .map_err(|_| {
                if called {
                    ClientError::ConsumerRejected
                } else {
                    ClientError::Remote(RemoteError::Gone)
                }
            })?;
        let ack = self.receiver.take_ack(&self.id, receipt)?;
        if cancel.is_cancelled() || self.state.closed.load(Ordering::Acquire) {
            return Err(self.unknown());
        }
        self.acks.push(codec::ack_item(ack));
        self.oldest.get_or_insert_with(Instant::now);
        self.reap()?;
        // Send at once while no ACK RPC is outstanding; otherwise queue until a
        // full batch, the next idle moment, or the ACK delay in `next`.
        if self.in_flight.is_empty() || self.acks.len() >= self.batch {
            let sent = cancel.guard(self.timeout, self.send_acks()).await;
            if sent.is_err() {
                return Err(self.unknown());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap()
    }
    // A subscription with no frame stream whose ACK RPCs are test tasks; the
    // lazy channel is never used because nothing is queued when it would send.
    fn detached() -> RemoteSubscription {
        RemoteSubscription {
            stream: None,
            grpc: Grpc::new(Endpoint::from_static("http://127.0.0.1:9").connect_lazy()),
            token: None,
            state: Arc::new(ConnectionState::new()),
            timeout: Duration::from_millis(500),
            receiver: ClientReceiver::new(),
            id: ConnectionId::detached(),
            subscription: 1,
            acks: Vec::new(),
            oldest: None,
            in_flight: std::collections::VecDeque::new(),
            batch: ACK_BATCH,
            lost: false,
            confirmed: Arc::new(AtomicU64::new(0)),
        }
    }
    fn finished(result: Result<(), ClientError>) -> AckRpc {
        tokio::spawn(async move { result })
    }
    async fn settled(sub: &RemoteSubscription) {
        while !sub.in_flight.iter().all(|rpc| rpc.is_finished()) {
            tokio::task::yield_now().await;
        }
    }
    // REV-071 RV-191: a closed connection with a handed-off ACK still queued
    // or still in flight reports it as unknown from next; only a connection
    // with nothing unsettled reports a plain disconnect. The ACK RPC's timing
    // is fixed here, which a loopback server cannot do.
    #[test]
    fn next_on_a_closed_connection_reports_unsettled_acks_as_unknown() {
        runtime().block_on(async {
            let cancel = Cancellation::new();
            let mut queued = detached();
            queued.acks.push(wire::AckItem::default());
            queued.oldest = Some(Instant::now());
            queued.state.close();
            assert_eq!(
                queued.next(&cancel).await.map(|_| ()),
                Err(ClientError::AcknowledgementUnknown)
            );
            let mut in_flight = detached();
            let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), ClientError>>();
            in_flight
                .in_flight
                .push_back(tokio::spawn(async move { rx.await.unwrap() }));
            in_flight.state.close();
            assert_eq!(
                in_flight.next(&cancel).await.map(|_| ()),
                Err(ClientError::AcknowledgementUnknown)
            );
            drop(tx);
            let mut confirmed = detached();
            confirmed.in_flight.push_back(finished(Ok(())));
            settled(&confirmed).await;
            confirmed.state.close();
            assert_eq!(
                confirmed.next(&cancel).await.map(|_| ()),
                Err(ClientError::Disconnected)
            );
        });
    }
    // A record ready for handoff on `sub`'s own receiver connection.
    fn pending(sub: &mut RemoteSubscription) -> PendingRecord {
        use dot_logger::{
            capture::*,
            config::*,
            platform::{PlatformCapabilities, Requirement},
        };
        let mib = 1024 * 1024;
        let plan = SessionBuilder::new(SessionConfig {
            memory: MemoryBudget {
                total_bytes: 8 * mib,
                input_bytes: mib,
                raw_bytes: 0,
                output_bytes: mib,
                external_bytes: 4 * mib,
                control_bytes: mib,
                worker_bytes: mib,
            },
            producers: vec![ProducerQuota {
                small: PoolQuota {
                    slots: 8,
                    slot_bytes: 256,
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
        .unwrap();
        let config = ReceiverConfig {
            records: 2,
            record_bytes: 8,
            ttl: Duration::from_secs(10),
        };
        sub.id = sub
            .receiver
            .begin_connection(&QueryBudget::new(&plan), config)
            .unwrap();
        let record = CapturedRecord {
            receipt: CaptureReceipt {
                session_id: 42,
                producer_id: 0,
                accepted_seq: 1,
            },
            metadata: RecordMetadata {
                channel_id: 7,
                schema_id: 99,
                schema_version: 1,
                mono_ns: 1,
                real_ns: 1,
                source_time: None,
                level: Level::Info,
                importance: Some(Importance::Normal),
            },
            revision: 1,
            payload_len: 1,
        };
        let chunk = Chunk::single(1, 1, 1, record, &[9], false);
        sub.receiver
            .push(&sub.id, chunk, Instant::now())
            .unwrap()
            .unwrap()
    }
    // REV-073 RV-210: `handoff` on a closed connection reports handed-off ACKs
    // still queued or in flight as unknown, never hands the new record to the
    // consumer and confirms nothing; with nothing unsettled it is a plain
    // disconnect. The ACK RPC timing is fixed here; in the loopback test the
    // ACK may settle first and only the disconnect branch runs.
    #[test]
    fn handoff_on_a_closed_connection_reports_unsettled_acks_as_unknown() {
        runtime().block_on(async {
            let cancel = Cancellation::new();
            let mut called = false;

            let mut queued = detached();
            let record = pending(&mut queued);
            queued.acks.push(wire::AckItem::default());
            queued.oldest = Some(Instant::now());
            queued.state.close();
            let result = queued
                .handoff(record, |_, _| std::mem::replace(&mut called, true), &cancel)
                .await;
            assert_eq!(result, Err(ClientError::AcknowledgementUnknown));
            assert_eq!(queued.acknowledged(), 0);
            assert_eq!(
                queued.flush(&cancel).await,
                Err(ClientError::AcknowledgementUnknown)
            );

            let mut in_flight = detached();
            let record = pending(&mut in_flight);
            let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), ClientError>>();
            in_flight
                .in_flight
                .push_back(tokio::spawn(async move { rx.await.unwrap() }));
            in_flight.state.close();
            let result = in_flight
                .handoff(record, |_, _| std::mem::replace(&mut called, true), &cancel)
                .await;
            assert_eq!(result, Err(ClientError::AcknowledgementUnknown));
            assert_eq!(in_flight.acknowledged(), 0);
            drop(tx);

            let mut confirmed = detached();
            let record = pending(&mut confirmed);
            confirmed.in_flight.push_back(finished(Ok(())));
            settled(&confirmed).await;
            confirmed.state.close();
            let result = confirmed
                .handoff(record, |_, _| std::mem::replace(&mut called, true), &cancel)
                .await;
            assert_eq!(result, Err(ClientError::Disconnected));
            assert!(!called, "a closed connection must not hand records over");
        });
    }
    // RV-163: the frame stream ended (EOF or a stream error) while a handed-off
    // record's ACK was still queued; flush must not claim it was confirmed.
    #[test]
    fn closing_with_a_queued_ack_keeps_flush_unknown() {
        runtime().block_on(async {
            let cancel = Cancellation::new();
            let mut sub = detached();
            sub.acks.push(wire::AckItem::default());
            sub.oldest = Some(Instant::now());
            sub.close();
            assert_eq!(
                sub.flush(&cancel).await,
                Err(ClientError::AcknowledgementUnknown)
            );
        });
    }
    // RV-163: once an ACK RPC failed, later RPCs that succeed cannot turn a
    // repeated flush into a success.
    #[test]
    fn an_earlier_ack_failure_stays_unknown_for_every_later_flush() {
        runtime().block_on(async {
            let cancel = Cancellation::new();
            let mut sub = detached();
            sub.in_flight
                .push_back(finished(Err(ClientError::Remote(RemoteError::Gone))));
            sub.in_flight.push_back(finished(Ok(())));
            settled(&sub).await;
            for _ in 0..2 {
                assert_eq!(
                    sub.flush(&cancel).await,
                    Err(ClientError::AcknowledgementUnknown)
                );
            }
        });
    }
    // RV-163: `next` may drop its wait while the oldest ACK RPC is being
    // settled (local timeout); that RPC and its failure must not be lost.
    #[test]
    fn a_dropped_settle_wait_keeps_the_ack_rpc_and_its_failure() {
        runtime().block_on(async {
            let cancel = Cancellation::new();
            let mut sub = detached();
            let (tx, rx) = tokio::sync::oneshot::channel();
            sub.in_flight
                .push_back(tokio::spawn(async move { rx.await.unwrap() }));
            assert!(
                tokio::time::timeout(Duration::from_millis(20), sub.settle_one())
                    .await
                    .is_err()
            );
            tx.send(Err(ClientError::Remote(RemoteError::TimedOut)))
                .unwrap();
            assert_eq!(
                sub.flush(&cancel).await,
                Err(ClientError::AcknowledgementUnknown)
            );
        });
    }
    // RV-166 (REV-070): the endpoint a viewer may display is the validated
    // scheme, host and port only; userinfo, path, query and other schemes fail.
    #[test]
    fn checked_endpoint_keeps_only_a_validated_authority() {
        for (endpoint, shown) in [
            ("http://127.0.0.1:9", "http://127.0.0.1:9"),
            ("http://127.0.0.1:9/", "http://127.0.0.1:9"),
            ("http://localhost", "http://localhost:80"),
            ("https://robot.example", "https://robot.example:443"),
            ("https://[::1]:8443", "https://[::1]:8443"),
        ] {
            assert_eq!(
                RemoteConfig::new(endpoint).checked_endpoint().as_deref(),
                Ok(shown),
                "{endpoint}"
            );
        }
        for endpoint in [
            "http://user:secret@127.0.0.1:9",
            "http://127.0.0.1:9/?token=secret",
            "http://127.0.0.1:9/Logger/Subscribe",
            "ftp://127.0.0.1:9",
            "127.0.0.1:9",
            "http://127.0.0.1:99999",
            "http://127.0.0.1:",
            "",
        ] {
            assert_eq!(
                RemoteConfig::new(endpoint).checked_endpoint(),
                Err(ClientError::Remote(RemoteError::InvalidInput)),
                "{endpoint}"
            );
        }
    }
}
