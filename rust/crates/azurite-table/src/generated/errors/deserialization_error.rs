use crate::generated::errors::middleware_error::MiddlewareError;

pub struct DeserializationError;

impl DeserializationError {
    pub fn new(message: impl Into<String>) -> MiddlewareError {
        let message = message.into();
        // TypeScript DeserializationError returns empty body (no body, contentType, or headers)
        // Only statusCode and message are set - matching TS line 3-7 in DeserializationError.ts
        MiddlewareError::new(400, message)
    }
}
