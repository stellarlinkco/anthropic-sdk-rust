use crate::error::Error;
use futures_core::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio_util::sync::CancellationToken;

pub struct RawStream<T> {
    inner: Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>>,
    cancel: CancellationToken,
    request_id: Option<String>,
}

impl<T> RawStream<T> {
    pub(crate) fn new(
        inner: Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>>,
        cancel: CancellationToken,
        request_id: Option<String>,
    ) -> Self {
        Self {
            inner,
            cancel,
            request_id,
        }
    }

    pub fn abort(&self) {
        self.cancel.cancel();
    }

    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }
}

impl<T> Stream for RawStream<T> {
    type Item = Result<T, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().inner.as_mut().poll_next(cx)
    }
}
