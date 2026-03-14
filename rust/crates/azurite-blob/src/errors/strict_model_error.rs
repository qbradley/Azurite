use std::ops::Deref;

use thiserror::Error;

use super::storage_error::StorageError;

#[derive(Debug, Clone, Error)]
#[error("{0}")]
pub struct StrictModelNotSupportedError(pub StorageError);

impl StrictModelNotSupportedError {
    pub fn new(feature: &str, requestID: Option<&str>) -> Self {
        Self(StorageError::new(
            500,
            "FeatureNotSupported",
            format!(
                "{feature} header or parameter is not supported in Azurite strict mode. Switch to loose model by Azurite command line parameter \"--loose\" or Visual Studio Code configuration \"Loose\". Please vote your wanted features to https://github.com/azure/azurite/issues"
            ),
            requestID.unwrap_or(""),
            StorageError::empty_extra(),
        ))
    }
}

impl Deref for StrictModelNotSupportedError {
    type Target = StorageError;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<StrictModelNotSupportedError> for StorageError {
    fn from(value: StrictModelNotSupportedError) -> Self {
        value.0
    }
}
