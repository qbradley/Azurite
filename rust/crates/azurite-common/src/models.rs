use std::str::FromStr;

use crate::storage_error::StorageError;

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OAuthLevel {
    BASIC,
}

impl FromStr for OAuthLevel {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "basic" => Ok(Self::BASIC),
            _ => Err(StorageError::invalid_header_value(value)),
        }
    }
}
