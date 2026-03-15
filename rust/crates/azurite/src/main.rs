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
use azurite_table::{
    utils::constants::{
        DEFAULT_TABLE_KEEP_ALIVE_TIMEOUT, DEFAULT_TABLE_LISTENING_PORT, DEFAULT_TABLE_LOKI_DB_PATH,
        DEFAULT_TABLE_SERVER_HOST_NAME,
    },
    TableConfiguration, TableServer,
};

fn build_table_configuration(
    env: &Environment,
    location: &str,
    debugFilePath: Option<String>,
) -> TableConfiguration {
    TableConfiguration::new(
        env.tableHost()
            .unwrap_or_else(|| DEFAULT_TABLE_SERVER_HOST_NAME.to_string()),
        env.tablePort().unwrap_or(DEFAULT_TABLE_LISTENING_PORT),
        env.tableKeepAliveTimeout()
            .unwrap_or(DEFAULT_TABLE_KEEP_ALIVE_TIMEOUT),
        location.to_string(),
        PathBuf::from(location)
            .join(DEFAULT_TABLE_LOKI_DB_PATH)
            .display()
            .to_string(),
        !env.silent(),
        None,
        debugFilePath.is_some(),
        debugFilePath,
        env.loose(),
        env.skipApiVersionCheck(),
        env.cert().unwrap_or_default(),
        env.key().unwrap_or_default(),
        env.pwd().unwrap_or_default(),
        env.oauth(),
        env.disableProductStyleUrl(),
        env.inMemoryPersistence(),
        None,
        None,
    )
}

#[allow(non_snake_case)]
async fn shutdown(
    mut blobServer: BlobServerFactoryResult,
    mut queueServer: QueueServer,
    mut tableServer: TableServer,
) {
    AzuriteTelemetryClient::TraceStopEvent("");

    if let Err(err) = blobServer.server.close().await {
        eprintln!("Error closing blob server: {}", err);
    }
    if let Err(err) = queueServer.close().await {
        eprintln!("Error closing queue server: {}", err);
    }
    if let Err(err) = tableServer.close().await {
        eprintln!("Error closing table server: {}", err);
    }
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
    let mut blobServer = blobServerFactory.createServerFromEnvironment(&env).await?;
    let blobConfig = blobServer.config.clone();

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

    let tableConfig = build_table_configuration(&env, &location, debugFilePath.clone());
    let mut tableServer = TableServer::withConfiguration(tableConfig);

    configLogger(
        blobConfig.enableDebugLog,
        blobConfig.debugLogFilePath.clone(),
    );

    let mut queueServer = QueueServer::withConfiguration(queueConfig.clone());

    setExtentMemoryLimit(&env, true)?;

    tokio::try_join!(
        blobServer.server.start(),
        queueServer.start(),
        tableServer.start(),
    )?;

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
    // Install a panic hook that prints to stderr so panics in spawned tasks
    // are never silently swallowed.
    std::panic::set_hook(Box::new(|info| {
        let bt = std::backtrace::Backtrace::force_capture();
        eprintln!("PANIC: {info}\n{bt}");
    }));

    if let Err(err) = main_impl().await {
        eprintln!("Exit due to unhandled error: {}", err);
        std::process::exit(1);
    }
}
