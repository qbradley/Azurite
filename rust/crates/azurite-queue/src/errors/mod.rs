pub mod not_implemented_error;
pub mod storage_error;
pub mod storage_error_factory;

pub use not_implemented_error::NotImplementedError;
pub use storage_error::StorageError;
pub use storage_error_factory::StorageErrorFactory;

#[derive(Debug, Clone, Default)]
pub struct QueueErrorsModule;
