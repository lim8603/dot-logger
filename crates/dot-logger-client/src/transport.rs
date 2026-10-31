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
        atomic::{AtomicBool, Ordering},
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
    ClientError::Remote(match status.code() {
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
        _ => RemoteError::Transport,
    })
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
        let uri: Uri = config.endpoint.parse().map_err(|_| invalid())?;
        let host = uri
            .host()
            .ok_or_else(invalid)?
            .trim_matches(['[', ']'])
            .to_owned();
        let scheme = uri.scheme_str().ok_or_else(invalid)?;
        if !matches!(scheme, "http" | "https")
            || uri.authority().is_none_or(|a| a.as_str().contains('@'))
            || uri
                .path_and_query()
                .is_some_and(|p| !matches!(p.as_str(), "" | "/"))
            || (uri.port().is_some() && uri.port_u16().is_none())
        {
            return Err(invalid());
        }
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
        let port = uri
            .port_u16()
            .unwrap_or(if scheme == "https" { 443 } else { 80 });
        let https = scheme == "https";
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
}
impl Drop for RemoteSubscription {
    fn drop(&mut self) {
        self.close();
    }
}
impl RemoteSubscription {
    fn close(&mut self) {
        self.stream = None;
        let _ = self.receiver.end_connection(&self.id);
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
        if self.state.closed.load(Ordering::Acquire) {
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
                self.close();
                Err(error)
            }
        }
    }
    async fn next_inner(&mut self) -> Result<Option<RemoteEvent>, ClientError> {
        loop {
            let Some(stream) = self.stream.as_mut() else {
                return Ok(None);
            };
            let Some(frame) = stream.message().await.map_err(rpc_error)? else {
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
    /// Synchronously hand off data, then ACK on the same non-reconnecting socket.
    /// Consumer rejection produces no ACK. A send error after acceptance reports
    /// AcknowledgementUnknown, never successful receiving-application completion.
    pub async fn handoff(
        &mut self,
        record: PendingRecord,
        consumer: impl FnOnce(&CapturedRecord, &[u8]) -> bool,
        cancel: &Cancellation,
    ) -> Result<(), ClientError> {
        if cancel.is_cancelled() {
            return Err(ClientError::Cancelled);
        }
        if self.stream.is_none() || self.state.closed.load(Ordering::Acquire) {
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
        let result = cancel
            .guard(self.timeout, async {
                if self.state.closed.load(Ordering::Acquire) {
                    return Err(ClientError::Disconnected);
                }
                self.grpc
                    .ready()
                    .await
                    .map_err(|_| ClientError::Remote(RemoteError::Transport))?;
                let reply: tonic::Response<wire::Reply> = self
                    .grpc
                    .unary(
                        auth(codec::ack(ack), &self.token, self.timeout),
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
                Ok(())
            })
            .await;
        if result.is_err() {
            self.close();
            return Err(ClientError::AcknowledgementUnknown);
        }
        Ok(())
    }
}
