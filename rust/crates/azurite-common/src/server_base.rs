use std::{fmt, net::SocketAddr, sync::Arc};

use axum::Router;
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};

use crate::{
    configuration_base::{CertOptions, ConfigurationBase},
    i_cleaner::ICleaner,
    i_request_listener_factory::IRequestListenerFactory,
    storage_error::StorageError,
};

pub type RequestListener = Router;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServerStatus {
    Closed,
    Starting,
    Running,
    Closing,
}

impl fmt::Display for ServerStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => formatter.write_str("Closed"),
            Self::Starting => formatter.write_str("Starting"),
            Self::Running => formatter.write_str("Running"),
            Self::Closing => formatter.write_str("Closing"),
        }
    }
}

struct DefaultRequestListenerFactory;

impl IRequestListenerFactory for DefaultRequestListenerFactory {
    fn createRequestListener(&self) -> RequestListener {
        Router::new()
    }
}

pub struct ServerBase {
    pub host: String,
    pub port: u16,
    pub config: ConfigurationBase,
    status: ServerStatus,
    requestListenerFactory: Arc<dyn IRequestListenerFactory>,
    requestListener: Option<RequestListener>,
    httpServerAddress: Option<SocketAddr>,
    shutdownSender: Option<oneshot::Sender<()>>,
    serverTask: Option<JoinHandle<()>>,
}

impl Default for ServerBase {
    fn default() -> Self {
        Self::new(
            "127.0.0.1".to_string(),
            10000,
            Arc::new(DefaultRequestListenerFactory),
            ConfigurationBase::default(),
        )
    }
}

impl ServerBase {
    pub fn new(
        host: String,
        port: u16,
        requestListenerFactory: Arc<dyn IRequestListenerFactory>,
        config: ConfigurationBase,
    ) -> Self {
        Self {
            host,
            port,
            config,
            status: ServerStatus::Closed,
            requestListenerFactory,
            requestListener: None,
            httpServerAddress: None,
            shutdownSender: None,
            serverTask: None,
        }
    }

    pub fn getHttpServerAddress(&self) -> String {
        let protocol = format!(
            "http{}://",
            if self.config.hasCert() == CertOptions::Default {
                ""
            } else {
                "s"
            }
        );

        match self.httpServerAddress {
            Some(address) => format!("{protocol}{}:{}", address.ip(), address.port()),
            None => String::new(),
        }
    }

    pub fn getStatus(&self) -> ServerStatus {
        self.status
    }

    pub async fn start(&mut self) -> Result<(), StorageError> {
        if self.status != ServerStatus::Closed {
            return Err(StorageError::new(format!(
                "Cannot start server in status {}",
                self.status
            )));
        }

        self.status = ServerStatus::Starting;
        match self.beforeStart().await {
            Ok(()) => {}
            Err(error) => {
                self.status = ServerStatus::Closed;
                return Err(error);
            }
        }

        let listener = match TcpListener::bind((self.host.as_str(), self.port)).await {
            Ok(listener) => listener,
            Err(error) => {
                self.status = ServerStatus::Closed;
                return Err(StorageError::new(error.to_string()));
            }
        };

        self.httpServerAddress = Some(
            listener
                .local_addr()
                .map_err(|error| StorageError::new(error.to_string()))?,
        );
        self.requestListener = Some(self.requestListenerFactory.createRequestListener());
        let router = self.requestListener.clone().unwrap_or_default();
        let (shutdown_sender, shutdown_receiver) = oneshot::channel();
        let server = axum::serve(listener, router).with_graceful_shutdown(async move {
            let _ = shutdown_receiver.await;
        });
        self.shutdownSender = Some(shutdown_sender);
        self.serverTask = Some(tokio::spawn(async move {
            let _ = server.await;
        }));
        self.status = ServerStatus::Running;

        self.afterStart().await
    }

    pub async fn close(&mut self) -> Result<(), StorageError> {
        if self.status != ServerStatus::Running {
            return Err(StorageError::new(format!(
                "Cannot close server in status {}",
                self.status
            )));
        }

        self.status = ServerStatus::Closing;
        self.beforeClose().await?;
        self.requestListener = Some(Router::new());
        if let Some(shutdownSender) = self.shutdownSender.take() {
            let _ = shutdownSender.send(());
        }
        self.serverTask.take();
        self.afterClose().await?;
        self.status = ServerStatus::Closed;
        Ok(())
    }

    pub async fn beforeStart(&mut self) -> Result<(), StorageError> {
        Ok(())
    }

    pub async fn afterStart(&mut self) -> Result<(), StorageError> {
        Ok(())
    }

    pub async fn beforeClose(&mut self) -> Result<(), StorageError> {
        Ok(())
    }

    pub async fn afterClose(&mut self) -> Result<(), StorageError> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl ICleaner for ServerBase {
    async fn clean(&mut self) -> Result<(), StorageError> {
        Ok(())
    }
}
