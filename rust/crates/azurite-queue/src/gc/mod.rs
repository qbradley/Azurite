pub mod queue_gc_manager;

pub use queue_gc_manager::{
    QueueGCManager, QueueGCManagerErrorHandler, Status as QueueGCManagerStatus,
};

#[derive(Debug, Clone, Default)]
pub struct QueueGcModule;
