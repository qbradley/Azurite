pub mod not_implemented_error;
pub mod storage_error;
pub mod storage_error_factory;
pub mod strict_model_error;

pub use not_implemented_error::{NotImplementedError, NotImplementedinSQLError};
pub use storage_error::StorageError;
pub use storage_error_factory::StorageErrorFactory;
pub use strict_model_error::StrictModelNotSupportedError;

#[derive(Debug, Clone, Default)]
pub struct BlobErrorsModule;
