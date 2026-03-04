//! Retry logic for transient HTTP errors.

use ararajuba_provider::errors::Error;
use futures::future::BoxFuture;
use std::time::Duration;

/// Configuration for automatic retry of transient errors.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retries (0 = no retries).
    pub max_retries: u32,
    /// Initial delay before the first retry.
    pub initial_delay: Duration,
    /// Multiplicative backoff factor.
    pub backoff_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 2,
            initial_delay: Duration::from_millis(500),
            backoff_factor: 2.0,
        }
    }
}

/// Determine if an error is retryable.
fn is_retryable(error: &Error) -> bool {
    match error {
        Error::ApiCallError { is_retryable, .. } => *is_retryable,
        Error::Http { .. } => true, // Network errors are generally retryable
        _ => false,
    }
}

/// Execute an async operation with retry on transient errors.
///
/// `make_attempt` is a closure that returns a new future each invocation.
pub async fn with_retry<T, F>(config: RetryConfig, make_attempt: F) -> Result<T, Error>
where
    F: Fn() -> BoxFuture<'static, Result<T, Error>>,
{
    let mut last_error: Option<Error> = None;
    let mut delay = config.initial_delay;

    for attempt in 0..=config.max_retries {
        if attempt > 0 {
            tracing::debug!(
                attempt,
                delay_ms = delay.as_millis() as u64,
                "Retrying after transient error"
            );
        }
        match make_attempt().await {
            Ok(value) => return Ok(value),
            Err(error) => {
                if attempt < config.max_retries && is_retryable(&error) {
                    tracing::warn!(
                        attempt,
                        max_retries = config.max_retries,
                        error = %error,
                        "Transient error, will retry"
                    );
                    last_error = Some(error);
                    tokio::time::sleep(delay).await;
                    delay = Duration::from_secs_f64(delay.as_secs_f64() * config.backoff_factor);
                } else {
                    return Err(error);
                }
            }
        }
    }

    Err(last_error.unwrap_or(Error::Other {
        message: "Retry exhausted with no error".to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_no_retry_on_success() {
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = Arc::clone(&attempts);

        let result = with_retry(RetryConfig::default(), move || {
            let attempts = Arc::clone(&attempts_clone);
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                Ok::<_, Error>(42)
            })
        })
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_on_retryable_error() {
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = Arc::clone(&attempts);

        let config = RetryConfig {
            max_retries: 2,
            initial_delay: Duration::from_millis(10),
            backoff_factor: 1.0,
        };

        let result = with_retry(config, move || {
            let attempts = Arc::clone(&attempts_clone);
            Box::pin(async move {
                let n = attempts.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    Err(Error::ApiCallError {
                        message: "Transient".to_string(),
                        url: "http://test".to_string(),
                        status_code: Some(429),
                        response_body: None,
                        is_retryable: true,
                        data: None,
                    })
                } else {
                    Ok(99)
                }
            })
        })
        .await;

        assert_eq!(result.unwrap(), 99);
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_no_retry_on_non_retryable() {
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = Arc::clone(&attempts);

        let result: Result<i32, Error> = with_retry(RetryConfig::default(), move || {
            let attempts = Arc::clone(&attempts_clone);
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err(Error::InvalidPrompt {
                    message: "Bad prompt".to_string(),
                })
            })
        })
        .await;

        assert!(result.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }
}
