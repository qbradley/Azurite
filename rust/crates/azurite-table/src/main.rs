#![allow(non_snake_case)]
#![allow(clippy::incompatible_msrv)]

use std::{path::PathBuf, process};

use azurite_common::{
    logger::configLogger, storage_error::StorageError, telemetry::AzuriteTelemetryClient,
};
use azurite_table::{ITableEnvironment, TableConfiguration, TableEnvironment, TableServer};
use tokio::signal;

use azurite_table::utils::constants::{
    DEFAULT_TABLE_KEEP_ALIVE_TIMEOUT, DEFAULT_TABLE_LISTENING_PORT, DEFAULT_TABLE_LOKI_DB_PATH,
    DEFAULT_TABLE_SERVER_HOST_NAME,
};

fn build_configuration(
    environment: &TableEnvironment,
    location: &str,
    debugLogFilePath: Option<String>,
) -> TableConfiguration {
    TableConfiguration::new(
        environment
            .tableHost()
            .unwrap_or_else(|| DEFAULT_TABLE_SERVER_HOST_NAME.to_string()),
        environment
            .tablePort()
            .unwrap_or(DEFAULT_TABLE_LISTENING_PORT),
        environment
            .tableKeepAliveTimeout()
            .unwrap_or(DEFAULT_TABLE_KEEP_ALIVE_TIMEOUT),
        location.to_string(),
        PathBuf::from(location)
            .join(DEFAULT_TABLE_LOKI_DB_PATH)
            .display()
            .to_string(),
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
    let environment = TableEnvironment::from_process()?;
    let location = environment.location().await?;
    let debugLogFilePath = environment.debug().await?;
    let configuration = build_configuration(&environment, &location, debugLogFilePath.clone());

    configLogger(
        configuration.enableDebugLog,
        configuration.debugLogFilePath.clone(),
    );

    println!(
        "Azurite Table service is starting on {}:{}",
        configuration.host, configuration.port
    );

    let mut server = TableServer::withConfiguration(configuration.clone());
    server.start().await?;

    println!(
        "Azurite Table service successfully listens on {}",
        server.getHttpServerAddress()
    );

    AzuriteTelemetryClient::init(location, !environment.disableTelemetry(), None, false);
    AzuriteTelemetryClient::TraceStartEvent("Table").await;

    wait_for_shutdown_signal().await;

    AzuriteTelemetryClient::TraceStopEvent("Table");
    println!("Azurite Table service is closing...");
    server.close().await?;
    println!("Azurite Table service successfully closed");
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
