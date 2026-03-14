use crate::generated::errors::middleware_error::MiddlewareError;

pub struct UnsupportedRequestError;

impl UnsupportedRequestError {
    pub fn new() -> MiddlewareError {
        MiddlewareError::new(
            400,
            "Incoming URL doesn't match any of swagger defined request patterns.",
        )
    }
}
