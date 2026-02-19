use crate::error::Error;
use crate::streaming::RawStream;
use bytes::Bytes;
use futures_util::stream::BoxStream;
use futures_util::StreamExt;
use serde::de::DeserializeOwned;
use tokio_util::sync::CancellationToken;

pub(crate) fn jsonl_stream_from_response<T>(
    response: reqwest::Response,
    cancel: CancellationToken,
    request_id: Option<String>,
) -> RawStream<T>
where
    T: DeserializeOwned + Send + 'static,
{
    let bytes_stream: BoxStream<'static, Result<Bytes, reqwest::Error>> =
        Box::pin(response.bytes_stream());
    let cancel_for_stream = cancel.clone();

    let stream = futures_util::stream::unfold(
        (bytes_stream, Vec::<u8>::new(), cancel_for_stream),
        move |(mut bytes_stream, mut buf, cancel)| async move {
            loop {
                if let Some(pos) = buf.iter().position(|b| *b == b'\n') {
                    let mut line = buf.drain(..=pos).collect::<Vec<u8>>();
                    if let Some(b'\n') = line.last() {
                        line.pop();
                    }
                    if let Some(b'\r') = line.last() {
                        line.pop();
                    }
                    if line.is_empty() {
                        continue;
                    }

                    let item = match serde_json::from_slice::<T>(&line) {
                        Ok(v) => v,
                        Err(e) => return Some((Err(Error::Json(e)), (bytes_stream, buf, cancel))),
                    };
                    return Some((Ok(item), (bytes_stream, buf, cancel)));
                }

                let next = tokio::select! {
                  _ = cancel.cancelled() => return None,
                  next = bytes_stream.next() => next,
                };

                match next {
                    Some(Ok(chunk)) => buf.extend_from_slice(&chunk),
                    Some(Err(e)) => {
                        return Some((Err(Error::Transport(e)), (bytes_stream, buf, cancel)))
                    }
                    None => {
                        if buf.is_empty() {
                            return None;
                        }
                        let line = std::mem::take(&mut buf);
                        let item = match serde_json::from_slice::<T>(&line) {
                            Ok(v) => v,
                            Err(e) => {
                                return Some((Err(Error::Json(e)), (bytes_stream, buf, cancel)))
                            }
                        };
                        return Some((Ok(item), (bytes_stream, buf, cancel)));
                    }
                }
            }
        },
    );

    RawStream::new(Box::pin(stream), cancel, request_id)
}
