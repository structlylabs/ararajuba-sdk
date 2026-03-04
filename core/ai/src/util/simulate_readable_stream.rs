//! Simulate a readable stream — create an async stream from an iterator
//! with optional delays between items.

use futures::stream::BoxStream;
use std::time::Duration;

/// Options for creating a simulated readable stream.
pub struct SimulateReadableStreamOptions<T> {
    /// The items to emit.
    pub values: Vec<T>,
    /// Delay between items (default: no delay).
    pub delay: Option<Duration>,
    /// Delay before the first item (default: no delay).
    pub initial_delay: Option<Duration>,
}

/// Create a simulated readable stream from a list of values.
///
/// Useful for testing and development when you want to simulate
/// a streaming response from a model.
///
/// # Example
/// ```ignore
/// use ararajuba_core::util::simulate_readable_stream::{simulate_readable_stream, SimulateReadableStreamOptions};
/// use std::time::Duration;
///
/// let stream = simulate_readable_stream(SimulateReadableStreamOptions {
///     values: vec!["Hello", " ", "World"],
///     delay: Some(Duration::from_millis(50)),
///     initial_delay: None,
/// });
/// ```
pub fn simulate_readable_stream<T: Send + 'static>(
    options: SimulateReadableStreamOptions<T>,
) -> BoxStream<'static, T> {
    let SimulateReadableStreamOptions {
        values,
        delay,
        initial_delay,
    } = options;

    let stream = async_stream::stream! {
        if let Some(d) = initial_delay {
            tokio::time::sleep(d).await;
        }

        for value in values {
            if let Some(d) = delay {
                tokio::time::sleep(d).await;
            }
            yield value;
        }
    };

    Box::pin(stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_simulate_stream_values() {
        let stream = simulate_readable_stream(SimulateReadableStreamOptions {
            values: vec![1, 2, 3],
            delay: None,
            initial_delay: None,
        });

        let items: Vec<i32> = stream.collect().await;
        assert_eq!(items, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_simulate_stream_empty() {
        let stream = simulate_readable_stream(SimulateReadableStreamOptions::<i32> {
            values: vec![],
            delay: None,
            initial_delay: None,
        });

        let items: Vec<i32> = stream.collect().await;
        assert!(items.is_empty());
    }
}
