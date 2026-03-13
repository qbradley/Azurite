use async_trait::async_trait;
use bytes::Bytes;
use std::{pin::Pin, sync::Arc};
use tokio::io::AsyncRead;

use crate::{i_cleaner::ICleaner, i_data_store::IDataStore, storage_error::StorageError};

use super::i_extent_metadata_store::IExtentMetadataStore;

pub type ReadableStream = Pin<Box<dyn AsyncRead + Send>>;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IExtentChunk {
    pub id: String,
    pub offset: u64,
    pub count: u64,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IStoreDestinationConfigure {
    pub locationPath: String,
    pub locationId: String,
    pub maxConcurrency: usize,
}

pub type StoreDestinationArray = Vec<IStoreDestinationConfigure>;

pub enum ExtentDataInput {
    Buffer(Bytes),
    Stream(ReadableStream),
}

#[allow(non_snake_case)]
#[async_trait]
pub trait IExtentStore: IDataStore + ICleaner + Send + Sync {
    async fn appendExtent(
        &mut self,
        data: ExtentDataInput,
        contextId: Option<&str>,
    ) -> Result<IExtentChunk, StorageError>;
    async fn readExtent(
        &self,
        extentChunk: Option<&IExtentChunk>,
        contextId: Option<&str>,
    ) -> Result<ReadableStream, StorageError>;
    async fn readExtents(
        &self,
        extentChunkArray: &[IExtentChunk],
        offset: u64,
        count: u64,
        contextId: Option<&str>,
    ) -> Result<ReadableStream, StorageError>;
    async fn deleteExtents(&mut self, persistency: Vec<String>) -> Result<u64, StorageError>;
    fn getMetadataStore(&self) -> Arc<dyn IExtentMetadataStore + Send + Sync>;
}
