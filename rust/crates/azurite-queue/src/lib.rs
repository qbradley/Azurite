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
pub mod context;
pub mod errors;
pub mod gc;
pub mod generated;
pub mod handlers;
pub mod middlewares;
pub mod persistence;
pub mod queue_configuration;
pub mod queue_environment;
pub mod queue_request_listener_factory;
pub mod queue_server;
pub mod utils;

pub use queue_server::QueueServer;
