use async_trait::async_trait;

use crate::generated::artifacts::models;
use crate::generated::context::Context;
use crate::generated::i_request::GeneratedReadableStream;

#[allow(non_snake_case)]
#[async_trait]
pub trait IBlobHandler: Send + Sync {
    async fn download(
        &self,
        options: models::BlobDownloadOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobDownloadResponse>;
    async fn getProperties(
        &self,
        options: models::BlobGetPropertiesOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobGetPropertiesResponse>;
    async fn delete(
        &self,
        options: models::BlobDeleteMethodOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobDeleteResponse>;
    async fn undelete(
        &self,
        options: models::BlobUndeleteOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobUndeleteResponse>;
    async fn setExpiry(
        &self,
        expiryOptions: models::BlobExpiryOptions,
        options: models::BlobSetExpiryOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobSetExpiryResponse>;
    async fn setHTTPHeaders(
        &self,
        options: models::BlobSetHTTPHeadersOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobSetHTTPHeadersResponse>;
    async fn setImmutabilityPolicy(
        &self,
        options: models::BlobSetImmutabilityPolicyOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobSetImmutabilityPolicyResponse>;
    async fn deleteImmutabilityPolicy(
        &self,
        options: models::BlobDeleteImmutabilityPolicyOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobDeleteImmutabilityPolicyResponse>;
    async fn setLegalHold(
        &self,
        legalHold: bool,
        options: models::BlobSetLegalHoldOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobSetLegalHoldResponse>;
    async fn setMetadata(
        &self,
        options: models::BlobSetMetadataOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobSetMetadataResponse>;
    async fn acquireLease(
        &self,
        options: models::BlobAcquireLeaseOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobAcquireLeaseResponse>;
    async fn releaseLease(
        &self,
        leaseId: String,
        options: models::BlobReleaseLeaseOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobReleaseLeaseResponse>;
    async fn renewLease(
        &self,
        leaseId: String,
        options: models::BlobRenewLeaseOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobRenewLeaseResponse>;
    async fn changeLease(
        &self,
        leaseId: String,
        proposedLeaseId: String,
        options: models::BlobChangeLeaseOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobChangeLeaseResponse>;
    async fn breakLease(
        &self,
        options: models::BlobBreakLeaseOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobBreakLeaseResponse>;
    async fn createSnapshot(
        &self,
        options: models::BlobCreateSnapshotOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobCreateSnapshotResponse>;
    async fn startCopyFromURL(
        &self,
        copySource: String,
        options: models::BlobStartCopyFromURLOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobStartCopyFromURLResponse>;
    async fn copyFromURL(
        &self,
        copySource: String,
        options: models::BlobCopyFromURLOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobCopyFromURLResponse>;
    async fn abortCopyFromURL(
        &self,
        copyId: String,
        options: models::BlobAbortCopyFromURLOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobAbortCopyFromURLResponse>;
    async fn setTier(
        &self,
        tier: models::AccessTier,
        options: models::BlobSetTierOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobSetTierResponse>;
    async fn getAccountInfo(
        &self,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobGetAccountInfoResponse>;
    async fn query(
        &self,
        options: models::BlobQueryOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobQueryResponse>;
    async fn getTags(
        &self,
        options: models::BlobGetTagsOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobGetTagsResponse>;
    async fn setTags(
        &self,
        options: models::BlobSetTagsOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlobSetTagsResponse>;
}
