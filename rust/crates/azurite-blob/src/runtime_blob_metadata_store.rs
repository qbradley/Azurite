use std::{collections::HashMap, sync::Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{
    errors::StorageError,
    generated::artifacts::models::{
        AppendBlobSealOptionalParams, AppendPositionAccessConditions,
        BlobAcquireLeaseOptionalParams, BlobBreakLeaseOptionalParams,
        BlobChangeLeaseOptionalParams, BlobCopyFromURLOptionalParams,
        BlobDeleteMethodOptionalParams, BlobHTTPHeaders, BlobMetadata, BlobPropertiesInternal,
        BlobReleaseLeaseOptionalParams, BlobRenewLeaseOptionalParams,
        BlobStartCopyFromURLOptionalParams, BlobTags, LeaseAccessConditions,
        ModifiedAccessConditions, SequenceNumberAccessConditions,
    },
    generated::context::Context,
    persistence::{
        AcquireBlobLeaseResponse, AcquireContainerLeaseResponse, BlobId, BlobModel,
        BlobPrefixModel, BlobTypeResult, BlockListEntry, BlockModel, BreakBlobLeaseResponse,
        BreakContainerLeaseResponse, ChangeBlobLeaseResponse, ChangeContainerLeaseResponse,
        ContainerModel, CreateSnapshotResponse, FilterBlobModel, GetBlobPropertiesRes,
        GetBlockListResult, GetContainerAccessPolicyResponse, GetContainerPropertiesResponse,
        GetPageRangeResponse, IBlobMetadataStore, IContainerMetadata, IExtentChunk,
        ReleaseBlobLeaseResponse, ReleaseContainerLeaseResponse, RenewBlobLeaseResponse,
        RenewContainerLeaseResponse, ServicePropertiesModel, SetContainerAccessPolicyOptions,
    },
};

fn unsupported() -> StorageError {
    StorageError::new(
        500,
        "NotImplemented",
        "Blob metadata operation is not implemented in the runtime bootstrap store.",
        "runtime-blob-metadata-store",
        StorageError::empty_extra(),
    )
}

#[derive(Default)]
pub struct RuntimeBlobMetadataStore {
    serviceProperties: Mutex<HashMap<String, ServicePropertiesModel>>,
}

impl RuntimeBlobMetadataStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl IBlobMetadataStore for RuntimeBlobMetadataStore {
    async fn setServiceProperties(
        &self,
        _context: &Context,
        serviceProperties: ServicePropertiesModel,
    ) -> Result<ServicePropertiesModel, StorageError> {
        self.serviceProperties.lock().unwrap().insert(
            serviceProperties.accountName.clone(),
            serviceProperties.clone(),
        );
        Ok(serviceProperties)
    }

    async fn getServiceProperties(
        &self,
        _context: &Context,
        account: &str,
    ) -> Result<Option<ServicePropertiesModel>, StorageError> {
        Ok(self.serviceProperties.lock().unwrap().get(account).cloned())
    }

    async fn listContainers(
        &self,
        _context: &Context,
        _account: &str,
        _prefix: Option<&str>,
        _maxResults: Option<i64>,
        _marker: Option<&str>,
    ) -> Result<(Vec<ContainerModel>, Option<String>), StorageError> {
        Ok((Vec::new(), None))
    }

    async fn createContainer(
        &self,
        _context: &Context,
        _container: ContainerModel,
    ) -> Result<ContainerModel, StorageError> {
        Err(unsupported())
    }

    async fn getContainerProperties(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
    ) -> Result<GetContainerPropertiesResponse, StorageError> {
        Err(unsupported())
    }

    async fn deleteContainer(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _options: Option<&crate::generated::artifacts::models::ContainerDeleteMethodOptionalParams>,
    ) -> Result<(), StorageError> {
        Err(unsupported())
    }

    async fn setContainerMetadata(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _lastModified: DateTime<Utc>,
        _etag: &str,
        _metadata: Option<&IContainerMetadata>,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<(), StorageError> {
        Err(unsupported())
    }

    async fn getContainerACL(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
    ) -> Result<Option<GetContainerAccessPolicyResponse>, StorageError> {
        Ok(None)
    }

    async fn setContainerACL(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _setAclModel: SetContainerAccessPolicyOptions,
    ) -> Result<(), StorageError> {
        Err(unsupported())
    }

    async fn acquireContainerLease(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _options: Option<&crate::generated::artifacts::models::ContainerAcquireLeaseOptionalParams>,
    ) -> Result<AcquireContainerLeaseResponse, StorageError> {
        Err(unsupported())
    }

    async fn releaseContainerLease(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _leaseId: &str,
        _options: Option<&crate::generated::artifacts::models::ContainerReleaseLeaseOptionalParams>,
    ) -> Result<ReleaseContainerLeaseResponse, StorageError> {
        Err(unsupported())
    }

    async fn renewContainerLease(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _leaseId: &str,
        _options: Option<&crate::generated::artifacts::models::ContainerRenewLeaseOptionalParams>,
    ) -> Result<RenewContainerLeaseResponse, StorageError> {
        Err(unsupported())
    }

    async fn breakContainerLease(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _breakPeriod: Option<i64>,
        _options: Option<&crate::generated::artifacts::models::ContainerBreakLeaseOptionalParams>,
    ) -> Result<BreakContainerLeaseResponse, StorageError> {
        Err(unsupported())
    }

    async fn changeContainerLease(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _leaseId: &str,
        _proposedLeaseId: &str,
        _options: Option<&crate::generated::artifacts::models::ContainerChangeLeaseOptionalParams>,
    ) -> Result<ChangeContainerLeaseResponse, StorageError> {
        Err(unsupported())
    }

    async fn checkContainerExist(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
    ) -> Result<(), StorageError> {
        Ok(())
    }

    async fn listBlobs(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _delimiter: Option<&str>,
        _blob: Option<&str>,
        _prefix: Option<&str>,
        _maxResults: Option<i64>,
        _marker: Option<&str>,
        _includeSnapshots: Option<bool>,
        _includeUncommittedBlobs: Option<bool>,
    ) -> Result<(Vec<BlobModel>, Vec<BlobPrefixModel>, Option<String>), StorageError> {
        Ok((Vec::new(), Vec::new(), None))
    }

    async fn listAllBlobs(
        &self,
        _maxResults: Option<i64>,
        _marker: Option<&str>,
        _includeSnapshots: Option<bool>,
        _includeUncommittedBlobs: Option<bool>,
    ) -> Result<(Vec<BlobModel>, Option<String>), StorageError> {
        Ok((Vec::new(), None))
    }

    async fn filterBlobs(
        &self,
        _context: &Context,
        _account: &str,
        _container: Option<&str>,
        _where: Option<&str>,
        _maxResults: Option<i64>,
        _marker: Option<&str>,
    ) -> Result<(Vec<FilterBlobModel>, Option<String>), StorageError> {
        Ok((Vec::new(), None))
    }

    async fn createBlob(
        &self,
        _context: &Context,
        _blob: BlobModel,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<(), StorageError> {
        Err(unsupported())
    }

    async fn createSnapshot(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _metadata: Option<&BlobMetadata>,
        _modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<CreateSnapshotResponse, StorageError> {
        Err(unsupported())
    }

    async fn downloadBlob(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _snapshot: Option<&str>,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<BlobModel, StorageError> {
        Err(unsupported())
    }

    async fn getBlobProperties(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _snapshot: Option<&str>,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<GetBlobPropertiesRes, StorageError> {
        Err(unsupported())
    }

    async fn deleteBlob(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _options: BlobDeleteMethodOptionalParams,
    ) -> Result<(), StorageError> {
        Err(unsupported())
    }

    async fn setBlobHTTPHeaders(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _blobHTTPHeaders: Option<&BlobHTTPHeaders>,
        _modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        Err(unsupported())
    }

    async fn setBlobMetadata(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _metadata: Option<&BlobMetadata>,
        _modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        Err(unsupported())
    }

    async fn checkBlobExist(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _snapshot: Option<&str>,
    ) -> Result<(), StorageError> {
        Ok(())
    }

    async fn getBlobType(
        &self,
        _account: &str,
        _container: &str,
        _blob: &str,
        _snapshot: Option<&str>,
    ) -> Result<Option<BlobTypeResult>, StorageError> {
        Ok(None)
    }

    async fn startCopyFromURL(
        &self,
        _context: &Context,
        _source: BlobId,
        _destination: BlobId,
        _copySource: &str,
        _metadata: Option<&BlobMetadata>,
        _tier: Option<&str>,
        _leaseAccessConditions: Option<&BlobStartCopyFromURLOptionalParams>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        Err(unsupported())
    }

    async fn copyFromURL(
        &self,
        _context: &Context,
        _source: BlobId,
        _destination: BlobId,
        _copySource: &str,
        _metadata: Option<&BlobMetadata>,
        _tier: Option<&str>,
        _leaseAccessConditions: Option<&BlobCopyFromURLOptionalParams>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        Err(unsupported())
    }

    async fn setTier(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _tier: &str,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
    ) -> Result<u16, StorageError> {
        Err(unsupported())
    }

    async fn acquireBlobLease(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _duration: i64,
        _proposedLeaseId: Option<&str>,
        _options: Option<&BlobAcquireLeaseOptionalParams>,
    ) -> Result<AcquireBlobLeaseResponse, StorageError> {
        Err(unsupported())
    }

    async fn releaseBlobLease(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _leaseId: &str,
        _options: Option<&BlobReleaseLeaseOptionalParams>,
    ) -> Result<ReleaseBlobLeaseResponse, StorageError> {
        Err(unsupported())
    }

    async fn renewBlobLease(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _leaseId: &str,
        _options: Option<&BlobRenewLeaseOptionalParams>,
    ) -> Result<RenewBlobLeaseResponse, StorageError> {
        Err(unsupported())
    }

    async fn changeBlobLease(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _leaseId: &str,
        _proposedLeaseId: &str,
        _option: Option<&BlobChangeLeaseOptionalParams>,
    ) -> Result<ChangeBlobLeaseResponse, StorageError> {
        Err(unsupported())
    }

    async fn breakBlobLease(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _breakPeriod: Option<i64>,
        _option: Option<&BlobBreakLeaseOptionalParams>,
    ) -> Result<BreakBlobLeaseResponse, StorageError> {
        Err(unsupported())
    }

    async fn stageBlock(
        &self,
        _context: &Context,
        _block: BlockModel,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
    ) -> Result<(), StorageError> {
        Err(unsupported())
    }

    async fn appendBlock(
        &self,
        _context: &Context,
        _block: BlockModel,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<&ModifiedAccessConditions>,
        _appendPositionAccessConditions: Option<&AppendPositionAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        Err(unsupported())
    }

    async fn commitBlockList(
        &self,
        _context: &Context,
        _blob: BlobModel,
        _blockList: Vec<BlockListEntry>,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<(), StorageError> {
        Err(unsupported())
    }

    async fn getBlockList(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _snapshot: Option<&str>,
        _isCommitted: Option<bool>,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<GetBlockListResult, StorageError> {
        Err(unsupported())
    }

    async fn uploadPages(
        &self,
        _context: &Context,
        _blob: BlobModel,
        _start: i64,
        _end: i64,
        _persistency: IExtentChunk,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<&ModifiedAccessConditions>,
        _sequenceNumberAccessConditions: Option<&SequenceNumberAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        Err(unsupported())
    }

    async fn clearRange(
        &self,
        _context: &Context,
        _blob: BlobModel,
        _start: i64,
        _end: i64,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<&ModifiedAccessConditions>,
        _sequenceNumberAccessConditions: Option<&SequenceNumberAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        Err(unsupported())
    }

    async fn getPageRanges(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _snapshot: Option<&str>,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<GetPageRangeResponse, StorageError> {
        Err(unsupported())
    }

    async fn resizePageBlob(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _blobContentLength: i64,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        Err(unsupported())
    }

    async fn updateSequenceNumber(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _sequenceNumberAction: &str,
        _blobSequenceNumber: Option<i64>,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        Err(unsupported())
    }

    async fn listUncommittedBlockPersistencyChunks(
        &self,
        _marker: Option<&str>,
        _maxResults: Option<i64>,
    ) -> Result<(Vec<IExtentChunk>, Option<String>), StorageError> {
        Ok((Vec::new(), None))
    }

    async fn setBlobTag(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _snapshot: Option<&str>,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _tags: Option<&BlobTags>,
        _modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<(), StorageError> {
        Err(unsupported())
    }

    async fn getBlobTag(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _snapshot: Option<&str>,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<Option<BlobTags>, StorageError> {
        Ok(None)
    }

    async fn sealBlob(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _snapshot: Option<&str>,
        _options: AppendBlobSealOptionalParams,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        Err(unsupported())
    }
}
