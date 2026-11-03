use super::{
    RemoteHub,
    hub::{Event, Subscription},
    types::*,
    wire,
};
use std::{
    convert::Infallible,
    future::Future,
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};
use subtle::ConstantTimeEq;
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::{TcpListener, TcpStream},
    sync::{OwnedSemaphorePermit, Semaphore, mpsc},
};
use tonic::{
    Request, Response, Status,
    codegen::{BoxFuture, Service, http, tokio_stream::Stream},
    transport::{
        Server,
        server::{Connected, TcpConnectInfo},
    },
};

/// Explicitly bound gRPC server. The host supplies and owns the Tokio runtime.
/// Dropping/cancelling serve closes subscriptions but never cancels actual host IO.
pub struct RemoteServer {
    listener: TcpListener,
    builder: Server,
    hub: RemoteHub,
    security: Arc<Security>,
}
struct Security {
    anonymous: bool,
    tokens: Vec<Credential>,
}
// Normalize stored secrets once; comparison never loops over their actual length.
// This protects the comparison primitive, not the timing of the entire RPC.
struct Credential {
    bytes: [u8; 256],
    length: u16,
    permission: Permission,
}
impl From<AccessToken> for Credential {
    fn from(token: AccessToken) -> Self {
        let mut bytes = [0u8; 256];
        bytes[..token.value.len()].copy_from_slice(token.value.as_bytes());
        Self {
            bytes,
            length: token.value.len() as u16,
            permission: token.permission,
        }
    }
}
impl Credential {
    fn matches(&self, supplied: &[u8]) -> bool {
        // The peer already knows its own supplied length. Stored length is secret.
        if supplied.len() > 256 {
            return false;
        }
        let mut padded = [0u8; 256];
        padded[..supplied.len()].copy_from_slice(supplied);
        bool::from(self.bytes.ct_eq(&padded) & self.length.ct_eq(&(supplied.len() as u16)))
    }
}
impl Security {
    fn check<T>(&self, request: &Request<T>, manage: bool) -> Result<SocketAddr, RemoteError> {
        let peer = request.remote_addr().ok_or(RemoteError::PermissionDenied)?;
        let supplied = request.metadata().contains_key("authorization");
        let token = request
            .metadata()
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "));
        let permission = token.and_then(|value| {
            self.tokens
                .iter()
                .find(|t| t.matches(value.as_bytes()))
                .map(|t| t.permission)
        });
        match permission {
            Some(Permission::Manage) => Ok(peer),
            Some(Permission::Read) if !manage => Ok(peer),
            None if !supplied && self.anonymous && !manage => Ok(peer),
            _ => Err(RemoteError::PermissionDenied),
        }
    }
}
fn status(e: RemoteError) -> Status {
    match e {
        RemoteError::Version => Status::failed_precondition(e.to_string()),
        RemoteError::InvalidInput => Status::invalid_argument(e.to_string()),
        RemoteError::Corrupt => Status::data_loss(e.to_string()),
        RemoteError::Limit => Status::resource_exhausted(e.to_string()),
        RemoteError::PermissionDenied => Status::permission_denied(e.to_string()),
        RemoteError::TimedOut => Status::deadline_exceeded(e.to_string()),
        RemoteError::Unsupported => Status::unimplemented(e.to_string()),
        RemoteError::Gone => Status::not_found(e.to_string()),
        RemoteError::Conflict => Status::aborted(e.to_string()),
        // Shared gRPC codes carry a fixed reason so clients never read a busy
        // revision as transport loss or a restart requirement as a version skew.
        RemoteError::Busy => reasoned(tonic::Code::Unavailable, e, "busy"),
        RemoteError::RestartRequired => {
            reasoned(tonic::Code::FailedPrecondition, e, "restart_required")
        }
        _ => Status::unavailable(e.to_string()),
    }
}
/// Fixed metadata key for reasons that share a gRPC code with another failure.
pub const REASON_KEY: &str = "dot-logger-reason";
fn reasoned(code: tonic::Code, e: RemoteError, reason: &'static str) -> Status {
    let mut metadata = tonic::metadata::MetadataMap::new();
    metadata.insert(
        REASON_KEY,
        tonic::metadata::MetadataValue::from_static(reason),
    );
    Status::with_metadata(code, e.to_string(), metadata)
}
impl RemoteServer {
    /// Bind only after explicit enable, security validation and TLS parsing.
    pub async fn bind(mut config: ServerConfig, hub: RemoteHub) -> Result<Self, RemoteError> {
        if !config.enabled {
            return Err(RemoteError::Disabled);
        }
        config.validate()?;
        let mut builder = Server::builder();
        if let Some(tls) = config.tls.take() {
            builder = builder
                .tls_config(tonic::transport::ServerTlsConfig::new().identity(
                    tonic::transport::Identity::from_pem(tls.certificate, tls.private_key),
                ))
                .map_err(|_| RemoteError::InvalidInput)?;
        }
        let security = Arc::new(Security {
            anonymous: config.address.ip().is_loopback() && config.anonymous_loopback_read,
            tokens: config.tokens.into_iter().map(Credential::from).collect(),
        });
        let listener = TcpListener::bind(config.address)
            .await
            .map_err(|_| RemoteError::Transport)?;
        Ok(Self {
            listener,
            builder,
            hub,
            security,
        })
    }
    /// Actual bound address, including an OS-selected ephemeral port.
    pub fn local_addr(&self) -> Result<SocketAddr, RemoteError> {
        self.listener
            .local_addr()
            .map_err(|_| RemoteError::Transport)
    }
    /// Serve until host shutdown. No detached application worker is created.
    /// Returns only after every server-side request and stream task has dropped
    /// its hub handle; host-owned commands keep theirs until the host drops them.
    pub async fn serve(self, shutdown: impl Future<Output = ()> + Send) -> Result<(), RemoteError> {
        struct Close(RemoteHub);
        impl Drop for Close {
            fn drop(&mut self) {
                self.0.close();
            }
        }
        let _close = Close(self.hub.clone());
        let limits = &self.hub.shared.limits;
        let incoming = Incoming::new(self.listener, limits.connections);
        // Never sent on: recv() ends once the last server-side clone is dropped.
        let (drain, mut drained) = mpsc::channel(1);
        let service = RpcService {
            hub: self.hub.clone(),
            security: self.security,
            _drain: drain,
        };
        let close_hub = self.hub.clone();
        let served = self
            .builder
            .concurrency_limit_per_connection(limits.commands)
            .max_concurrent_streams((limits.commands + limits.subscriptions) as u32)
            .initial_stream_window_size(limits.message_bytes.min(1024 * 1024) as u32)
            .initial_connection_window_size((limits.message_bytes * 2).min(2 * 1024 * 1024) as u32)
            .http2_adaptive_window(Some(false))
            .http2_max_header_list_size(8192)
            .http2_max_pending_accept_reset_streams(Some(32))
            .max_frame_size(Some(16384))
            .max_connection_age(Duration::from_secs(86400))
            .serve_with_incoming_shutdown(service, incoming, async move {
                shutdown.await;
                close_hub.close();
            })
            .await
            .map_err(|_| RemoteError::Transport);
        // Hyper runs each HTTP/2 stream on its own task, which may still hold
        // the hub reservation after the connection future tonic awaits ends.
        if served.is_ok() {
            let _ = drained.recv().await;
        }
        served
    }
}
struct LimitedIo {
    stream: TcpStream,
    _permit: OwnedSemaphorePermit,
}
impl Connected for LimitedIo {
    type ConnectInfo = TcpConnectInfo;
    fn connect_info(&self) -> Self::ConnectInfo {
        self.stream.connect_info()
    }
}
impl AsyncRead for LimitedIo {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_read(cx, buf)
    }
}
impl AsyncWrite for LimitedIo {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.stream).poll_write(cx, buf)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}
struct Incoming {
    listener: TcpListener,
    permits: Arc<Semaphore>,
    retry: Pin<Box<tokio::time::Sleep>>,
    cooling: bool,
}
impl Incoming {
    fn new(listener: TcpListener, connections: usize) -> Self {
        Self {
            listener,
            permits: Arc::new(Semaphore::new(connections)),
            retry: Box::pin(tokio::time::sleep(Duration::ZERO)),
            cooling: false,
        }
    }
}
impl Stream for Incoming {
    type Item = Result<LimitedIo, std::io::Error>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            if self.cooling {
                std::task::ready!(self.retry.as_mut().poll(cx));
                self.cooling = false;
            }
            match self.listener.poll_accept(cx) {
                Poll::Ready(Ok((stream, _))) => match self.permits.clone().try_acquire_owned() {
                    Ok(permit) => {
                        return Poll::Ready(Some(Ok(LimitedIo {
                            stream,
                            _permit: permit,
                        })));
                    }
                    Err(_) => {
                        drop(stream);
                        // Preserve excess-connection rejection, but register a timer
                        // wake instead of continuously repolling a ready backlog.
                        self.retry
                            .as_mut()
                            .reset(tokio::time::Instant::now() + Duration::from_millis(10));
                        self.cooling = true;
                    }
                },
                Poll::Ready(Err(e)) => return Poll::Ready(Some(Err(e))),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}
#[derive(Clone)]
struct RpcService {
    hub: RemoteHub,
    security: Arc<Security>,
    _drain: mpsc::Sender<()>,
}
impl Service<http::Request<tonic::body::Body>> for RpcService {
    type Response = http::Response<tonic::body::Body>;
    type Error = Infallible;
    type Future = BoxFuture<Self::Response, Self::Error>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, request: http::Request<tonic::body::Body>) -> Self::Future {
        let method = request
            .uri()
            .path()
            .strip_prefix("/dot_logger.remote.v1.Logger/")
            .map(str::to_owned);
        let service = self.clone();
        let max = self.hub.shared.limits.message_bytes;
        Box::pin(async move {
            macro_rules! grpc {
                () => {
                    tonic::server::Grpc::new(tonic::codec::ProstCodec::default())
                        .max_decoding_message_size(max)
                        .max_encoding_message_size(max)
                };
            }
            match method.as_deref() {
                Some("Subscribe") => Ok(grpc!()
                    .server_streaming(SubscribeRpc(service), request)
                    .await),
                Some("Ack") => Ok(grpc!().unary(AckRpc(service), request).await),
                Some(
                    name @ ("GetStatus" | "Query" | "ReadBlob" | "Replay" | "GetCompletion"
                    | "ApplyConfig" | "PlanCheck" | "RunCheck" | "Compare"),
                ) => Ok(grpc!()
                    .unary(
                        UnaryRpc {
                            service,
                            method: name.to_owned(),
                        },
                        request,
                    )
                    .await),
                _ => Ok(Status::unimplemented("unknown remote method").into_http()),
            }
        })
    }
}
struct UnaryRpc {
    service: RpcService,
    method: String,
}
impl tonic::server::UnaryService<wire::Request> for UnaryRpc {
    type Response = wire::Reply;
    type Future = BoxFuture<Response<Self::Response>, Status>;
    fn call(&mut self, request: Request<wire::Request>) -> Self::Future {
        let s = self.service.clone();
        let method = self.method.clone();
        Box::pin(async move {
            s.security
                .check(
                    &request,
                    matches!(method.as_str(), "ApplyConfig" | "RunCheck"),
                )
                .map_err(status)?;
            let max = s.hub.shared.limits.message_bytes;
            let native = request.into_inner().native(&method, max).map_err(status)?;
            let reply = s.hub.request(native).await.map_err(status)?;
            let wire = wire::Reply::native(reply, &method, max, 1000).map_err(status)?;
            Ok(Response::new(wire))
        })
    }
}
struct AckRpc(RpcService);
impl tonic::server::UnaryService<wire::AckRequest> for AckRpc {
    type Response = wire::Reply;
    type Future = BoxFuture<Response<Self::Response>, Status>;
    fn call(&mut self, request: Request<wire::AckRequest>) -> Self::Future {
        let s = self.0.clone();
        Box::pin(async move {
            let peer = s.security.check(&request, false).map_err(status)?;
            let ack = request.into_inner().native().map_err(status)?;
            s.hub.ack(ack, peer).map_err(status)?;
            Ok(Response::new(wire::Reply {
                major: PROTOCOL_MAJOR,
                ..wire::Reply::default()
            }))
        })
    }
}
struct SubscribeRpc(RpcService);
impl tonic::server::ServerStreamingService<wire::SubscribeRequest> for SubscribeRpc {
    type Response = wire::Frame;
    type ResponseStream = SubscriptionStream;
    type Future = BoxFuture<Response<Self::ResponseStream>, Status>;
    fn call(&mut self, request: Request<wire::SubscribeRequest>) -> Self::Future {
        let s = self.0.clone();
        Box::pin(async move {
            let peer = s.security.check(&request, false).map_err(status)?;
            let r = request.into_inner();
            wire::version(r.major).map_err(status)?;
            let filter = SubscriptionFilter {
                channel: r.channel,
                minimum_level: wire::level(r.minimum_level).map_err(status)?,
                originals: r.originals,
                overflow: if r.disconnect {
                    OverflowPolicy::Disconnect
                } else {
                    OverflowPolicy::Gap
                },
            };
            let age = Duration::from_millis(r.age_ms);
            let sub = s
                .hub
                .subscribe(
                    peer,
                    filter,
                    r.records as usize,
                    usize::try_from(r.bytes).map_err(|_| status(RemoteError::Limit))?,
                    age,
                )
                .map_err(status)?;
            Ok(Response::new(SubscriptionStream {
                sub,
                _drain: s._drain.clone(),
                started: false,
                finished: false,
                timer: Box::pin(tokio::time::sleep(age)),
                age,
            }))
        })
    }
}
struct SubscriptionStream {
    sub: Arc<Subscription>,
    _drain: mpsc::Sender<()>,
    started: bool,
    finished: bool,
    timer: Pin<Box<tokio::time::Sleep>>,
    age: Duration,
}
impl Drop for SubscriptionStream {
    fn drop(&mut self) {
        self.sub.close();
    }
}
impl Stream for SubscriptionStream {
    type Item = Result<wire::Frame, Status>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.finished {
            return Poll::Ready(None);
        }
        if !self.started {
            self.started = true;
            return Poll::Ready(Some(Ok(wire::Frame {
                major: PROTOCOL_MAJOR,
                subscription: self.sub.id,
                kind: 0,
                ..wire::Frame::default()
            })));
        }
        match self.sub.next(cx.waker()) {
            Ok(Some(event)) => {
                let age = self.age;
                self.timer.as_mut().reset(tokio::time::Instant::now() + age);
                return Poll::Ready(Some(Ok(match event {
                    Event::Chunk(c) => wire::Frame::chunk(c),
                    Event::Gap {
                        subscription,
                        first,
                        last,
                    } => wire::Frame {
                        major: PROTOCOL_MAJOR,
                        subscription,
                        gap_first: first,
                        gap_last: last,
                        gap_reason: 1,
                        kind: 2,
                        ..wire::Frame::default()
                    },
                })));
            }
            Err(e) => {
                self.finished = true;
                return Poll::Ready(Some(Err(status(e))));
            }
            Ok(None) => {}
        }
        if self.timer.as_mut().poll(cx).is_ready() {
            self.sub.close();
            self.finished = true;
            return Poll::Ready(Some(Err(status(RemoteError::TimedOut))));
        }
        Poll::Pending
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod admission_tests {
    use super::*;
    #[cfg(not(miri))]
    use std::{
        future::poll_fn,
        sync::atomic::{AtomicUsize, Ordering},
        task::{Wake, Waker},
    };

    #[cfg(not(miri))]
    struct CountWake {
        count: AtomicUsize,
        parent: Waker,
    }
    #[cfg(not(miri))]
    impl Wake for CountWake {
        fn wake(self: Arc<Self>) {
            self.wake_by_ref();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.count.fetch_add(1, Ordering::Relaxed);
            self.parent.wake_by_ref();
        }
    }

    #[test]
    #[cfg(not(miri))]
    fn saturated_listener_waits_without_self_wake_and_recovers() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let address = listener.local_addr().unwrap();
                let mut incoming = Incoming::new(listener, 1);
                let first_client = TcpStream::connect(address).await.unwrap();
                let first = poll_fn(|cx| Pin::new(&mut incoming).poll_next(cx))
                    .await
                    .unwrap()
                    .unwrap();
                let mut surplus = TcpStream::connect(address).await.unwrap();
                tokio::time::timeout(
                    Duration::from_secs(2),
                    poll_fn(|cx| {
                        let wake = Arc::new(CountWake {
                            count: AtomicUsize::new(0),
                            parent: cx.waker().clone(),
                        });
                        let waker = Waker::from(wake.clone());
                        let mut context = Context::from_waker(&waker);
                        assert!(Pin::new(&mut incoming).poll_next(&mut context).is_pending());
                        assert_eq!(
                            wake.count.load(Ordering::Relaxed),
                            0,
                            "admission must wait for an external event"
                        );
                        let mut bytes = [0u8; 1];
                        let mut buf = ReadBuf::new(&mut bytes);
                        match Pin::new(&mut surplus).poll_read(cx, &mut buf) {
                            Poll::Ready(Ok(())) if buf.filled().is_empty() => Poll::Ready(()),
                            Poll::Ready(result) => {
                                panic!("surplus connection was not closed: {result:?}")
                            }
                            Poll::Pending => Poll::Pending,
                        }
                    }),
                )
                .await
                .unwrap();
                assert_eq!(incoming.permits.available_permits(), 0);
                drop(first);
                let next_client = TcpStream::connect(address).await.unwrap();
                let next = tokio::time::timeout(
                    Duration::from_secs(2),
                    poll_fn(|cx| Pin::new(&mut incoming).poll_next(cx)),
                )
                .await
                .unwrap()
                .unwrap()
                .unwrap();
                assert_eq!(incoming.permits.available_permits(), 0);
                drop(next);
                assert_eq!(incoming.permits.available_permits(), 1);
                drop((first_client, surplus, next_client));
            });
    }

    #[test]
    fn credential_comparison_rejects_length_padding_and_content_mismatches() {
        let token = AccessToken::new("test-only-read-credential".into(), Permission::Read).unwrap();
        let credential = Credential::from(token);
        assert!(credential.matches(b"test-only-read-credential"));
        for candidate in [
            &b""[..],
            &b"test-only-read-credentiaL"[..],
            &b"test-only-read-credentia"[..],
            &b"test-only-read-credential!"[..],
            &b"test-only-read-credential\0"[..],
            &[b'x'; 257][..],
        ] {
            assert!(!credential.matches(candidate));
        }
        let maximum =
            Credential::from(AccessToken::new("x".repeat(256), Permission::Manage).unwrap());
        assert!(maximum.matches(&[b'x'; 256]));
        assert!(!maximum.matches(&[b'x'; 255]));
        let minimum = Credential::from(AccessToken::new("x".repeat(16), Permission::Read).unwrap());
        assert!(minimum.matches(&[b'x'; 16]));
    }
}
