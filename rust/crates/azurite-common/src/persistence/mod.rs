pub mod all_extents_async_iterator;
pub mod fs_extent_store;
pub mod i_extent_metadata;
pub mod i_extent_metadata_store;
pub mod i_extent_store;
pub mod i_operation_queue;
pub mod loki_extent_metadata_store;
pub mod memory_extent_store;
pub mod operation_queue;

pub const ZERO_EXTENT_ID: &str = "*ZERO*";

#[derive(Debug, Clone, Default)]
pub struct PersistenceModule;
