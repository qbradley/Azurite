use async_trait::async_trait;

use crate::storage_error::StorageError;

#[allow(non_snake_case)]
#[async_trait]
pub trait IServerFactory: Send + Sync {
    type Server: Send + Sync;

    async fn createServer(&self) -> Result<Self::Server, StorageError>;
}
