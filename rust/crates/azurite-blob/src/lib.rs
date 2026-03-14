#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(clippy::incompatible_msrv)]
#![allow(clippy::result_large_err)]
#![allow(clippy::too_many_arguments)]
#![allow(non_upper_case_globals)]
#![allow(clippy::module_inception)]
#![allow(clippy::new_ret_no_self)]
#![allow(clippy::type_complexity)]
#![allow(clippy::new_without_default)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::suspicious_open_options)]
#![allow(clippy::await_holding_lock)]
#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::assertions_on_constants)]

pub mod authentication;
pub mod blob_configuration;
pub mod blob_environment;
pub mod blob_request_listener_factory;
pub mod blob_server;
pub mod blob_server_factory;
pub mod conditions;
pub mod context;
pub mod errors;
pub mod gc;
pub mod generated;
pub mod handlers;
pub mod i_blob_environment;
pub mod lease;
#[path = "main.rs"]
pub mod main_entry;
pub mod middlewares;
pub mod persistence;
pub mod runtime_blob_metadata_store;
pub mod utils;

pub use blob_configuration::BlobConfiguration;
pub use blob_environment::BlobEnvironment;
pub use blob_request_listener_factory::BlobRequestListenerFactory;
pub use blob_server::BlobServer;
pub use blob_server_factory::{BlobServerFactory, BlobServerFactoryResult};
pub use gc::{BlobGCManager, BlobGCManagerStatus};
pub use i_blob_environment::IBlobEnvironment;
pub use main_entry::{
    createBlobServiceRuntime, initializeBlobServiceRuntime, runBlobService,
    shutdownBlobServiceRuntime, BlobServiceRuntime,
};
pub use middlewares::{
    blobStorageContextMiddleware, createStorageBlobContextMiddleware, extractStoragePartsFromPath,
    internalBlobStorageContextMiddleware, AuthenticationMiddleware,
    AuthenticationMiddlewareFactory, BlobStorageContextMiddlewareOptions, CorsRequestMiddleware,
    OptionsHandlerMiddleware, PreflightMiddlewareFactory, StrictModelMiddleware,
    StrictModelMiddlewareFactory, StrictModelRequestValidator, TelemetryMiddleware,
    TelemetryMiddlewareFactory, UnsupportedHeadersBlocker, UnsupportedParametersBlocker,
};
