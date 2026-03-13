pub mod account_sas_permissions;
pub mod account_sas_resource_types;
pub mod account_sas_services;
pub mod i_account_sas_signature_values;
pub mod i_ip_range;

pub use account_sas_permissions::{AccountSASPermission, AccountSASPermissions};
pub use account_sas_resource_types::{AccountSASResourceType, AccountSASResourceTypes};
pub use account_sas_services::{AccountSASService, AccountSASServices};
pub use i_account_sas_signature_values::{
    generateAccountSASSignature, generate_account_sas_signature, AccountSASPermissionsOrString,
    AccountSASResourceTypesOrString, AccountSASServicesOrString, DateOrString,
    IAccountSASSignatureValues, SASProtocol, SASProtocolOrString, SasIPRange, SasIPRangeOrString,
};
pub use i_ip_range::{ipRangeToString, ip_range_to_string, IIPRange};

#[derive(Debug, Clone, Default)]
pub struct AuthenticationModule;
