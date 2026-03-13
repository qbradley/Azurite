use std::fmt::{self, Display, Formatter};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, SecondsFormat, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use super::{
    account_sas_permissions::AccountSASPermissions,
    account_sas_resource_types::AccountSASResourceTypes,
    account_sas_services::AccountSASServices,
    i_ip_range::{ipRangeToString, IIPRange},
};

type HmacSha256 = Hmac<Sha256>;

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SASProtocol {
    #[serde(rename = "https")]
    HTTPS,
    #[serde(rename = "https,http")]
    HTTPSandHTTP,
}

impl SASProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HTTPS => "https",
            Self::HTTPSandHTTP => "https,http",
        }
    }
}

impl Display for SASProtocol {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SASProtocolOrString {
    SASProtocol(SASProtocol),
    String(String),
}

#[allow(non_snake_case)]
impl SASProtocolOrString {
    pub fn toString(&self) -> String {
        match self {
            Self::SASProtocol(protocol) => protocol.to_string(),
            Self::String(protocol) => protocol.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DateOrString {
    Date(DateTime<Utc>),
    String(String),
}

#[allow(non_snake_case)]
impl DateOrString {
    pub fn toString(&self) -> String {
        match self {
            Self::Date(date) => truncatedISO8061Date(date, false),
            Self::String(date) => date.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AccountSASPermissionsOrString {
    AccountSASPermissions(AccountSASPermissions),
    String(String),
}

#[allow(non_snake_case)]
impl AccountSASPermissionsOrString {
    pub fn toString(&self) -> String {
        match self {
            Self::AccountSASPermissions(permissions) => permissions.toString(),
            Self::String(permissions) => permissions.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AccountSASServicesOrString {
    AccountSASServices(AccountSASServices),
    String(String),
}

#[allow(non_snake_case)]
impl AccountSASServicesOrString {
    pub fn toString(&self) -> String {
        match self {
            Self::AccountSASServices(services) => services.toString(),
            Self::String(services) => services.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AccountSASResourceTypesOrString {
    AccountSASResourceTypes(AccountSASResourceTypes),
    String(String),
}

#[allow(non_snake_case)]
impl AccountSASResourceTypesOrString {
    pub fn toString(&self) -> String {
        match self {
            Self::AccountSASResourceTypes(resource_types) => resource_types.toString(),
            Self::String(resource_types) => resource_types.clone(),
        }
    }
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SasIPRange {
    pub start: String,
    pub end: Option<String>,
}

impl From<&SasIPRange> for IIPRange {
    fn from(value: &SasIPRange) -> Self {
        Self {
            start: value.start.clone(),
            end: value.end.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SasIPRangeOrString {
    SasIPRange(SasIPRange),
    String(String),
}

#[allow(non_snake_case)]
impl SasIPRangeOrString {
    pub fn toString(&self) -> String {
        match self {
            Self::SasIPRange(ip_range) => {
                let ip_range = IIPRange::from(ip_range);
                ipRangeToString(&ip_range)
            }
            Self::String(ip_range) => ip_range.clone(),
        }
    }
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IAccountSASSignatureValues {
    pub version: String,
    pub protocol: Option<SASProtocolOrString>,
    pub startTime: Option<DateOrString>,
    pub expiryTime: DateOrString,
    pub permissions: AccountSASPermissionsOrString,
    pub ipRange: Option<SasIPRangeOrString>,
    pub services: AccountSASServicesOrString,
    pub resourceTypes: AccountSASResourceTypesOrString,
    pub encryptionScope: Option<String>,
}

#[allow(non_snake_case)]
pub fn generateAccountSASSignature(
    accountSASSignatureValues: &IAccountSASSignatureValues,
    accountName: &str,
    sharedKey: &[u8],
) -> (String, String) {
    if accountSASSignatureValues.version.as_str() >= "2020-12-06" {
        return generateAccountSASSignature20201206(
            accountSASSignatureValues,
            accountName,
            sharedKey,
        );
    }

    generateAccountSASSignature20150405(accountSASSignatureValues, accountName, sharedKey)
}

pub use generateAccountSASSignature as generate_account_sas_signature;

#[allow(non_snake_case)]
fn generateAccountSASSignature20201206(
    accountSASSignatureValues: &IAccountSASSignatureValues,
    accountName: &str,
    sharedKey: &[u8],
) -> (String, String) {
    let parsedPermissions = accountSASSignatureValues.permissions.toString();
    let parsedServices = accountSASSignatureValues.services.toString();
    let parsedResourceTypes = accountSASSignatureValues.resourceTypes.toString();
    let parsedStartTime = match &accountSASSignatureValues.startTime {
        Some(startTime) => startTime.toString(),
        None => String::new(),
    };
    let parsedExpiryTime = accountSASSignatureValues.expiryTime.toString();
    let parsedIPRange = match &accountSASSignatureValues.ipRange {
        Some(ipRange) => ipRange.toString(),
        None => String::new(),
    };
    let parsedProtocol = match &accountSASSignatureValues.protocol {
        Some(protocol) => protocol.toString(),
        None => String::new(),
    };
    let version = accountSASSignatureValues.version.clone();
    let encryptionScope = accountSASSignatureValues
        .encryptionScope
        .clone()
        .unwrap_or_default();

    let stringToSign = vec![
        accountName.to_owned(),
        parsedPermissions,
        parsedServices,
        parsedResourceTypes,
        parsedStartTime,
        parsedExpiryTime,
        parsedIPRange,
        parsedProtocol,
        version,
        encryptionScope,
        String::new(),
    ]
    .join("\n");

    let signature = computeHMACSHA256(&stringToSign, sharedKey);
    (signature, stringToSign)
}

#[allow(non_snake_case)]
fn generateAccountSASSignature20150405(
    accountSASSignatureValues: &IAccountSASSignatureValues,
    accountName: &str,
    sharedKey: &[u8],
) -> (String, String) {
    let parsedPermissions = accountSASSignatureValues.permissions.toString();
    let parsedServices = accountSASSignatureValues.services.toString();
    let parsedResourceTypes = accountSASSignatureValues.resourceTypes.toString();
    let parsedStartTime = match &accountSASSignatureValues.startTime {
        Some(startTime) => startTime.toString(),
        None => String::new(),
    };
    let parsedExpiryTime = accountSASSignatureValues.expiryTime.toString();
    let parsedIPRange = match &accountSASSignatureValues.ipRange {
        Some(ipRange) => ipRange.toString(),
        None => String::new(),
    };
    let parsedProtocol = match &accountSASSignatureValues.protocol {
        Some(protocol) => protocol.toString(),
        None => String::new(),
    };
    let version = accountSASSignatureValues.version.clone();

    let stringToSign = vec![
        accountName.to_owned(),
        parsedPermissions,
        parsedServices,
        parsedResourceTypes,
        parsedStartTime,
        parsedExpiryTime,
        parsedIPRange,
        parsedProtocol,
        version,
        String::new(),
    ]
    .join("\n");

    let signature = computeHMACSHA256(&stringToSign, sharedKey);
    (signature, stringToSign)
}

#[allow(non_snake_case)]
fn computeHMACSHA256(stringToSign: &str, sharedKey: &[u8]) -> String {
    let mut hmac =
        HmacSha256::new_from_slice(sharedKey).expect("HMAC-SHA256 accepts arbitrary key lengths");
    hmac.update(stringToSign.as_bytes());
    let signed = hmac.finalize().into_bytes();
    STANDARD.encode(signed)
}

#[allow(non_snake_case)]
fn truncatedISO8061Date(date: &DateTime<Utc>, withMilliseconds: bool) -> String {
    let secondsFormat = if withMilliseconds {
        SecondsFormat::Millis
    } else {
        SecondsFormat::Secs
    };

    date.to_rfc3339_opts(secondsFormat, true)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::{
        generateAccountSASSignature, AccountSASPermissionsOrString,
        AccountSASResourceTypesOrString, AccountSASServicesOrString, DateOrString,
        IAccountSASSignatureValues, SASProtocol, SASProtocolOrString, SasIPRange,
        SasIPRangeOrString,
    };

    #[test]
    fn generates_20201206_account_sas_string_to_sign_with_encryption_scope() {
        let values = IAccountSASSignatureValues {
            version: "2020-12-06".to_owned(),
            protocol: Some(SASProtocolOrString::SASProtocol(SASProtocol::HTTPS)),
            startTime: Some(DateOrString::Date(
                chrono::Utc.with_ymd_and_hms(2024, 1, 2, 3, 4, 5).unwrap(),
            )),
            expiryTime: DateOrString::Date(
                chrono::Utc.with_ymd_and_hms(2024, 1, 2, 4, 5, 6).unwrap(),
            ),
            permissions: AccountSASPermissionsOrString::String("rw".to_owned()),
            ipRange: Some(SasIPRangeOrString::SasIPRange(SasIPRange {
                start: "1.1.1.1".to_owned(),
                end: Some("2.2.2.2".to_owned()),
            })),
            services: AccountSASServicesOrString::String("btqf".to_owned()),
            resourceTypes: AccountSASResourceTypesOrString::String("sco".to_owned()),
            encryptionScope: Some("scope-a".to_owned()),
        };

        let (signature, stringToSign) =
            generateAccountSASSignature(&values, "devstoreaccount1", b"key-value");

        assert_eq!(
            stringToSign,
            "devstoreaccount1\nrw\nbtqf\nsco\n2024-01-02T03:04:05Z\n2024-01-02T04:05:06Z\n1.1.1.1-2.2.2.2\nhttps\n2020-12-06\nscope-a\n"
        );
        assert_eq!(signature, "s9oEYTCYzZ9g/khhSCg2Ua/7Xjntalrb8nBf83RJEag=");
    }

    #[test]
    fn generates_20150405_account_sas_string_to_sign_without_encryption_scope() {
        let values = IAccountSASSignatureValues {
            version: "2015-04-05".to_owned(),
            protocol: Some(SASProtocolOrString::String("https,http".to_owned())),
            startTime: None,
            expiryTime: DateOrString::String("2024-01-02T04:05:06Z".to_owned()),
            permissions: AccountSASPermissionsOrString::String("rw".to_owned()),
            ipRange: None,
            services: AccountSASServicesOrString::String("btqf".to_owned()),
            resourceTypes: AccountSASResourceTypesOrString::String("sco".to_owned()),
            encryptionScope: Some("ignored-scope".to_owned()),
        };

        let (_signature, stringToSign) =
            generateAccountSASSignature(&values, "devstoreaccount1", b"key-value");

        assert_eq!(
            stringToSign,
            "devstoreaccount1\nrw\nbtqf\nsco\n\n2024-01-02T04:05:06Z\n\nhttps,http\n2015-04-05\n"
        );
    }
}
