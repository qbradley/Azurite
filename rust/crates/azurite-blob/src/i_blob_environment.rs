use async_trait::async_trait;
use azurite_common::storage_error::StorageError;

#[allow(non_snake_case)]
#[async_trait]
pub trait IBlobEnvironment: Send + Sync {
    fn blobHost(&self) -> Option<String>;
    fn blobPort(&self) -> Option<u16>;
    fn blobKeepAliveTimeout(&self) -> Option<u64>;
    async fn location(&self) -> Result<String, StorageError>;
    fn silent(&self) -> bool;
    fn loose(&self) -> bool;
    fn skipApiVersionCheck(&self) -> bool;
    fn cert(&self) -> Option<String>;
    fn key(&self) -> Option<String>;
    fn pwd(&self) -> Option<String>;
    async fn debug(&self) -> Result<Option<String>, StorageError>;
    fn oauth(&self) -> Option<String>;
    fn disableProductStyleUrl(&self) -> bool;
    fn inMemoryPersistence(&self) -> bool;
    fn extentMemoryLimit(&self) -> Option<f64>;
    fn disableTelemetry(&self) -> bool;
}
