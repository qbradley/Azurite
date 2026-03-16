#![allow(non_snake_case)]
#![allow(clippy::incompatible_msrv)]

use azurite_common::{
    configuration_base::setExtentMemoryLimit, i_environment::IEnvironment, logger::configLogger,
    storage_error::StorageError, telemetry::AzuriteTelemetryClient,
};

use crate::{BlobEnvironment, BlobServerFactory, BlobServerFactoryResult};

#[derive(Debug)]
pub struct BlobServiceRuntime {
    pub blobEnvironment: BlobEnvironment,
    pub createdServer: BlobServerFactoryResult,
}

pub async fn createBlobServiceRuntime(
    args: Vec<String>,
) -> Result<BlobServiceRuntime, StorageError> {
    let blobEnvironment = BlobEnvironment::new(args)?;
    let blobServerFactory = BlobServerFactory::new();
    let createdServer = blobServerFactory
        .createServer(Some(blobEnvironment.clone()))
        .await?;

    Ok(BlobServiceRuntime {
        blobEnvironment,
        createdServer,
    })
}

pub async fn initializeBlobServiceRuntime(
    runtime: &mut BlobServiceRuntime,
) -> Result<(), StorageError> {
    configLogger(
        runtime.createdServer.config.enableDebugLog,
        runtime.createdServer.config.debugLogFilePath.clone(),
    );
    setExtentMemoryLimit(&runtime.blobEnvironment, true)?;

    runtime.createdServer.server.start().await?;

    println!(
        "Azurite Blob service is starting on {}:{}",
        runtime.createdServer.config.host, runtime.createdServer.config.port
    );
    println!(
        "Azurite Blob service runtime prepared for {}",
        runtime.createdServer.config.getHttpServerAddress()
    );

    let location = runtime.blobEnvironment.location().await?;
    AzuriteTelemetryClient::init(
        location,
        !runtime.blobEnvironment.disableTelemetry(),
        None,
        false,
    );
    AzuriteTelemetryClient::TraceStartEvent("Blob").await;
    Ok(())
}

pub async fn shutdownBlobServiceRuntime(
    runtime: &mut BlobServiceRuntime,
) -> Result<(), StorageError> {
    println!("Azurite Blob service is closing...");
    AzuriteTelemetryClient::TraceStopEvent("Blob");
    if let Err(err) = runtime.createdServer.server.close().await {
        eprintln!("Error closing blob server: {}", err);
    }
    println!("Azurite Blob service successfully closed");
    Ok(())
}

pub async fn runBlobService(args: Vec<String>) -> Result<BlobServiceRuntime, StorageError> {
    let mut runtime = createBlobServiceRuntime(args).await?;
    initializeBlobServiceRuntime(&mut runtime).await?;
    Ok(runtime)
}
