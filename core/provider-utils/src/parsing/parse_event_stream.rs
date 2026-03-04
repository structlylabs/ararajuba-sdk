//! Server-Sent Events (SSE) stream parsing.

use bytes::Bytes;
use ararajuba_provider::errors::Error;
use futures::stream::{BoxStream, StreamExt};
use std::sync::Arc;

/// Parse a byte stream (from an HTTP response) as a Server-Sent Events stream.
///
/// Each SSE `data:` line is parsed as JSON and passed to `parse_chunk`.
/// Lines starting with `:` (comments) and `event:` are ignored.
/// `data: [DONE]` signals the end of the stream.
pub fn parse_event_stream<T, S>(
    byte_stream: S,
    parse_chunk: Arc<dyn Fn(serde_json::Value) -> Result<T, Error> + Send + Sync + 'static>,
) -> BoxStream<'static, Result<T, Error>>
where
    T: Send + 'static,
    S: futures::Stream<Item = reqwest::Result<Bytes>> + Send + 'static,
{
    let stream = byte_stream
        .map(move |chunk_result| {
            chunk_result.map_err(|e| Error::Http {
                message: e.to_string(),
            })
        })
        // Accumulate bytes into lines and parse SSE events.
        .flat_map(move |result| {
            let parse_chunk = Arc::clone(&parse_chunk);
            match result {
                Err(e) => {
                    Box::pin(futures::stream::once(async move { Err(e) }))
                        as BoxStream<'static, Result<T, Error>>
                }
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes).to_string();
                    let lines: Vec<String> =
                        text.lines().map(|l| l.to_string()).collect();

                    let items: Vec<Result<T, Error>> = lines
                        .into_iter()
                        .filter_map(move |line| {
                            let line = line.trim();
                            if line.is_empty() || line.starts_with(':') || line.starts_with("event:") {
                                return None;
                            }
                            if let Some(data) = line.strip_prefix("data:") {
                                let data = data.trim();
                                if data == "[DONE]" {
                                    return None;
                                }
                                match serde_json::from_str::<serde_json::Value>(data) {
                                    Ok(value) => Some(parse_chunk(value)),
                                    Err(e) => Some(Err(Error::JsonParse {
                                        message: e.to_string(),
                                        text: data.to_string(),
                                    })),
                                }
                            } else {
                                None
                            }
                        })
                        .collect();

                    Box::pin(futures::stream::iter(items))
                        as BoxStream<'static, Result<T, Error>>
                }
            }
        });

    Box::pin(stream)
}
