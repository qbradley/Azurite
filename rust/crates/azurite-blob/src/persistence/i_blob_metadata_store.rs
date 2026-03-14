use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::errors::StorageError;
use crate::generated::artifacts::models::{
    AccessPolicy, BlobPropertiesInternal, ContainerProperties, SignedIdentifier,
};
use crate::generated::context::Context;

/// Mirrors TypeScript `IBlobAdditionalProperties & ... & BlobItemInternal`.
/// Holds only the fields the lease subsystem needs; other BlobItemInternal
/// fields are carried in `properties` (BlobPropertiesInternal = GeneratedObject).
#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct BlobModel {
    pub properties: BlobPropertiesInternal,
    pub leaseId: Option<String>,
    pub leaseDurationSeconds: Option<i64>,
    pub leaseExpireTime: Option<DateTime<Utc>>,
    pub leaseBreakTime: Option<DateTime<Utc>>,
    /// Remaining BlobItemInternal / IBlobAdditionalProperties fields.
    pub accountName: String,
    pub containerName: String,
}

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
}

#[derive(Clone, Debug, Default)]
pub struct GetContainerAccessPolicyResponse {
    pub properties: ContainerProperties,
    pub containerAcl: Option<Vec<SignedIdentifier>>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct BlobTypeResult {
    pub blobType: Option<String>,
    pub isCommitted: bool,
}

#[async_trait]
pub trait IBlobMetadataStore: Send + Sync {
    async fn getContainerACL(
        &self,
        context: &Context,
        account: &str,
        container: &str,
    ) -> Result<Option<GetContainerAccessPolicyResponse>, StorageError>;

    async fn getBlobType(
        &self,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
    ) -> Result<Option<BlobTypeResult>, StorageError>;
}

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
    properties.get("publicAccess").and_then(|value| value.as_string())
}
