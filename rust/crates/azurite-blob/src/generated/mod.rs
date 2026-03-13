pub mod artifacts;
pub mod context;
pub mod errors;
pub mod express_middleware_factory;
pub mod express_request_adapter;
pub mod express_response_adapter;
pub mod handlers;
pub mod i_request;
pub mod i_response;
pub mod middleware;
pub mod middleware_factory;
pub mod utils;

pub type GeneratedResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
