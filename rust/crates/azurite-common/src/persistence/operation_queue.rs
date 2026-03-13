use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::{i_logger::ILogger, storage_error::StorageError};

use super::i_operation_queue::IOperationQueue;

#[derive(Debug, Default)]
struct NoopLogger;

impl ILogger for NoopLogger {
    fn error(&self, _message: &str, _contextID: Option<&str>) {}
    fn warn(&self, _message: &str, _contextID: Option<&str>) {}
    fn info(&self, _message: &str, _contextID: Option<&str>) {}
    fn verbose(&self, _message: &str, _contextID: Option<&str>) {}
    fn debug(&self, _message: &str, _contextID: Option<&str>) {}
}

pub struct OperationQueue {
    maxConcurrency: usize,
    logger: Arc<dyn ILogger + Send + Sync>,
    semaphore: Arc<Semaphore>,
}

impl Clone for OperationQueue {
    fn clone(&self) -> Self {
        Self {
            maxConcurrency: self.maxConcurrency,
            logger: self.logger.clone(),
            semaphore: self.semaphore.clone(),
        }
    }
}

impl OperationQueue {
    pub fn new() -> Self {
        Self::withMaxConcurrency(1, Arc::new(NoopLogger))
    }

    pub fn withMaxConcurrency(
        maxConcurrency: usize,
        logger: Arc<dyn ILogger + Send + Sync>,
    ) -> Self {
        Self {
            maxConcurrency,
            logger,
            semaphore: Arc::new(Semaphore::new(maxConcurrency)),
        }
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl IOperationQueue for OperationQueue {
    async fn operate<T, F, Fut>(&self, op: F, contextId: Option<&str>) -> Result<T, StorageError>
    where
        T: Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, StorageError>> + Send + 'static,
    {
        let id = Uuid::new_v4().to_string();
        self.logger.debug(
            &format!("OperationQueue.operate() Schedule incoming job {id}"),
            contextId,
        );

        let permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|error| StorageError::new(error.to_string()))?;
        let runningConcurrency = self
            .maxConcurrency
            .saturating_sub(self.semaphore.available_permits());
        self.logger.debug(
            &format!(
                "OperationQueue:execute() Current runningConcurrency:{runningConcurrency} maxConcurrency:{}",
                self.maxConcurrency
            ),
            contextId,
        );

        let result = op().await;
        drop(permit);
        tokio::task::yield_now().await;

        match result {
            Ok(value) => {
                self.logger.debug(
                    &format!("OperationQueue.operate() Job {id} completes callback, resolve."),
                    contextId,
                );
                Ok(value)
            }
            Err(error) => {
                self.logger.debug(
                    &format!("OperationQueue.operate() Job {id} error, reject."),
                    contextId,
                );
                Err(error)
            }
        }
    }
}
