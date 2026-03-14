pub mod authentication_middleware_factory;
pub mod preflight_middleware_factory;
pub mod table_storage_context_middleware;
pub mod telemetry;

pub use authentication_middleware_factory::{
    AuthenticationMiddleware, AuthenticationMiddlewareFactory, SharedAuthenticator,
    SharedMiddlewareLogger,
};
pub use preflight_middleware_factory::{
    BoxedMiddlewareError, CorsRequestMiddleware, OptionsHandlerMiddleware,
    PreflightMiddlewareFactory, SharedPreflightLogger, SharedTableMetadataStore,
};
pub use table_storage_context_middleware::{
    createTableStorageContextMiddleware, extractStoragePartsFromPath,
    internalTableStorageContextMiddleware, tableStorageContextMiddleware,
    TableStorageContextMiddlewareOptions,
};
pub use telemetry::{telemetryMiddleware, TelemetryMiddleware, TelemetryMiddlewareFactory};

#[derive(Debug, Clone, Default)]
pub struct TableMiddlewaresModule;
