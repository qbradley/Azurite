use crate::generated::artifacts::models::GeneratedValue;
use crate::generated::errors::middleware_error::MiddlewareError;

pub struct UnsupportedRequestError;

impl UnsupportedRequestError {
    pub fn new() -> MiddlewareError {
        let mut error = MiddlewareError::new(404, "The request is not supported by this service.");
        error.body = Some(GeneratedValue::String(String::from(
            "The request is not supported by this service.",
        )));
        error.contentType = Some(String::from("text/plain"));
        error
    }
}
