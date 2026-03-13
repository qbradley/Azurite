use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::BoxStream;

use crate::{i_data_store::IDataStore, storage_error::StorageError};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IExtentModel {
    pub id: String,
    pub persistencyId: String,
    pub path: String,
    pub size: u64,
    pub LastModifyInMS: i64,
}

#[allow(non_snake_case)]
#[async_trait]
pub trait IExtentMetadata: IDataStore + Send + Sync {
    async fn updateExtent(&mut self, extent: IExtentModel) -> Result<(), StorageError>;
    async fn listExtents(
        &self,
        id: Option<&str>,
        maxResults: Option<u64>,
        marker: Option<u64>,
        queryTime: Option<DateTime<Utc>>,
        UnmodifiedTime: Option<u64>,
    ) -> Result<(Vec<IExtentModel>, Option<u64>), StorageError>;
    fn getExtentIterator(&self) -> BoxStream<'_, Vec<String>>;
    async fn deleteExtent(&mut self, extentId: &str) -> Result<(), StorageError>;
    async fn getExtentPersistencyId(&self, extentId: &str) -> Result<String, StorageError>;
}
