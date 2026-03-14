use std::sync::Arc;

use azurite_common::persistence::i_extent_store::IExtentStore;
use tokio::sync::Mutex;

use crate::generated::utils::i_logger::ILogger;
use crate::persistence::IBlobMetadataStore;

pub type SharedExtentStore = Arc<Mutex<Box<dyn IExtentStore + Send + Sync>>>;
pub type SharedBlobMetadataStore = Arc<dyn IBlobMetadataStore + Send + Sync>;
pub type SharedLogger = Arc<dyn ILogger + Send + Sync>;

#[derive(Clone)]
pub struct BaseHandler {
    pub metadataStore: SharedBlobMetadataStore,
    pub extentStore: SharedExtentStore,
    pub logger: SharedLogger,
    pub loose: bool,
}

impl BaseHandler {
    pub fn new(
        metadataStore: SharedBlobMetadataStore,
        extentStore: SharedExtentStore,
        logger: SharedLogger,
        loose: bool,
    ) -> Self {
        Self {
            metadataStore,
            extentStore,
            logger,
            loose,
        }
    }
}
