use crate::generated::artifacts::models::GeneratedValue;
use crate::generated::errors::middleware_error::MiddlewareError;

pub struct OperationMismatchError;

impl OperationMismatchError {
    pub fn new() -> MiddlewareError {
        let mut error = MiddlewareError::new(500, "Operation is undefined.");
        error.body = Some(GeneratedValue::String(String::from(
            "Operation is undefined.",
        )));
        error.contentType = Some(String::from("text/plain"));
        error
    }
}
