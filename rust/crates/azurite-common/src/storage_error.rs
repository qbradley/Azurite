use thiserror::Error;

#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[error("{message}")]
pub struct StorageError {
    pub message: String,
}

impl StorageError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn invalid_header_value(value: impl AsRef<str>) -> Self {
        Self::new(format!("Invalid header value: {}", value.as_ref()))
    }

    pub fn invalid_metadata(_contextID: impl AsRef<str>) -> Self {
        Self::new("The metadata specified is invalid. It has characters that are not permitted.")
    }
}
