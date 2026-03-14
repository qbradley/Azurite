#![allow(non_snake_case)]
#![allow(clippy::incompatible_msrv)]

use std::{path::PathBuf, process};

use azurite_common::{
    configuration_base::setExtentMemoryLimit, i_environment::IEnvironment, logger::configLogger,
    storage_error::StorageError, telemetry::AzuriteTelemetryClient,
};
use azurite_queue::{QueueConfiguration, QueueEnvironment, QueueServer};
use tokio::signal;

use azurite_queue::utils::constants::{
    DEFAULT_QUEUE_EXTENT_LOKI_DB_PATH, DEFAULT_QUEUE_KEEP_ALIVE_TIMEOUT,
    DEFAULT_QUEUE_LISTENING_PORT, DEFAULT_QUEUE_LOKI_DB_PATH, DEFAULT_QUEUE_PERSISTENCE_ARRAY,
    DEFAULT_QUEUE_PERSISTENCE_PATH, DEFAULT_QUEUE_SERVER_HOST_NAME,
};

fn build_configuration(
    environment: &QueueEnvironment,
    location: &str,
    debugLogFilePath: Option<String>,
) -> QueueConfiguration {
    let mut persistencePathArray = (*DEFAULT_QUEUE_PERSISTENCE_ARRAY).clone();
    if let Some(firstDestination) = persistencePathArray.get_mut(0) {
        firstDestination.locationPath = PathBuf::from(location)
            .join(DEFAULT_QUEUE_PERSISTENCE_PATH)
            .display()
            .to_string();
    }

    QueueConfiguration::new(
        environment
            .queueHost()
            .unwrap_or_else(|| DEFAULT_QUEUE_SERVER_HOST_NAME.to_string()),
        environment
            .queuePort()
            .unwrap_or(DEFAULT_QUEUE_LISTENING_PORT),
        environment
            .queueKeepAliveTimeout()
            .unwrap_or(DEFAULT_QUEUE_KEEP_ALIVE_TIMEOUT),
        PathBuf::from(location)
            .join(DEFAULT_QUEUE_LOKI_DB_PATH)
            .display()
            .to_string(),
        PathBuf::from(location)
            .join(DEFAULT_QUEUE_EXTENT_LOKI_DB_PATH)
            .display()
            .to_string(),
        persistencePathArray,
        !environment.silent(),
        None,
        debugLogFilePath.is_some(),
        debugLogFilePath,
        environment.loose(),
        environment.skipApiVersionCheck(),
        environment.cert().unwrap_or_default(),
        environment.key().unwrap_or_default(),
        environment.pwd().unwrap_or_default(),
        environment.oauth(),
        environment.disableProductStyleUrl(),
        environment.inMemoryPersistence(),
        None,
    )
}

async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
            .expect("failed to install SIGINT handler");
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler");

        tokio::select! {
            _ = signal::ctrl_c() => {}
            _ = sigint.recv() => {}
            _ = sigterm.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        let _ = signal::ctrl_c().await;
    }
}

async fn run() -> Result<(), StorageError> {
    let environment = QueueEnvironment::from_process()?;
    let location = environment.location().await?;
    let debugLogFilePath = environment.debug().await?;
    let configuration = build_configuration(&environment, &location, debugLogFilePath.clone());

    configLogger(
        configuration.enableDebugLog,
        configuration.debugLogFilePath.clone(),
    );
    setExtentMemoryLimit(&environment, true)?;

    println!(
        "Azurite Queue service is starting on {}:{}",
        configuration.host, configuration.port
    );

    let mut server = QueueServer::withConfiguration(configuration.clone());
    server.start().await?;

    println!(
        "Azurite Queue service successfully listens on {}",
        server.getHttpServerAddress()
    );

    AzuriteTelemetryClient::init(location, !environment.disableTelemetry(), None, false);
    AzuriteTelemetryClient::TraceStartEvent("Queue").await;

    wait_for_shutdown_signal().await;

    AzuriteTelemetryClient::TraceStopEvent("Queue");
    println!("Azurite Queue service is closing...");
    server.close().await?;
    println!("Azurite Queue service successfully closed");
    Ok(())
}

#[tokio::main]
async fn main() {
    let _ = tracing_subscriber::fmt::try_init();

    if let Err(error) = run().await {
        eprintln!("Exit due to unhandled error: {}", error.message);
        process::exit(1);
    }
}
