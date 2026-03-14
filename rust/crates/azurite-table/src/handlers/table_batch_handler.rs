use crate::errors::{StorageError, StorageErrorFactory};
use crate::generated::context::Context;

#[derive(Clone)]
pub struct TableBatchHandler {
    context: Context,
}

impl TableBatchHandler {
    pub fn new(context: &Context) -> Self {
        Self {
            context: context.clone(),
        }
    }

    pub async fn process_batch_request_and_serialize_response(
        &self,
        _request_body: &str,
    ) -> Result<String, StorageError> {
        Err(StorageErrorFactory::getNotImplementedError(&self.context))
    }
}
