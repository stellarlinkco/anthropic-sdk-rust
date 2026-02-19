use crate::error::Error;
use crate::streaming::RawStream;
use crate::types::messages::{Message, RawContentBlockDelta, RawMessageStreamEvent};
use futures_core::Stream;
use futures_util::StreamExt;
use serde_json::{Map, Value};
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct MessageStream {
    raw: RawStream<RawMessageStreamEvent>,
    snapshot: Option<Message>,
    final_message: Option<Message>,
}

impl MessageStream {
    pub(crate) fn new(raw: RawStream<RawMessageStreamEvent>) -> Self {
        Self {
            raw,
            snapshot: None,
            final_message: None,
        }
    }

    pub fn abort(&self) {
        self.raw.abort();
    }

    pub fn request_id(&self) -> Option<&str> {
        self.raw.request_id()
    }

    pub fn snapshot(&self) -> Option<&Message> {
        self.snapshot.as_ref()
    }

    pub fn final_message(&self) -> Option<&Message> {
        self.final_message.as_ref()
    }

    pub async fn into_final_message(mut self) -> Result<Message, Error> {
        while let Some(item) = self.next().await {
            item?;
        }
        self.final_message
            .ok_or_else(|| Error::InvalidSse("stream ended without a final message".to_string()))
    }

    fn snapshot_mut(&mut self) -> Result<&mut Message, Error> {
        self.snapshot.as_mut().ok_or_else(|| {
            Error::InvalidSse("expected message_start before other events".to_string())
        })
    }

    fn content_block_mut(snapshot: &mut Message, index: usize) -> Result<&mut Value, Error> {
        snapshot
            .content
            .get_mut(index)
            .ok_or_else(|| Error::InvalidSse(format!("content block index out of bounds: {index}")))
    }

    fn ensure_object(value: &mut Value) -> Result<&mut Map<String, Value>, Error> {
        value
            .as_object_mut()
            .ok_or_else(|| Error::InvalidSse("expected JSON object".to_string()))
    }

    fn append_string_field(block: &mut Value, key: &str, delta: &str) -> Result<(), Error> {
        let obj = Self::ensure_object(block)?;
        let entry = obj
            .entry(key.to_string())
            .or_insert_with(|| Value::String(String::new()));
        match entry {
            Value::String(s) => {
                s.push_str(delta);
                Ok(())
            }
            _ => Err(Error::InvalidSse(format!(
                "expected '{key}' to be a string"
            ))),
        }
    }

    fn push_citation(block: &mut Value, citation: Value) -> Result<(), Error> {
        let obj = Self::ensure_object(block)?;
        let entry = obj
            .entry("citations".to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        match entry {
            Value::Null => {
                *entry = Value::Array(vec![citation]);
                Ok(())
            }
            Value::Array(arr) => {
                arr.push(citation);
                Ok(())
            }
            _ => Err(Error::InvalidSse(
                "expected 'citations' to be null or array".to_string(),
            )),
        }
    }

    fn handle_event(&mut self, event: &RawMessageStreamEvent) -> Result<(), Error> {
        match event {
            RawMessageStreamEvent::MessageStart { message } => {
                self.snapshot = Some(message.clone());
                self.final_message = None;
            }
            RawMessageStreamEvent::ContentBlockStart {
                index,
                content_block,
            } => {
                let snapshot = self.snapshot_mut()?;
                if *index > snapshot.content.len() {
                    return Err(Error::InvalidSse(format!(
                        "content_block_start index out of bounds: {index}"
                    )));
                }
                if *index == snapshot.content.len() {
                    snapshot.content.push(content_block.clone());
                } else {
                    snapshot.content[*index] = content_block.clone();
                }
            }
            RawMessageStreamEvent::ContentBlockDelta { index, delta } => {
                let snapshot = self.snapshot_mut()?;
                let block = Self::content_block_mut(snapshot, *index)?;
                match delta {
                    RawContentBlockDelta::TextDelta { text } => {
                        Self::append_string_field(block, "text", text)?
                    }
                    RawContentBlockDelta::ThinkingDelta { thinking } => {
                        Self::append_string_field(block, "thinking", thinking)?
                    }
                    RawContentBlockDelta::SignatureDelta { signature } => {
                        Self::append_string_field(block, "signature", signature)?
                    }
                    RawContentBlockDelta::InputJsonDelta { partial_json } => {
                        let obj = Self::ensure_object(block)?;
                        obj.insert(
                            "_partial_json".to_string(),
                            Value::String(partial_json.clone()),
                        );
                    }
                    RawContentBlockDelta::CitationsDelta { citation } => {
                        Self::push_citation(block, citation.clone())?
                    }
                    RawContentBlockDelta::Unknown => {}
                }
            }
            RawMessageStreamEvent::ContentBlockStop { .. } => {}
            RawMessageStreamEvent::MessageDelta { delta, usage } => {
                let snapshot = self.snapshot_mut()?;
                if let Some(v) = &delta.stop_reason {
                    snapshot.stop_reason = Some(v.clone());
                }
                if let Some(v) = &delta.stop_sequence {
                    snapshot.stop_sequence = Some(v.clone());
                }

                // Keep untyped fields in `extra` to avoid a huge type surface in v0.1.
                snapshot.extra.insert(
                    "container".to_string(),
                    delta.container.clone().unwrap_or(Value::Null),
                );
                snapshot.usage = serde_json::to_value(usage)?;
            }
            RawMessageStreamEvent::MessageStop => {
                if let Some(snapshot) = self.snapshot.clone() {
                    self.final_message = Some(snapshot);
                }
            }
        }
        Ok(())
    }
}

impl Stream for MessageStream {
    type Item = Result<RawMessageStreamEvent, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match Pin::new(&mut this.raw).poll_next(cx) {
            Poll::Ready(Some(Ok(event))) => {
                if let Err(err) = this.handle_event(&event) {
                    return Poll::Ready(Some(Err(err)));
                }
                Poll::Ready(Some(Ok(event)))
            }
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
