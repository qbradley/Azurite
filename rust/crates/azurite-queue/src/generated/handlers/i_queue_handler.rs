use async_trait::async_trait;

use crate::generated::artifacts::models;
use crate::generated::context::Context;

#[allow(non_snake_case)]
#[async_trait]
pub trait IQueueHandler: Send + Sync {
    async fn create(
        &self,
        options: models::QueueCreateOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::QueueCreateResponse>;
    async fn delete(
        &self,
        options: models::QueueDeleteMethodOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::QueueDeleteResponse>;
    async fn getProperties(
        &self,
        options: models::QueueGetPropertiesOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::QueueGetPropertiesResponse>;
    async fn getPropertiesWithHead(
        &self,
        options: models::QueueGetPropertiesWithHeadOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::QueueGetPropertiesWithHeadResponse>;
    async fn setMetadata(
        &self,
        options: models::QueueSetMetadataOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::QueueSetMetadataResponse>;
    async fn getAccessPolicy(
        &self,
        options: models::QueueGetAccessPolicyOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::QueueGetAccessPolicyResponse>;
    async fn getAccessPolicyWithHead(
        &self,
        options: models::QueueGetAccessPolicyWithHeadOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::QueueGetAccessPolicyWithHeadResponse>;
    async fn setAccessPolicy(
        &self,
        options: models::QueueSetAccessPolicyOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::QueueSetAccessPolicyResponse>;
}
