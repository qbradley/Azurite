use chrono::{DateTime, Utc};
use futures::{stream, stream::BoxStream};

use crate::{storage_error::StorageError, utils::constants::DEFAULT_EXTENT_GC_PROTECT_TIME_IN_MS};

use super::i_extent_metadata_store::IExtentMetadataStore;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IteratorResult<T> {
    pub done: bool,
    pub value: T,
}

#[allow(non_snake_case)]
#[derive(Clone)]
pub struct AllExtentsAsyncIterator<M>
where
    M: IExtentMetadataStore + Clone + Send + Sync + 'static,
{
    unit: u64,
    done: bool,
    marker: Option<u64>,
    time: DateTime<Utc>,
    extentMetadata: M,
}

impl<M> AllExtentsAsyncIterator<M>
where
    M: IExtentMetadataStore + Clone + Send + Sync + 'static,
{
    pub fn new(extentMetadata: M) -> Self {
        Self {
            unit: 1000,
            done: false,
            marker: None,
            time: Utc::now(),
            extentMetadata,
        }
    }

    pub async fn next(&mut self) -> Result<IteratorResult<Vec<String>>, StorageError> {
        if self.done {
            return Ok(IteratorResult {
                done: true,
                value: Vec::new(),
            });
        }

        let (extents, nextMarker) = self
            .extentMetadata
            .listExtents(
                None,
                Some(self.unit),
                self.marker,
                Some(self.time),
                Some(DEFAULT_EXTENT_GC_PROTECT_TIME_IN_MS),
            )
            .await?;
        self.marker = nextMarker;

        if nextMarker.is_none() {
            self.done = true;
        }

        Ok(IteratorResult {
            done: false,
            value: extents.into_iter().map(|value| value.id).collect(),
        })
    }

    pub fn into_stream(self) -> BoxStream<'static, Vec<String>> {
        Box::pin(stream::unfold(self, |mut iterator| async move {
            match iterator.next().await {
                Ok(result) if result.done => None,
                Ok(result) => Some((result.value, iterator)),
                Err(_) => None,
            }
        }))
    }
}
