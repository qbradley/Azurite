use std::{
    fmt,
    ops::{Deref, DerefMut},
};

use azurite_common::{
    configuration_base::{AccessLogWriteStream, ConfigurationBase},
    persistence::{
        i_extent_store::StoreDestinationArray, memory_extent_store::MemoryExtentChunkStore,
    },
};

use crate::utils::constants::{
    DEFAULT_BLOB_EXTENT_LOKI_DB_PATH, DEFAULT_BLOB_KEEP_ALIVE_TIMEOUT, DEFAULT_BLOB_LISTENING_PORT,
    DEFAULT_BLOB_LOKI_DB_PATH, DEFAULT_BLOB_PERSISTENCE_ARRAY, DEFAULT_BLOB_SERVER_HOST_NAME,
    DEFAULT_ENABLE_ACCESS_LOG, DEFAULT_ENABLE_DEBUG_LOG,
};

#[allow(non_snake_case)]
pub struct BlobConfiguration {
    pub base: ConfigurationBase,
    pub metadataDBPath: String,
    pub extentDBPath: String,
    pub persistencePathArray: StoreDestinationArray,
    pub isMemoryPersistence: bool,
    pub memoryStore: Option<MemoryExtentChunkStore>,
}

impl BlobConfiguration {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host: String,
        port: u16,
        keepAliveTimeout: u64,
        metadataDBPath: String,
        extentDBPath: String,
        persistencePathArray: StoreDestinationArray,
        enableAccessLog: bool,
        accessLogWriteStream: Option<AccessLogWriteStream>,
        enableDebugLog: bool,
        debugLogFilePath: Option<String>,
        loose: bool,
        skipApiVersionCheck: bool,
        cert: String,
        key: String,
        pwd: String,
        oauth: Option<String>,
        disableProductStyleUrl: bool,
        isMemoryPersistence: bool,
        memoryStore: Option<MemoryExtentChunkStore>,
    ) -> Self {
        Self {
            base: ConfigurationBase::new(
                host,
                port,
                keepAliveTimeout,
                enableAccessLog,
                accessLogWriteStream,
                enableDebugLog,
                debugLogFilePath,
                loose,
                skipApiVersionCheck,
                cert,
                key,
                pwd,
                oauth,
                disableProductStyleUrl,
            ),
            metadataDBPath,
            extentDBPath,
            persistencePathArray,
            isMemoryPersistence,
            memoryStore,
        }
    }
}

impl Clone for BlobConfiguration {
    fn clone(&self) -> Self {
        Self {
            base: ConfigurationBase::new(
                self.base.host.clone(),
                self.base.port,
                self.base.keepAliveTimeout,
                self.base.enableAccessLog,
                self.base.accessLogWriteStream.clone(),
                self.base.enableDebugLog,
                self.base.debugLogFilePath.clone(),
                self.base.loose,
                self.base.skipApiVersionCheck,
                self.base.cert.clone(),
                self.base.key.clone(),
                self.base.pwd.clone(),
                self.base.oauth.clone(),
                self.base.disableProductStyleUrl,
            ),
            metadataDBPath: self.metadataDBPath.clone(),
            extentDBPath: self.extentDBPath.clone(),
            persistencePathArray: self.persistencePathArray.clone(),
            isMemoryPersistence: self.isMemoryPersistence,
            memoryStore: self.memoryStore.clone(),
        }
    }
}

impl Default for BlobConfiguration {
    fn default() -> Self {
        Self::new(
            DEFAULT_BLOB_SERVER_HOST_NAME.to_string(),
            DEFAULT_BLOB_LISTENING_PORT,
            DEFAULT_BLOB_KEEP_ALIVE_TIMEOUT,
            DEFAULT_BLOB_LOKI_DB_PATH.to_string(),
            DEFAULT_BLOB_EXTENT_LOKI_DB_PATH.to_string(),
            (*DEFAULT_BLOB_PERSISTENCE_ARRAY).clone(),
            DEFAULT_ENABLE_ACCESS_LOG,
            None,
            DEFAULT_ENABLE_DEBUG_LOG,
            None,
            false,
            false,
            String::new(),
            String::new(),
            String::new(),
            None,
            false,
            false,
            None,
        )
    }
}

impl Deref for BlobConfiguration {
    type Target = ConfigurationBase;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for BlobConfiguration {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl fmt::Debug for BlobConfiguration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BlobConfiguration")
            .field("host", &self.base.host)
            .field("port", &self.base.port)
            .field("keepAliveTimeout", &self.base.keepAliveTimeout)
            .field("metadataDBPath", &self.metadataDBPath)
            .field("extentDBPath", &self.extentDBPath)
            .field("persistencePathArray", &self.persistencePathArray)
            .field("enableAccessLog", &self.base.enableAccessLog)
            .field("enableDebugLog", &self.base.enableDebugLog)
            .field("debugLogFilePath", &self.base.debugLogFilePath)
            .field("loose", &self.base.loose)
            .field("skipApiVersionCheck", &self.base.skipApiVersionCheck)
            .field("oauth", &self.base.oauth)
            .field("disableProductStyleUrl", &self.base.disableProductStyleUrl)
            .field("isMemoryPersistence", &self.isMemoryPersistence)
            .finish()
    }
}
