// Largely taken from
// https://github.com/Finomnis/tokio-graceful-shutdown/blob/ec444f6/src/future_ext.rs
use std::error::Error;
use std::fmt::Display;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use futures::Future;
use pin_project_lite::pin_project;

#[derive(Debug)]
pub struct CancelledByShutdown;

impl Display for CancelledByShutdown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("A shutdown request caused this task to be cancelled")
    }
}

impl Error for CancelledByShutdown {}

pin_project! {
    #[must_use = "futures do nothing unless polled"]
    pub struct CancelOnShutdownFuture<F: Future, C: Future<Output=()>>{
        #[pin]
        future: F,
        #[pin]
        cancellation: C,
        #[pin]
        timeout: Option<tokio::time::Sleep>,
        timeout_duration: Duration,
        start_timeout: bool
    }
}

impl<F: Future, C: Future<Output = ()>> Future for CancelOnShutdownFuture<F, C> {
    type Output = Result<F::Output, CancelledByShutdown>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        if *this.start_timeout {
            if let Some(timeout) = this.timeout.as_pin_mut() {
                return match this.future.as_mut().poll(cx) {
                    Poll::Ready(res) => Poll::Ready(Ok(res)),
                    Poll::Pending => match timeout.poll(cx) {
                        Poll::Ready(_) => Poll::Ready(Err(CancelledByShutdown)),
                        Poll::Pending => Poll::Pending,
                    },
                };
            } else {
                return Poll::Ready(Err(CancelledByShutdown));
            }
        }

        let mut should_wake = false;
        match this.cancellation.as_mut().poll(cx) {
            Poll::Ready(()) => {
                if let Some(timeout) = this.timeout.as_pin_mut() {
                    timeout.reset(tokio::time::Instant::now() + *this.timeout_duration);
                    *this.start_timeout = true;
                    should_wake = true;
                } else {
                    return Poll::Ready(Err(CancelledByShutdown));
                }
            }
            Poll::Pending => {}
        }

        match this.future.as_mut().poll(cx) {
            Poll::Ready(res) => Poll::Ready(Ok(res)),
            Poll::Pending => {
                if should_wake {
                    cx.waker().wake_by_ref();
                }
                Poll::Pending
            }
        }
    }
}

pub trait FutureExt {
    type Future: Future;

    fn cancel_with<F: Future<Output = ()>>(
        self,
        cancellation_token: F,
    ) -> CancelOnShutdownFuture<Self::Future, F>;

    fn cancel_with_timeout<F: Future<Output = ()>>(
        self,
        cancellation_token: F,
        timeout: Duration,
    ) -> CancelOnShutdownFuture<Self::Future, F>;

    #[cfg(feature = "cancellation-token")]
    fn cancel_on_shutdown(
        self,
        cancellation_token: &tokio_util::sync::CancellationToken,
    ) -> CancelOnShutdownFuture<Self::Future, tokio_util::sync::WaitForCancellationFuture<'_>>;

    #[cfg(feature = "cancellation-token")]
    fn cancel_on_shutdown_with_timeout(
        self,
        cancellation_token: &tokio_util::sync::CancellationToken,
        timeout: Duration,
    ) -> CancelOnShutdownFuture<Self::Future, tokio_util::sync::WaitForCancellationFuture<'_>>;
}

impl<F: Future> FutureExt for F {
    type Future = F;

    fn cancel_with<C: Future<Output = ()>>(
        self,
        cancellation: C,
    ) -> CancelOnShutdownFuture<Self::Future, C> {
        CancelOnShutdownFuture {
            future: self,
            cancellation,
            timeout: None,
            timeout_duration: Duration::default(),
            start_timeout: false,
        }
    }

    fn cancel_with_timeout<C: Future<Output = ()>>(
        self,
        cancellation: C,
        timeout: Duration,
    ) -> CancelOnShutdownFuture<Self::Future, C> {
        CancelOnShutdownFuture {
            future: self,
            cancellation,
            timeout: Some(tokio::time::sleep(timeout)),
            timeout_duration: timeout,
            start_timeout: false,
        }
    }

    #[cfg(feature = "cancellation-token")]
    fn cancel_on_shutdown(
        self,
        cancellation_token: &tokio_util::sync::CancellationToken,
    ) -> CancelOnShutdownFuture<Self::Future, tokio_util::sync::WaitForCancellationFuture<'_>> {
        let cancellation = cancellation_token.cancelled();
        CancelOnShutdownFuture {
            future: self,
            cancellation,
            timeout: None,
            timeout_duration: Duration::default(),
            start_timeout: false,
        }
    }

    #[cfg(feature = "cancellation-token")]
    fn cancel_on_shutdown_with_timeout(
        self,
        cancellation_token: &tokio_util::sync::CancellationToken,
        timeout: Duration,
    ) -> CancelOnShutdownFuture<Self::Future, tokio_util::sync::WaitForCancellationFuture<'_>> {
        let cancellation = cancellation_token.cancelled();
        CancelOnShutdownFuture {
            future: self,
            cancellation,
            timeout: Some(tokio::time::sleep(timeout)),
            timeout_duration: timeout,
            start_timeout: false,
        }
    }
}
