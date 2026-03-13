use async_trait::async_trait;

use crate::storage_error::StorageError;

#[allow(non_snake_case)]
#[async_trait]
pub trait ICleaner: Send + Sync {
    async fn clean(&mut self) -> Result<(), StorageError>;
}
