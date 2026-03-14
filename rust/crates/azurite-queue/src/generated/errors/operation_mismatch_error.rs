use crate::generated::errors::middleware_error::MiddlewareError;

pub struct OperationMismatchError;

impl OperationMismatchError {
    pub fn new() -> MiddlewareError {
        MiddlewareError::new(
            500,
            "No operation provided in context, please make sure dispatchMiddleware is properly used.",
        )
    }
}
