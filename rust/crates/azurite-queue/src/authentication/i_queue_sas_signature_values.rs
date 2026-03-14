pub use azurite_common::authentication::DateOrString;
use azurite_common::authentication::{ipRangeToString, IIPRange, SASProtocolOrString};
use azurite_common::utils::utils::{computeHMACSHA256, truncatedISO8061Date};
use serde::{Deserialize, Serialize};

const QUEUE_CANONICAL_NAME_PREFIX: &str = "/queueservices";

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
pub struct IQueueSASSignatureValues {
    pub version: String,
    pub protocol: Option<SASProtocolOrString>,
    pub startTime: Option<DateOrString>,
    pub expiryTime: Option<DateOrString>,
    pub permissions: Option<String>,
    pub ipRange: Option<IIPRangeOrString>,
    pub queueName: String,
    pub identifier: Option<String>,
}

#[allow(non_snake_case)]
pub fn generateQueueSASSignature(
    queueSASSignatureValues: &IQueueSASSignatureValues,
    accountName: &str,
    sharedKey: &[u8],
) -> (String, String) {
    ensure_permissions_and_expiry(queueSASSignatureValues);

    let stringToSign = [option_string(queueSASSignatureValues.permissions.clone()),
        option_date_string(queueSASSignatureValues.startTime.clone()),
        option_date_string(queueSASSignatureValues.expiryTime.clone()),
        getCanonicalName(accountName, &queueSASSignatureValues.queueName),
        option_string(queueSASSignatureValues.identifier.clone()),
        option_ip_range(queueSASSignatureValues.ipRange.clone()),
        option_protocol(queueSASSignatureValues.protocol.clone()),
        queueSASSignatureValues.version.clone()]
    .join("\n");

    let signature = computeHMACSHA256(&stringToSign, sharedKey);
    (signature, stringToSign)
}

pub use generateQueueSASSignature as generate_queue_sas_signature;

fn ensure_permissions_and_expiry(queueSASSignatureValues: &IQueueSASSignatureValues) {
    if queueSASSignatureValues.identifier.is_none()
        && queueSASSignatureValues.permissions.is_none()
        && queueSASSignatureValues.expiryTime.is_none()
    {
        panic!(
            "generateQueueSASSignature(): Must provide 'permissions' and 'expiryTime' for Queue SAS generation when 'identifier' is not provided."
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
fn getCanonicalName(accountName: &str, queueName: &str) -> String {
    format!("{QUEUE_CANONICAL_NAME_PREFIX}/{accountName}/{queueName}")
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::{
        generateQueueSASSignature, DateOrString, IIPRangeOrString, IQueueSASSignatureValues,
    };

    #[test]
    fn generate_queue_sas_signature_uses_queue_canonical_resource() {
        let values = IQueueSASSignatureValues {
            version: String::from("2020-08-04"),
            protocol: None,
            startTime: Some(DateOrString::Date(
                Utc.with_ymd_and_hms(2020, 4, 16, 13, 31, 48).unwrap(),
            )),
            expiryTime: Some(DateOrString::Date(
                Utc.with_ymd_and_hms(2099, 4, 16, 13, 31, 48).unwrap(),
            )),
            permissions: Some(String::from("raup")),
            ipRange: Some(IIPRangeOrString::String(String::from("10.0.0.1-10.0.0.9"))),
            queueName: String::from("queue"),
            identifier: None,
        };

        let (_, string_to_sign) = generateQueueSASSignature(&values, "devstoreaccount1", b"key");
        assert_eq!(
            string_to_sign,
            "raup\n2020-04-16T13:31:48Z\n2099-04-16T13:31:48Z\n/queueservices/devstoreaccount1/queue\n\n10.0.0.1-10.0.0.9\n\n2020-08-04"
        );
    }
}
