use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Cooperative cancellation shared by UI, CLI and general-path workers.
/// Cancellation does not claim that already running host or OS IO has stopped.
#[derive(Clone, Default)]
pub struct Cancellation {
    flag: Arc<AtomicBool>,
    #[cfg(feature = "remote")]
    notify: Arc<tokio::sync::Notify>,
}
impl Cancellation {
    /// Initially uncancelled token.
    pub fn new() -> Self {
        Self::default()
    }
    /// Request cancellation and wake asynchronous client operations.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Release);
        #[cfg(feature = "remote")]
        self.notify.notify_waiters();
    }
    /// Whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }
    /// Borrow the same flag for synchronous core query/replay operations.
    pub fn as_atomic(&self) -> &AtomicBool {
        &self.flag
    }
    #[cfg(feature = "remote")]
    pub(crate) async fn guard<T>(
        &self,
        timeout: std::time::Duration,
        future: impl std::future::Future<Output = Result<T, crate::ClientError>>,
    ) -> Result<T, crate::ClientError> {
        use std::{future::Future, task::Poll};
        let mut future = std::pin::pin!(future);
        let mut timer = std::pin::pin!(tokio::time::sleep(timeout));
        let mut notify = std::pin::pin!(self.notify.notified());
        std::future::poll_fn(|cx| {
            // Register before checking the flag, avoiding cancellation lost-wakes.
            let _ = notify.as_mut().poll(cx);
            if self.is_cancelled() {
                return Poll::Ready(Err(crate::ClientError::Cancelled));
            }
            if timer.as_mut().poll(cx).is_ready() {
                return Poll::Ready(Err(crate::ClientError::Remote(
                    dot_logger::adapters::remote::RemoteError::TimedOut,
                )));
            }
            future.as_mut().poll(cx)
        })
        .await
    }
}
