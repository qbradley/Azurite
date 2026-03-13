use azurite_queue::QueueServer;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "azurite-queue")]
struct QueueCli {}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let _cli = QueueCli::parse();
    let _server = QueueServer::new();
    tracing::info!("azurite-queue workspace scaffold initialized");
}
