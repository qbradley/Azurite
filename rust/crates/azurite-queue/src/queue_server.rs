use std::{fmt, sync::Arc};

use tokio::sync::Mutex;

use azurite_common::{
    account_data_store::AccountDataStore,
    configuration_base::ConfigurationBase,
    i_cleaner::ICleaner,
    i_data_store::IDataStore,
    i_logger::ILogger,
    i_request_listener_factory::IRequestListenerFactory,
    logger::logger as global_logger,
    persistence::{
        fs_extent_store::FSExtentStore,
        i_extent_store::IExtentStore,
        loki_extent_metadata_store::LokiExtentMetadata,
        memory_extent_store::{MemoryExtentStore, SharedChunkStore},
    },
    server_base::{RequestListener, ServerBase, ServerStatus},
    storage_error::StorageError,
};

use crate::{
    handlers::base_handler::{SharedExtentStore, SharedQueueMetadataStore},
    persistence::loki_queue_metadata_store::LokiQueueMetadataStore,
    queue_configuration::QueueConfiguration,
    queue_request_listener_factory::QueueRequestListenerFactory,
};

const BEFORE_CLOSE_MESSAGE: &str = "Azurite Queue service is closing...";
const AFTER_CLOSE_MESSAGE: &str = "Azurite Queue service successfully closed";

#[derive(Default)]
struct SharedCommonLogger;

impl ILogger for SharedCommonLogger {
    fn error(&self, message: &str, contextID: Option<&str>) {
        global_logger.error(message, contextID);
    }

    fn warn(&self, message: &str, contextID: Option<&str>) {
        global_logger.warn(message, contextID);
    }

    fn info(&self, message: &str, contextID: Option<&str>) {
        global_logger.info(message, contextID);
    }

    fn verbose(&self, message: &str, contextID: Option<&str>) {
        global_logger.verbose(message, contextID);
    }

    fn debug(&self, message: &str, contextID: Option<&str>) {
        global_logger.debug(message, contextID);
    }
}

fn clone_configuration_base(base: &ConfigurationBase) -> ConfigurationBase {
    ConfigurationBase::new(
        base.host.clone(),
        base.port,
        base.keepAliveTimeout,
        base.enableAccessLog,
        base.accessLogWriteStream.clone(),
        base.enableDebugLog,
        base.debugLogFilePath.clone(),
        base.loose,
        base.skipApiVersionCheck,
        base.cert.clone(),
        base.key.clone(),
        base.pwd.clone(),
        base.oauth.clone(),
        base.disableProductStyleUrl,
    )
}

fn build_extent_store(
    configuration: &QueueConfiguration,
    extentMetadataStore: LokiExtentMetadata,
    logger: Arc<dyn ILogger + Send + Sync>,
) -> SharedExtentStore {
    let extentStore: Box<dyn IExtentStore + Send + Sync> = if configuration.isMemoryPersistence {
        Box::new(MemoryExtentStore::new(
            "queue".to_string(),
            configuration
                .memoryStore
                .clone()
                .unwrap_or_else(|| (*SharedChunkStore).clone()),
            extentMetadataStore,
            logger,
            |statusCode, storageErrorCode, storageErrorMessage, storageRequestID| {
                StorageError::new(format!(
                    "Queue extent error {statusCode} {storageErrorCode}: {storageErrorMessage} ({storageRequestID})"
                ))
            },
        ))
    } else {
        Box::new(FSExtentStore::new(
            extentMetadataStore,
            configuration.persistencePathArray.clone(),
            logger,
        ))
    };

    Arc::new(Mutex::new(extentStore))
}

#[allow(non_snake_case)]
pub struct QueueServer {
    pub configuration: QueueConfiguration,
    requestListenerFactory: Arc<dyn IRequestListenerFactory>,
    serverBase: ServerBase,
    metadataStore: LokiQueueMetadataStore,
    extentMetadataStore: LokiExtentMetadata,
    extentStore: SharedExtentStore,
}

impl Default for QueueServer {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueServer {
    pub fn new() -> Self {
        Self::withConfiguration(QueueConfiguration::default())
    }

    #[allow(non_snake_case)]
    pub fn withConfiguration(configuration: QueueConfiguration) -> Self {
        let logger: Arc<dyn ILogger + Send + Sync> = Arc::new(SharedCommonLogger);
        let metadataStore = LokiQueueMetadataStore::new(
            configuration.metadataDBPath.clone().into(),
            configuration.isMemoryPersistence,
        );
        let metadataStoreForFactory: SharedQueueMetadataStore = Arc::new(metadataStore.clone());
        let extentMetadataStore = LokiExtentMetadata::new(
            configuration.extentDBPath.clone(),
            configuration.isMemoryPersistence,
        );
        let extentStore =
            build_extent_store(&configuration, extentMetadataStore.clone(), logger.clone());
        let accountDataStore = {
            let store = AccountDataStore::new(logger);
            store.refresh();
            Arc::new(store)
        };
        let requestListenerFactory: Arc<dyn IRequestListenerFactory> =
            Arc::new(QueueRequestListenerFactory::new(
                metadataStoreForFactory,
                Arc::clone(&extentStore),
                accountDataStore,
                configuration.enableAccessLog,
                configuration.accessLogWriteStream.clone(),
                Some(configuration.skipApiVersionCheck),
                configuration.getOAuthLevel(),
                Some(configuration.disableProductStyleUrl),
            ));
        let serverBase = ServerBase::new(
            configuration.host.clone(),
            configuration.port,
            Arc::clone(&requestListenerFactory),
            clone_configuration_base(&configuration.base),
        );

        Self {
            configuration,
            requestListenerFactory,
            serverBase,
            metadataStore,
            extentMetadataStore,
            extentStore,
        }
    }

    #[allow(non_snake_case)]
    pub fn createRequestListener(&self) -> RequestListener {
        self.requestListenerFactory.createRequestListener()
    }

    #[allow(non_snake_case)]
    pub fn getHttpServerAddress(&self) -> String {
        self.serverBase.getHttpServerAddress()
    }

    #[allow(non_snake_case)]
    pub fn getStatus(&self) -> ServerStatus {
        self.serverBase.getStatus()
    }

    pub fn configuration(&self) -> &QueueConfiguration {
        &self.configuration
    }

    pub async fn start(&mut self) -> Result<(), StorageError> {
        global_logger.info(
            &format!(
                "Azurite Queue service is starting on {}:{}",
                self.configuration.host, self.configuration.port
            ),
            None,
        );
        self.metadataStore.init().await?;
        {
            let mut extentStore = self.extentStore.lock().await;
            if !extentStore.isInitialized() {
                extentStore.init().await?;
            }
        }
        self.serverBase.start().await?;
        global_logger.info(
            &format!(
                "Azurite Queue service successfully listens on {}",
                self.serverBase.getHttpServerAddress()
            ),
            None,
        );
        Ok(())
    }

    pub async fn close(&mut self) -> Result<(), StorageError> {
        global_logger.info(BEFORE_CLOSE_MESSAGE, None);
        self.serverBase.close().await?;
        {
            let mut extentStore = self.extentStore.lock().await;
            if !extentStore.isClosed() {
                extentStore.close().await?;
            }
        }
        if !self.metadataStore.isClosed() {
            self.metadataStore.close().await?;
        }
        global_logger.info(AFTER_CLOSE_MESSAGE, None);
        Ok(())
    }

    pub async fn clean(&mut self) -> Result<(), StorageError> {
        if self.getStatus() != ServerStatus::Closed {
            return Err(StorageError::new(format!(
                "Cannot clean up queue server in status {}.",
                self.getStatus()
            )));
        }

        {
            let mut extentStore = self.extentStore.lock().await;
            if !extentStore.isClosed() {
                extentStore.close().await?;
            }
            extentStore.clean().await?;
        }
        if !self.extentMetadataStore.isClosed() {
            self.extentMetadataStore.close().await?;
        }
        self.extentMetadataStore.clean().await?;
        if !self.metadataStore.isClosed() {
            self.metadataStore.close().await?;
        }
        self.metadataStore.clean().await
    }
}

impl fmt::Debug for QueueServer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QueueServer")
            .field("configuration", &self.configuration)
            .field("status", &self.getStatus())
            .finish()
    }
}
