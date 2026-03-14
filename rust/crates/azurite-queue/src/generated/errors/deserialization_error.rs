use crate::generated::errors::middleware_error::MiddlewareError;

pub struct DeserializationError;

impl DeserializationError {
    pub fn new(message: impl Into<String>) -> MiddlewareError {
        MiddlewareError::new(400, message)
    }
}
