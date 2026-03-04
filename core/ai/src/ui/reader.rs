//! Read a UI message stream — parse SSE events back into `UIMessageChunk`s.
//!
//! This is the client-side counterpart of `chunks_to_sse`. It reads a stream
//! of SSE-formatted strings and yields `UIMessageChunk` values.

use super::chunk::UIMessageChunk;
use crate::error::Error;
use futures::stream::{BoxStream, StreamExt};

/// Read an SSE text stream and parse each event into a `UIMessageChunk`.
///
/// Each SSE event is expected to be in the format `data: <json>\n\n`.
/// Lines that don't start with `data: ` are ignored (e.g., comments, empty lines).
///
/// # Example
/// ```ignore
/// use ararajuba_core::ui::reader::read_ui_message_stream;
///
/// let chunk_stream = read_ui_message_stream(sse_text_stream);
/// ```
pub fn read_ui_message_stream(
    input: BoxStream<'static, Result<String, Error>>,
) -> BoxStream<'static, Result<UIMessageChunk, Error>> {
    let stream = input.filter_map(|line_result| async move {
        match line_result {
            Err(e) => Some(Err(e)),
            Ok(line) => {
                let trimmed = line.trim();
                // Skip empty lines, comments, and non-data lines.
                if let Some(json_str) = trimmed.strip_prefix("data: ") {
                    match serde_json::from_str::<UIMessageChunk>(json_str) {
                        Ok(chunk) => Some(Ok(chunk)),
                        Err(e) => Some(Err(Error::InvalidStreamPart {
                            message: format!("Failed to parse UI message chunk: {e}"),
                        })),
                    }
                } else {
                    None // Skip non-data lines
                }
            }
        }
    });

    Box::pin(stream)
}

/// A convenience function that splits a raw byte/text stream on `\n\n`
/// boundaries and then parses each SSE event.
///
/// This handles the case where the input stream yields arbitrary chunks
/// of text that may not be aligned on event boundaries.
pub fn split_sse_and_parse(
    input: BoxStream<'static, Result<String, Error>>,
) -> BoxStream<'static, Result<UIMessageChunk, Error>> {
    let line_stream = sse_line_splitter(input);
    read_ui_message_stream(line_stream)
}

/// Split a text stream on `\n\n` boundaries, yielding complete SSE events.
fn sse_line_splitter(
    input: BoxStream<'static, Result<String, Error>>,
) -> BoxStream<'static, Result<String, Error>> {
    let stream = futures::stream::unfold(
        (input, String::new()),
        |(mut input, mut buffer)| async move {
            loop {
                // Check if buffer contains a complete event.
                if let Some(pos) = buffer.find("\n\n") {
                    let event = buffer[..pos].to_string();
                    buffer = buffer[pos + 2..].to_string();
                    return Some((Ok(event), (input, buffer)));
                }

                // Pull more data.
                match input.next().await {
                    Some(Ok(chunk)) => {
                        buffer.push_str(&chunk);
                    }
                    Some(Err(e)) => {
                        return Some((Err(e), (input, buffer)));
                    }
                    None => {
                        // End of stream. Emit remaining buffer if non-empty.
                        if !buffer.trim().is_empty() {
                            let remaining = std::mem::take(&mut buffer);
                            return Some((Ok(remaining), (input, buffer)));
                        }
                        return None;
                    }
                }
            }
        },
    );

    Box::pin(stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    #[tokio::test]
    async fn test_read_ui_message_stream() {
        let events = vec![
            Ok("data: {\"type\":\"start\",\"message_id\":\"m1\"}\n\n".to_string()),
            Ok("data: {\"type\":\"text-delta\",\"id\":\"t1\",\"delta\":\"Hi\"}\n\n".to_string()),
        ];
        let input: BoxStream<'static, Result<String, Error>> =
            Box::pin(stream::iter(events));

        let chunks: Vec<_> = read_ui_message_stream(input).collect().await;
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].is_ok());
        assert!(chunks[1].is_ok());
    }

    #[tokio::test]
    async fn test_read_ignores_non_data_lines() {
        let events = vec![
            Ok(": comment\n\n".to_string()),
            Ok("data: {\"type\":\"start\",\"message_id\":\"m1\"}\n\n".to_string()),
            Ok("\n".to_string()),
        ];
        let input: BoxStream<'static, Result<String, Error>> =
            Box::pin(stream::iter(events));

        let chunks: Vec<_> = read_ui_message_stream(input).collect().await;
        assert_eq!(chunks.len(), 1);
    }

    #[tokio::test]
    async fn test_read_invalid_json() {
        let events = vec![Ok("data: not-json\n\n".to_string())];
        let input: BoxStream<'static, Result<String, Error>> =
            Box::pin(stream::iter(events));

        let chunks: Vec<_> = read_ui_message_stream(input).collect().await;
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].is_err());
    }
}
