#![allow(non_snake_case)]
#![allow(clippy::incompatible_msrv)]

use std::process;

#[tokio::main]
async fn main() {
    std::panic::set_hook(Box::new(|info| {
        let bt = std::backtrace::Backtrace::force_capture();
        eprintln!("PANIC: {info}\n{bt}");
    }));

    let _ = tracing_subscriber::fmt::try_init();

    let mut runtime = match azurite_blob::runBlobService(std::env::args().collect()).await {
        Ok(r) => r,
        Err(error) => {
            eprintln!("Exit due to unhandled error: {}", error.message);
            process::exit(1);
        }
    };

    // Wait for shutdown signal
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sighup = signal(SignalKind::hangup()).expect("failed to install SIGHUP handler");
        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => { break; }
                _ = sigterm.recv() => { break; }
                _ = sighup.recv() => { continue; }
            }
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install CTRL+C signal handler");
    }

    if let Err(error) = azurite_blob::shutdownBlobServiceRuntime(&mut runtime).await {
        eprintln!("Error during shutdown: {}", error.message);
        process::exit(1);
    }
}
