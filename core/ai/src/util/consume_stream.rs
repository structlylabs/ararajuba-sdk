//! Consume an async stream to completion, discarding values.

use futures::stream::BoxStream;
use futures::StreamExt;

/// Consume an async stream to completion, discarding all values.
///
/// Useful when you want to trigger side effects of a stream (e.g. callbacks)
/// without collecting the output.
///
/// # Example
/// ```ignore
/// use ararajuba_core::util::consume_stream::consume_stream;
///
/// consume_stream(some_stream).await;
/// ```
pub async fn consume_stream<T>(mut stream: BoxStream<'_, T>) {
    while stream.next().await.is_some() {}
}

/// Consume a stream collecting all items into a Vec.
pub async fn collect_stream<T>(mut stream: BoxStream<'_, T>) -> Vec<T> {
    let mut items = Vec::new();
    while let Some(item) = stream.next().await {
        items.push(item);
    }
    items
}
