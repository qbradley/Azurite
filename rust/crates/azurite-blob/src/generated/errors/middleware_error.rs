use std::collections::BTreeMap;

use thiserror::Error;

use crate::generated::artifacts::models::GeneratedValue;
use crate::generated::i_response::ResponseHeaderValue;

#[derive(Debug, Clone, Error)]
#[error("{message}")]
pub struct MiddlewareError {
    pub statusCode: u16,
    pub message: String,
    pub statusMessage: Option<String>,
    pub headers: Option<BTreeMap<String, ResponseHeaderValue>>,
    pub body: Option<GeneratedValue>,
    pub contentType: Option<String>,
}

impl MiddlewareError {
    pub fn new(statusCode: u16, message: impl Into<String>) -> Self {
        Self {
            statusCode,
            message: message.into(),
            statusMessage: None,
            headers: None,
            body: None,
            contentType: None,
        }
    }
}
