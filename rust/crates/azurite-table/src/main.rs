use azurite_table::TableServer;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "azurite-table")]
struct TableCli {}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let _cli = TableCli::parse();
    let _server = TableServer::new();
    tracing::info!("azurite-table workspace scaffold initialized");
}
