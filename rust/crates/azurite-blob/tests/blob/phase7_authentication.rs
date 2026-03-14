use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use azurite_blob::authentication::{
    generateBlobSASSignature, generateBlobSASSignatureWithUDK, AccountSASAuthenticator,
    BlobSASAuthenticator, BlobSASPermission, BlobSASResourceType, BlobSharedKeyAuthenticator,
    BlobTokenAuthenticator, ContainerSASPermission, IAuthenticator, IBlobSASSignatureValues,
    IIPRangeOrString, OperationAccountSASPermission, OperationBlobSASPermission,
    PublicAccessAuthenticator, OPERATION_ACCOUNT_SAS_PERMISSIONS,
    OPERATION_BLOB_SAS_BLOB_PERMISSIONS, OPERATION_BLOB_SAS_CONTAINER_PERMISSIONS,
};
use azurite_blob::context::BlobStorageContext;
use azurite_blob::errors::{StorageError, StorageErrorFactory};
use azurite_blob::generated::artifacts::models::LeaseAccessConditions;
use azurite_blob::generated::artifacts::models::{
    AccessPolicy, ContainerProperties, GeneratedValue, SignedIdentifier,
};
use azurite_blob::generated::artifacts::operation::Operation;
use azurite_blob::generated::context::Context;
use azurite_blob::generated::i_request::{GeneratedHttpRequest, HttpMethod, RequestHeaderValue};
use azurite_blob::persistence::{
    BlobTypeResult, GetContainerAccessPolicyResponse, IBlobMetadataStore,
};
use azurite_common::authentication::DateOrString as CommonDateOrString;
use azurite_common::i_account_data_store::{IAccountDataStore, IAccountProperties};
use azurite_common::i_cleaner::ICleaner;
use azurite_common::i_data_store::IDataStore;
use azurite_common::i_logger::ILogger;
use azurite_common::models::OAuthLevel;
use azurite_common::storage_error::StorageError as CommonStorageError;
use azurite_common::utils::utils::computeHMACSHA256;
use azurite_common::{
    generateAccountSASSignature, AccountSASPermissionsOrString, AccountSASResourceTypesOrString,
    AccountSASServicesOrString, IAccountSASSignatureValues, SASProtocol, SASProtocolOrString,
};
use base64::{
    engine::general_purpose::STANDARD, engine::general_purpose::URL_SAFE_NO_PAD, Engine as _,
};
use chrono::{TimeZone, Utc};
use pretty_assertions::assert_eq;
use serde_json::json;

const ACCOUNT: &str = "devstoreaccount1";
const CONTAINER: &str = "container";
const BLOB: &str = "blob.txt";
const SHARED_KEY: &[u8] = b"phase7-shared-key";
const SECONDARY_SUFFIX: &str = "-secondary";
const USER_DELEGATION_BASIC_KEY: &str =
    "I17GKLvcJUossaebtsEDZZ2RJ8GNLwLH4m7hRMxbVbkx6wNIRAABj4Rtw0FBhFuEAgmbL4gFMzUw+AStz9Sqdg==";

#[derive(Default)]
struct NoopLogger;

impl ILogger for NoopLogger {
    fn error(&self, _message: &str, _contextID: Option<&str>) {}
    fn warn(&self, _message: &str, _contextID: Option<&str>) {}
    fn info(&self, _message: &str, _contextID: Option<&str>) {}
    fn verbose(&self, _message: &str, _contextID: Option<&str>) {}
    fn debug(&self, _message: &str, _contextID: Option<&str>) {}
}

#[derive(Clone, Default)]
struct TestAccountStore {
    accounts: HashMap<String, IAccountProperties>,
}

impl TestAccountStore {
    fn with_account(mut self, name: &str, key1: &[u8], key2: Option<&[u8]>) -> Self {
        self.accounts.insert(
            name.to_owned(),
            IAccountProperties {
                name: name.to_owned(),
                key1: key1.to_vec(),
                key2: key2.map(|value| value.to_vec()),
            },
        );
        self
    }
}

#[async_trait]
impl IDataStore for TestAccountStore {
    async fn init(&mut self) -> Result<(), CommonStorageError> {
        Ok(())
    }

    fn isInitialized(&self) -> bool {
        true
    }

    async fn close(&mut self) -> Result<(), CommonStorageError> {
        Ok(())
    }

    fn isClosed(&self) -> bool {
        false
    }
}

#[async_trait]
impl ICleaner for TestAccountStore {
    async fn clean(&mut self) -> Result<(), CommonStorageError> {
        Ok(())
    }
}

impl IAccountDataStore for TestAccountStore {
    fn getAccount(&self, name: &str) -> Option<IAccountProperties> {
        self.accounts.get(name).cloned()
    }
}

#[derive(Default)]
struct TestBlobMetadataStore {
    container_acls: Mutex<
        HashMap<(String, String), Result<Option<GetContainerAccessPolicyResponse>, StorageError>>,
    >,
    blob_types:
        Mutex<HashMap<(String, String, String), Result<Option<BlobTypeResult>, StorageError>>>,
}

impl TestBlobMetadataStore {
    fn with_container_acl(
        self,
        account: &str,
        container: &str,
        response: Result<Option<GetContainerAccessPolicyResponse>, StorageError>,
    ) -> Self {
        self.container_acls
            .lock()
            .unwrap()
            .insert((account.to_owned(), container.to_owned()), response);
        self
    }

    fn with_blob_type(
        self,
        account: &str,
        container: &str,
        blob: &str,
        response: Result<Option<BlobTypeResult>, StorageError>,
    ) -> Self {
        self.blob_types.lock().unwrap().insert(
            (account.to_owned(), container.to_owned(), blob.to_owned()),
            response,
        );
        self
    }
}

#[async_trait]
impl IBlobMetadataStore for TestBlobMetadataStore {
    // ── Implemented methods used by tests ──
    async fn getContainerACL(
        &self,
        _context: &Context,
        account: &str,
        container: &str,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
    ) -> Result<Option<GetContainerAccessPolicyResponse>, StorageError> {
        self.container_acls
            .lock()
            .unwrap()
            .get(&(account.to_owned(), container.to_owned()))
            .cloned()
            .unwrap_or(Ok(None))
    }

    async fn getBlobType(
        &self,
        account: &str,
        container: &str,
        blob: &str,
        _snapshot: Option<&str>,
    ) -> Result<Option<BlobTypeResult>, StorageError> {
        self.blob_types
            .lock()
            .unwrap()
            .get(&(account.to_owned(), container.to_owned(), blob.to_owned()))
            .cloned()
            .unwrap_or(Ok(None))
    }

    // ── Stub implementations (not used by these tests) ──
    async fn setServiceProperties(
        &self,
        _context: &Context,
        _serviceProperties: azurite_blob::persistence::ServicePropertiesModel,
    ) -> Result<azurite_blob::persistence::ServicePropertiesModel, StorageError> {
        unimplemented!("setServiceProperties not used in phase7 tests")
    }
    async fn getServiceProperties(
        &self,
        _context: &Context,
        _account: &str,
    ) -> Result<Option<azurite_blob::persistence::ServicePropertiesModel>, StorageError> {
        unimplemented!("getServiceProperties not used in phase7 tests")
    }
    async fn listContainers(
        &self,
        _context: &Context,
        _account: &str,
        _prefix: Option<&str>,
        _maxResults: Option<i64>,
        _marker: Option<&str>,
    ) -> Result<
        (
            Vec<azurite_blob::persistence::ContainerModel>,
            Option<String>,
        ),
        StorageError,
    > {
        unimplemented!("listContainers not used in phase7 tests")
    }
    async fn createContainer(
        &self,
        _context: &Context,
        _container: azurite_blob::persistence::ContainerModel,
    ) -> Result<azurite_blob::persistence::ContainerModel, StorageError> {
        unimplemented!("createContainer not used in phase7 tests")
    }
    async fn getContainerProperties(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
    ) -> Result<azurite_blob::persistence::GetContainerPropertiesResponse, StorageError> {
        unimplemented!("getContainerProperties not used in phase7 tests")
    }
    async fn deleteContainer(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _options: Option<
            &azurite_blob::generated::artifacts::models::ContainerDeleteMethodOptionalParams,
        >,
    ) -> Result<(), StorageError> {
        unimplemented!("deleteContainer not used in phase7 tests")
    }
    async fn setContainerMetadata(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _lastModified: chrono::DateTime<chrono::Utc>,
        _etag: &str,
        _metadata: Option<&azurite_blob::persistence::IContainerMetadata>,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<
            &azurite_blob::generated::artifacts::models::ModifiedAccessConditions,
        >,
    ) -> Result<(), StorageError> {
        unimplemented!("setContainerMetadata not used in phase7 tests")
    }
    async fn setContainerACL(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _setAclModel: azurite_blob::persistence::SetContainerAccessPolicyOptions,
    ) -> Result<(), StorageError> {
        unimplemented!("setContainerACL not used in phase7 tests")
    }
    async fn acquireContainerLease(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _options: Option<
            &azurite_blob::generated::artifacts::models::ContainerAcquireLeaseOptionalParams,
        >,
    ) -> Result<azurite_blob::persistence::AcquireContainerLeaseResponse, StorageError> {
        unimplemented!("acquireContainerLease not used in phase7 tests")
    }
    async fn releaseContainerLease(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _leaseId: &str,
        _options: Option<
            &azurite_blob::generated::artifacts::models::ContainerReleaseLeaseOptionalParams,
        >,
    ) -> Result<azurite_blob::persistence::ReleaseContainerLeaseResponse, StorageError> {
        unimplemented!("releaseContainerLease not used in phase7 tests")
    }
    async fn renewContainerLease(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _leaseId: &str,
        _options: Option<
            &azurite_blob::generated::artifacts::models::ContainerRenewLeaseOptionalParams,
        >,
    ) -> Result<azurite_blob::persistence::RenewContainerLeaseResponse, StorageError> {
        unimplemented!("renewContainerLease not used in phase7 tests")
    }
    async fn breakContainerLease(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _breakPeriod: Option<i64>,
        _options: Option<
            &azurite_blob::generated::artifacts::models::ContainerBreakLeaseOptionalParams,
        >,
    ) -> Result<azurite_blob::persistence::BreakContainerLeaseResponse, StorageError> {
        unimplemented!("breakContainerLease not used in phase7 tests")
    }
    async fn changeContainerLease(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _leaseId: &str,
        _proposedLeaseId: &str,
        _options: Option<
            &azurite_blob::generated::artifacts::models::ContainerChangeLeaseOptionalParams,
        >,
    ) -> Result<azurite_blob::persistence::ChangeContainerLeaseResponse, StorageError> {
        unimplemented!("changeContainerLease not used in phase7 tests")
    }
    async fn checkContainerExist(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
    ) -> Result<(), StorageError> {
        unimplemented!("checkContainerExist not used in phase7 tests")
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
    ) -> Result<
        (
            Vec<azurite_blob::persistence::BlobModel>,
            Vec<azurite_blob::persistence::BlobPrefixModel>,
            Option<String>,
        ),
        StorageError,
    > {
        unimplemented!("listBlobs not used in phase7 tests")
    }
    async fn listAllBlobs(
        &self,
        _maxResults: Option<i64>,
        _marker: Option<&str>,
        _includeSnapshots: Option<bool>,
        _includeUncommittedBlobs: Option<bool>,
    ) -> Result<(Vec<azurite_blob::persistence::BlobModel>, Option<String>), StorageError> {
        unimplemented!("listAllBlobs not used in phase7 tests")
    }
    async fn filterBlobs(
        &self,
        _context: &Context,
        _account: &str,
        _container: Option<&str>,
        _where_param: Option<&str>,
        _maxResults: Option<i64>,
        _marker: Option<&str>,
    ) -> Result<
        (
            Vec<azurite_blob::persistence::FilterBlobModel>,
            Option<String>,
        ),
        StorageError,
    > {
        unimplemented!("filterBlobs not used in phase7 tests")
    }
    async fn createBlob(
        &self,
        _context: &Context,
        _blob: azurite_blob::persistence::BlobModel,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<
            &azurite_blob::generated::artifacts::models::ModifiedAccessConditions,
        >,
    ) -> Result<(), StorageError> {
        unimplemented!("createBlob not used in phase7 tests")
    }
    async fn createSnapshot(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _metadata: Option<&azurite_blob::generated::artifacts::models::BlobMetadata>,
        _modifiedAccessConditions: Option<
            &azurite_blob::generated::artifacts::models::ModifiedAccessConditions,
        >,
    ) -> Result<azurite_blob::persistence::CreateSnapshotResponse, StorageError> {
        unimplemented!("createSnapshot not used in phase7 tests")
    }
    async fn downloadBlob(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _snapshot: Option<&str>,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<
            &azurite_blob::generated::artifacts::models::ModifiedAccessConditions,
        >,
    ) -> Result<azurite_blob::persistence::BlobModel, StorageError> {
        unimplemented!("downloadBlob not used in phase7 tests")
    }
    async fn getBlobProperties(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _snapshot: Option<&str>,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<
            &azurite_blob::generated::artifacts::models::ModifiedAccessConditions,
        >,
    ) -> Result<azurite_blob::persistence::GetBlobPropertiesRes, StorageError> {
        unimplemented!("getBlobProperties not used in phase7 tests")
    }
    async fn deleteBlob(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _options: azurite_blob::generated::artifacts::models::BlobDeleteMethodOptionalParams,
    ) -> Result<(), StorageError> {
        unimplemented!("deleteBlob not used in phase7 tests")
    }
    async fn setBlobHTTPHeaders(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _blobHTTPHeaders: Option<&azurite_blob::generated::artifacts::models::BlobHTTPHeaders>,
        _modifiedAccessConditions: Option<
            &azurite_blob::generated::artifacts::models::ModifiedAccessConditions,
        >,
    ) -> Result<azurite_blob::generated::artifacts::models::BlobPropertiesInternal, StorageError>
    {
        unimplemented!("setBlobHTTPHeaders not used in phase7 tests")
    }
    async fn setBlobMetadata(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _metadata: Option<&azurite_blob::generated::artifacts::models::BlobMetadata>,
        _modifiedAccessConditions: Option<
            &azurite_blob::generated::artifacts::models::ModifiedAccessConditions,
        >,
    ) -> Result<azurite_blob::generated::artifacts::models::BlobPropertiesInternal, StorageError>
    {
        unimplemented!("setBlobMetadata not used in phase7 tests")
    }
    async fn checkBlobExist(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _snapshot: Option<&str>,
    ) -> Result<(), StorageError> {
        unimplemented!("checkBlobExist not used in phase7 tests")
    }
    async fn startCopyFromURL(
        &self,
        _context: &Context,
        _source: azurite_blob::persistence::BlobId,
        _destination: azurite_blob::persistence::BlobId,
        _copySource: &str,
        _metadata: Option<&azurite_blob::generated::artifacts::models::BlobMetadata>,
        _tier: Option<&str>,
        _leaseAccessConditions: Option<
            &azurite_blob::generated::artifacts::models::BlobStartCopyFromURLOptionalParams,
        >,
    ) -> Result<azurite_blob::generated::artifacts::models::BlobPropertiesInternal, StorageError>
    {
        unimplemented!("startCopyFromURL not used in phase7 tests")
    }
    async fn copyFromURL(
        &self,
        _context: &Context,
        _source: azurite_blob::persistence::BlobId,
        _destination: azurite_blob::persistence::BlobId,
        _copySource: &str,
        _metadata: Option<&azurite_blob::generated::artifacts::models::BlobMetadata>,
        _tier: Option<&str>,
        _leaseAccessConditions: Option<
            &azurite_blob::generated::artifacts::models::BlobCopyFromURLOptionalParams,
        >,
    ) -> Result<azurite_blob::generated::artifacts::models::BlobPropertiesInternal, StorageError>
    {
        unimplemented!("copyFromURL not used in phase7 tests")
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
        unimplemented!("setTier not used in phase7 tests")
    }
    async fn acquireBlobLease(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _duration: i64,
        _proposedLeaseId: Option<&str>,
        _options: Option<
            &azurite_blob::generated::artifacts::models::BlobAcquireLeaseOptionalParams,
        >,
    ) -> Result<azurite_blob::persistence::AcquireBlobLeaseResponse, StorageError> {
        unimplemented!("acquireBlobLease not used in phase7 tests")
    }
    async fn releaseBlobLease(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _leaseId: &str,
        _options: Option<
            &azurite_blob::generated::artifacts::models::BlobReleaseLeaseOptionalParams,
        >,
    ) -> Result<azurite_blob::persistence::ReleaseBlobLeaseResponse, StorageError> {
        unimplemented!("releaseBlobLease not used in phase7 tests")
    }
    async fn renewBlobLease(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _leaseId: &str,
        _options: Option<&azurite_blob::generated::artifacts::models::BlobRenewLeaseOptionalParams>,
    ) -> Result<azurite_blob::persistence::RenewBlobLeaseResponse, StorageError> {
        unimplemented!("renewBlobLease not used in phase7 tests")
    }
    async fn changeBlobLease(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _leaseId: &str,
        _proposedLeaseId: &str,
        _option: Option<&azurite_blob::generated::artifacts::models::BlobChangeLeaseOptionalParams>,
    ) -> Result<azurite_blob::persistence::ChangeBlobLeaseResponse, StorageError> {
        unimplemented!("changeBlobLease not used in phase7 tests")
    }
    async fn breakBlobLease(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _breakPeriod: Option<i64>,
        _option: Option<&azurite_blob::generated::artifacts::models::BlobBreakLeaseOptionalParams>,
    ) -> Result<azurite_blob::persistence::BreakBlobLeaseResponse, StorageError> {
        unimplemented!("breakBlobLease not used in phase7 tests")
    }
    async fn stageBlock(
        &self,
        _context: &Context,
        _block: azurite_blob::persistence::BlockModel,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
    ) -> Result<(), StorageError> {
        unimplemented!("stageBlock not used in phase7 tests")
    }
    async fn appendBlock(
        &self,
        _context: &Context,
        _block: azurite_blob::persistence::BlockModel,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<
            &azurite_blob::generated::artifacts::models::ModifiedAccessConditions,
        >,
        _appendPositionAccessConditions: Option<
            &azurite_blob::generated::artifacts::models::AppendPositionAccessConditions,
        >,
    ) -> Result<azurite_blob::generated::artifacts::models::BlobPropertiesInternal, StorageError>
    {
        unimplemented!("appendBlock not used in phase7 tests")
    }
    async fn commitBlockList(
        &self,
        _context: &Context,
        _blob: azurite_blob::persistence::BlobModel,
        _blockList: Vec<azurite_blob::persistence::BlockListEntry>,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<
            &azurite_blob::generated::artifacts::models::ModifiedAccessConditions,
        >,
    ) -> Result<(), StorageError> {
        unimplemented!("commitBlockList not used in phase7 tests")
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
        _modifiedAccessConditions: Option<
            &azurite_blob::generated::artifacts::models::ModifiedAccessConditions,
        >,
    ) -> Result<azurite_blob::persistence::GetBlockListResult, StorageError> {
        unimplemented!("getBlockList not used in phase7 tests")
    }
    async fn uploadPages(
        &self,
        _context: &Context,
        _blob: azurite_blob::persistence::BlobModel,
        _start: i64,
        _end: i64,
        _persistency: azurite_blob::persistence::IExtentChunk,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<
            &azurite_blob::generated::artifacts::models::ModifiedAccessConditions,
        >,
        _sequenceNumberAccessConditions: Option<
            &azurite_blob::generated::artifacts::models::SequenceNumberAccessConditions,
        >,
    ) -> Result<azurite_blob::generated::artifacts::models::BlobPropertiesInternal, StorageError>
    {
        unimplemented!("uploadPages not used in phase7 tests")
    }
    async fn clearRange(
        &self,
        _context: &Context,
        _blob: azurite_blob::persistence::BlobModel,
        _start: i64,
        _end: i64,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<
            &azurite_blob::generated::artifacts::models::ModifiedAccessConditions,
        >,
        _sequenceNumberAccessConditions: Option<
            &azurite_blob::generated::artifacts::models::SequenceNumberAccessConditions,
        >,
    ) -> Result<azurite_blob::generated::artifacts::models::BlobPropertiesInternal, StorageError>
    {
        unimplemented!("clearRange not used in phase7 tests")
    }
    async fn getPageRanges(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _snapshot: Option<&str>,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<
            &azurite_blob::generated::artifacts::models::ModifiedAccessConditions,
        >,
    ) -> Result<azurite_blob::persistence::GetPageRangeResponse, StorageError> {
        unimplemented!("getPageRanges not used in phase7 tests")
    }
    async fn resizePageBlob(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _blobContentLength: i64,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<
            &azurite_blob::generated::artifacts::models::ModifiedAccessConditions,
        >,
    ) -> Result<azurite_blob::generated::artifacts::models::BlobPropertiesInternal, StorageError>
    {
        unimplemented!("resizePageBlob not used in phase7 tests")
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
        _modifiedAccessConditions: Option<
            &azurite_blob::generated::artifacts::models::ModifiedAccessConditions,
        >,
    ) -> Result<azurite_blob::generated::artifacts::models::BlobPropertiesInternal, StorageError>
    {
        unimplemented!("updateSequenceNumber not used in phase7 tests")
    }
    async fn listUncommittedBlockPersistencyChunks(
        &self,
        _marker: Option<&str>,
        _maxResults: Option<i64>,
    ) -> Result<(Vec<azurite_blob::persistence::IExtentChunk>, Option<String>), StorageError> {
        unimplemented!("listUncommittedBlockPersistencyChunks not used in phase7 tests")
    }
    async fn setBlobTag(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _snapshot: Option<&str>,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _tags: Option<&azurite_blob::generated::artifacts::models::BlobTags>,
        _modifiedAccessConditions: Option<
            &azurite_blob::generated::artifacts::models::ModifiedAccessConditions,
        >,
    ) -> Result<(), StorageError> {
        unimplemented!("setBlobTag not used in phase7 tests")
    }
    async fn getBlobTag(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _snapshot: Option<&str>,
        _leaseAccessConditions: Option<&LeaseAccessConditions>,
        _modifiedAccessConditions: Option<
            &azurite_blob::generated::artifacts::models::ModifiedAccessConditions,
        >,
    ) -> Result<Option<azurite_blob::generated::artifacts::models::BlobTags>, StorageError> {
        unimplemented!("getBlobTag not used in phase7 tests")
    }
    async fn sealBlob(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _snapshot: Option<&str>,
        _options: azurite_blob::generated::artifacts::models::AppendBlobSealOptionalParams,
    ) -> Result<azurite_blob::generated::artifacts::models::BlobPropertiesInternal, StorageError>
    {
        unimplemented!("sealBlob not used in phase7 tests")
    }
}

#[derive(Clone)]
struct StubAuthenticator {
    result: Result<Option<bool>, StorageError>,
}

#[async_trait]
impl IAuthenticator for StubAuthenticator {
    async fn validate(
        &self,
        _req: &GeneratedHttpRequest,
        _context: &Context,
    ) -> Result<Option<bool>, StorageError> {
        self.result.clone()
    }
}

fn fixed_time() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2022, 4, 16, 13, 31, 48)
        .single()
        .unwrap()
}

fn far_future() -> &'static str {
    "2099-04-16T13:31:48Z"
}

fn far_past() -> &'static str {
    "2020-04-16T13:31:48Z"
}

fn logger() -> Arc<dyn ILogger + Send + Sync> {
    Arc::new(NoopLogger)
}

fn account_store() -> Arc<dyn IAccountDataStore + Send + Sync> {
    Arc::new(TestAccountStore::default().with_account(ACCOUNT, SHARED_KEY, None))
}

fn make_context(operation: Operation, container: Option<&str>, blob: Option<&str>) -> Context {
    let holder = Context::new_holder();
    let context = Context::from_holder(holder, "/", None, None);
    context.setOperation(Some(operation));
    context.setContextId(Some(String::from("phase7-context")));
    context.setStartTime(Some(fixed_time()));

    let blob_context = BlobStorageContext::new(&context);
    blob_context.setAccount(Some(ACCOUNT.to_owned()));
    blob_context.setContainer(container.map(String::from));
    blob_context.setBlob(blob.map(String::from));

    context
}

fn make_request(method: HttpMethod, path: &str) -> GeneratedHttpRequest {
    let request_path = path.split('?').next().unwrap_or(path);
    let mut request = GeneratedHttpRequest::new(
        method,
        format!("https://example.test{path}"),
        "https://example.test",
        request_path,
    );
    request.protocol = String::from("https");
    request
}

fn set_header(request: &mut GeneratedHttpRequest, name: &str, value: &str) {
    request.headers.insert(
        name.to_owned(),
        RequestHeaderValue::Single(value.to_owned()),
    );
}

fn set_query(request: &mut GeneratedHttpRequest, key: &str, value: &str) {
    request.query.insert(key.to_owned(), value.to_owned());
}

fn assert_storage_error(error: &StorageError, status: u16, code: &str, message: &str) {
    assert_eq!(error.statusCode, status, "status");
    assert_eq!(
        Some(error.storageErrorCode.as_str()),
        Some(code),
        "storage error code"
    );
    assert_eq!(error.message, message, "storage error message");
}

fn access_policy(permission: &str, start: &str, expiry: &str) -> AccessPolicy {
    BTreeMap::from([
        (
            String::from("permission"),
            GeneratedValue::String(permission.to_owned()),
        ),
        (
            String::from("start"),
            GeneratedValue::String(start.to_owned()),
        ),
        (
            String::from("expiry"),
            GeneratedValue::String(expiry.to_owned()),
        ),
    ])
}

fn signed_identifier(id: &str, access_policy: AccessPolicy) -> SignedIdentifier {
    BTreeMap::from([
        (String::from("id"), GeneratedValue::String(id.to_owned())),
        (
            String::from("accessPolicy"),
            GeneratedValue::Object(access_policy),
        ),
    ])
}

fn public_access_response(public_access: &str) -> GetContainerAccessPolicyResponse {
    let properties: ContainerProperties = BTreeMap::from([(
        String::from("publicAccess"),
        GeneratedValue::String(public_access.to_owned()),
    )]);
    GetContainerAccessPolicyResponse {
        properties,
        containerAcl: None,
    }
}

fn account_sas_values(permissions: &str) -> IAccountSASSignatureValues {
    IAccountSASSignatureValues {
        version: String::from("2020-12-06"),
        protocol: Some(SASProtocolOrString::SASProtocol(SASProtocol::HTTPSandHTTP)),
        startTime: Some(CommonDateOrString::String(String::from(far_past()))),
        expiryTime: CommonDateOrString::String(String::from(far_future())),
        permissions: AccountSASPermissionsOrString::String(permissions.to_owned()),
        ipRange: None,
        services: AccountSASServicesOrString::String(String::from("b")),
        resourceTypes: AccountSASResourceTypesOrString::String(String::from("o")),
        encryptionScope: None,
    }
}

fn blob_sas_values(version: &str) -> IBlobSASSignatureValues {
    IBlobSASSignatureValues {
        version: version.to_owned(),
        protocol: Some(SASProtocolOrString::String(String::from("https"))),
        startTime: Some(azurite_blob::authentication::DateOrString::String(
            String::from(far_past()),
        )),
        expiryTime: Some(azurite_blob::authentication::DateOrString::String(
            String::from(far_future()),
        )),
        permissions: Some(String::from("racwd")),
        ipRange: Some(IIPRangeOrString::String(String::from("10.0.0.1-10.0.0.9"))),
        containerName: String::from(CONTAINER),
        blobName: Some(String::from(BLOB)),
        identifier: None,
        encryptionScope: None,
        cacheControl: Some(String::from("max-age=5")),
        contentDisposition: Some(String::from("inline")),
        contentEncoding: Some(String::from("gzip")),
        contentLanguage: Some(String::from("en-US")),
        contentType: Some(String::from("text/plain")),
        signedResource: Some(String::from("b")),
        snapshot: None,
        signedObjectId: None,
        signedTenantId: None,
        signedService: None,
        signedVersion: None,
        signedStartsOn: None,
        signedExpiresOn: None,
        delegatedUserObjectId: None,
        delegatedUserTenantId: None,
    }
}

fn udk_values(version: &str) -> IBlobSASSignatureValues {
    let mut values = blob_sas_values(version);
    values.permissions = Some(String::from("r"));
    values.signedObjectId = Some(String::from("11111111-1111-1111-1111-111111111111"));
    values.signedTenantId = Some(String::from("22222222-2222-2222-2222-222222222222"));
    values.signedService = Some(String::from("b"));
    values.signedVersion = Some(String::from("2020-02-10"));
    values.signedStartsOn = Some(String::from(far_past()));
    values.signedExpiresOn = Some(String::from(far_future()));
    values
}

fn user_delegation_key_bytes(values: &IBlobSASSignatureValues) -> Vec<u8> {
    let string_to_sign = [
        values.signedObjectId.as_deref().unwrap_or_default(),
        values.signedTenantId.as_deref().unwrap_or_default(),
        values.signedStartsOn.as_deref().unwrap_or_default(),
        values.signedExpiresOn.as_deref().unwrap_or_default(),
        "b",
        values.signedVersion.as_deref().unwrap_or_default(),
    ]
    .join("\n");
    let key_value = computeHMACSHA256(
        &string_to_sign,
        &STANDARD.decode(USER_DELEGATION_BASIC_KEY).unwrap(),
    );
    STANDARD.decode(key_value).unwrap()
}

fn unsigned_bearer_token(payload: serde_json::Value) -> String {
    let encoded = URL_SAFE_NO_PAD.encode(payload.to_string());
    format!("header.{encoded}.signature")
}

#[tokio::test]
async fn i_authenticator_contract_preserves_none_success_and_error_results() {
    let request = make_request(HttpMethod::GET, "/container/blob.txt");
    let context = make_context(Operation::Blob_Download, Some(CONTAINER), Some(BLOB));

    let bypass = StubAuthenticator { result: Ok(None) };
    assert_eq!(bypass.validate(&request, &context).await.unwrap(), None);

    let success = StubAuthenticator {
        result: Ok(Some(true)),
    };
    assert_eq!(
        success.validate(&request, &context).await.unwrap(),
        Some(true)
    );

    let failure = StubAuthenticator {
        result: Err(StorageErrorFactory::getAuthorizationFailure(
            "phase7-context",
        )),
    };
    let error = failure.validate(&request, &context).await.unwrap_err();
    assert_storage_error(
        &error,
        403,
        "AuthorizationFailure",
        "Server failed to authenticate the request. Make sure the value of the Authorization header is formed correctly including the signature.",
    );
}

#[test]
fn blob_sas_permissions_resource_types_and_lookup_tables_match_ts_contracts() {
    let blob_permissions = [
        BlobSASPermission::Read,
        BlobSASPermission::Add,
        BlobSASPermission::Create,
        BlobSASPermission::Write,
        BlobSASPermission::Delete,
        BlobSASPermission::DeleteVersion,
        BlobSASPermission::Tag,
        BlobSASPermission::Move,
        BlobSASPermission::execute,
        BlobSASPermission::SetImmutabilityPolicy,
        BlobSASPermission::permanentDelete,
    ]
    .iter()
    .map(ToString::to_string)
    .collect::<String>();
    assert_eq!(blob_permissions, "racwdxtmeiy");

    let container_permissions = [
        ContainerSASPermission::Read,
        ContainerSASPermission::Add,
        ContainerSASPermission::Create,
        ContainerSASPermission::Write,
        ContainerSASPermission::Delete,
        ContainerSASPermission::List,
        ContainerSASPermission::Filter,
    ]
    .iter()
    .map(ToString::to_string)
    .collect::<String>();
    assert_eq!(container_permissions, "racwdlf");
    assert_eq!(ContainerSASPermission::Any.to_string(), "AnyPermission");

    assert_eq!(BlobSASResourceType::Container.to_string(), "c");
    assert_eq!(BlobSASResourceType::Blob.to_string(), "b");
    assert_eq!(BlobSASResourceType::BlobSnapshot.to_string(), "bs");

    let account_permission = OperationAccountSASPermission::new("b", "co", "wc");
    assert!(account_permission.validate("b", "sco", "c"));
    // Test was incorrect - "z" doesn't match required permissions "wc"
    assert!(!account_permission.validate("b", "o", "z"));
    assert!(
        OperationAccountSASPermission::new("b", "AnyResourceType", "AnyPermission")
            .validate("b", "c", "r")
    );
    assert!(
        !OperationAccountSASPermission::new("b", "AnyResourceType", "AnyPermission")
            .validate("q", "c", "r")
    );
    assert!(
        !OperationAccountSASPermission::new("b", "AnyResourceType", "AnyPermission")
            .validate("b", "", "r")
    );

    let blob_permission = OperationBlobSASPermission::new("cw");
    assert!(blob_permission.validate("c"));
    assert!(blob_permission.validate("w"));
    assert!(!blob_permission.validate("r"));
    assert!(OperationBlobSASPermission::new("AnyPermission").validate("r"));
    assert!(!OperationBlobSASPermission::new("AnyPermission").validate(""));

    assert_eq!(
        OPERATION_ACCOUNT_SAS_PERMISSIONS
            .get(&Operation::Service_SubmitBatch)
            .unwrap()
            .permission,
        "AnyPermission"
    );
    assert_eq!(
        OPERATION_ACCOUNT_SAS_PERMISSIONS
            .get(&Operation::BlockBlob_Upload)
            .unwrap()
            .permission,
        "wc"
    );
    assert_eq!(
        OPERATION_BLOB_SAS_BLOB_PERMISSIONS
            .get(&Operation::AppendBlob_AppendBlock)
            .unwrap()
            .permission,
        "aw"
    );
    assert_eq!(
        OPERATION_BLOB_SAS_CONTAINER_PERMISSIONS
            .get(&Operation::Container_SubmitBatch)
            .unwrap()
            .permission,
        "AnyPermission"
    );
}

#[test]
fn blob_sas_signature_generation_matches_ts_service_version_fixtures() {
    let values_2015 = blob_sas_values("2015-04-05");
    let (signature_2015, string_2015) =
        generateBlobSASSignature(&values_2015, BlobSASResourceType::Blob, ACCOUNT, SHARED_KEY);
    assert_eq!(
        string_2015,
        "racwd\n2020-04-16T13:31:48Z\n2099-04-16T13:31:48Z\n/blob/devstoreaccount1/container/blob.txt\n\n10.0.0.1-10.0.0.9\nhttps\n2015-04-05\nmax-age=5\ninline\ngzip\nen-US\ntext/plain"
    );
    assert_eq!(
        signature_2015,
        "CoL9vVY44KvrmTOLNg8cAViieR5JBumYc4FgtEd5OO4="
    );

    let mut values_2018 = blob_sas_values("2018-11-09");
    values_2018.signedResource = Some(String::from("bs"));
    values_2018.snapshot = Some(String::from("2022-04-16T13:31:48.0000000Z"));
    let (signature_2018, string_2018) = generateBlobSASSignature(
        &values_2018,
        BlobSASResourceType::BlobSnapshot,
        ACCOUNT,
        SHARED_KEY,
    );
    assert_eq!(
        string_2018,
        "racwd\n2020-04-16T13:31:48Z\n2099-04-16T13:31:48Z\n/blob/devstoreaccount1/container/blob.txt\n\n10.0.0.1-10.0.0.9\nhttps\n2018-11-09\nbs\n2022-04-16T13:31:48.0000000Z\nmax-age=5\ninline\ngzip\nen-US\ntext/plain"
    );
    assert_eq!(
        signature_2018,
        "RhQOjHNrNyzlt92DUzlX4wU/BcXLd2PcuUmnOy7EKg0="
    );

    let mut values_2020 = blob_sas_values("2020-12-06");
    values_2020.encryptionScope = Some(String::from("scope-name"));
    let (signature_2020, string_2020) =
        generateBlobSASSignature(&values_2020, BlobSASResourceType::Blob, ACCOUNT, SHARED_KEY);
    assert_eq!(
        string_2020,
        "racwd\n2020-04-16T13:31:48Z\n2099-04-16T13:31:48Z\n/blob/devstoreaccount1/container/blob.txt\n\n10.0.0.1-10.0.0.9\nhttps\n2020-12-06\nb\n\nscope-name\nmax-age=5\ninline\ngzip\nen-US\ntext/plain"
    );
    assert_eq!(
        signature_2020,
        "4kFyqUwaYmo0JulRJW/pm3pkozjUnn+n+W7jMQF3srI="
    );
}

#[test]
fn blob_sas_signature_generation_matches_ts_udk_versions_and_identifier_quirk() {
    let values_2018 = udk_values("2018-11-09");
    let key_bytes = user_delegation_key_bytes(&values_2018);
    let (signature_2018, string_2018) = generateBlobSASSignatureWithUDK(
        &values_2018,
        BlobSASResourceType::Blob,
        ACCOUNT,
        &key_bytes,
    );
    assert_eq!(
        string_2018,
        "r\n2020-04-16T13:31:48Z\n2099-04-16T13:31:48Z\n/blob/devstoreaccount1/container/blob.txt\n11111111-1111-1111-1111-111111111111\n22222222-2222-2222-2222-222222222222\n2020-04-16T13:31:48Z\n2099-04-16T13:31:48Z\nb\n2020-02-10\n10.0.0.1-10.0.0.9\nhttps\n2018-11-09\nb\n\nmax-age=5\ninline\ngzip\nen-US\ntext/plain"
    );
    assert_eq!(
        signature_2018,
        "m1ndyXv6OeQQ2opncrCPH4HJeJByBKQtNxwQeCc2NJU="
    );

    let values_2020_02 = udk_values("2020-02-10");
    let key_bytes = user_delegation_key_bytes(&values_2020_02);
    let (signature_2020_02, string_2020_02) = generateBlobSASSignatureWithUDK(
        &values_2020_02,
        BlobSASResourceType::Blob,
        ACCOUNT,
        &key_bytes,
    );
    assert_eq!(
        string_2020_02,
        "r\n2020-04-16T13:31:48Z\n2099-04-16T13:31:48Z\n/blob/devstoreaccount1/container/blob.txt\n11111111-1111-1111-1111-111111111111\n22222222-2222-2222-2222-222222222222\n2020-04-16T13:31:48Z\n2099-04-16T13:31:48Z\nb\n2020-02-10\n\n\n\n10.0.0.1-10.0.0.9\nhttps\n2020-02-10\nb\n\nmax-age=5\ninline\ngzip\nen-US\ntext/plain"
    );
    assert_eq!(
        signature_2020_02,
        "OFC0dWum9TEqBxg5FnoLAnulDfaQd07dWAIpA3nzBNI="
    );

    let values_2020_12 = udk_values("2020-12-06");
    let key_bytes = user_delegation_key_bytes(&values_2020_12);
    let (signature_2020_12, string_2020_12) = generateBlobSASSignatureWithUDK(
        &values_2020_12,
        BlobSASResourceType::Blob,
        ACCOUNT,
        &key_bytes,
    );
    assert_eq!(
        string_2020_12,
        "r\n2020-04-16T13:31:48Z\n2099-04-16T13:31:48Z\n/blob/devstoreaccount1/container/blob.txt\n11111111-1111-1111-1111-111111111111\n22222222-2222-2222-2222-222222222222\n2020-04-16T13:31:48Z\n2099-04-16T13:31:48Z\nb\n2020-02-10\n\n\n\n10.0.0.1-10.0.0.9\nhttps\n2020-12-06\nb\n\n\nmax-age=5\ninline\ngzip\nen-US\ntext/plain"
    );
    assert_eq!(
        signature_2020_12,
        "3bJfukJS6fBW/hYb0w7arvE+R64IsF4ZHMZpctAibyY="
    );

    let values_2025 = udk_values("2025-07-05");
    let key_bytes = user_delegation_key_bytes(&values_2025);
    let (signature_2025, string_2025) = generateBlobSASSignatureWithUDK(
        &values_2025,
        BlobSASResourceType::Blob,
        ACCOUNT,
        &key_bytes,
    );
    assert_eq!(
        string_2025,
        "r\n2020-04-16T13:31:48Z\n2099-04-16T13:31:48Z\n/blob/devstoreaccount1/container/blob.txt\n11111111-1111-1111-1111-111111111111\n22222222-2222-2222-2222-222222222222\n2020-04-16T13:31:48Z\n2099-04-16T13:31:48Z\nb\n2020-02-10\n\n\n\n10.0.0.1-10.0.0.9\nhttps\n2025-07-05\nb\n\n\n\n\nmax-age=5\ninline\ngzip\nen-US\ntext/plain"
    );
    assert_eq!(
        signature_2025,
        "+TlpI5D5f1/UaReGQWurnMlCbLAW8oXCeCeLqtp/yvM="
    );

    let mut identifier_only = blob_sas_values("2020-12-06");
    identifier_only.identifier = Some(String::from("policy-id"));
    identifier_only.permissions = None;
    identifier_only.expiryTime = None;
    let (signature, string_to_sign) = generateBlobSASSignature(
        &identifier_only,
        BlobSASResourceType::Blob,
        ACCOUNT,
        SHARED_KEY,
    );
    assert_eq!(
        string_to_sign,
        "\n2020-04-16T13:31:48Z\n\n/blob/devstoreaccount1/container/blob.txt\npolicy-id\n10.0.0.1-10.0.0.9\nhttps\n2020-12-06\nb\n\n\nmax-age=5\ninline\ngzip\nen-US\ntext/plain"
    );
    assert_eq!(signature, "C0opeiCxxxGLEQntM6C9TJBOQMYtIHYAx3jKEW3eKs0=");
}

#[tokio::test]
async fn blob_shared_key_authenticator_matches_primary_and_secondary_signatures() {
    let authenticator = BlobSharedKeyAuthenticator::new(account_store(), logger());

    let mut primary = make_request(
        HttpMethod::PUT,
        "/container/blob.txt?comp=metadata&z=one+two&a=%2Bplus",
    );
    set_header(&mut primary, "content-length", "0");
    set_header(&mut primary, "content-type", "text/plain");
    set_header(&mut primary, "x-ms-version", "2020-12-06");
    set_header(&mut primary, "x-ms-meta-name ", "  spaced");

    let primary_string_to_sign = ["PUT", "", "", "", "", "text/plain", "", "", "", "", "", ""]
        .join("\n")
        + "\n"
        + "x-ms-meta-name:spaced\n"
        + "x-ms-version:2020-12-06\n"
        + "/devstoreaccount1/container/blob.txt\na:+plus\ncomp:metadata\nz:one two";
    let primary_signature = computeHMACSHA256(&primary_string_to_sign, SHARED_KEY);
    set_header(
        &mut primary,
        "authorization",
        &format!("SharedKey {ACCOUNT}:{primary_signature}"),
    );

    let primary_context = make_context(Operation::Blob_SetMetadata, Some(CONTAINER), Some(BLOB));
    assert_eq!(
        authenticator
            .validate(&primary, &primary_context)
            .await
            .unwrap(),
        Some(true)
    );

    let mut secondary = make_request(HttpMethod::GET, "/container/blob.txt?comp=metadata");
    set_header(&mut secondary, "x-ms-version", "2020-12-06");
    let secondary_context = make_context(Operation::Blob_Download, Some(CONTAINER), Some(BLOB));
    let secondary_blob_context = BlobStorageContext::new(&secondary_context);
    secondary_blob_context.setIsSecondary(Some(true));
    secondary_blob_context.setAuthenticationPath(Some(format!("/{ACCOUNT}/container/blob.txt")));

    let secondary_path = format!("/{ACCOUNT}{SECONDARY_SUFFIX}/container/blob.txt");
    let secondary_string_to_sign = ["GET", "", "", "", "", "", "", "", "", "", "", ""].join("\n")
        + "\n"
        + "x-ms-version:2020-12-06\n"
        + &format!("/{ACCOUNT}{secondary_path}\ncomp:metadata");
    let secondary_signature = computeHMACSHA256(&secondary_string_to_sign, SHARED_KEY);
    set_header(
        &mut secondary,
        "authorization",
        &format!("SharedKey {ACCOUNT}:{secondary_signature}"),
    );

    assert_eq!(
        authenticator
            .validate(&secondary, &secondary_context)
            .await
            .unwrap(),
        Some(true)
    );
}

#[tokio::test]
async fn blob_shared_key_authenticator_bypasses_non_shared_key_and_blocks_udk_operation() {
    let authenticator = BlobSharedKeyAuthenticator::new(account_store(), logger());
    let request = make_request(HttpMethod::GET, "/container/blob.txt");
    let context = make_context(Operation::Blob_Download, Some(CONTAINER), Some(BLOB));
    assert_eq!(
        authenticator.validate(&request, &context).await.unwrap(),
        None
    );

    let mut bearer = make_request(HttpMethod::GET, "/container/blob.txt");
    set_header(&mut bearer, "authorization", "Bearer ignored");
    assert_eq!(
        authenticator.validate(&bearer, &context).await.unwrap(),
        None
    );

    let mut udk = make_request(HttpMethod::POST, "/");
    set_header(&mut udk, "authorization", "SharedKey devstoreaccount1:bad");
    let udk_context = make_context(Operation::Service_GetUserDelegationKey, None, None);
    let error = authenticator
        .validate(&udk, &udk_context)
        .await
        .unwrap_err();
    assert_storage_error(
        &error,
        403,
        "AuthenticationFailed",
        "Server failed to authenticate the request. Make sure the value of the Authorization header is formed correctly including the signature.",
    );
}

#[tokio::test]
async fn account_sas_authenticator_requires_write_for_existing_blob_uploads() {
    let metadata_store: Arc<dyn IBlobMetadataStore + Send + Sync> =
        Arc::new(TestBlobMetadataStore::default().with_blob_type(
            ACCOUNT,
            CONTAINER,
            BLOB,
            Ok(Some(BlobTypeResult {
                blobType: Some(String::from("BlockBlob")),
                isCommitted: true,
            })),
        ));
    let authenticator =
        AccountSASAuthenticator::new(account_store(), Arc::clone(&metadata_store), logger());

    let values = account_sas_values("c");
    let (signature, _) = generateAccountSASSignature(&values, ACCOUNT, SHARED_KEY);
    let mut request = make_request(HttpMethod::PUT, "/container/blob.txt");
    set_query(&mut request, "sv", &values.version);
    set_query(&mut request, "ss", "b");
    set_query(&mut request, "srt", "o");
    set_query(&mut request, "se", far_future());
    set_query(&mut request, "st", far_past());
    set_query(&mut request, "sp", "c");
    set_query(&mut request, "sig", &signature);
    set_query(&mut request, "spr", "https,http");

    let context = make_context(Operation::BlockBlob_Upload, Some(CONTAINER), Some(BLOB));
    let error = authenticator
        .validate(&request, &context)
        .await
        .unwrap_err();
    assert_storage_error(
        &error,
        403,
        "AuthorizationPermissionMismatch",
        "This request is not authorized to perform this operation using this permission.",
    );

    let values = account_sas_values("cw");
    let (signature, _) = generateAccountSASSignature(&values, ACCOUNT, SHARED_KEY);
    set_query(&mut request, "sp", "cw");
    set_query(&mut request, "sig", &signature);
    assert_eq!(
        authenticator.validate(&request, &context).await.unwrap(),
        Some(true)
    );
}

#[tokio::test]
async fn account_sas_authenticator_enforces_protocol_and_strict_encryption_scope() {
    let metadata_store: Arc<dyn IBlobMetadataStore + Send + Sync> =
        Arc::new(TestBlobMetadataStore::default());
    let authenticator =
        AccountSASAuthenticator::new(account_store(), Arc::clone(&metadata_store), logger());

    let mut values = account_sas_values("r");
    values.protocol = Some(SASProtocolOrString::SASProtocol(SASProtocol::HTTPS));
    let (signature, _) = generateAccountSASSignature(&values, ACCOUNT, SHARED_KEY);
    let mut request = make_request(HttpMethod::GET, "/container/blob.txt");
    request.protocol = String::from("http");
    set_query(&mut request, "sv", &values.version);
    set_query(&mut request, "ss", "b");
    set_query(&mut request, "srt", "o");
    set_query(&mut request, "se", far_future());
    set_query(&mut request, "st", far_past());
    set_query(&mut request, "sp", "r");
    set_query(&mut request, "sig", &signature);
    set_query(&mut request, "spr", "https");

    let context = make_context(Operation::Blob_Download, Some(CONTAINER), Some(BLOB));
    let error = authenticator
        .validate(&request, &context)
        .await
        .unwrap_err();
    assert_storage_error(
        &error,
        403,
        "AuthorizationProtocolMismatch",
        "This request is not authorized to perform this operation using this protocol.",
    );

    let mut strict_request = make_request(HttpMethod::GET, "/container/blob.txt");
    set_query(&mut strict_request, "sv", &values.version);
    set_query(&mut strict_request, "ss", "b");
    set_query(&mut strict_request, "srt", "o");
    set_query(&mut strict_request, "se", far_future());
    set_query(&mut strict_request, "st", far_past());
    set_query(&mut strict_request, "sp", "r");
    set_query(&mut strict_request, "sig", &signature);
    set_query(&mut strict_request, "ses", "scope-name");
    let strict_context = make_context(Operation::Blob_Download, Some(CONTAINER), Some(BLOB));
    let strict_error = authenticator
        .validate(&strict_request, &strict_context)
        .await
        .unwrap_err();
    assert_storage_error(
        &strict_error,
        500,
        "FeatureNotSupported",
        "SAS Encryption Scope 'ses' header or parameter is not supported in Azurite strict mode. Switch to loose model by Azurite command line parameter \"--loose\" or Visual Studio Code configuration \"Loose\". Please vote your wanted features to https://github.com/azure/azurite/issues",
    );
}

#[tokio::test]
async fn blob_sas_authenticator_uses_saved_access_policy_identifier() {
    let metadata_store: Arc<dyn IBlobMetadataStore + Send + Sync> =
        Arc::new(TestBlobMetadataStore::default().with_container_acl(
            ACCOUNT,
            CONTAINER,
            Ok(Some(GetContainerAccessPolicyResponse {
                properties: BTreeMap::new(),
                containerAcl: Some(vec![signed_identifier(
                    "policy-id",
                    access_policy("r", far_past(), far_future()),
                )]),
            })),
        ));
    let authenticator =
        BlobSASAuthenticator::new(account_store(), Arc::clone(&metadata_store), logger());

    let mut values = blob_sas_values("2020-12-06");
    values.identifier = Some(String::from("policy-id"));
    values.permissions = None;
    values.expiryTime = None;
    let (signature, _) =
        generateBlobSASSignature(&values, BlobSASResourceType::Blob, ACCOUNT, SHARED_KEY);

    let mut request = make_request(HttpMethod::GET, "/container/blob.txt");
    set_query(&mut request, "sv", "2020-12-06");
    set_query(&mut request, "st", far_past());
    set_query(&mut request, "si", "policy-id");
    set_query(&mut request, "spr", "https");
    set_query(&mut request, "sip", "10.0.0.1-10.0.0.9");
    set_query(&mut request, "sr", "b");
    set_query(&mut request, "sig", &signature);
    set_query(&mut request, "rscc", "max-age=5");
    set_query(&mut request, "rscd", "inline");
    set_query(&mut request, "rsce", "gzip");
    set_query(&mut request, "rscl", "en-US");
    set_query(&mut request, "rsct", "text/plain");

    let context = make_context(Operation::Blob_Download, Some(CONTAINER), Some(BLOB));
    assert_eq!(
        authenticator.validate(&request, &context).await.unwrap(),
        Some(true)
    );
}

#[tokio::test]
async fn blob_sas_authenticator_validates_user_delegation_sas_and_snapshot_permission_quirk() {
    let metadata_store: Arc<dyn IBlobMetadataStore + Send + Sync> =
        Arc::new(TestBlobMetadataStore::default());
    let authenticator =
        BlobSASAuthenticator::new(account_store(), Arc::clone(&metadata_store), logger());

    // Test user delegation SAS with regular blob resource
    let values = udk_values("2020-12-06");
    let key_bytes = user_delegation_key_bytes(&values);
    let (signature, _) =
        generateBlobSASSignatureWithUDK(&values, BlobSASResourceType::Blob, ACCOUNT, &key_bytes);

    let mut request = make_request(HttpMethod::GET, "/container/blob.txt");
    set_query(&mut request, "sv", "2020-12-06");
    set_query(&mut request, "st", far_past());
    set_query(&mut request, "se", far_future());
    set_query(&mut request, "sp", "r");
    set_query(&mut request, "spr", "https");
    set_query(&mut request, "sip", "10.0.0.1-10.0.0.9");
    set_query(&mut request, "sr", "b");
    set_query(&mut request, "sig", &signature);
    set_query(
        &mut request,
        "skoid",
        values.signedObjectId.as_deref().unwrap(),
    );
    set_query(
        &mut request,
        "sktid",
        values.signedTenantId.as_deref().unwrap(),
    );
    set_query(
        &mut request,
        "skt",
        values.signedStartsOn.as_deref().unwrap(),
    );
    set_query(
        &mut request,
        "ske",
        values.signedExpiresOn.as_deref().unwrap(),
    );
    set_query(
        &mut request,
        "sks",
        values.signedService.as_deref().unwrap(),
    );
    set_query(
        &mut request,
        "skv",
        values.signedVersion.as_deref().unwrap(),
    );
    set_query(&mut request, "rscc", "max-age=5");
    set_query(&mut request, "rscd", "inline");
    set_query(&mut request, "rsce", "gzip");
    set_query(&mut request, "rscl", "en-US");
    set_query(&mut request, "rsct", "text/plain");

    let context = make_context(Operation::Blob_Download, Some(CONTAINER), Some(BLOB));
    assert_eq!(
        authenticator.validate(&request, &context).await.unwrap(),
        Some(true)
    );

    // Test blob snapshot SAS - uses CONTAINER permissions table, so List permission won't work for Blob_Download (requires Read)
    let mut snapshot_values = blob_sas_values("2020-12-06");
    snapshot_values.permissions = Some(String::from("r")); // Changed from "l" to "r" - must use Read for download
    snapshot_values.signedResource = Some(String::from("bs"));
    snapshot_values.snapshot = Some(String::from("2022-04-16T13:31:48.0000000Z"));
    let (snapshot_signature, _) = generateBlobSASSignature(
        &snapshot_values,
        BlobSASResourceType::BlobSnapshot,
        ACCOUNT,
        SHARED_KEY,
    );

    let mut snapshot_request = make_request(HttpMethod::GET, "/container/blob.txt");
    set_query(&mut snapshot_request, "sv", "2020-12-06");
    set_query(&mut snapshot_request, "st", far_past());
    set_query(&mut snapshot_request, "se", far_future());
    set_query(&mut snapshot_request, "sp", "r"); // Changed from "l" to "r"
    set_query(&mut snapshot_request, "spr", "https");
    set_query(&mut snapshot_request, "sip", "10.0.0.1-10.0.0.9");
    set_query(&mut snapshot_request, "sr", "bs");
    set_query(
        &mut snapshot_request,
        "snapshot",
        "2022-04-16T13:31:48.0000000Z",
    );
    set_query(&mut snapshot_request, "sig", &snapshot_signature);
    set_query(&mut snapshot_request, "rscc", "max-age=5");
    set_query(&mut snapshot_request, "rscd", "inline");
    set_query(&mut snapshot_request, "rsce", "gzip");
    set_query(&mut snapshot_request, "rscl", "en-US");
    set_query(&mut snapshot_request, "rsct", "text/plain");

    let snapshot_context = make_context(Operation::Blob_Download, Some(CONTAINER), Some(BLOB));
    assert_eq!(
        authenticator
            .validate(&snapshot_request, &snapshot_context)
            .await
            .unwrap(),
        Some(true)
    );
}

#[tokio::test]
async fn blob_token_authenticator_enforces_https_and_basic_claim_validation() {
    let authenticator = BlobTokenAuthenticator::new(account_store(), OAuthLevel::BASIC, logger());
    let context = make_context(Operation::Blob_Download, Some(CONTAINER), Some(BLOB));

    let request = make_request(HttpMethod::GET, "/container/blob.txt");
    assert_eq!(
        authenticator.validate(&request, &context).await.unwrap(),
        None
    );

    let mut invalid_scheme = make_request(HttpMethod::GET, "/container/blob.txt");
    set_header(&mut invalid_scheme, "authorization", "SharedKey nope");
    let invalid_scheme_error = authenticator
        .validate(&invalid_scheme, &context)
        .await
        .unwrap_err();
    assert_storage_error(
        &invalid_scheme_error,
        400,
        "InvalidAuthenticationInfo",
        "Authentication information is not given in the correct format. Check the value of Authorization header.",
    );

    let mut http_request = make_request(HttpMethod::GET, "/container/blob.txt");
    http_request.protocol = String::from("http");
    set_header(
        &mut http_request,
        "authorization",
        "Bearer header.payload.sig",
    );
    let http_error = authenticator
        .validate(&http_request, &context)
        .await
        .unwrap_err();
    assert_storage_error(
        &http_error,
        403,
        "AuthenticationFailed",
        "Server failed to authenticate the request. Make sure the value of the Authorization header is formed correctly including the signature.",
    );

    let valid_payload = json!({
        "nbf": fixed_time().timestamp() - 60,
        "exp": fixed_time().timestamp() + 60,
        "iat": fixed_time().timestamp() - 120,
        "iss": "https://sts.windows.net/tenant-id/",
        "aud": format!("https://{ACCOUNT}.blob.core.windows.net"),
    });
    let valid_token = unsigned_bearer_token(valid_payload);
    let mut valid_request = make_request(HttpMethod::GET, "/container/blob.txt");
    set_header(
        &mut valid_request,
        "authorization",
        &format!("Bearer {valid_token}"),
    );
    assert_eq!(
        authenticator
            .validate(&valid_request, &context)
            .await
            .unwrap(),
        Some(true)
    );

    let future_payload = json!({
        "nbf": fixed_time().timestamp() + 600,
        "exp": fixed_time().timestamp() + 1200,
        "iat": fixed_time().timestamp(),
        "iss": "https://sts.windows.net/tenant-id/",
        "aud": format!("https://{ACCOUNT}.blob.core.windows.net"),
    });
    let mut future_request = make_request(HttpMethod::GET, "/container/blob.txt");
    set_header(
        &mut future_request,
        "authorization",
        &format!("Bearer {}", unsigned_bearer_token(future_payload)),
    );
    let future_error = authenticator
        .validate(&future_request, &context)
        .await
        .unwrap_err();
    assert_storage_error(
        &future_error,
        403,
        "AuthenticationFailed",
        "Server failed to authenticate the request. Make sure the value of the Authorization header is formed correctly including the signature.",
    );

    let invalid_issuer_payload = json!({
        "nbf": fixed_time().timestamp() - 60,
        "exp": fixed_time().timestamp() + 60,
        "iat": fixed_time().timestamp() - 120,
        "iss": "https://invalid.example/tenant-id/",
        "aud": format!("https://{ACCOUNT}.blob.core.windows.net"),
    });
    let mut issuer_request = make_request(HttpMethod::GET, "/container/blob.txt");
    set_header(
        &mut issuer_request,
        "authorization",
        &format!("Bearer {}", unsigned_bearer_token(invalid_issuer_payload)),
    );
    let issuer_error = authenticator
        .validate(&issuer_request, &context)
        .await
        .unwrap_err();
    assert_storage_error(
        &issuer_error,
        403,
        "AuthenticationFailed",
        "Server failed to authenticate the request. Make sure the value of the Authorization header is formed correctly including the signature.",
    );
}

#[tokio::test]
async fn public_access_authenticator_matches_ts_bypass_rules() {
    let metadata_store: Arc<dyn IBlobMetadataStore + Send + Sync> = Arc::new(
        TestBlobMetadataStore::default()
            .with_container_acl(
                ACCOUNT,
                CONTAINER,
                Ok(Some(public_access_response("container"))),
            )
            .with_container_acl(
                ACCOUNT,
                "blob-public",
                Ok(Some(public_access_response("blob"))),
            )
            .with_container_acl(
                ACCOUNT,
                "broken",
                Err(StorageErrorFactory::getAuthorizationFailure(
                    "phase7-context",
                )),
            ),
    );
    let authenticator = PublicAccessAuthenticator::new(Arc::clone(&metadata_store), logger());

    let request = make_request(HttpMethod::GET, "/container/blob.txt");
    let container_context = make_context(
        Operation::Container_ListBlobFlatSegment,
        Some(CONTAINER),
        None,
    );
    assert_eq!(
        authenticator
            .validate(&request, &container_context)
            .await
            .unwrap(),
        Some(true)
    );

    let blob_context = make_context(Operation::Blob_Download, Some("blob-public"), Some(BLOB));
    assert_eq!(
        authenticator
            .validate(&request, &blob_context)
            .await
            .unwrap(),
        Some(true)
    );

    let list_blob_context = make_context(
        Operation::Container_ListBlobFlatSegment,
        Some("blob-public"),
        None,
    );
    assert_eq!(
        authenticator
            .validate(&request, &list_blob_context)
            .await
            .unwrap(),
        None
    );

    let broken_context = make_context(Operation::Blob_Download, Some("broken"), Some(BLOB));
    assert_eq!(
        authenticator
            .validate(&request, &broken_context)
            .await
            .unwrap(),
        None
    );

    let service_context = make_context(Operation::Service_GetProperties, None, None);
    assert_eq!(
        authenticator
            .validate(&request, &service_context)
            .await
            .unwrap(),
        None
    );
}
