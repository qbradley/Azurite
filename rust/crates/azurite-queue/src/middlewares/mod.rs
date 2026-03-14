pub mod authentication_middleware_factory;
pub mod preflight_middleware_factory;
pub mod queue_storage_context;
pub mod telemetry;

pub use authentication_middleware_factory::{
    AuthenticationMiddleware, AuthenticationMiddlewareFactory, SharedAuthenticator,
    SharedMiddlewareLogger,
};
pub use preflight_middleware_factory::{
    BoxedMiddlewareError, CorsRequestMiddleware, OptionsHandlerMiddleware,
    PreflightMiddlewareFactory, SharedPreflightLogger, SharedQueueMetadataStore,
};
pub use queue_storage_context::{
    createQueueStorageContextMiddleware, extractStoragePartsFromPath,
    internalQueueStorageContextMiddleware, queueStorageContextMiddleware,
    QueueStorageContextMiddlewareOptions,
};
pub use telemetry::{telemetryMiddleware, TelemetryMiddleware, TelemetryMiddlewareFactory};

#[derive(Debug, Clone, Default)]
pub struct QueueMiddlewaresModule;
