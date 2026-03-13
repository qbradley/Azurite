use async_trait::async_trait;

use crate::storage_error::StorageError;

#[allow(non_snake_case)]
#[async_trait]
pub trait IDataStore: Send + Sync {
    async fn init(&mut self) -> Result<(), StorageError>;
    fn isInitialized(&self) -> bool;
    async fn close(&mut self) -> Result<(), StorageError>;
    fn isClosed(&self) -> bool;
}
