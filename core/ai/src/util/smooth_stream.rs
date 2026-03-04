//! Smooth streaming — throttled delivery of text chunks for smoother UI output.
//!
//! Instead of emitting text in bursts (which looks jarring in a UI), the smooth
//! stream adapter buffers incoming text and re-emits it in smaller chunks with
//! an artificial delay between them.
//!
//! # Example
//! ```ignore
//! use ararajuba_core::util::smooth_stream::{smooth_stream, SmoothStreamOptions, Chunking};
//! use std::time::Duration;
//!
//! let smoothed = smooth_stream(raw_text_stream, SmoothStreamOptions {
//!     delay: Duration::from_millis(10),
//!     chunking: Chunking::Word,
//! });
//! ```

use crate::error::Error;
use futures::stream::{BoxStream, StreamExt};
use std::time::Duration;

/// How to split buffered text into chunks.
#[derive(Debug, Clone)]
pub enum Chunking {
    /// Emit one word at a time (split on whitespace boundaries).
    Word,
    /// Emit one line at a time (split on newlines).
    Line,
    /// Emit one character at a time.
    Character,
    /// Emit chunks matching a custom regex pattern.
    ///
    /// The regex should match the chunk to emit (including trailing whitespace
    /// if desired). For example, `r"\S+\s*"` behaves like `Word`.
    Regex(String),
}

/// Options for smooth streaming.
#[derive(Debug, Clone)]
pub struct SmoothStreamOptions {
    /// Delay between emitted chunks (default: 10ms).
    pub delay: Duration,
    /// Chunking strategy (default: `Word`).
    pub chunking: Chunking,
}

impl Default for SmoothStreamOptions {
    fn default() -> Self {
        Self {
            delay: Duration::from_millis(10),
            chunking: Chunking::Word,
        }
    }
}

/// Wrap a text stream to produce smoother output.
///
/// The returned stream buffers incoming text deltas and re-emits them in
/// smaller chunks (word, line, or character) with an artificial delay between
/// each chunk.
pub fn smooth_stream(
    input: BoxStream<'static, Result<String, Error>>,
    options: SmoothStreamOptions,
) -> BoxStream<'static, Result<String, Error>> {
    let delay = options.delay;
    let chunking = options.chunking;

    let stream = futures::stream::unfold(
        SmoothState {
            input,
            buffer: String::new(),
            chunking,
            delay,
            done: false,
        },
        |mut state| async move {
            loop {
                // Try to extract a chunk from the buffer first.
                if let Some(chunk) = extract_chunk(&mut state.buffer, &state.chunking) {
                    if !state.delay.is_zero() {
                        tokio::time::sleep(state.delay).await;
                    }
                    return Some((Ok(chunk), state));
                }

                if state.done {
                    // Flush remaining buffer.
                    if !state.buffer.is_empty() {
                        let rest = std::mem::take(&mut state.buffer);
                        return Some((Ok(rest), state));
                    }
                    return None;
                }

                // Pull more data from the upstream.
                match state.input.next().await {
                    Some(Ok(delta)) => {
                        state.buffer.push_str(&delta);
                        // Continue the loop — will try to extract a chunk.
                    }
                    Some(Err(e)) => {
                        state.done = true;
                        return Some((Err(e), state));
                    }
                    None => {
                        state.done = true;
                        // Continue — will flush the buffer.
                    }
                }
            }
        },
    );

    Box::pin(stream)
}

struct SmoothState {
    input: BoxStream<'static, Result<String, Error>>,
    buffer: String,
    chunking: Chunking,
    delay: Duration,
    done: bool,
}

/// Extract the next chunk from the buffer according to the chunking strategy.
///
/// Returns `Some(chunk)` if a complete chunk was found, removing it from the
/// buffer. Returns `None` if the buffer doesn't contain a complete chunk yet.
fn extract_chunk(buffer: &mut String, chunking: &Chunking) -> Option<String> {
    if buffer.is_empty() {
        return None;
    }

    match chunking {
        Chunking::Character => {
            let ch = buffer.remove(0);
            Some(ch.to_string())
        }
        Chunking::Word => {
            // Find the end of the first word (non-whitespace followed by whitespace).
            let bytes = buffer.as_bytes();
            let mut i = 0;
            // Skip leading whitespace
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            // Skip non-whitespace (the word)
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            // Include trailing whitespace
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i == 0 {
                return None;
            }
            // Only emit if we found at least one complete word+whitespace boundary,
            // or if the buffer has no more whitespace coming.
            // We require trailing whitespace to confirm the word is complete.
            let has_trailing_ws = i > 0 && buffer.as_bytes()[i - 1].is_ascii_whitespace();
            if has_trailing_ws {
                let chunk = buffer[..i].to_string();
                buffer.drain(..i);
                Some(chunk)
            } else {
                // Word is not complete yet — wait for more input.
                None
            }
        }
        Chunking::Line => {
            if let Some(pos) = buffer.find('\n') {
                let chunk = buffer[..=pos].to_string();
                buffer.drain(..=pos);
                Some(chunk)
            } else {
                None
            }
        }
        Chunking::Regex(pattern) => {
            // Use a simple find approach. In production, you'd compile the regex once.
            // For now, we compile per-call (acceptable for streaming with delays).
            // Note: we treat this as find-from-start.
            if let Ok(re) = regex_lite::Regex::new(pattern) {
                if let Some(m) = re.find(buffer) {
                    if m.start() == 0 {
                        let chunk = m.as_str().to_string();
                        buffer.drain(..m.end());
                        return Some(chunk);
                    }
                }
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_word() {
        let mut buf = "hello world ".to_string();
        assert_eq!(extract_chunk(&mut buf, &Chunking::Word), Some("hello ".to_string()));
        assert_eq!(buf, "world ");
        assert_eq!(extract_chunk(&mut buf, &Chunking::Word), Some("world ".to_string()));
        assert_eq!(buf, "");
    }

    #[test]
    fn test_extract_word_incomplete() {
        let mut buf = "hello".to_string();
        // No trailing whitespace — incomplete word.
        assert_eq!(extract_chunk(&mut buf, &Chunking::Word), None);
    }

    #[test]
    fn test_extract_character() {
        let mut buf = "abc".to_string();
        assert_eq!(extract_chunk(&mut buf, &Chunking::Character), Some("a".to_string()));
        assert_eq!(buf, "bc");
    }

    #[test]
    fn test_extract_line() {
        let mut buf = "first line\nsecond".to_string();
        assert_eq!(extract_chunk(&mut buf, &Chunking::Line), Some("first line\n".to_string()));
        assert_eq!(buf, "second");
        assert_eq!(extract_chunk(&mut buf, &Chunking::Line), None);
    }
}
