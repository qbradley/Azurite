use async_trait::async_trait;
use std::future::Future;

use crate::storage_error::StorageError;

#[allow(non_snake_case)]
#[async_trait]
pub trait IOperationQueue: Send + Sync {
    async fn operate<T, F, Fut>(&self, op: F, contextId: Option<&str>) -> Result<T, StorageError>
    where
        T: Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, StorageError>> + Send + 'static;
}
