use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::errors::StorageError;
use crate::generated::artifacts::models::{
    AccessPolicy, AppendBlobSealOptionalParams, AppendPositionAccessConditions,
    BlobAcquireLeaseOptionalParams, BlobBreakLeaseOptionalParams, BlobChangeLeaseOptionalParams,
    BlobCopyFromURLOptionalParams, BlobDeleteMethodOptionalParams, BlobHTTPHeaders, BlobMetadata,
    BlobPropertiesInternal, BlobReleaseLeaseOptionalParams, BlobRenewLeaseOptionalParams,
    BlobStartCopyFromURLOptionalParams, BlobTags, Block, ContainerAcquireLeaseOptionalParams,
    ContainerBreakLeaseOptionalParams, ContainerChangeLeaseOptionalParams,
    ContainerDeleteMethodOptionalParams, ContainerProperties, ContainerReleaseLeaseOptionalParams,
    ContainerRenewLeaseOptionalParams, GeneratedObject, LeaseAccessConditions,
    ModifiedAccessConditions, PageRange, PublicAccessType, SequenceNumberAccessConditions,
    SignedIdentifier, StorageServiceProperties,
};
use crate::generated::context::Context;

// ─── IExtentChunk ───────────────────────────────────────────────────────────

/// Mirrors TypeScript `IExtentChunk`.
/// A chunk inside a persistency extent for a given extent ID.
#[derive(Clone, Debug, Default)]
pub struct IExtentChunk {
    /// The persistency layer storage extent ID where the chunk belongs to
    pub id: String,
    /// Chunk offset inside the extent where chunk starts in bytes
    pub offset: i64,
    /// Chunk length in bytes
    pub count: i64,
}

pub const ZERO_EXTENT_ID: &str = "*ZERO*";

// ─── Service models ─────────────────────────────────────────────────────────

/// Mirrors `ServicePropertiesModel = StorageServiceProperties & { accountName }`.
#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct ServicePropertiesModel {
    pub accountName: String,
    pub properties: StorageServiceProperties,
}

// ─── Container models ───────────────────────────────────────────────────────

/// Mirrors TypeScript `ContainerItem & IContainerAdditionalProperties`.
#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct ContainerModel {
    pub properties: ContainerProperties,
    pub leaseId: Option<String>,
    pub leaseDurationSeconds: Option<i64>,
    pub leaseExpireTime: Option<DateTime<Utc>>,
    pub leaseBreakTime: Option<DateTime<Utc>>,
    pub accountName: String,
    pub containerAcl: Option<Vec<SignedIdentifier>>,
    pub name: Option<String>,
    pub metadata: Option<IContainerMetadata>,
}

/// `IContainerMetadata` — simple string→string map.
pub type IContainerMetadata = std::collections::HashMap<String, String>;

pub type GetContainerPropertiesResponse = GeneratedObject;

#[derive(Clone, Debug, Default)]
pub struct GetContainerAccessPolicyResponse {
    pub properties: ContainerProperties,
    pub containerAcl: Option<Vec<SignedIdentifier>>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct SetContainerAccessPolicyOptions {
    pub lastModified: Option<DateTime<Utc>>,
    pub etag: Option<String>,
    pub containerAcl: Option<Vec<SignedIdentifier>>,
    pub publicAccess: Option<PublicAccessType>,
    pub leaseAccessConditions: Option<LeaseAccessConditions>,
    pub modifiedAccessConditions: Option<ModifiedAccessConditions>,
}

// Container lease response types
#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct ContainerLeaseResponse {
    pub properties: ContainerProperties,
    pub leaseId: Option<String>,
    pub leaseTime: Option<i64>,
}

pub type AcquireContainerLeaseResponse = ContainerLeaseResponse;
pub type ReleaseContainerLeaseResponse = ContainerProperties;
pub type RenewContainerLeaseResponse = ContainerLeaseResponse;
pub type BreakContainerLeaseResponse = ContainerLeaseResponse;
pub type ChangeContainerLeaseResponse = ContainerLeaseResponse;

// ─── Blob models ────────────────────────────────────────────────────────────

/// Mirrors TypeScript `PersistencyPageRange = IPersistencyPropertiesRequired & Models.PageRange`.
#[derive(Clone, Debug, Default)]
pub struct PersistencyPageRange {
    pub persistency: IExtentChunk,
    pub range: PageRange,
}

/// Mirrors TypeScript `IBlobAdditionalProperties & IPageBlobAdditionalProperties &
/// IBlockBlobAdditionalProperties & BlobItemInternal & IPersistencyPropertiesOptional`.
#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct BlobModel {
    pub properties: BlobPropertiesInternal,
    pub leaseId: Option<String>,
    pub leaseDurationSeconds: Option<i64>,
    pub leaseExpireTime: Option<DateTime<Utc>>,
    pub leaseBreakTime: Option<DateTime<Utc>>,
    pub accountName: String,
    pub containerName: String,

    /// BlobItemInternal fields
    pub name: Option<String>,
    pub snapshot: Option<String>,
    pub deleted: Option<bool>,
    pub metadata: Option<BlobMetadata>,
    pub blobTags: Option<BlobTags>,

    /// Block blob additional properties
    pub isCommitted: Option<bool>,
    pub committedBlocksInOrder: Option<Vec<PersistencyBlockModel>>,

    /// Page blob additional properties
    pub pageRangesInOrder: Option<Vec<PersistencyPageRange>>,

    /// Persistency (optional)
    pub persistency: Option<IExtentChunk>,
}

/// Mirrors `BlobPrefixModel = IPersistencyPropertiesOptional & Models.BlobPrefix`.
#[derive(Clone, Debug, Default)]
pub struct BlobPrefixModel {
    pub name: String,
    pub persistency: Option<IExtentChunk>,
}

/// Mirrors `GetBlobPropertiesRes`.
#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct GetBlobPropertiesRes {
    pub properties: BlobPropertiesInternal,
    pub metadata: Option<BlobMetadata>,
    pub blobCommittedBlockCount: Option<i64>,
}

/// Mirrors `FilterBlobModel = FilterBlobItem`.
/// Kept as a concrete struct to match TS shape used by conditions and query interpreter.
#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct FilterBlobModel {
    pub name: String,
    pub containerName: String,
    pub tags: Option<BlobTags>,
}

// Blob lease response types
#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct BlobLeaseResponse {
    pub properties: BlobPropertiesInternal,
    pub leaseId: Option<String>,
    pub leaseTime: Option<i64>,
}

pub type AcquireBlobLeaseResponse = BlobLeaseResponse;
pub type ReleaseBlobLeaseResponse = ContainerProperties;
pub type RenewBlobLeaseResponse = BlobLeaseResponse;
pub type BreakBlobLeaseResponse = BlobLeaseResponse;
pub type ChangeBlobLeaseResponse = BlobLeaseResponse;

/// Mirrors `CreateSnapshotResponse`.
#[derive(Clone, Debug, Default)]
pub struct CreateSnapshotResponse {
    pub properties: BlobPropertiesInternal,
    pub snapshot: String,
}

/// Mirrors `BlobId`.
#[derive(Clone, Debug, Default)]
pub struct BlobId {
    pub account: String,
    pub container: String,
    pub blob: String,
    pub snapshot: Option<String>,
}

/// Mirrors `GetPageRangeResponse`.
#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct GetPageRangeResponse {
    pub pageRangesInOrder: Option<Vec<PersistencyPageRange>>,
    pub properties: BlobPropertiesInternal,
}

// ─── Block models ───────────────────────────────────────────────────────────

/// Mirrors `PersistencyBlockModel = Models.Block & IPersistencyPropertiesRequired`.
#[derive(Clone, Debug, Default)]
pub struct PersistencyBlockModel {
    pub name: Option<String>,
    pub size: Option<i64>,
    pub persistency: IExtentChunk,
}

/// Mirrors `BlockModel = IBlockAdditionalProperties & PersistencyBlockModel`.
#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct BlockModel {
    pub accountName: String,
    pub containerName: String,
    pub blobName: String,
    pub isCommitted: bool,
    pub name: Option<String>,
    pub size: Option<i64>,
    pub persistency: IExtentChunk,
}

// ─── BlobTypeResult (kept from Phase 8) ─────────────────────────────────────

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct BlobTypeResult {
    pub blobType: Option<String>,
    pub isCommitted: bool,
}

/// Block list entry used in commitBlockList.
#[allow(non_snake_case)]
#[derive(Clone, Debug)]
pub struct BlockListEntry {
    pub blockName: String,
    pub blockCommitType: String,
}

// ─── IBlobMetadataStore trait ───────────────────────────────────────────────

/// Mirrors TypeScript `IBlobMetadataStore extends IGCExtentProvider, IDataStore, ICleaner`.
///
/// Full interface surface of the blob metadata persistence layer.
#[async_trait]
#[allow(non_snake_case)]
pub trait IBlobMetadataStore: Send + Sync {
    // ── Service ──
    async fn setServiceProperties(
        &self,
        context: &Context,
        serviceProperties: ServicePropertiesModel,
    ) -> Result<ServicePropertiesModel, StorageError>;

    async fn getServiceProperties(
        &self,
        context: &Context,
        account: &str,
    ) -> Result<Option<ServicePropertiesModel>, StorageError>;

    // ── Containers ──
    async fn listContainers(
        &self,
        context: &Context,
        account: &str,
        prefix: Option<&str>,
        maxResults: Option<i64>,
        marker: Option<&str>,
    ) -> Result<(Vec<ContainerModel>, Option<String>), StorageError>;

    async fn createContainer(
        &self,
        context: &Context,
        container: ContainerModel,
    ) -> Result<ContainerModel, StorageError>;

    async fn getContainerProperties(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
    ) -> Result<GetContainerPropertiesResponse, StorageError>;

    async fn deleteContainer(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        options: Option<&ContainerDeleteMethodOptionalParams>,
    ) -> Result<(), StorageError>;

    async fn setContainerMetadata(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        lastModified: DateTime<Utc>,
        etag: &str,
        metadata: Option<&IContainerMetadata>,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<(), StorageError>;

    async fn getContainerACL(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
    ) -> Result<Option<GetContainerAccessPolicyResponse>, StorageError>;

    async fn setContainerACL(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        setAclModel: SetContainerAccessPolicyOptions,
    ) -> Result<(), StorageError>;

    // ── Container leases ──
    async fn acquireContainerLease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        options: Option<&ContainerAcquireLeaseOptionalParams>,
    ) -> Result<AcquireContainerLeaseResponse, StorageError>;

    async fn releaseContainerLease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        leaseId: &str,
        options: Option<&ContainerReleaseLeaseOptionalParams>,
    ) -> Result<ReleaseContainerLeaseResponse, StorageError>;

    async fn renewContainerLease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        leaseId: &str,
        options: Option<&ContainerRenewLeaseOptionalParams>,
    ) -> Result<RenewContainerLeaseResponse, StorageError>;

    async fn breakContainerLease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        breakPeriod: Option<i64>,
        options: Option<&ContainerBreakLeaseOptionalParams>,
    ) -> Result<BreakContainerLeaseResponse, StorageError>;

    async fn changeContainerLease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        leaseId: &str,
        proposedLeaseId: &str,
        options: Option<&ContainerChangeLeaseOptionalParams>,
    ) -> Result<ChangeContainerLeaseResponse, StorageError>;

    async fn checkContainerExist(
        &self,
        context: &Context,
        account: &str,
        container: &str,
    ) -> Result<(), StorageError>;

    // ── Blob listing / filtering ──
    async fn listBlobs(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        delimiter: Option<&str>,
        blob: Option<&str>,
        prefix: Option<&str>,
        maxResults: Option<i64>,
        marker: Option<&str>,
        includeSnapshots: Option<bool>,
        includeUncommittedBlobs: Option<bool>,
    ) -> Result<(Vec<BlobModel>, Vec<BlobPrefixModel>, Option<String>), StorageError>;

    async fn listAllBlobs(
        &self,
        maxResults: Option<i64>,
        marker: Option<&str>,
        includeSnapshots: Option<bool>,
        includeUncommittedBlobs: Option<bool>,
    ) -> Result<(Vec<BlobModel>, Option<String>), StorageError>;

    async fn filterBlobs(
        &self,
        context: &Context,
        account: &str,
        container: Option<&str>,
        r#where: Option<&str>,
        maxResults: Option<i64>,
        marker: Option<&str>,
    ) -> Result<(Vec<FilterBlobModel>, Option<String>), StorageError>;

    // ── Blob CRUD ──
    async fn createBlob(
        &self,
        context: &Context,
        blob: BlobModel,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<(), StorageError>;

    async fn createSnapshot(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
        metadata: Option<&BlobMetadata>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<CreateSnapshotResponse, StorageError>;

    async fn downloadBlob(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<BlobModel, StorageError>;

    async fn getBlobProperties(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<GetBlobPropertiesRes, StorageError>;

    async fn deleteBlob(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        options: BlobDeleteMethodOptionalParams,
    ) -> Result<(), StorageError>;

    async fn setBlobHTTPHeaders(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
        blobHTTPHeaders: Option<&BlobHTTPHeaders>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError>;

    async fn setBlobMetadata(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
        metadata: Option<&BlobMetadata>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError>;

    async fn checkBlobExist(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
    ) -> Result<(), StorageError>;

    async fn getBlobType(
        &self,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
    ) -> Result<Option<BlobTypeResult>, StorageError>;

    // ── Blob copy / tier ──
    async fn startCopyFromURL(
        &self,
        context: &Context,
        source: BlobId,
        destination: BlobId,
        copySource: &str,
        metadata: Option<&BlobMetadata>,
        tier: Option<&str>,
        leaseAccessConditions: Option<&BlobStartCopyFromURLOptionalParams>,
    ) -> Result<BlobPropertiesInternal, StorageError>;

    async fn copyFromURL(
        &self,
        context: &Context,
        source: BlobId,
        destination: BlobId,
        copySource: &str,
        metadata: Option<&BlobMetadata>,
        tier: Option<&str>,
        leaseAccessConditions: Option<&BlobCopyFromURLOptionalParams>,
    ) -> Result<BlobPropertiesInternal, StorageError>;

    async fn setTier(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        tier: &str,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
    ) -> Result<u16, StorageError>; // Returns 200 or 202

    // ── Blob leases ──
    async fn acquireBlobLease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        duration: i64,
        proposedLeaseId: Option<&str>,
        options: Option<&BlobAcquireLeaseOptionalParams>,
    ) -> Result<AcquireBlobLeaseResponse, StorageError>;

    async fn releaseBlobLease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        leaseId: &str,
        options: Option<&BlobReleaseLeaseOptionalParams>,
    ) -> Result<ReleaseBlobLeaseResponse, StorageError>;

    async fn renewBlobLease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        leaseId: &str,
        options: Option<&BlobRenewLeaseOptionalParams>,
    ) -> Result<RenewBlobLeaseResponse, StorageError>;

    async fn changeBlobLease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        leaseId: &str,
        proposedLeaseId: &str,
        option: Option<&BlobChangeLeaseOptionalParams>,
    ) -> Result<ChangeBlobLeaseResponse, StorageError>;

    async fn breakBlobLease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        breakPeriod: Option<i64>,
        option: Option<&BlobBreakLeaseOptionalParams>,
    ) -> Result<BreakBlobLeaseResponse, StorageError>;

    // ── Block / Page / Append ──
    async fn stageBlock(
        &self,
        context: &Context,
        block: BlockModel,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
    ) -> Result<(), StorageError>;

    async fn appendBlock(
        &self,
        context: &Context,
        block: BlockModel,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
        appendPositionAccessConditions: Option<&AppendPositionAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError>;

    async fn commitBlockList(
        &self,
        context: &Context,
        blob: BlobModel,
        blockList: Vec<BlockListEntry>,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<(), StorageError>;

    async fn getBlockList(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
        isCommitted: Option<bool>,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<GetBlockListResult, StorageError>;

    async fn uploadPages(
        &self,
        context: &Context,
        blob: BlobModel,
        start: i64,
        end: i64,
        persistency: IExtentChunk,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
        sequenceNumberAccessConditions: Option<&SequenceNumberAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError>;

    async fn clearRange(
        &self,
        context: &Context,
        blob: BlobModel,
        start: i64,
        end: i64,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
        sequenceNumberAccessConditions: Option<&SequenceNumberAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError>;

    async fn getPageRanges(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<GetPageRangeResponse, StorageError>;

    async fn resizePageBlob(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        blobContentLength: i64,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError>;

    async fn updateSequenceNumber(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        sequenceNumberAction: &str,
        blobSequenceNumber: Option<i64>,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError>;

    async fn listUncommittedBlockPersistencyChunks(
        &self,
        marker: Option<&str>,
        maxResults: Option<i64>,
    ) -> Result<(Vec<IExtentChunk>, Option<String>), StorageError>;

    // ── Tags and seal ──
    async fn setBlobTag(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
        tags: Option<&BlobTags>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<(), StorageError>;

    async fn getBlobTag(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<Option<BlobTags>, StorageError>;

    async fn sealBlob(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
        options: AppendBlobSealOptionalParams,
    ) -> Result<BlobPropertiesInternal, StorageError>;
}

/// Result from `getBlockList`.
#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct GetBlockListResult {
    pub properties: BlobPropertiesInternal,
    pub uncommittedBlocks: Vec<Block>,
    pub committedBlocks: Vec<Block>,
}

// ─── Helper accessors (preserved from Phase 8) ─────────────────────────────

pub fn signed_identifier_id(identifier: &SignedIdentifier) -> Option<String> {
    identifier.get("id").and_then(|value| value.as_string())
}

pub fn signed_identifier_access_policy(identifier: &SignedIdentifier) -> Option<AccessPolicy> {
    identifier
        .get("accessPolicy")
        .and_then(|value| value.as_object())
        .cloned()
}

pub fn access_policy_field(accessPolicy: &AccessPolicy, key: &str) -> Option<String> {
    accessPolicy.get(key).and_then(|value| value.as_string())
}

pub fn container_public_access(properties: &ContainerProperties) -> Option<String> {
    properties
        .get("publicAccess")
        .and_then(|value| value.as_string())
}
