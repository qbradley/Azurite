use crate::errors::StorageError;

use super::i_queue_metadata_store::IQueueMetadataStore;

const DEFAULT_BATCH_SIZE: u64 = 1000;

#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct QueueReferredExtentsAsyncIterator<T>
where
    T: IQueueMetadataStore + Clone,
{
    unit: u64,
    done: bool,
    marker: Option<u64>,
    queueMetadata: T,
}

impl<T> QueueReferredExtentsAsyncIterator<T>
where
    T: IQueueMetadataStore + Clone,
{
    pub fn new(queueMetadata: T) -> Self {
        Self {
            unit: DEFAULT_BATCH_SIZE,
            done: false,
            marker: None,
            queueMetadata,
        }
    }

    pub async fn next(&mut self) -> Result<(Vec<String>, bool), StorageError> {
        if self.done {
            return Ok((Vec::new(), true));
        }

        let (messages, nextMarker) = self
            .queueMetadata
            .listMessages(Some(self.unit), self.marker)
            .await?;
        self.marker = nextMarker;

        if nextMarker.is_none() {
            self.done = true;
        }

        Ok((
            messages
                .into_iter()
                .map(|message| message.persistency.id)
                .collect(),
            false,
        ))
    }
}
