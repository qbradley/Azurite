use std::ops::Deref;

use thiserror::Error;

use super::storage_error::StorageError;

#[derive(Debug, Clone, Error)]
#[error("{0}")]
pub struct NotImplementedError(pub StorageError);

impl NotImplementedError {
    pub fn new(requestID: Option<&str>) -> Self {
        Self(StorageError::new(
            501,
            "APINotImplemented",
            "Current API is not implemented yet. Please vote your wanted features to https://github.com/azure/azurite/issues",
            requestID.unwrap_or(""),
            StorageError::empty_extra(),
        ))
    }
}

impl Deref for NotImplementedError {
    type Target = StorageError;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<NotImplementedError> for StorageError {
    fn from(value: NotImplementedError) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Error)]
#[error("{0}")]
pub struct NotImplementedinSQLError(pub StorageError);

impl NotImplementedinSQLError {
    pub fn new(requestID: Option<&str>) -> Self {
        Self(StorageError::new(
            501,
            "APINotImplemented",
            "Current API is not implemented yet when use a SQL database based metadata storage. Please vote your wanted features to https://github.com/azure/azurite/issues",
            requestID.unwrap_or(""),
            StorageError::empty_extra(),
        ))
    }
}

impl Deref for NotImplementedinSQLError {
    type Target = StorageError;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<NotImplementedinSQLError> for StorageError {
    fn from(value: NotImplementedinSQLError) -> Self {
        value.0
    }
}
