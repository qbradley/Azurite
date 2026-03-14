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
pub mod batch;
pub mod context;
pub mod entity;
pub mod errors;
pub mod generated;
pub mod handlers;
pub mod i_table_environment;
pub mod middlewares;
pub mod persistence;
pub mod table_configuration;
pub mod table_environment;
pub mod table_request_listener_factory;
pub mod table_server;
pub mod utils;

pub use context::TableStorageContext;
pub use errors::{NotImplementedError, StorageError, StorageErrorFactory};
pub use i_table_environment::ITableEnvironment;
pub use persistence::query_interpreter;
pub use table_configuration::{
    TableAccountKey, TableConfiguration, TableCorsConfiguration, TableLoggingConfiguration,
    TableTlsConfiguration,
};
pub use table_environment::TableEnvironment;
pub use table_request_listener_factory::TableRequestListenerFactory;
pub use table_server::TableServer;
