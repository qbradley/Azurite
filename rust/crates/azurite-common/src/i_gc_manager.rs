use async_trait::async_trait;

use crate::storage_error::StorageError;

#[allow(non_snake_case)]
#[async_trait]
pub trait IGCManager: Send + Sync {
    async fn start(&mut self) -> Result<(), StorageError>;
    async fn close(&mut self) -> Result<(), StorageError>;
}
