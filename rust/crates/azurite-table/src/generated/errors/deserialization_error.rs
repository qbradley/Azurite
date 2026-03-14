use crate::generated::artifacts::models::GeneratedValue;
use crate::generated::errors::middleware_error::MiddlewareError;

pub struct DeserializationError;

impl DeserializationError {
    pub fn new(message: impl Into<String>) -> MiddlewareError {
        let message = message.into();
        let mut error = MiddlewareError::new(400, message.clone());
        error.body = Some(GeneratedValue::String(message));
        error.contentType = Some(String::from("text/plain"));
        error
    }
}
