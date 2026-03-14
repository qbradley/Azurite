pub mod i_blob_metadata_store;

pub use i_blob_metadata_store::{
    access_policy_field, container_public_access, signed_identifier_access_policy,
    signed_identifier_id, BlobModel, BlobTypeResult, ContainerModel,
    GetContainerAccessPolicyResponse, IBlobMetadataStore,
};

#[derive(Debug, Clone, Default)]
pub struct BlobPersistenceModule;
