use async_trait::async_trait;

use crate::errors::StorageError;
use crate::generated::artifacts::models;
use crate::generated::context::Context;

#[allow(non_snake_case)]
#[async_trait]
pub trait IServiceHandler: Send + Sync {
    async fn setProperties(
        &self,
        storageServiceProperties: models::StorageServiceProperties,
        options: models::ServiceSetPropertiesOptionalParams,
        context: Context,
    ) -> Result<models::ServiceSetPropertiesResponse, StorageError>;
    async fn getProperties(
        &self,
        options: models::ServiceGetPropertiesOptionalParams,
        context: Context,
    ) -> Result<models::ServiceGetPropertiesResponse, StorageError>;
    async fn getStatistics(
        &self,
        options: models::ServiceGetStatisticsOptionalParams,
        context: Context,
    ) -> Result<models::ServiceGetStatisticsResponse, StorageError>;
}
