#![allow(non_snake_case)]
#![allow(clippy::incompatible_msrv)]

use azurite_blob::blob_server::BlobServer;
use azurite_queue::queue_server::QueueServer;
use azurite_table::table_server::TableServer;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "azurite")]
struct AzuriteCli {}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let _cli = AzuriteCli::parse();
    let _blob_server = BlobServer::new();
    let _queue_server = QueueServer::new();
    let _table_server = TableServer::new();

    tracing::info!("azurite workspace scaffold initialized");
}
