// Largely taken from https://github.com/Finomnis/tokio-graceful-shutdown/blob/ec444f69e884d27a48bef7ad88abe91b9ab7a648/src/future_ext.rs
use futures::Future;
use pin_project_lite::pin_project;
use std::{
    error::Error,
    fmt::Display,
    pin::Pin,
    task::{Context, Poll},
};

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
    pub struct CancelOnShutdownFuture< F: Future, C: Future<Output=()>>{
        #[pin]
        future: F,
        #[pin]
        cancellation: C,
    }
}

impl<F: Future, C: Future<Output = ()>> Future for CancelOnShutdownFuture<F, C> {
    type Output = Result<F::Output, CancelledByShutdown>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        match this.cancellation.as_mut().poll(cx) {
            Poll::Ready(()) => return Poll::Ready(Err(CancelledByShutdown)),
            Poll::Pending => (),
        }

        match this.future.as_mut().poll(cx) {
            Poll::Ready(res) => Poll::Ready(Ok(res)),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub trait FutureExt {
    type Future: Future;

    fn cancel_with<F: Future<Output = ()>>(
        self,
        cancellation_token: F,
    ) -> CancelOnShutdownFuture<Self::Future, F>;

    #[cfg(feature = "cancellation-token")]
    fn cancel_on_shutdown(
        self,
        cancellation_token: &tokio_util::sync::CancellationToken,
    ) -> CancelOnShutdownFuture<Self::Future, tokio_util::sync::WaitForCancellationFuture>;
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
        }
    }

    #[cfg(feature = "cancellation-token")]
    fn cancel_on_shutdown(
        self,
        cancellation_token: &tokio_util::sync::CancellationToken,
    ) -> CancelOnShutdownFuture<Self::Future, tokio_util::sync::WaitForCancellationFuture> {
        let cancellation = cancellation_token.cancelled();
        CancelOnShutdownFuture {
            future: self,
            cancellation,
        }
    }
}
