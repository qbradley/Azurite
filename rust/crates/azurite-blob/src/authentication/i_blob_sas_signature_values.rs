pub use azurite_common::authentication::DateOrString;
use azurite_common::authentication::{ipRangeToString, IIPRange, SASProtocolOrString};
use azurite_common::utils::utils::{computeHMACSHA256, truncatedISO8061Date};
use serde::{Deserialize, Serialize};

use super::blob_sas_resource_type::BlobSASResourceType;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IIPRangeOrString {
    IIPRange(IIPRange),
    String(String),
}

#[allow(non_snake_case)]
impl IIPRangeOrString {
    pub fn toString(&self) -> String {
        match self {
            Self::IIPRange(ip_range) => ipRangeToString(ip_range),
            Self::String(ip_range) => ip_range.clone(),
        }
    }
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IBlobSASSignatureValues {
    pub version: String,
    pub protocol: Option<SASProtocolOrString>,
    pub startTime: Option<DateOrString>,
    pub expiryTime: Option<DateOrString>,
    pub permissions: Option<String>,
    pub ipRange: Option<IIPRangeOrString>,
    pub containerName: String,
    pub blobName: Option<String>,
    pub identifier: Option<String>,
    pub encryptionScope: Option<String>,
    pub cacheControl: Option<String>,
    pub contentDisposition: Option<String>,
    pub contentEncoding: Option<String>,
    pub contentLanguage: Option<String>,
    pub contentType: Option<String>,
    pub signedResource: Option<String>,
    pub snapshot: Option<String>,
    pub signedObjectId: Option<String>,
    pub signedTenantId: Option<String>,
    pub signedService: Option<String>,
    pub signedVersion: Option<String>,
    pub signedStartsOn: Option<String>,
    pub signedExpiresOn: Option<String>,
    pub delegatedUserObjectId: Option<String>,
    pub delegatedUserTenantId: Option<String>,
}

#[allow(non_snake_case)]
pub fn generateBlobSASSignature(
    blobSASSignatureValues: &IBlobSASSignatureValues,
    resource: BlobSASResourceType,
    accountName: &str,
    sharedKey: &[u8],
) -> (String, String) {
    if blobSASSignatureValues.version.as_str() >= "2020-12-06" {
        return generateBlobSASSignature20201206(
            blobSASSignatureValues,
            resource,
            accountName,
            sharedKey,
        );
    } else if blobSASSignatureValues.version.as_str() >= "2018-11-09" {
        return generateBlobSASSignature20181109(
            blobSASSignatureValues,
            resource,
            accountName,
            sharedKey,
        );
    }

    generateBlobSASSignature20150405(blobSASSignatureValues, resource, accountName, sharedKey)
}

pub use generateBlobSASSignature as generate_blob_sas_signature;

#[allow(non_snake_case)]
pub fn generateBlobSASSignatureWithUDK(
    blobSASSignatureValues: &IBlobSASSignatureValues,
    resource: BlobSASResourceType,
    accountName: &str,
    udkValue: &[u8],
) -> (String, String) {
    if blobSASSignatureValues.version.as_str() >= "2025-07-05" {
        return generateBlobSASBlobSASSignatureWithUDK20250705(
            blobSASSignatureValues,
            resource,
            accountName,
            udkValue,
        );
    } else if blobSASSignatureValues.version.as_str() >= "2020-12-06" {
        return generateBlobSASBlobSASSignatureWithUDK20201206(
            blobSASSignatureValues,
            resource,
            accountName,
            udkValue,
        );
    } else if blobSASSignatureValues.version.as_str() >= "2020-02-10" {
        return generateBlobSASSignatureWithUDK20200210(
            blobSASSignatureValues,
            resource,
            accountName,
            udkValue,
        );
    } else if blobSASSignatureValues.version.as_str() >= "2018-11-09" {
        return generateBlobSASSignatureUDK20181109(
            blobSASSignatureValues,
            resource,
            accountName,
            udkValue,
        );
    }

    panic!("SAS token version is not valid");
}

pub use generateBlobSASSignatureWithUDK as generate_blob_sas_signature_with_udk;

#[allow(non_snake_case)]
fn generateBlobSASSignature20201206(
    blobSASSignatureValues: &IBlobSASSignatureValues,
    resource: BlobSASResourceType,
    accountName: &str,
    sharedKey: &[u8],
) -> (String, String) {
    ensure_permissions_and_expiry(blobSASSignatureValues);

    let stringToSign = vec![
        option_string(blobSASSignatureValues.permissions.clone()),
        option_date_string(blobSASSignatureValues.startTime.clone()),
        option_date_string(blobSASSignatureValues.expiryTime.clone()),
        getCanonicalName(
            accountName,
            &blobSASSignatureValues.containerName,
            if resource == BlobSASResourceType::Blob
                || resource == BlobSASResourceType::BlobSnapshot
            {
                blobSASSignatureValues.blobName.as_deref()
            } else {
                Some("")
            },
        ),
        option_string(blobSASSignatureValues.identifier.clone()),
        option_ip_range(blobSASSignatureValues.ipRange.clone()),
        option_protocol(blobSASSignatureValues.protocol.clone()),
        blobSASSignatureValues.version.clone(),
        option_string(blobSASSignatureValues.signedResource.clone()),
        option_string(blobSASSignatureValues.snapshot.clone()),
        option_string(blobSASSignatureValues.encryptionScope.clone()),
        option_string(blobSASSignatureValues.cacheControl.clone()),
        option_string(blobSASSignatureValues.contentDisposition.clone()),
        option_string(blobSASSignatureValues.contentEncoding.clone()),
        option_string(blobSASSignatureValues.contentLanguage.clone()),
        option_string(blobSASSignatureValues.contentType.clone()),
    ]
    .join("\n");

    let signature = computeHMACSHA256(&stringToSign, sharedKey);
    (signature, stringToSign)
}

#[allow(non_snake_case)]
fn generateBlobSASSignature20181109(
    blobSASSignatureValues: &IBlobSASSignatureValues,
    resource: BlobSASResourceType,
    accountName: &str,
    sharedKey: &[u8],
) -> (String, String) {
    ensure_permissions_and_expiry(blobSASSignatureValues);

    let stringToSign = vec![
        option_string(blobSASSignatureValues.permissions.clone()),
        option_date_string(blobSASSignatureValues.startTime.clone()),
        option_date_string(blobSASSignatureValues.expiryTime.clone()),
        getCanonicalName(
            accountName,
            &blobSASSignatureValues.containerName,
            if resource == BlobSASResourceType::Blob
                || resource == BlobSASResourceType::BlobSnapshot
            {
                blobSASSignatureValues.blobName.as_deref()
            } else {
                Some("")
            },
        ),
        option_string(blobSASSignatureValues.identifier.clone()),
        option_ip_range(blobSASSignatureValues.ipRange.clone()),
        option_protocol(blobSASSignatureValues.protocol.clone()),
        blobSASSignatureValues.version.clone(),
        option_string(blobSASSignatureValues.signedResource.clone()),
        option_string(blobSASSignatureValues.snapshot.clone()),
        option_string(blobSASSignatureValues.cacheControl.clone()),
        option_string(blobSASSignatureValues.contentDisposition.clone()),
        option_string(blobSASSignatureValues.contentEncoding.clone()),
        option_string(blobSASSignatureValues.contentLanguage.clone()),
        option_string(blobSASSignatureValues.contentType.clone()),
    ]
    .join("\n");

    let signature = computeHMACSHA256(&stringToSign, sharedKey);
    (signature, stringToSign)
}

#[allow(non_snake_case)]
fn generateBlobSASSignature20150405(
    blobSASSignatureValues: &IBlobSASSignatureValues,
    resource: BlobSASResourceType,
    accountName: &str,
    sharedKey: &[u8],
) -> (String, String) {
    ensure_permissions_and_expiry(blobSASSignatureValues);

    let stringToSign = vec![
        option_string(blobSASSignatureValues.permissions.clone()),
        option_date_string(blobSASSignatureValues.startTime.clone()),
        option_date_string(blobSASSignatureValues.expiryTime.clone()),
        getCanonicalName(
            accountName,
            &blobSASSignatureValues.containerName,
            if resource == BlobSASResourceType::Blob {
                blobSASSignatureValues.blobName.as_deref()
            } else {
                Some("")
            },
        ),
        option_string(blobSASSignatureValues.identifier.clone()),
        option_ip_range(blobSASSignatureValues.ipRange.clone()),
        option_protocol(blobSASSignatureValues.protocol.clone()),
        blobSASSignatureValues.version.clone(),
        option_string(blobSASSignatureValues.cacheControl.clone()),
        option_string(blobSASSignatureValues.contentDisposition.clone()),
        option_string(blobSASSignatureValues.contentEncoding.clone()),
        option_string(blobSASSignatureValues.contentLanguage.clone()),
        option_string(blobSASSignatureValues.contentType.clone()),
    ]
    .join("\n");

    let signature = computeHMACSHA256(&stringToSign, sharedKey);
    (signature, stringToSign)
}

#[allow(non_snake_case)]
fn generateBlobSASSignatureUDK20181109(
    blobSASSignatureValues: &IBlobSASSignatureValues,
    resource: BlobSASResourceType,
    accountName: &str,
    userDelegationKeyValue: &[u8],
) -> (String, String) {
    ensure_permissions_and_expiry(blobSASSignatureValues);

    let stringToSign = vec![
        option_string(blobSASSignatureValues.permissions.clone()),
        option_date_string(blobSASSignatureValues.startTime.clone()),
        option_date_string(blobSASSignatureValues.expiryTime.clone()),
        getCanonicalName(
            accountName,
            &blobSASSignatureValues.containerName,
            blobSASSignatureValues.blobName.as_deref(),
        ),
        option_string(blobSASSignatureValues.signedObjectId.clone()),
        option_string(blobSASSignatureValues.signedTenantId.clone()),
        option_string(blobSASSignatureValues.signedStartsOn.clone()),
        option_string(blobSASSignatureValues.signedExpiresOn.clone()),
        option_string(blobSASSignatureValues.signedService.clone()),
        option_string(blobSASSignatureValues.signedVersion.clone()),
        option_ip_range(blobSASSignatureValues.ipRange.clone()),
        option_protocol(blobSASSignatureValues.protocol.clone()),
        blobSASSignatureValues.version.clone(),
        String::from(resource.as_str()),
        String::new(),
        option_string(blobSASSignatureValues.cacheControl.clone()),
        option_string(blobSASSignatureValues.contentDisposition.clone()),
        option_string(blobSASSignatureValues.contentEncoding.clone()),
        option_string(blobSASSignatureValues.contentLanguage.clone()),
        option_string(blobSASSignatureValues.contentType.clone()),
    ]
    .join("\n");

    let signature = computeHMACSHA256(&stringToSign, userDelegationKeyValue);
    (signature, stringToSign)
}

#[allow(non_snake_case)]
fn generateBlobSASSignatureWithUDK20200210(
    blobSASSignatureValues: &IBlobSASSignatureValues,
    resource: BlobSASResourceType,
    accountName: &str,
    userDelegationKeyValue: &[u8],
) -> (String, String) {
    ensure_permissions_and_expiry(blobSASSignatureValues);

    let stringToSign = vec![
        option_string(blobSASSignatureValues.permissions.clone()),
        option_date_string(blobSASSignatureValues.startTime.clone()),
        option_date_string(blobSASSignatureValues.expiryTime.clone()),
        getCanonicalName(
            accountName,
            &blobSASSignatureValues.containerName,
            blobSASSignatureValues.blobName.as_deref(),
        ),
        option_string(blobSASSignatureValues.signedObjectId.clone()),
        option_string(blobSASSignatureValues.signedTenantId.clone()),
        option_string(blobSASSignatureValues.signedStartsOn.clone()),
        option_string(blobSASSignatureValues.signedExpiresOn.clone()),
        option_string(blobSASSignatureValues.signedService.clone()),
        option_string(blobSASSignatureValues.signedVersion.clone()),
        String::new(),
        String::new(),
        String::new(),
        option_ip_range(blobSASSignatureValues.ipRange.clone()),
        option_protocol(blobSASSignatureValues.protocol.clone()),
        blobSASSignatureValues.version.clone(),
        String::from(resource.as_str()),
        String::new(),
        option_string(blobSASSignatureValues.cacheControl.clone()),
        option_string(blobSASSignatureValues.contentDisposition.clone()),
        option_string(blobSASSignatureValues.contentEncoding.clone()),
        option_string(blobSASSignatureValues.contentLanguage.clone()),
        option_string(blobSASSignatureValues.contentType.clone()),
    ]
    .join("\n");

    let signature = computeHMACSHA256(&stringToSign, userDelegationKeyValue);
    (signature, stringToSign)
}

#[allow(non_snake_case)]
fn generateBlobSASBlobSASSignatureWithUDK20201206(
    blobSASSignatureValues: &IBlobSASSignatureValues,
    resource: BlobSASResourceType,
    accountName: &str,
    userDelegationKeyValue: &[u8],
) -> (String, String) {
    ensure_permissions_and_expiry(blobSASSignatureValues);

    let stringToSign = vec![
        option_string(blobSASSignatureValues.permissions.clone()),
        option_date_string(blobSASSignatureValues.startTime.clone()),
        option_date_string(blobSASSignatureValues.expiryTime.clone()),
        getCanonicalName(
            accountName,
            &blobSASSignatureValues.containerName,
            if resource == BlobSASResourceType::Blob {
                blobSASSignatureValues.blobName.as_deref()
            } else {
                Some("")
            },
        ),
        option_string(blobSASSignatureValues.signedObjectId.clone()),
        option_string(blobSASSignatureValues.signedTenantId.clone()),
        option_string(blobSASSignatureValues.signedStartsOn.clone()),
        option_string(blobSASSignatureValues.signedExpiresOn.clone()),
        option_string(blobSASSignatureValues.signedService.clone()),
        option_string(blobSASSignatureValues.signedVersion.clone()),
        String::new(),
        String::new(),
        String::new(),
        option_ip_range(blobSASSignatureValues.ipRange.clone()),
        option_protocol(blobSASSignatureValues.protocol.clone()),
        blobSASSignatureValues.version.clone(),
        String::from(resource.as_str()),
        String::new(),
        option_string(blobSASSignatureValues.encryptionScope.clone()),
        option_string(blobSASSignatureValues.cacheControl.clone()),
        option_string(blobSASSignatureValues.contentDisposition.clone()),
        option_string(blobSASSignatureValues.contentEncoding.clone()),
        option_string(blobSASSignatureValues.contentLanguage.clone()),
        option_string(blobSASSignatureValues.contentType.clone()),
    ]
    .join("\n");

    let signature = computeHMACSHA256(&stringToSign, userDelegationKeyValue);
    (signature, stringToSign)
}

#[allow(non_snake_case)]
fn generateBlobSASBlobSASSignatureWithUDK20250705(
    blobSASSignatureValues: &IBlobSASSignatureValues,
    resource: BlobSASResourceType,
    accountName: &str,
    userDelegationKeyValue: &[u8],
) -> (String, String) {
    ensure_permissions_and_expiry(blobSASSignatureValues);

    let stringToSign = vec![
        option_string(blobSASSignatureValues.permissions.clone()),
        option_date_string(blobSASSignatureValues.startTime.clone()),
        option_date_string(blobSASSignatureValues.expiryTime.clone()),
        getCanonicalName(
            accountName,
            &blobSASSignatureValues.containerName,
            if resource == BlobSASResourceType::Blob {
                blobSASSignatureValues.blobName.as_deref()
            } else {
                Some("")
            },
        ),
        option_string(blobSASSignatureValues.signedObjectId.clone()),
        option_string(blobSASSignatureValues.signedTenantId.clone()),
        option_string(blobSASSignatureValues.signedStartsOn.clone()),
        option_string(blobSASSignatureValues.signedExpiresOn.clone()),
        option_string(blobSASSignatureValues.signedService.clone()),
        option_string(blobSASSignatureValues.signedVersion.clone()),
        String::new(),
        String::new(),
        String::new(),
        option_ip_range(blobSASSignatureValues.ipRange.clone()),
        option_protocol(blobSASSignatureValues.protocol.clone()),
        blobSASSignatureValues.version.clone(),
        String::from(resource.as_str()),
        option_string(blobSASSignatureValues.delegatedUserTenantId.clone()),
        option_string(blobSASSignatureValues.delegatedUserObjectId.clone()),
        String::new(),
        option_string(blobSASSignatureValues.encryptionScope.clone()),
        option_string(blobSASSignatureValues.cacheControl.clone()),
        option_string(blobSASSignatureValues.contentDisposition.clone()),
        option_string(blobSASSignatureValues.contentEncoding.clone()),
        option_string(blobSASSignatureValues.contentLanguage.clone()),
        option_string(blobSASSignatureValues.contentType.clone()),
    ]
    .join("\n");

    let signature = computeHMACSHA256(&stringToSign, userDelegationKeyValue);
    (signature, stringToSign)
}

fn ensure_permissions_and_expiry(blobSASSignatureValues: &IBlobSASSignatureValues) {
    if blobSASSignatureValues.identifier.is_none()
        && blobSASSignatureValues.permissions.is_none()
        && blobSASSignatureValues.expiryTime.is_none()
    {
        panic!(
            "generateBlobSASSignature(): Must provide 'permissions' and 'expiryTime' for Blob SAS generation when 'identifier' is not provided."
        );
    }
}

fn option_string(value: Option<String>) -> String {
    value.unwrap_or_default()
}

fn option_date_string(value: Option<DateOrString>) -> String {
    match value {
        Some(DateOrString::Date(value)) => truncatedISO8061Date(value, false, false),
        Some(DateOrString::String(value)) => value,
        None => String::new(),
    }
}

fn option_protocol(value: Option<SASProtocolOrString>) -> String {
    match value {
        Some(protocol) => protocol.toString(),
        None => String::new(),
    }
}

fn option_ip_range(value: Option<IIPRangeOrString>) -> String {
    match value {
        Some(ip_range) => ip_range.toString(),
        None => String::new(),
    }
}

#[allow(non_snake_case)]
fn getCanonicalName(accountName: &str, containerName: &str, blobName: Option<&str>) -> String {
    let mut elements = vec![format!("/blob/{accountName}/{containerName}")];
    if let Some(blobName) = blobName {
        if !blobName.is_empty() {
            elements.push(format!("/{blobName}"));
        }
    }
    elements.join("")
}
