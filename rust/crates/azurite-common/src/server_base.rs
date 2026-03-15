use std::{fmt, net::SocketAddr, sync::Arc};

use axum::Router;
use hyper_util::{rt::TokioIo, server::conn::auto::Builder as AutoBuilder};
use rustls::ServerConfig;
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};
use tokio_rustls::TlsAcceptor;
use tower::Service;

use crate::{
    configuration_base::{CertMaterial, CertOptions, ConfigurationBase},
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

        let cert_option = self.config.hasCert();
        match cert_option {
            CertOptions::PEM | CertOptions::PFX => {
                let cert_material = self
                    .config
                    .getCert(cert_option)?
                    .ok_or_else(|| StorageError::new("Failed to load certificate material"))?;
                let tls_acceptor = build_tls_acceptor(cert_material)?;
                self.serverTask = Some(tokio::spawn(serve_tls(
                    listener,
                    router,
                    tls_acceptor,
                    shutdown_receiver,
                )));
            }
            CertOptions::Default => {
                let server = axum::serve(listener, router).with_graceful_shutdown(async move {
                    let _ = shutdown_receiver.await;
                });
                self.serverTask = Some(tokio::spawn(async move {
                    let _ = server.await;
                }));
            }
        }

        self.shutdownSender = Some(shutdown_sender);
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

fn build_tls_acceptor(cert_material: CertMaterial) -> Result<TlsAcceptor, StorageError> {
    match cert_material {
        CertMaterial::PEM { cert, key } => {
            let certs = rustls_pemfile::certs(&mut &cert[..])
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| StorageError::new(format!("Failed to parse PEM certificate: {e}")))?;

            if certs.is_empty() {
                return Err(StorageError::new(
                    "No certificates found in PEM certificate file",
                ));
            }

            let private_key = rustls_pemfile::private_key(&mut &key[..])
                .map_err(|e| StorageError::new(format!("Failed to parse PEM private key: {e}")))?
                .ok_or_else(|| StorageError::new("No private key found in PEM key file"))?;

            let config = ServerConfig::builder_with_provider(Arc::new(
                rustls::crypto::aws_lc_rs::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .map_err(|e| StorageError::new(format!("Failed to set TLS protocol versions: {e}")))?
            .with_no_client_auth()
            .with_single_cert(certs, private_key)
            .map_err(|e| StorageError::new(format!("Failed to build TLS config: {e}")))?;

            Ok(TlsAcceptor::from(Arc::new(config)))
        }
        CertMaterial::PFX { .. } => Err(StorageError::new(
            "PFX/PKCS12 certificates are not yet supported. Please use PEM format (--cert and --key flags).",
        )),
    }
}

async fn serve_tls(
    listener: TcpListener,
    router: Router,
    tls_acceptor: TlsAcceptor,
    shutdown_receiver: oneshot::Receiver<()>,
) {
    let mut shutdown_receiver = shutdown_receiver;
    let mut make_service = router.into_make_service();

    loop {
        let tcp_stream = tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _addr)) => stream,
                    Err(_) => continue,
                }
            }
            _ = &mut shutdown_receiver => {
                break;
            }
        };

        let acceptor = tls_acceptor.clone();
        let tower_service = unwrap_infallible(Service::call(&mut make_service, ()).await);

        tokio::spawn(async move {
            let tls_stream = match acceptor.accept(tcp_stream).await {
                Ok(s) => s,
                Err(_) => return,
            };
            let io = TokioIo::new(tls_stream);
            let hyper_service =
                hyper::service::service_fn(move |req| tower_service.clone().call(req));
            let _ = AutoBuilder::new(hyper_util::rt::TokioExecutor::new())
                .serve_connection_with_upgrades(io, hyper_service)
                .await;
        });
    }
}

fn unwrap_infallible<T>(result: Result<T, std::convert::Infallible>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => match err {},
    }
}
