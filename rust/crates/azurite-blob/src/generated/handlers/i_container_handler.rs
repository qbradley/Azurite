use async_trait::async_trait;

use crate::generated::artifacts::models;
use crate::generated::context::Context;
use crate::generated::i_request::GeneratedReadableStream;

#[allow(non_snake_case)]
#[async_trait]
pub trait IContainerHandler: Send + Sync {
    async fn create(
        &self,
        options: models::ContainerCreateOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ContainerCreateResponse>;
    async fn getProperties(
        &self,
        options: models::ContainerGetPropertiesOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ContainerGetPropertiesResponse>;
    async fn getPropertiesWithHead(
        &self,
        options: models::ContainerGetPropertiesWithHeadOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ContainerGetPropertiesWithHeadResponse>;
    async fn delete(
        &self,
        options: models::ContainerDeleteMethodOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ContainerDeleteResponse>;
    async fn setMetadata(
        &self,
        options: models::ContainerSetMetadataOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ContainerSetMetadataResponse>;
    async fn getAccessPolicy(
        &self,
        options: models::ContainerGetAccessPolicyOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ContainerGetAccessPolicyResponse>;
    async fn setAccessPolicy(
        &self,
        options: models::ContainerSetAccessPolicyOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ContainerSetAccessPolicyResponse>;
    async fn restore(
        &self,
        options: models::ContainerRestoreOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ContainerRestoreResponse>;
    async fn submitBatch(
        &self,
        body: GeneratedReadableStream,
        contentLength: f64,
        multipartContentType: String,
        options: models::ContainerSubmitBatchOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ContainerSubmitBatchResponse>;
    async fn filterBlobs(
        &self,
        options: models::ContainerFilterBlobsOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ContainerFilterBlobsResponse>;
    async fn acquireLease(
        &self,
        options: models::ContainerAcquireLeaseOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ContainerAcquireLeaseResponse>;
    async fn releaseLease(
        &self,
        leaseId: String,
        options: models::ContainerReleaseLeaseOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ContainerReleaseLeaseResponse>;
    async fn renewLease(
        &self,
        leaseId: String,
        options: models::ContainerRenewLeaseOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ContainerRenewLeaseResponse>;
    async fn breakLease(
        &self,
        options: models::ContainerBreakLeaseOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ContainerBreakLeaseResponse>;
    async fn changeLease(
        &self,
        leaseId: String,
        proposedLeaseId: String,
        options: models::ContainerChangeLeaseOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ContainerChangeLeaseResponse>;
    async fn listBlobFlatSegment(
        &self,
        options: models::ContainerListBlobFlatSegmentOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ContainerListBlobFlatSegmentResponse>;
    async fn listBlobHierarchySegment(
        &self,
        delimiter: String,
        options: models::ContainerListBlobHierarchySegmentOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ContainerListBlobHierarchySegmentResponse>;
    async fn getAccountInfo(
        &self,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::ContainerGetAccountInfoResponse>;
}
