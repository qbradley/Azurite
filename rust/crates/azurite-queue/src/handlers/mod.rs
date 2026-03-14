pub mod base_handler;
pub mod message_id_handler;
pub mod messages_handler;
pub mod queue_handler;
pub mod service_handler;

pub use base_handler::{BaseHandler, SharedExtentStore, SharedLogger, SharedQueueMetadataStore};
pub use message_id_handler::MessageIdHandler;
pub use messages_handler::MessagesHandler;
pub use queue_handler::QueueHandler;
pub use service_handler::ServiceHandler;

#[derive(Debug, Clone, Default)]
pub struct QueueHandlersModule;
