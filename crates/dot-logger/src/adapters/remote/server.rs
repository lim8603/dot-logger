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
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::{TcpListener, TcpStream},
    sync::{OwnedSemaphorePermit, Semaphore},
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
    tokens: Vec<AccessToken>,
}
fn equal(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |d, (a, b)| d | (a ^ b)) == 0
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
                .find(|t| equal(t.value.as_bytes(), value.as_bytes()))
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
        RemoteError::InvalidInput | RemoteError::Corrupt => Status::invalid_argument(e.to_string()),
        RemoteError::Limit => Status::resource_exhausted(e.to_string()),
        RemoteError::PermissionDenied => Status::permission_denied(e.to_string()),
        RemoteError::TimedOut => Status::deadline_exceeded(e.to_string()),
        RemoteError::Unsupported => Status::unimplemented(e.to_string()),
        RemoteError::Gone => Status::not_found(e.to_string()),
        _ => Status::unavailable(e.to_string()),
    }
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
            tokens: config.tokens,
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
    pub async fn serve(self, shutdown: impl Future<Output = ()> + Send) -> Result<(), RemoteError> {
        struct Close(RemoteHub);
        impl Drop for Close {
            fn drop(&mut self) {
                self.0.close();
            }
        }
        let _close = Close(self.hub.clone());
        let limits = &self.hub.shared.limits;
        let incoming = Incoming {
            listener: self.listener,
            permits: Arc::new(Semaphore::new(limits.connections)),
        };
        let service = RpcService {
            hub: self.hub.clone(),
            security: self.security,
        };
        let close_hub = self.hub.clone();
        self.builder
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
            .map_err(|_| RemoteError::Transport)
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
}
impl Stream for Incoming {
    type Item = Result<LimitedIo, std::io::Error>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.listener.poll_accept(cx) {
            Poll::Ready(Ok((stream, _))) => match self.permits.clone().try_acquire_owned() {
                Ok(permit) => Poll::Ready(Some(Ok(LimitedIo {
                    stream,
                    _permit: permit,
                }))),
                Err(_) => {
                    drop(stream);
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            },
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}
#[derive(Clone)]
struct RpcService {
    hub: RemoteHub,
    security: Arc<Security>,
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
                    | "ApplyConfig" | "RunCheck" | "Compare"),
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
