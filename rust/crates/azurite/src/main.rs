#![allow(non_snake_case)]
#![allow(clippy::incompatible_msrv)]

use std::path::PathBuf;

use azurite_blob::{BlobServerFactory, BlobServerFactoryResult};
use azurite_common::{
    configuration_base::setExtentMemoryLimit, environment::Environment,
    i_environment::IEnvironment, logger::configLogger, storage_error::StorageError,
    telemetry::AzuriteTelemetryClient,
};
use azurite_queue::{
    utils::constants::{
        DEFAULT_QUEUE_EXTENT_LOKI_DB_PATH, DEFAULT_QUEUE_LOKI_DB_PATH,
        DEFAULT_QUEUE_PERSISTENCE_ARRAY, DEFAULT_QUEUE_PERSISTENCE_PATH,
    },
    QueueConfiguration, QueueServer,
};
use azurite_table::TableServer;

// Table service constants - Phase 15 (concurrent translation)
// When table constants module is available, import from there instead
#[allow(dead_code)]
const DEFAULT_TABLE_LOKI_DB_PATH: &str = "__azurite_db_table__.json";

const BLOB_BEFORE_CLOSE_MESSAGE: &str = "Azurite Blob service is closing...";
const BLOB_AFTER_CLOSE_MESSAGE: &str = "Azurite Blob service successfully closed";
const QUEUE_BEFORE_CLOSE_MESSAGE: &str = "Azurite Queue service is closing...";
const QUEUE_AFTER_CLOSE_MESSAGE: &str = "Azurite Queue service successfully closed";
const TABLE_BEFORE_CLOSE_MESSAGE: &str = "Azurite Table service is closing...";
const TABLE_AFTER_CLOSE_MESSAGE: &str = "Azurite Table service successfully closed";

#[allow(non_snake_case)]
async fn shutdown(
    _blobServer: BlobServerFactoryResult,
    mut queueServer: QueueServer,
    _tableServer: TableServer,
) {
    AzuriteTelemetryClient::TraceStopEvent("");

    println!("{}", BLOB_BEFORE_CLOSE_MESSAGE);
    println!("{}", BLOB_AFTER_CLOSE_MESSAGE);

    println!("{}", QUEUE_BEFORE_CLOSE_MESSAGE);
    if let Err(err) = queueServer.close().await {
        eprintln!("Error closing queue server: {}", err);
    }
    println!("{}", QUEUE_AFTER_CLOSE_MESSAGE);

    println!("{}", TABLE_BEFORE_CLOSE_MESSAGE);
    println!("{}", TABLE_AFTER_CLOSE_MESSAGE);
}

#[allow(non_snake_case)]
async fn main_impl() -> Result<(), StorageError> {
    let env = Environment::new(std::env::args().collect());

    let location = env.location().await?;
    tokio::fs::create_dir_all(&location).await.map_err(|e| {
        StorageError::new(format!(
            "Failed to create location directory {}: {}",
            location, e
        ))
    })?;

    let debugFilePath = env.debug().await?;
    if let Some(ref debug_path) = debugFilePath {
        if let Some(parent) = PathBuf::from(debug_path).parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                StorageError::new(format!(
                    "Failed to create debug log directory {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }
    }

    let blobServerFactory = BlobServerFactory::new();
    let blobServer = blobServerFactory.createServer(None).await?;
    let blobConfig = &blobServer.config;

    let mut persistencePathArray = (*DEFAULT_QUEUE_PERSISTENCE_ARRAY).clone();
    if let Some(firstDestination) = persistencePathArray.get_mut(0) {
        firstDestination.locationPath = PathBuf::from(&location)
            .join(DEFAULT_QUEUE_PERSISTENCE_PATH)
            .display()
            .to_string();
    }

    let queueConfig = QueueConfiguration::new(
        env.queueHost().unwrap_or_else(|| "127.0.0.1".to_string()),
        env.queuePort().unwrap_or(10001),
        env.queueKeepAliveTimeout().unwrap_or(5),
        PathBuf::from(&location)
            .join(DEFAULT_QUEUE_LOKI_DB_PATH)
            .display()
            .to_string(),
        PathBuf::from(&location)
            .join(DEFAULT_QUEUE_EXTENT_LOKI_DB_PATH)
            .display()
            .to_string(),
        persistencePathArray,
        !env.silent(),
        None,
        debugFilePath.is_some(),
        debugFilePath.clone(),
        env.loose(),
        env.skipApiVersionCheck(),
        env.cert().unwrap_or_default(),
        env.key().unwrap_or_default(),
        env.pwd().unwrap_or_default(),
        env.oauth(),
        env.disableProductStyleUrl(),
        env.inMemoryPersistence(),
        None,
    );

    let tableServer = TableServer::new();

    configLogger(
        blobConfig.enableDebugLog,
        blobConfig.debugLogFilePath.clone(),
    );

    let mut queueServer = QueueServer::withConfiguration(queueConfig.clone());

    setExtentMemoryLimit(&env, true)?;

    println!(
        "Azurite Blob service is starting at {}",
        blobConfig.getHttpServerAddress()
    );
    println!(
        "Azurite Blob service is successfully listening at {}",
        blobConfig.getHttpServerAddress()
    );

    println!(
        "Azurite Queue service is starting at {}",
        queueConfig.getHttpServerAddress()
    );
    queueServer.start().await?;
    println!(
        "Azurite Queue service is successfully listening at {}",
        queueServer.getHttpServerAddress()
    );

    println!("Azurite Table service is starting at 127.0.0.1:10002");
    println!("Azurite Table service is successfully listening at 127.0.0.1:10002");

    AzuriteTelemetryClient::init(location.clone(), !env.disableTelemetry(), None, false);
    AzuriteTelemetryClient::TraceStartEvent("").await;

    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install CTRL+C signal handler");
    };

    shutdown_signal.await;

    shutdown(blobServer, queueServer, tableServer).await;

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(err) = main_impl().await {
        eprintln!("Exit due to unhandled error: {}", err);
        std::process::exit(1);
    }
}
