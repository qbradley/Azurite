#![allow(non_snake_case)]
#![allow(clippy::incompatible_msrv)]

use std::process;

#[tokio::main]
async fn main() {
    let _ = tracing_subscriber::fmt::try_init();

    if let Err(error) = azurite_blob::runBlobService(std::env::args().collect()).await {
        eprintln!("Exit due to unhandled error: {}", error.message);
        process::exit(1);
    }
}
