#![allow(non_snake_case)]
#![allow(clippy::incompatible_msrv)]

use azurite_blob::BlobServer;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "azurite-blob")]
struct BlobCli {}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let _cli = BlobCli::parse();
    let _server = BlobServer::new();
    tracing::info!("azurite-blob workspace scaffold initialized");
}
