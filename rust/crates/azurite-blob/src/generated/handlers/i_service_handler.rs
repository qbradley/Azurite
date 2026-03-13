use async_trait::async_trait;

use crate::generated::artifacts::models;
use crate::generated::context::Context;
use crate::generated::i_request::GeneratedReadableStream;

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
    async fn listContainersSegment(
        &self,
        options: models::ServiceListContainersSegmentOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ServiceListContainersSegmentResponse>;
    async fn getUserDelegationKey(
        &self,
        keyInfo: models::KeyInfo,
        options: models::ServiceGetUserDelegationKeyOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ServiceGetUserDelegationKeyResponse>;
    async fn getAccountInfo(
        &self,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ServiceGetAccountInfoResponse>;
    async fn submitBatch(
        &self,
        body: GeneratedReadableStream,
        contentLength: f64,
        multipartContentType: String,
        options: models::ServiceSubmitBatchOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ServiceSubmitBatchResponse>;
    async fn filterBlobs(
        &self,
        options: models::ServiceFilterBlobsOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ServiceFilterBlobsResponse>;
}
