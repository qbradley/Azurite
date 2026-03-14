use async_trait::async_trait;

use crate::errors::StorageError;
use crate::generated::artifacts::models;
use crate::generated::context::Context;
use crate::generated::i_request::GeneratedReadableStream;

#[allow(non_snake_case)]
#[async_trait]
pub trait ITableHandler: Send + Sync {
    async fn query(
        &self,
        options: models::TableQueryOptionalParams,
        context: Context,
    ) -> Result<models::TableQueryResponse, StorageError>;
    async fn create(
        &self,
        table: models::TableProperties,
        options: models::TableCreateOptionalParams,
        context: Context,
    ) -> Result<models::TableCreateResponse, StorageError>;
    async fn batch(
        &self,
        body: GeneratedReadableStream,
        options: models::TableBatchOptionalParams,
        context: Context,
    ) -> Result<models::TableBatchResponse, StorageError>;
    async fn delete(
        &self,
        options: models::TableDeleteMethodOptionalParams,
        context: Context,
    ) -> Result<models::TableDeleteResponse, StorageError>;
    async fn queryEntities(
        &self,
        options: models::TableQueryEntitiesOptionalParams,
        context: Context,
    ) -> Result<models::TableQueryEntitiesResponse, StorageError>;
    async fn queryEntitiesWithPartitionAndRowKey(
        &self,
        options: models::TableQueryEntitiesWithPartitionAndRowKeyOptionalParams,
        context: Context,
    ) -> Result<models::TableQueryEntitiesWithPartitionAndRowKeyResponse, StorageError>;
    async fn updateEntity(
        &self,
        entity: models::TableEntity,
        options: models::TableUpdateEntityOptionalParams,
        context: Context,
    ) -> Result<models::TableUpdateEntityResponse, StorageError>;
    async fn mergeEntity(
        &self,
        entity: models::TableEntity,
        options: models::TableMergeEntityOptionalParams,
        context: Context,
    ) -> Result<models::TableMergeEntityResponse, StorageError>;
    async fn deleteEntity(
        &self,
        options: models::TableDeleteEntityOptionalParams,
        context: Context,
    ) -> Result<models::TableDeleteEntityResponse, StorageError>;
    async fn insertEntity(
        &self,
        entity: models::TableEntity,
        options: models::TableInsertEntityOptionalParams,
        context: Context,
    ) -> Result<models::TableInsertEntityResponse, StorageError>;
    async fn getAccessPolicy(
        &self,
        options: models::TableGetAccessPolicyOptionalParams,
        context: Context,
    ) -> Result<models::TableGetAccessPolicyResponse, StorageError>;
    async fn setAccessPolicy(
        &self,
        signedIdentifiers: Vec<models::SignedIdentifier>,
        options: models::TableSetAccessPolicyOptionalParams,
        context: Context,
    ) -> Result<models::TableSetAccessPolicyResponse, StorageError>;
}
