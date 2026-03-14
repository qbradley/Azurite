pub mod i_queue_metadata_store;
pub mod loki_queue_metadata_store;
pub mod queue_referred_extents_async_iterator;

pub use i_queue_metadata_store::{
    IQueueMetadata, IQueueMetadataStore, MessageModel, MessageUpdateProperties, QueueACL,
    QueueModel, ServicePropertiesModel,
};
pub use loki_queue_metadata_store::LokiQueueMetadataStore;
pub use queue_referred_extents_async_iterator::QueueReferredExtentsAsyncIterator;

#[derive(Debug, Clone, Default)]
pub struct QueuePersistenceModule;
