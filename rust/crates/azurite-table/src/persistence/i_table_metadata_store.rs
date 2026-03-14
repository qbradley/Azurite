use async_trait::async_trait;

use crate::errors::StorageError;
use crate::generated::{artifacts::models, context::Context};

pub type TableACL = Vec<SignedIdentifier>;
pub type QueryOptions = models::QueryOptions;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AccessPolicy {
    pub start: String,
    pub expiry: String,
    pub permission: String,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SignedIdentifier {
    pub id: String,
    pub accessPolicy: AccessPolicy,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Table {
    pub account: String,
    pub table: String,
    pub tableAcl: Option<TableACL>,
    pub odatametadata: Option<String>,
    pub odatatype: Option<String>,
    pub odataid: Option<String>,
    pub odataeditLink: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct Entity {
    pub PartitionKey: String,
    pub RowKey: String,
    pub eTag: String,
    pub lastModifiedTime: String,
    pub properties: models::GeneratedObject,
    pub odatametadata: Option<String>,
    pub odatatype: Option<String>,
    pub odataid: Option<String>,
    pub odataeditLink: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct ServicePropertiesModel {
    pub accountName: String,
    pub properties: models::GeneratedObject,
}

#[allow(non_snake_case)]
#[async_trait]
pub trait ITableMetadataStore: Send + Sync {
    async fn init(&self) -> Result<(), StorageError>;
    async fn close(&self) -> Result<(), StorageError>;

    async fn createTable(&self, context: &Context, tableModel: Table) -> Result<(), StorageError>;
    async fn getTable(
        &self,
        account: &str,
        table: &str,
        context: &Context,
    ) -> Result<Option<Table>, StorageError>;
    async fn queryTable(
        &self,
        context: &Context,
        account: &str,
        queryOptions: models::QueryOptions,
        nextTable: Option<&str>,
    ) -> Result<(Vec<Table>, Option<String>), StorageError>;
    async fn deleteTable(
        &self,
        context: &Context,
        table: &str,
        account: &str,
    ) -> Result<(), StorageError>;
    async fn setTableACL(
        &self,
        account: &str,
        table: &str,
        context: &Context,
        tableACL: Option<TableACL>,
    ) -> Result<(), StorageError>;

    async fn getTableAccessPolicy(
        &self,
        context: &Context,
        table: &str,
        options: models::TableGetAccessPolicyOptionalParams,
    ) -> Result<models::TableGetAccessPolicyResponse, StorageError>;
    async fn setTableAccessPolicy(
        &self,
        context: &Context,
        table: &str,
        options: models::TableSetAccessPolicyOptionalParams,
    ) -> Result<models::TableSetAccessPolicyResponse, StorageError>;

    async fn queryTableEntities(
        &self,
        context: &Context,
        account: &str,
        table: &str,
        queryOptions: models::QueryOptions,
        nextPartitionKey: Option<&str>,
        nextRowKey: Option<&str>,
    ) -> Result<(Vec<Entity>, Option<String>, Option<String>), StorageError>;
    async fn queryTableEntitiesWithPartitionAndRowKey(
        &self,
        context: &Context,
        table: &str,
        account: &str,
        partitionKey: &str,
        rowKey: &str,
        batchID: Option<&str>,
    ) -> Result<Option<Entity>, StorageError>;
    async fn insertTableEntity(
        &self,
        context: &Context,
        table: &str,
        account: &str,
        entity: Entity,
        batchID: Option<&str>,
    ) -> Result<Entity, StorageError>;
    async fn insertOrUpdateTableEntity(
        &self,
        context: &Context,
        table: &str,
        account: &str,
        entity: Entity,
        ifMatch: Option<&str>,
        batchID: Option<&str>,
    ) -> Result<Entity, StorageError>;
    async fn insertOrMergeTableEntity(
        &self,
        context: &Context,
        table: &str,
        account: &str,
        entity: Entity,
        ifMatch: Option<&str>,
        batchID: Option<&str>,
    ) -> Result<Entity, StorageError>;
    async fn deleteTableEntity(
        &self,
        context: &Context,
        table: &str,
        account: &str,
        partitionKey: &str,
        rowKey: &str,
        etag: &str,
        batchID: Option<&str>,
    ) -> Result<(), StorageError>;

    async fn getServiceProperties(
        &self,
        context: &Context,
        account: &str,
    ) -> Result<Option<ServicePropertiesModel>, StorageError>;
    async fn setServiceProperties(
        &self,
        context: &Context,
        serviceProperties: ServicePropertiesModel,
    ) -> Result<ServicePropertiesModel, StorageError>;

    async fn beginBatchTransaction(&self, batchID: &str) -> Result<(), StorageError>;
    async fn endBatchTransaction(
        &self,
        account: &str,
        table: &str,
        batchID: &str,
        context: &Context,
        succeeded: bool,
    ) -> Result<(), StorageError>;
}
