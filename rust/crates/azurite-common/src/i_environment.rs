use async_trait::async_trait;

use crate::storage_error::StorageError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DebugValue {
    String(String),
    Boolean(bool),
}

#[allow(non_snake_case)]
#[async_trait]
pub trait IEnvironment: Send + Sync {
    fn blobHost(&self) -> Option<String>;
    fn blobPort(&self) -> Option<u16>;
    fn blobKeepAliveTimeout(&self) -> Option<u64>;
    fn queueHost(&self) -> Option<String>;
    fn queuePort(&self) -> Option<u16>;
    fn queueKeepAliveTimeout(&self) -> Option<u64>;
    fn tableHost(&self) -> Option<String>;
    fn tablePort(&self) -> Option<u16>;
    fn tableKeepAliveTimeout(&self) -> Option<u64>;
    async fn location(&self) -> Result<String, StorageError>;
    fn silent(&self) -> bool;
    fn loose(&self) -> bool;
    fn skipApiVersionCheck(&self) -> bool;
    fn disableProductStyleUrl(&self) -> bool;
    fn cert(&self) -> Option<String>;
    fn key(&self) -> Option<String>;
    fn pwd(&self) -> Option<String>;
    async fn debug(&self) -> Result<Option<DebugValue>, StorageError>;
    fn oauth(&self) -> Option<String>;
    fn inMemoryPersistence(&self) -> bool;
    fn extentMemoryLimit(&self) -> Option<u64>;
    fn disableTelemetry(&self) -> bool;
}
