pub mod not_implemented_error;
pub mod storage_error;
pub mod storage_error_factory;

pub use crate::utils::constants::{
    FULL_METADATA_ACCEPT, MINIMAL_METADATA_ACCEPT, NO_METADATA_ACCEPT, TABLE_API_VERSION,
    XML_METADATA,
};
pub use not_implemented_error::NotImplementedError;
pub use storage_error::StorageError;
pub use storage_error_factory::StorageErrorFactory;

#[derive(Debug, Clone, Default)]
pub struct TableErrorsModule;
