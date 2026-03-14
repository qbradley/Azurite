pub mod account_sas_authenticator;
pub mod i_authentication_context;
pub mod i_authenticator;
pub mod i_table_sas_signature_values;
pub mod operation_account_sas_permission;
pub mod operation_table_sas_permission;
pub mod table_sas_authenticator;
pub mod table_sas_permissions;
pub mod table_shared_key_authenticator;
pub mod table_shared_key_lite_authenticator;
pub mod table_token_authenticator;

pub use account_sas_authenticator::AccountSASAuthenticator;
pub use i_authentication_context::IAuthenticationContext;
pub use i_authenticator::IAuthenticator;
pub use i_table_sas_signature_values::{
    generateTableSASSignature, generate_table_sas_signature, DateOrString, IIPRangeOrString,
    ITableSASSignatureValues, SASProtocol, SASProtocolOrString,
};
pub use operation_account_sas_permission::{
    OperationAccountSASPermission, OPERATION_ACCOUNT_SAS_PERMISSIONS,
};
pub use operation_table_sas_permission::{
    OperationTableSASPermission, OPERATION_TABLE_SAS_TABLE_PERMISSIONS,
};
pub use table_sas_authenticator::TableSASAuthenticator;
pub use table_sas_permissions::TableSASPermission;
pub use table_shared_key_authenticator::TableSharedKeyAuthenticator;
pub use table_shared_key_lite_authenticator::TableSharedKeyLiteAuthenticator;
pub use table_token_authenticator::TableTokenAuthenticator;

#[derive(Debug, Clone, Default)]
pub struct TableAuthenticationModule;
