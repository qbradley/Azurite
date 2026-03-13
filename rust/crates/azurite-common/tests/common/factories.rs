use async_trait::async_trait;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use azurite_common::{
    i_request_listener_factory::IRequestListenerFactory,
    i_server_factory::IServerFactory,
    server_base::{RequestListener, ServerBase},
    storage_error::StorageError,
};
use bytes::Bytes;
use pretty_assertions::assert_eq;
use tower::ServiceExt;

struct RequestListenerFactoryFixture;

impl IRequestListenerFactory for RequestListenerFactoryFixture {
    fn createRequestListener(&self) -> RequestListener {
        Router::new().route("/", get(|| async { "listener ok" }))
    }
}

fn assert_request_listener_factory<T: IRequestListenerFactory>(_factory: &T) {}

#[tokio::test]
async fn request_listener_factory_returns_routable_listener() {
    let factory = RequestListenerFactoryFixture;

    assert_request_listener_factory(&factory);

    let response = factory
        .createRequestListener()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .expect("listener should handle request");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should collect"),
        Bytes::from_static(b"listener ok")
    );
}

struct ServerFactoryFixture;

#[async_trait]
impl IServerFactory for ServerFactoryFixture {
    type Server = ServerBase;

    async fn createServer(&self) -> Result<Self::Server, StorageError> {
        Ok(ServerBase::new())
    }
}

fn assert_server_factory<T: IServerFactory<Server = ServerBase>>(_factory: &T) {}

#[tokio::test]
async fn server_factory_uses_associated_server_type() {
    let factory = ServerFactoryFixture;

    assert_server_factory(&factory);

    let server: ServerBase = factory
        .createServer()
        .await
        .expect("server construction should succeed");

    assert!(!server.is_started);
}
