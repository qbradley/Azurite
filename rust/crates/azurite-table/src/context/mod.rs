pub mod table_storage_context;

pub use crate::generated::context::Context;
pub use table_storage_context::TableStorageContext;

#[derive(Debug, Clone, Default)]
pub struct TableContextModule;
