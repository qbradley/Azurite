use std::collections::BTreeMap;

use indexmap::IndexMap;

use crate::generated::i_request::GeneratedReadableStream;
use crate::generated::i_response::ResponseHeaderValue;

#[derive(Debug, Clone)]
pub enum GeneratedValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<GeneratedValue>),
    Object(GeneratedObject),
    Stream(GeneratedReadableStream),
}

pub type GeneratedObject = IndexMap<String, GeneratedValue>;

#[derive(Debug, Clone)]
pub enum GeneratedBody {
    Text(String),
    Value(GeneratedValue),
    Stream(GeneratedReadableStream),
}

#[derive(Debug, Clone, Default)]
pub struct GeneratedResponse {
    pub statusCode: u16,
    pub statusMessage: Option<String>,
    pub headers: BTreeMap<String, ResponseHeaderValue>,
    pub contentType: Option<String>,
    pub body: Option<GeneratedBody>,
    pub fields: GeneratedObject,
}

impl Default for GeneratedValue {
    fn default() -> Self {
        Self::Null
    }
}

impl GeneratedValue {
    pub fn as_object(&self) -> Option<&GeneratedObject> {
        match self {
            GeneratedValue::Object(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<String> {
        match self {
            GeneratedValue::String(value) => Some(value.clone()),
            GeneratedValue::Number(value) => Some(value.to_string()),
            GeneratedValue::Bool(value) => Some(value.to_string()),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            GeneratedValue::Number(value) => Some(*value),
            GeneratedValue::String(value) => value.parse().ok(),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            GeneratedValue::Bool(value) => Some(*value),
            GeneratedValue::String(value) => value.parse().ok(),
            _ => None,
        }
    }

    pub fn as_stream(&self) -> Option<GeneratedReadableStream> {
        match self {
            GeneratedValue::Stream(stream) => Some(stream.clone()),
            _ => None,
        }
    }

    pub fn to_json_value(&self) -> serde_json::Value {
        match self {
            GeneratedValue::Null => serde_json::Value::Null,
            GeneratedValue::Bool(value) => serde_json::Value::Bool(*value),
            GeneratedValue::Number(value) => serde_json::Number::from_f64(*value)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            GeneratedValue::String(value) => serde_json::Value::String(value.clone()),
            GeneratedValue::Array(values) => {
                serde_json::Value::Array(values.iter().map(GeneratedValue::to_json_value).collect())
            }
            GeneratedValue::Object(values) => serde_json::Value::Object(
                values
                    .iter()
                    .map(|(key, value)| (key.clone(), value.to_json_value()))
                    .collect(),
            ),
            GeneratedValue::Stream(stream) => serde_json::Value::String(stream.read_to_string()),
        }
    }
}

impl From<serde_json::Value> for GeneratedValue {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => GeneratedValue::Null,
            serde_json::Value::Bool(value) => GeneratedValue::Bool(value),
            serde_json::Value::Number(value) => {
                GeneratedValue::Number(value.as_f64().unwrap_or_default())
            }
            serde_json::Value::String(value) => GeneratedValue::String(value),
            serde_json::Value::Array(values) => {
                GeneratedValue::Array(values.into_iter().map(GeneratedValue::from).collect())
            }
            serde_json::Value::Object(values) => GeneratedValue::Object(
                values
                    .into_iter()
                    .map(|(key, value)| (key, GeneratedValue::from(value)))
                    .collect(),
            ),
        }
    }
}

impl GeneratedResponse {
    pub fn new(statusCode: u16) -> Self {
        Self {
            statusCode,
            ..Self::default()
        }
    }

    pub fn insert_field(&mut self, key: impl Into<String>, value: GeneratedValue) {
        self.fields.insert(key.into(), value);
    }

    pub fn get_field(&self, key: &str) -> Option<&GeneratedValue> {
        self.fields.get(key)
    }

    pub fn body_value(&self) -> GeneratedValue {
        if let Some(body) = &self.body {
            match body {
                GeneratedBody::Text(value) => GeneratedValue::String(value.clone()),
                GeneratedBody::Value(value) => value.clone(),
                GeneratedBody::Stream(stream) => GeneratedValue::Stream(stream.clone()),
            }
        } else {
            GeneratedValue::Object(self.fields.clone())
        }
    }
}

pub type KeyInfo = GeneratedObject;
pub type UserDelegationKey = GeneratedObject;
pub type StorageError = GeneratedObject;
pub type AccessPolicy = GeneratedObject;
pub type BlobPropertiesInternal = GeneratedObject;
pub type BlobMetadata = GeneratedObject;
pub type BlobTag = GeneratedObject;
pub type BlobTags = GeneratedObject;
pub type BlobItemInternal = GeneratedObject;
pub type BlobFlatListSegment = GeneratedObject;
pub type ListBlobsFlatSegmentResponse = GeneratedObject;
pub type BlobPrefix = GeneratedObject;
pub type BlobHierarchyListSegment = GeneratedObject;
pub type ListBlobsHierarchySegmentResponse = GeneratedObject;
pub type BlobName = GeneratedObject;
pub type Block = GeneratedObject;
pub type BlockList = GeneratedObject;
pub type BlockLookupList = GeneratedObject;
pub type ContainerProperties = GeneratedObject;
pub type ContainerItem = GeneratedObject;
pub type DelimitedTextConfiguration = GeneratedObject;
pub type JsonTextConfiguration = GeneratedObject;
pub type ArrowField = GeneratedObject;
pub type ArrowConfiguration = GeneratedObject;
pub type ListContainersSegmentResponse = GeneratedObject;
pub type CorsRule = GeneratedObject;
pub type FilterBlobItem = GeneratedObject;
pub type FilterBlobSegment = GeneratedObject;
pub type GeoReplication = GeneratedObject;
pub type RetentionPolicy = GeneratedObject;
pub type Logging = GeneratedObject;
pub type Metrics = GeneratedObject;
pub type PageRange = GeneratedObject;
pub type ClearRange = GeneratedObject;
pub type PageList = GeneratedObject;
pub type QueryFormat = GeneratedObject;
pub type QuerySerialization = GeneratedObject;
pub type QueryRequest = GeneratedObject;
pub type SignedIdentifier = GeneratedObject;
pub type StaticWebsite = GeneratedObject;
pub type StorageServiceProperties = GeneratedObject;
pub type StorageServiceStats = GeneratedObject;
pub type ContainerCpkScopeInfo = GeneratedObject;
pub type LeaseAccessConditions = GeneratedObject;
pub type ModifiedAccessConditions = GeneratedObject;
pub type CpkInfo = GeneratedObject;
pub type BlobHTTPHeaders = GeneratedObject;
pub type CpkScopeInfo = GeneratedObject;
pub type SourceModifiedAccessConditions = GeneratedObject;
pub type SequenceNumberAccessConditions = GeneratedObject;
pub type AppendPositionAccessConditions = GeneratedObject;
pub type AzuriteServerBlobOptions = GeneratedObject;
pub type ServiceSetPropertiesOptionalParams = GeneratedObject;
pub type ServiceGetPropertiesOptionalParams = GeneratedObject;
pub type ServiceGetStatisticsOptionalParams = GeneratedObject;
pub type ServiceListContainersSegmentOptionalParams = GeneratedObject;
pub type ServiceGetUserDelegationKeyOptionalParams = GeneratedObject;
pub type ServiceSubmitBatchOptionalParams = GeneratedObject;
pub type ServiceFilterBlobsOptionalParams = GeneratedObject;
pub type ContainerCreateOptionalParams = GeneratedObject;
pub type ContainerGetPropertiesOptionalParams = GeneratedObject;
pub type ContainerGetPropertiesWithHeadOptionalParams = GeneratedObject;
pub type ContainerDeleteMethodOptionalParams = GeneratedObject;
pub type ContainerSetMetadataOptionalParams = GeneratedObject;
pub type ContainerGetAccessPolicyOptionalParams = GeneratedObject;
pub type ContainerSetAccessPolicyOptionalParams = GeneratedObject;
pub type ContainerRestoreOptionalParams = GeneratedObject;
pub type ContainerSubmitBatchOptionalParams = GeneratedObject;
pub type ContainerFilterBlobsOptionalParams = GeneratedObject;
pub type ContainerAcquireLeaseOptionalParams = GeneratedObject;
pub type ContainerReleaseLeaseOptionalParams = GeneratedObject;
pub type ContainerRenewLeaseOptionalParams = GeneratedObject;
pub type ContainerBreakLeaseOptionalParams = GeneratedObject;
pub type ContainerChangeLeaseOptionalParams = GeneratedObject;
pub type ContainerListBlobFlatSegmentOptionalParams = GeneratedObject;
pub type ContainerListBlobHierarchySegmentOptionalParams = GeneratedObject;
pub type BlobDownloadOptionalParams = GeneratedObject;
pub type BlobGetPropertiesOptionalParams = GeneratedObject;
pub type BlobDeleteMethodOptionalParams = GeneratedObject;
pub type BlobUndeleteOptionalParams = GeneratedObject;
pub type BlobSetExpiryOptionalParams = GeneratedObject;
pub type BlobSetHTTPHeadersOptionalParams = GeneratedObject;
pub type BlobSetImmutabilityPolicyOptionalParams = GeneratedObject;
pub type BlobDeleteImmutabilityPolicyOptionalParams = GeneratedObject;
pub type BlobSetLegalHoldOptionalParams = GeneratedObject;
pub type BlobSetMetadataOptionalParams = GeneratedObject;
pub type BlobAcquireLeaseOptionalParams = GeneratedObject;
pub type BlobReleaseLeaseOptionalParams = GeneratedObject;
pub type BlobRenewLeaseOptionalParams = GeneratedObject;
pub type BlobChangeLeaseOptionalParams = GeneratedObject;
pub type BlobBreakLeaseOptionalParams = GeneratedObject;
pub type BlobCreateSnapshotOptionalParams = GeneratedObject;
pub type BlobStartCopyFromURLOptionalParams = GeneratedObject;
pub type BlobCopyFromURLOptionalParams = GeneratedObject;
pub type BlobAbortCopyFromURLOptionalParams = GeneratedObject;
pub type BlobSetTierOptionalParams = GeneratedObject;
pub type BlobQueryOptionalParams = GeneratedObject;
pub type BlobGetTagsOptionalParams = GeneratedObject;
pub type BlobSetTagsOptionalParams = GeneratedObject;
pub type PageBlobCreateOptionalParams = GeneratedObject;
pub type PageBlobUploadPagesOptionalParams = GeneratedObject;
pub type PageBlobClearPagesOptionalParams = GeneratedObject;
pub type PageBlobUploadPagesFromURLOptionalParams = GeneratedObject;
pub type PageBlobGetPageRangesOptionalParams = GeneratedObject;
pub type PageBlobGetPageRangesDiffOptionalParams = GeneratedObject;
pub type PageBlobResizeOptionalParams = GeneratedObject;
pub type PageBlobUpdateSequenceNumberOptionalParams = GeneratedObject;
pub type PageBlobCopyIncrementalOptionalParams = GeneratedObject;
pub type AppendBlobCreateOptionalParams = GeneratedObject;
pub type AppendBlobAppendBlockOptionalParams = GeneratedObject;
pub type AppendBlobAppendBlockFromUrlOptionalParams = GeneratedObject;
pub type AppendBlobSealOptionalParams = GeneratedObject;
pub type BlockBlobUploadOptionalParams = GeneratedObject;
pub type BlockBlobPutBlobFromUrlOptionalParams = GeneratedObject;
pub type BlockBlobStageBlockOptionalParams = GeneratedObject;
pub type BlockBlobStageBlockFromURLOptionalParams = GeneratedObject;
pub type BlockBlobCommitBlockListOptionalParams = GeneratedObject;
pub type BlockBlobGetBlockListOptionalParams = GeneratedObject;
pub type ServiceSetPropertiesHeaders = GeneratedObject;
pub type ServiceGetPropertiesHeaders = GeneratedObject;
pub type ServiceGetStatisticsHeaders = GeneratedObject;
pub type ServiceListContainersSegmentHeaders = GeneratedObject;
pub type ServiceGetUserDelegationKeyHeaders = GeneratedObject;
pub type ServiceGetAccountInfoHeaders = GeneratedObject;
pub type ServiceGetAccountInfoWithHeadHeaders = GeneratedObject;
pub type ServiceSubmitBatchHeaders = GeneratedObject;
pub type ServiceFilterBlobsHeaders = GeneratedObject;
pub type ContainerCreateHeaders = GeneratedObject;
pub type ContainerGetPropertiesHeaders = GeneratedObject;
pub type ContainerGetPropertiesWithHeadHeaders = GeneratedObject;
pub type ContainerDeleteHeaders = GeneratedObject;
pub type ContainerSetMetadataHeaders = GeneratedObject;
pub type ContainerGetAccessPolicyHeaders = GeneratedObject;
pub type ContainerSetAccessPolicyHeaders = GeneratedObject;
pub type ContainerRestoreHeaders = GeneratedObject;
pub type ContainerSubmitBatchHeaders = GeneratedObject;
pub type ContainerFilterBlobsHeaders = GeneratedObject;
pub type ContainerAcquireLeaseHeaders = GeneratedObject;
pub type ContainerReleaseLeaseHeaders = GeneratedObject;
pub type ContainerRenewLeaseHeaders = GeneratedObject;
pub type ContainerBreakLeaseHeaders = GeneratedObject;
pub type ContainerChangeLeaseHeaders = GeneratedObject;
pub type ContainerListBlobFlatSegmentHeaders = GeneratedObject;
pub type ContainerListBlobHierarchySegmentHeaders = GeneratedObject;
pub type ContainerGetAccountInfoHeaders = GeneratedObject;
pub type ContainerGetAccountInfoWithHeadHeaders = GeneratedObject;
pub type BlobDownloadHeaders = GeneratedObject;
pub type BlobGetPropertiesHeaders = GeneratedObject;
pub type BlobDeleteHeaders = GeneratedObject;
pub type PageBlobCreateHeaders = GeneratedObject;
pub type AppendBlobCreateHeaders = GeneratedObject;
pub type BlockBlobUploadHeaders = GeneratedObject;
pub type BlockBlobPutBlobFromUrlHeaders = GeneratedObject;
pub type BlobUndeleteHeaders = GeneratedObject;
pub type BlobSetExpiryHeaders = GeneratedObject;
pub type BlobSetHTTPHeadersHeaders = GeneratedObject;
pub type BlobSetImmutabilityPolicyHeaders = GeneratedObject;
pub type BlobDeleteImmutabilityPolicyHeaders = GeneratedObject;
pub type BlobSetLegalHoldHeaders = GeneratedObject;
pub type BlobSetMetadataHeaders = GeneratedObject;
pub type BlobAcquireLeaseHeaders = GeneratedObject;
pub type BlobReleaseLeaseHeaders = GeneratedObject;
pub type BlobRenewLeaseHeaders = GeneratedObject;
pub type BlobChangeLeaseHeaders = GeneratedObject;
pub type BlobBreakLeaseHeaders = GeneratedObject;
pub type BlobCreateSnapshotHeaders = GeneratedObject;
pub type BlobStartCopyFromURLHeaders = GeneratedObject;
pub type BlobCopyFromURLHeaders = GeneratedObject;
pub type BlobAbortCopyFromURLHeaders = GeneratedObject;
pub type BlobSetTierHeaders = GeneratedObject;
pub type BlobGetAccountInfoHeaders = GeneratedObject;
pub type BlobGetAccountInfoWithHeadHeaders = GeneratedObject;
pub type BlockBlobStageBlockHeaders = GeneratedObject;
pub type BlockBlobStageBlockFromURLHeaders = GeneratedObject;
pub type BlockBlobCommitBlockListHeaders = GeneratedObject;
pub type BlockBlobGetBlockListHeaders = GeneratedObject;
pub type PageBlobUploadPagesHeaders = GeneratedObject;
pub type PageBlobClearPagesHeaders = GeneratedObject;
pub type PageBlobUploadPagesFromURLHeaders = GeneratedObject;
pub type PageBlobGetPageRangesHeaders = GeneratedObject;
pub type PageBlobGetPageRangesDiffHeaders = GeneratedObject;
pub type PageBlobResizeHeaders = GeneratedObject;
pub type PageBlobUpdateSequenceNumberHeaders = GeneratedObject;
pub type PageBlobCopyIncrementalHeaders = GeneratedObject;
pub type AppendBlobAppendBlockHeaders = GeneratedObject;
pub type AppendBlobAppendBlockFromUrlHeaders = GeneratedObject;
pub type AppendBlobSealHeaders = GeneratedObject;
pub type BlobQueryHeaders = GeneratedObject;
pub type BlobGetTagsHeaders = GeneratedObject;
pub type BlobSetTagsHeaders = GeneratedObject;
pub type ServiceSetPropertiesResponse = GeneratedResponse;
pub type ServiceGetPropertiesResponse = GeneratedResponse;
pub type ServiceGetStatisticsResponse = GeneratedResponse;
pub type ServiceListContainersSegmentResponse = GeneratedResponse;
pub type ServiceGetUserDelegationKeyResponse = GeneratedResponse;
pub type ServiceGetAccountInfoResponse = GeneratedResponse;
pub type ServiceGetAccountInfoWithHeadResponse = GeneratedResponse;
pub type ServiceSubmitBatchResponse = GeneratedResponse;
pub type ServiceFilterBlobsResponse = GeneratedResponse;
pub type ContainerCreateResponse = GeneratedResponse;
pub type ContainerGetPropertiesResponse = GeneratedResponse;
pub type ContainerGetPropertiesWithHeadResponse = GeneratedResponse;
pub type ContainerDeleteResponse = GeneratedResponse;
pub type ContainerSetMetadataResponse = GeneratedResponse;
pub type ContainerGetAccessPolicyResponse = GeneratedResponse;
pub type ContainerSetAccessPolicyResponse = GeneratedResponse;
pub type ContainerRestoreResponse = GeneratedResponse;
pub type ContainerSubmitBatchResponse = GeneratedResponse;
pub type ContainerFilterBlobsResponse = GeneratedResponse;
pub type ContainerAcquireLeaseResponse = GeneratedResponse;
pub type ContainerReleaseLeaseResponse = GeneratedResponse;
pub type ContainerRenewLeaseResponse = GeneratedResponse;
pub type ContainerBreakLeaseResponse = GeneratedResponse;
pub type ContainerChangeLeaseResponse = GeneratedResponse;
pub type ContainerListBlobFlatSegmentResponse = GeneratedResponse;
pub type ContainerListBlobHierarchySegmentResponse = GeneratedResponse;
pub type ContainerGetAccountInfoResponse = GeneratedResponse;
pub type ContainerGetAccountInfoWithHeadResponse = GeneratedResponse;
pub type BlobDownloadResponse = GeneratedResponse;
pub type BlobGetPropertiesResponse = GeneratedResponse;
pub type BlobDeleteResponse = GeneratedResponse;
pub type BlobUndeleteResponse = GeneratedResponse;
pub type BlobSetExpiryResponse = GeneratedResponse;
pub type BlobSetHTTPHeadersResponse = GeneratedResponse;
pub type BlobSetImmutabilityPolicyResponse = GeneratedResponse;
pub type BlobDeleteImmutabilityPolicyResponse = GeneratedResponse;
pub type BlobSetLegalHoldResponse = GeneratedResponse;
pub type BlobSetMetadataResponse = GeneratedResponse;
pub type BlobAcquireLeaseResponse = GeneratedResponse;
pub type BlobReleaseLeaseResponse = GeneratedResponse;
pub type BlobRenewLeaseResponse = GeneratedResponse;
pub type BlobChangeLeaseResponse = GeneratedResponse;
pub type BlobBreakLeaseResponse = GeneratedResponse;
pub type BlobCreateSnapshotResponse = GeneratedResponse;
pub type BlobStartCopyFromURLResponse = GeneratedResponse;
pub type BlobCopyFromURLResponse = GeneratedResponse;
pub type BlobAbortCopyFromURLResponse = GeneratedResponse;
pub type BlobSetTierResponse = GeneratedResponse;
pub type BlobGetAccountInfoResponse = GeneratedResponse;
pub type BlobGetAccountInfoWithHeadResponse = GeneratedResponse;
pub type BlobQueryResponse = GeneratedResponse;
pub type BlobGetTagsResponse = GeneratedResponse;
pub type BlobSetTagsResponse = GeneratedResponse;
pub type PageBlobCreateResponse = GeneratedResponse;
pub type PageBlobUploadPagesResponse = GeneratedResponse;
pub type PageBlobClearPagesResponse = GeneratedResponse;
pub type PageBlobUploadPagesFromURLResponse = GeneratedResponse;
pub type PageBlobGetPageRangesResponse = GeneratedResponse;
pub type PageBlobGetPageRangesDiffResponse = GeneratedResponse;
pub type PageBlobResizeResponse = GeneratedResponse;
pub type PageBlobUpdateSequenceNumberResponse = GeneratedResponse;
pub type PageBlobCopyIncrementalResponse = GeneratedResponse;
pub type AppendBlobCreateResponse = GeneratedResponse;
pub type AppendBlobAppendBlockResponse = GeneratedResponse;
pub type AppendBlobAppendBlockFromUrlResponse = GeneratedResponse;
pub type AppendBlobSealResponse = GeneratedResponse;
pub type BlockBlobUploadResponse = GeneratedResponse;
pub type BlockBlobPutBlobFromUrlResponse = GeneratedResponse;
pub type BlockBlobStageBlockResponse = GeneratedResponse;
pub type BlockBlobStageBlockFromURLResponse = GeneratedResponse;
pub type BlockBlobCommitBlockListResponse = GeneratedResponse;
pub type BlockBlobGetBlockListResponse = GeneratedResponse;
pub type PublicAccessType = String;
pub type CopyStatusType = String;
pub type LeaseDurationType = String;
pub type LeaseStateType = String;
pub type LeaseStatusType = String;
pub type AccessTier = String;
pub type ArchiveStatus = String;
pub type BlobType = String;
pub type RehydratePriority = String;
pub type BlobImmutabilityPolicyMode = String;
pub type StorageErrorCode = String;
pub type GeoReplicationStatusType = String;
pub type QueryFormatType = String;
pub type PremiumPageBlobAccessTier = String;
pub type BlobDeleteType = String;
pub type BlobExpiryOptions = String;
pub type BlockListType = String;
pub type BlobCopySourceTags = String;
pub type DeleteSnapshotsOptionType = String;
pub type EncryptionAlgorithmType = String;
pub type FilterBlobsIncludeItem = String;
pub type ListBlobsIncludeItem = String;
pub type ListContainersIncludeType = String;
pub type SequenceNumberActionType = String;
pub type SkuName = String;
pub type AccountKind = String;
pub type SyncCopyStatusType = String;
pub type Version = String;
pub type Version1 = String;
pub type Version2 = String;
pub type Version3 = String;
pub type Version4 = String;
pub type Version5 = String;
pub type Version6 = String;
pub type Version7 = String;
pub type Version8 = String;
pub type Version9 = String;
pub type Version10 = String;
pub type Version11 = String;
pub type Version12 = String;
pub type Version13 = String;
pub type Version14 = String;
pub type Version15 = String;
pub type Version16 = String;
pub type Version17 = String;
pub type Version18 = String;
pub type Version19 = String;
pub type Version20 = String;
pub type Version21 = String;
pub type Version22 = String;
pub type Version23 = String;
pub type Version24 = String;
pub type Version25 = String;
pub type Version26 = String;
pub type Version27 = String;
pub type Version28 = String;
pub type Version29 = String;
pub type Version30 = String;
pub type Version31 = String;
pub type Version32 = String;
pub type Version33 = String;
pub type Version34 = String;
pub type Version35 = String;
pub type Version36 = String;
pub type Version37 = String;
pub type Version38 = String;
pub type Version39 = String;
pub type Version40 = String;
pub type Version41 = String;
pub type Version42 = String;
pub type Version43 = String;
pub type Version44 = String;
pub type Version45 = String;
pub type Version46 = String;
pub type Version47 = String;
pub type Version48 = String;
pub type Version49 = String;
pub type Version50 = String;
pub type Version51 = String;
pub type Version52 = String;
pub type Version53 = String;
pub type Version54 = String;
pub type Version55 = String;
pub type Version56 = String;
pub type Version57 = String;
pub type Version58 = String;
pub type Version59 = String;
pub type Version60 = String;
pub type Version61 = String;
pub type Version62 = String;
pub type Version63 = String;
pub type Version64 = String;
pub type Version65 = String;
pub type Version66 = String;
pub type Version67 = String;
pub type Version68 = String;
pub type Version69 = String;
pub type Version70 = String;
pub type Version71 = String;
pub type Version72 = String;
