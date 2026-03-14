use async_trait::async_trait;

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
    ) -> crate::generated::GeneratedResult<models::ServiceSetPropertiesResponse>;
    async fn getProperties(
        &self,
        options: models::ServiceGetPropertiesOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ServiceGetPropertiesResponse>;
    async fn getStatistics(
        &self,
        options: models::ServiceGetStatisticsOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ServiceGetStatisticsResponse>;
    async fn listQueuesSegment(
        &self,
        options: models::ServiceListQueuesSegmentOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ServiceListQueuesSegmentResponse>;
}
