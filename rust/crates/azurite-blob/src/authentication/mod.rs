pub mod account_sas_authenticator;
pub mod blob_sas_authenticator;
pub mod blob_sas_permissions;
pub mod blob_sas_resource_type;
pub mod blob_shared_key_authenticator;
pub mod blob_token_authenticator;
pub mod container_sas_permissions;
pub mod i_authentication_context;
pub mod i_authenticator;
pub mod i_blob_sas_signature_values;
pub mod i_range;
pub mod operation_account_sas_permission;
pub mod operation_blob_sas_permission;
pub mod public_access_authenticator;

pub use account_sas_authenticator::AccountSASAuthenticator;
pub use blob_sas_authenticator::BlobSASAuthenticator;
pub use blob_sas_permissions::BlobSASPermission;
pub use blob_sas_resource_type::BlobSASResourceType;
pub use blob_shared_key_authenticator::BlobSharedKeyAuthenticator;
pub use blob_token_authenticator::BlobTokenAuthenticator;
pub use container_sas_permissions::ContainerSASPermission;
pub use i_authentication_context::IAuthenticationContext;
pub use i_authenticator::IAuthenticator;
pub use i_blob_sas_signature_values::{
    generateBlobSASSignature, generateBlobSASSignatureWithUDK, generate_blob_sas_signature,
    generate_blob_sas_signature_with_udk, DateOrString, IBlobSASSignatureValues, IIPRangeOrString,
};
pub use i_range::{rangeToString, range_to_string, IRange, RangeError};
pub use operation_account_sas_permission::{
    OperationAccountSASPermission, OPERATION_ACCOUNT_SAS_PERMISSIONS,
};
pub use operation_blob_sas_permission::{
    OperationBlobSASPermission, OPERATION_BLOB_SAS_BLOB_PERMISSIONS,
    OPERATION_BLOB_SAS_CONTAINER_PERMISSIONS,
};
pub use public_access_authenticator::PublicAccessAuthenticator;

#[derive(Debug, Clone, Default)]
pub struct BlobAuthenticationModule;
