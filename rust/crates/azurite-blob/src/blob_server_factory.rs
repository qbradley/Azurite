use std::{env, path::PathBuf};

use async_trait::async_trait;
use azurite_common::{i_server_factory::IServerFactory, storage_error::StorageError};

use crate::{
    blob_configuration::BlobConfiguration,
    blob_environment::BlobEnvironment,
    blob_server::BlobServer,
    i_blob_environment::IBlobEnvironment,
    utils::constants::{
        DEFAULT_BLOB_EXTENT_LOKI_DB_PATH, DEFAULT_BLOB_KEEP_ALIVE_TIMEOUT,
        DEFAULT_BLOB_LISTENING_PORT, DEFAULT_BLOB_LOKI_DB_PATH, DEFAULT_BLOB_PERSISTENCE_ARRAY,
        DEFAULT_BLOB_PERSISTENCE_PATH, DEFAULT_BLOB_SERVER_HOST_NAME,
    },
};

#[allow(non_snake_case)]
#[derive(Debug)]
pub struct BlobServerFactoryResult {
    pub server: BlobServer,
    pub config: BlobConfiguration,
    pub isSQL: bool,
    pub databaseConnectionString: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct BlobServerFactory;

impl BlobServerFactory {
    pub fn new() -> Self {
        Self
    }

    pub async fn createServer(
        &self,
        blobEnvironment: Option<BlobEnvironment>,
    ) -> Result<BlobServerFactoryResult, StorageError> {
        let blobEnvironment = match blobEnvironment {
            Some(blobEnvironment) => blobEnvironment,
            None => BlobEnvironment::from_process()?,
        };
        self.createServerFromEnvironment(&blobEnvironment).await
    }

    pub async fn createServerFromEnvironment<E>(
        &self,
        blobEnvironment: &E,
    ) -> Result<BlobServerFactoryResult, StorageError>
    where
        E: IBlobEnvironment,
    {
        let location = blobEnvironment.location().await?;
        let debugFilePath = blobEnvironment.debug().await?;

        let mut persistencePathArray = (*DEFAULT_BLOB_PERSISTENCE_ARRAY).clone();
        if let Some(firstDestination) = persistencePathArray.get_mut(0) {
            firstDestination.locationPath = PathBuf::from(&location)
                .join(DEFAULT_BLOB_PERSISTENCE_PATH)
                .display()
                .to_string();
        }

        let databaseConnectionString = env::var("AZURITE_DB").ok();
        let isSQL = databaseConnectionString.is_some();
        if isSQL && blobEnvironment.inMemoryPersistence() {
            return Err(StorageError::new(
                "The --inMemoryPersistence option is not supported when using SQL-based metadata storage.",
            ));
        }
        if isSQL && blobEnvironment.extentMemoryLimit().is_some() {
            return Err(StorageError::new(
                "The --extentMemoryLimit option is not supported when using SQL-based metadata storage.",
            ));
        }

        let config = BlobConfiguration::new(
            blobEnvironment
                .blobHost()
                .unwrap_or_else(|| DEFAULT_BLOB_SERVER_HOST_NAME.to_string()),
            blobEnvironment
                .blobPort()
                .unwrap_or(DEFAULT_BLOB_LISTENING_PORT),
            blobEnvironment
                .blobKeepAliveTimeout()
                .unwrap_or(DEFAULT_BLOB_KEEP_ALIVE_TIMEOUT),
            if isSQL {
                String::new()
            } else {
                PathBuf::from(&location)
                    .join(DEFAULT_BLOB_LOKI_DB_PATH)
                    .display()
                    .to_string()
            },
            if isSQL {
                String::new()
            } else {
                PathBuf::from(&location)
                    .join(DEFAULT_BLOB_EXTENT_LOKI_DB_PATH)
                    .display()
                    .to_string()
            },
            persistencePathArray,
            !blobEnvironment.silent(),
            None,
            debugFilePath.is_some(),
            debugFilePath,
            blobEnvironment.loose(),
            blobEnvironment.skipApiVersionCheck(),
            blobEnvironment.cert().unwrap_or_default(),
            blobEnvironment.key().unwrap_or_default(),
            blobEnvironment.pwd().unwrap_or_default(),
            blobEnvironment.oauth(),
            blobEnvironment.disableProductStyleUrl(),
            blobEnvironment.bugForBugCompatibility(),
            blobEnvironment.inMemoryPersistence(),
            None,
        );

        Ok(BlobServerFactoryResult {
            server: BlobServer::withConfiguration(config.clone()),
            config,
            isSQL,
            databaseConnectionString,
        })
    }
}

#[async_trait]
impl IServerFactory for BlobServerFactory {
    type Server = BlobServerFactoryResult;

    async fn createServer(&self) -> Result<Self::Server, StorageError> {
        BlobServerFactory::createServer(self, None).await
    }
}
