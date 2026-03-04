//! Serial job executor — executes async jobs one at a time.
//!
//! Ensures that concurrent submissions are serialized, with each job
//! waiting for the previous one to complete before starting.

use futures::future::BoxFuture;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A serial job executor that ensures only one job runs at a time.
///
/// Jobs are submitted as async closures and queued behind any currently
/// running job.
///
/// # Example
/// ```ignore
/// use ararajuba_core::util::serial_executor::SerialJobExecutor;
///
/// let executor = SerialJobExecutor::new();
/// executor.run(|| async { do_work().await }).await;
/// ```
pub struct SerialJobExecutor {
    lock: Arc<Mutex<()>>,
}

impl SerialJobExecutor {
    /// Create a new serial job executor.
    pub fn new() -> Self {
        Self {
            lock: Arc::new(Mutex::new(())),
        }
    }

    /// Run a job, waiting for any previously submitted job to complete first.
    pub async fn run<F, T>(&self, job: F) -> T
    where
        F: FnOnce() -> BoxFuture<'static, T> + Send,
        T: Send,
    {
        let _guard = self.lock.lock().await;
        job().await
    }

    /// Try to run a job immediately. Returns `None` if another job is running.
    pub async fn try_run<F, T>(&self, job: F) -> Option<T>
    where
        F: FnOnce() -> BoxFuture<'static, T> + Send,
        T: Send,
    {
        let guard = self.lock.try_lock();
        match guard {
            Ok(_guard) => Some(job().await),
            Err(_) => None,
        }
    }
}

impl Default for SerialJobExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn test_serial_execution() {
        let executor = SerialJobExecutor::new();
        let counter = Arc::new(AtomicU32::new(0));

        let c1 = Arc::clone(&counter);
        executor
            .run(|| {
                Box::pin(async move {
                    c1.fetch_add(1, Ordering::SeqCst);
                })
            })
            .await;

        let c2 = Arc::clone(&counter);
        executor
            .run(|| {
                Box::pin(async move {
                    c2.fetch_add(1, Ordering::SeqCst);
                })
            })
            .await;

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_try_run_succeeds_when_idle() {
        let executor = SerialJobExecutor::new();
        let result = executor
            .try_run(|| Box::pin(async { 42 }))
            .await;
        assert_eq!(result, Some(42));
    }
}
