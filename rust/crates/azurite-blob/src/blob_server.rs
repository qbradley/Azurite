use std::{fmt, sync::Arc};

use tokio::sync::Mutex;

use azurite_common::{
    account_data_store::AccountDataStore,
    configuration_base::ConfigurationBase,
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
    blob_configuration::BlobConfiguration,
    blob_request_listener_factory::BlobRequestListenerFactory,
    handlers::base_handler::{SharedBlobMetadataStore, SharedExtentStore},
    persistence::LokiBlobMetadataStore,
};

const BEFORE_CLOSE_MESSAGE: &str = "Azurite Blob service is closing...";
const AFTER_CLOSE_MESSAGE: &str = "Azurite Blob service successfully closed";

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
    configuration: &BlobConfiguration,
    extentMetadataStore: LokiExtentMetadata,
    logger: Arc<dyn ILogger + Send + Sync>,
) -> SharedExtentStore {
    let extentStore: Box<dyn IExtentStore + Send + Sync> = if configuration.isMemoryPersistence {
        Box::new(MemoryExtentStore::new(
            "blob".to_string(),
            configuration
                .memoryStore
                .clone()
                .unwrap_or_else(|| (*SharedChunkStore).clone()),
            extentMetadataStore,
            logger,
            |statusCode, storageErrorCode, storageErrorMessage, storageRequestID| {
                StorageError::new(format!(
                    "Blob extent error {statusCode} {storageErrorCode}: {storageErrorMessage} ({storageRequestID})"
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
pub struct BlobServer {
    pub configuration: BlobConfiguration,
    requestListenerFactory: Option<Arc<dyn IRequestListenerFactory>>,
    requestListener: Option<RequestListener>,
    serverBase: Option<ServerBase>,
}

impl Default for BlobServer {
    fn default() -> Self {
        Self::new()
    }
}

impl BlobServer {
    pub fn new() -> Self {
        Self::withConfiguration(BlobConfiguration::default())
    }

    #[allow(non_snake_case)]
    pub fn withConfiguration(configuration: BlobConfiguration) -> Self {
        Self {
            configuration,
            requestListenerFactory: None,
            requestListener: None,
            serverBase: None,
        }
    }

    #[allow(non_snake_case)]
    pub fn withRequestListenerFactory(
        configuration: BlobConfiguration,
        requestListenerFactory: Arc<dyn IRequestListenerFactory>,
    ) -> Self {
        let serverBase = ServerBase::new(
            configuration.host.clone(),
            configuration.port,
            Arc::clone(&requestListenerFactory),
            clone_configuration_base(&configuration.base),
        );

        Self {
            configuration,
            requestListenerFactory: Some(requestListenerFactory),
            requestListener: None,
            serverBase: Some(serverBase),
        }
    }

    async fn initializeServerBase(&mut self) -> Result<(), StorageError> {
        if self.serverBase.is_some() {
            return Ok(());
        }

        let logger: Arc<dyn ILogger + Send + Sync> = Arc::new(SharedCommonLogger);
        let mut lokiStore = LokiBlobMetadataStore::new(
            self.configuration.metadataDBPath.clone().into(),
            self.configuration.isMemoryPersistence,
        );
        azurite_common::i_data_store::IDataStore::init(&mut lokiStore).await?;
        let metadataStore: SharedBlobMetadataStore = Arc::new(lokiStore);

        let extentMetadataStore = LokiExtentMetadata::new(
            self.configuration.extentDBPath.clone(),
            self.configuration.isMemoryPersistence,
        );
        let extentStore =
            build_extent_store(&self.configuration, extentMetadataStore, logger.clone());
        {
            let mut extentStore = extentStore.lock().await;
            if !extentStore.isInitialized() {
                extentStore.init().await?;
            }
        }

        let accountDataStore = {
            let mut store = AccountDataStore::new(logger);
            azurite_common::i_data_store::IDataStore::init(&mut store).await?;
            Arc::new(store)
        };
        let requestListenerFactory: Arc<dyn IRequestListenerFactory> =
            Arc::new(BlobRequestListenerFactory::new(
                metadataStore,
                Arc::clone(&extentStore),
                accountDataStore,
                self.configuration.enableAccessLog,
                self.configuration.accessLogWriteStream.clone(),
                Some(self.configuration.loose),
                Some(self.configuration.skipApiVersionCheck),
                self.configuration.getOAuthLevel(),
                Some(self.configuration.disableProductStyleUrl),
                Some(self.configuration.bugForBugCompatibility),
            ));
        self.serverBase = Some(ServerBase::new(
            self.configuration.host.clone(),
            self.configuration.port,
            Arc::clone(&requestListenerFactory),
            clone_configuration_base(&self.configuration.base),
        ));
        self.requestListenerFactory = Some(requestListenerFactory);
        Ok(())
    }

    #[allow(non_snake_case)]
    pub fn createRequestListener(&mut self) -> RequestListener {
        let listener = self
            .requestListenerFactory
            .as_ref()
            .map(|factory| factory.createRequestListener())
            .unwrap_or_default();
        self.requestListener = Some(listener.clone());
        listener
    }

    #[allow(non_snake_case)]
    pub fn requestListener(&self) -> Option<RequestListener> {
        self.requestListener.clone()
    }

    #[allow(non_snake_case)]
    pub fn getHttpServerAddress(&self) -> String {
        self.serverBase
            .as_ref()
            .map(ServerBase::getHttpServerAddress)
            .unwrap_or_default()
    }

    #[allow(non_snake_case)]
    pub fn getStatus(&self) -> ServerStatus {
        self.serverBase
            .as_ref()
            .map(ServerBase::getStatus)
            .unwrap_or(ServerStatus::Closed)
    }

    pub async fn start(&mut self) -> Result<(), StorageError> {
        global_logger.info(
            &format!(
                "Azurite Blob service is starting on {}:{}",
                self.configuration.host, self.configuration.port
            ),
            None,
        );
        self.initializeServerBase().await?;
        self.serverBase
            .as_mut()
            .ok_or_else(|| StorageError::new("Blob server base was not initialized."))?
            .start()
            .await?;
        global_logger.info(
            &format!(
                "Azurite Blob service successfully listens on {}",
                self.getHttpServerAddress()
            ),
            None,
        );
        Ok(())
    }

    pub async fn close(&mut self) -> Result<(), StorageError> {
        global_logger.info(BEFORE_CLOSE_MESSAGE, None);
        self.serverBase
            .as_mut()
            .ok_or_else(|| StorageError::new("Blob server base was not initialized."))?
            .close()
            .await?;
        global_logger.info(AFTER_CLOSE_MESSAGE, None);
        Ok(())
    }

    pub fn configuration(&self) -> &BlobConfiguration {
        &self.configuration
    }
}

impl fmt::Debug for BlobServer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BlobServer")
            .field("configuration", &self.configuration)
            .field("status", &self.getStatus())
            .field(
                "hasRequestListenerFactory",
                &self
                    .requestListenerFactory
                    .as_ref()
                    .map(|_| true)
                    .unwrap_or(false),
            )
            .field(
                "hasRequestListener",
                &self.requestListener.as_ref().map(|_| true).unwrap_or(false),
            )
            .finish()
    }
}
