use std::{
    collections::HashMap,
    fmt,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use async_trait::async_trait;
use azurite_common::{
    configuration_base::ConfigurationBase,
    i_account_data_store::{IAccountDataStore, IAccountProperties},
    i_cleaner::ICleaner,
    i_data_store::IDataStore,
    i_logger::ILogger,
    i_request_listener_factory::IRequestListenerFactory,
    logger::logger as global_logger,
    server_base::{RequestListener, ServerBase, ServerStatus},
    storage_error::StorageError as CommonStorageError,
};

use crate::{
    generated::{
        artifacts::models::{GeneratedObject, GeneratedValue},
        context::Context,
    },
    persistence::{ITableMetadataStore, LokiTableMetadataStore, ServicePropertiesModel},
    table_configuration::{TableAccountKey, TableConfiguration},
    table_request_listener_factory::TableRequestListenerFactory,
    utils::constants::TABLE_API_VERSION,
};

const BEFORE_CLOSE_MESSAGE: &str = "Azurite Table service is closing...";
const AFTER_CLOSE_MESSAGE: &str = "Azurite Table service successfully closed";

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

#[derive(Clone)]
struct StaticTableAccountDataStore {
    accounts: Arc<HashMap<String, IAccountProperties>>,
    initialized: Arc<AtomicBool>,
}

impl StaticTableAccountDataStore {
    fn new(accounts: &[TableAccountKey]) -> Self {
        let mapped = accounts
            .iter()
            .map(|account| {
                (
                    account.name.clone(),
                    IAccountProperties {
                        name: account.name.clone(),
                        key1: account.key1.clone(),
                        key2: account.key2.clone(),
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        Self {
            accounts: Arc::new(mapped),
            initialized: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[async_trait]
impl IDataStore for StaticTableAccountDataStore {
    async fn init(&mut self) -> Result<(), CommonStorageError> {
        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn isInitialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    async fn close(&mut self) -> Result<(), CommonStorageError> {
        self.initialized.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn isClosed(&self) -> bool {
        !self.isInitialized()
    }
}

#[async_trait]
impl ICleaner for StaticTableAccountDataStore {
    async fn clean(&mut self) -> Result<(), CommonStorageError> {
        Ok(())
    }
}

impl IAccountDataStore for StaticTableAccountDataStore {
    fn getAccount(&self, name: &str) -> Option<IAccountProperties> {
        self.accounts.get(name).cloned()
    }
}

#[allow(non_snake_case)]
pub struct TableServer {
    pub configuration: TableConfiguration,
    requestListenerFactory: Arc<dyn IRequestListenerFactory>,
    serverBase: ServerBase,
    metadataStore: LokiTableMetadataStore,
    accountDataStore: StaticTableAccountDataStore,
}

impl Default for TableServer {
    fn default() -> Self {
        Self::new()
    }
}

impl TableServer {
    pub fn new() -> Self {
        Self::withConfiguration(TableConfiguration::default())
    }

    #[allow(non_snake_case)]
    pub fn withConfiguration(configuration: TableConfiguration) -> Self {
        let logger: Arc<dyn ILogger + Send + Sync> = Arc::new(SharedCommonLogger);
        let metadataStore = LokiTableMetadataStore::new(
            configuration.metadataDBPath.clone(),
            configuration.isMemoryPersistence,
        );
        let metadataStoreForFactory: Arc<
            dyn crate::persistence::ITableMetadataStore + Send + Sync,
        > = Arc::new(metadataStore.clone());
        let accountDataStore = StaticTableAccountDataStore::new(&configuration.accountKeys);
        let accountDataStoreForFactory: Arc<dyn IAccountDataStore + Send + Sync> =
            Arc::new(accountDataStore.clone());
        let requestListenerFactory: Arc<dyn IRequestListenerFactory> =
            Arc::new(TableRequestListenerFactory::new(
                metadataStoreForFactory,
                accountDataStoreForFactory,
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
        let _ = logger;

        Self {
            configuration,
            requestListenerFactory,
            serverBase,
            metadataStore,
            accountDataStore,
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

    pub fn configuration(&self) -> &TableConfiguration {
        &self.configuration
    }

    pub async fn start(&mut self) -> Result<(), CommonStorageError> {
        global_logger.info(
            &format!(
                "Azurite Table service is starting on {}:{}",
                self.configuration.host, self.configuration.port
            ),
            None,
        );
        self.accountDataStore.init().await?;
        self.metadataStore
            .init()
            .await
            .map_err(Self::map_table_error)?;
        self.apply_configured_service_properties().await?;
        self.serverBase.start().await?;
        global_logger.info(
            &format!(
                "Azurite Table service successfully listens on {}",
                self.serverBase.getHttpServerAddress()
            ),
            None,
        );
        Ok(())
    }

    pub async fn close(&mut self) -> Result<(), CommonStorageError> {
        global_logger.info(BEFORE_CLOSE_MESSAGE, None);
        self.serverBase.close().await?;
        if !self.metadataStore.isClosed() {
            self.metadataStore
                .close()
                .await
                .map_err(Self::map_table_error)?;
        }
        if !self.accountDataStore.isClosed() {
            self.accountDataStore.close().await?;
        }
        global_logger.info(AFTER_CLOSE_MESSAGE, None);
        Ok(())
    }

    pub async fn clean(&mut self) -> Result<(), CommonStorageError> {
        if self.getStatus() != ServerStatus::Closed {
            return Err(CommonStorageError::new(format!(
                "Cannot clean up table server in status {}.",
                self.getStatus()
            )));
        }

        if !self.metadataStore.isClosed() {
            self.metadataStore
                .close()
                .await
                .map_err(Self::map_table_error)?;
        }
        self.metadataStore.clean().await?;
        if !self.accountDataStore.isClosed() {
            self.accountDataStore.close().await?;
        }
        self.accountDataStore.clean().await
    }

    async fn apply_configured_service_properties(&self) -> Result<(), CommonStorageError> {
        if self.configuration.cors.rules.is_empty() {
            return Ok(());
        }

        let properties = self.configured_service_properties();
        for account in &self.configuration.accountKeys {
            self.metadataStore
                .setServiceProperties(
                    &Context::default(),
                    ServicePropertiesModel {
                        accountName: account.name.clone(),
                        properties: properties.clone(),
                    },
                )
                .await
                .map_err(Self::map_table_error)?;
        }
        Ok(())
    }

    fn configured_service_properties(&self) -> GeneratedObject {
        let mut properties = GeneratedObject::new();
        properties.insert(
            String::from("cors"),
            GeneratedValue::Array(
                self.configuration
                    .cors
                    .rules
                    .iter()
                    .cloned()
                    .map(GeneratedValue::Object)
                    .collect(),
            ),
        );
        properties.insert(
            String::from("defaultServiceVersion"),
            GeneratedValue::String(TABLE_API_VERSION.to_string()),
        );
        properties
    }

    fn map_table_error(error: crate::errors::StorageError) -> CommonStorageError {
        CommonStorageError::new(error.message)
    }
}

impl fmt::Debug for TableServer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TableServer")
            .field("configuration", &self.configuration)
            .field("status", &self.getStatus())
            .finish()
    }
}
