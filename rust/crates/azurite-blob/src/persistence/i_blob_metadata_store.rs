use async_trait::async_trait;

use crate::errors::StorageError;
use crate::generated::artifacts::models::{AccessPolicy, ContainerProperties, SignedIdentifier};
use crate::generated::context::Context;

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
