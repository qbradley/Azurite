pub mod account_sas_authenticator;
pub mod i_authentication_context;
pub mod i_authenticator;
pub mod i_queue_sas_signature_values;
pub mod operation_account_sas_permission;
pub mod operation_queue_sas_permission;
pub mod queue_sas_authenticator;
pub mod queue_sas_permissions;
pub mod queue_shared_key_authenticator;
pub mod queue_token_authenticator;

pub use account_sas_authenticator::AccountSASAuthenticator;
pub use i_authentication_context::IAuthenticationContext;
pub use i_authenticator::IAuthenticator;
pub use i_queue_sas_signature_values::{
    generateQueueSASSignature, generate_queue_sas_signature, IIPRangeOrString,
    IQueueSASSignatureValues,
};
pub use operation_account_sas_permission::{
    OperationAccountSASPermission, OPERATION_ACCOUNT_SAS_PERMISSIONS,
};
pub use operation_queue_sas_permission::{
    OperationQueueSASPermission, OPERATION_QUEUE_SAS_PERMISSIONS,
};
pub use queue_sas_authenticator::{QueueSASAccessPolicySource, QueueSASAuthenticator};
pub use queue_sas_permissions::QueueSASPermission;
pub use queue_shared_key_authenticator::QueueSharedKeyAuthenticator;
pub use queue_token_authenticator::QueueTokenAuthenticator;

#[derive(Debug, Clone, Default)]
pub struct QueueAuthenticationModule;
