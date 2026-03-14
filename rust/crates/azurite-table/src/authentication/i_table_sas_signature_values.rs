use azurite_common::authentication::SasIPRangeOrString;
use azurite_common::utils::utils::computeHMACSHA256;

pub use azurite_common::authentication::SasIPRangeOrString as IIPRangeOrString;
pub use azurite_common::authentication::{DateOrString, SASProtocol, SASProtocolOrString};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ITableSASSignatureValues {
    pub version: String,
    pub protocol: Option<SASProtocolOrString>,
    pub startTime: Option<DateOrString>,
    pub expiryTime: Option<DateOrString>,
    pub permissions: Option<String>,
    pub ipRange: Option<IIPRangeOrString>,
    pub tableName: String,
    pub identifier: Option<String>,
    pub startingPartitionKey: Option<String>,
    pub startingRowKey: Option<String>,
    pub endingPartitionKey: Option<String>,
    pub endingRowKey: Option<String>,
}

#[allow(non_snake_case)]
pub fn generateTableSASSignature(
    tableSASSignatureValues: &ITableSASSignatureValues,
    accountName: &str,
    sharedKey: &[u8],
) -> (String, String) {
    if tableSASSignatureValues.version.as_str() >= "2018-11-09" {
        generateTableSASSignature20181109(tableSASSignatureValues, accountName, sharedKey)
    } else {
        generateTableSASSignature20150405(tableSASSignatureValues, accountName, sharedKey)
    }
}

pub use generateTableSASSignature as generate_table_sas_signature;

#[allow(non_snake_case)]
fn generateTableSASSignature20181109(
    tableSASSignatureValues: &ITableSASSignatureValues,
    accountName: &str,
    sharedKey: &[u8],
) -> (String, String) {
    if tableSASSignatureValues.identifier.is_none()
        && tableSASSignatureValues.permissions.is_none()
        && tableSASSignatureValues.expiryTime.is_none()
    {
        panic!(
            "generateTableSASSignature(): Must provide 'permissions' and 'expiryTime' for Table SAS generation when 'identifier' is not provided."
        );
    }

    generate_string_to_sign(tableSASSignatureValues, accountName, sharedKey)
}

#[allow(non_snake_case)]
fn generateTableSASSignature20150405(
    tableSASSignatureValues: &ITableSASSignatureValues,
    accountName: &str,
    sharedKey: &[u8],
) -> (String, String) {
    if tableSASSignatureValues.identifier.is_none()
        && tableSASSignatureValues.permissions.is_none()
        && tableSASSignatureValues.expiryTime.is_none()
    {
        panic!(
            "generateTableSASSignature(): Must provide 'permissions' and 'expiryTime' for Table SAS generation when 'identifier' is not provided."
        );
    }

    generate_string_to_sign(tableSASSignatureValues, accountName, sharedKey)
}

fn generate_string_to_sign(
    values: &ITableSASSignatureValues,
    accountName: &str,
    sharedKey: &[u8],
) -> (String, String) {
    let stringToSign = [
        values.permissions.clone().unwrap_or_default(),
        values
            .startTime
            .as_ref()
            .map(DateOrString::toString)
            .unwrap_or_default(),
        values
            .expiryTime
            .as_ref()
            .map(DateOrString::toString)
            .unwrap_or_default(),
        getCanonicalName(accountName, &values.tableName),
        values.identifier.clone().unwrap_or_default(),
        values
            .ipRange
            .as_ref()
            .map(SasIPRangeOrString::toString)
            .unwrap_or_default(),
        values
            .protocol
            .as_ref()
            .map(SASProtocolOrString::toString)
            .unwrap_or_default(),
        values.version.clone(),
        values.startingPartitionKey.clone().unwrap_or_default(),
        values.startingRowKey.clone().unwrap_or_default(),
        values.endingPartitionKey.clone().unwrap_or_default(),
        values.endingRowKey.clone().unwrap_or_default(),
    ]
    .join("\n");

    let signature = computeHMACSHA256(&stringToSign, sharedKey);
    (signature, stringToSign)
}

fn getCanonicalName(accountName: &str, tableName: &str) -> String {
    format!("/table/{accountName}/{}", tableName.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::{
        generateTableSASSignature, DateOrString, ITableSASSignatureValues, SASProtocol,
        SASProtocolOrString,
    };

    #[test]
    fn generates_table_sas_string_to_sign_with_key_ranges() {
        let values = ITableSASSignatureValues {
            version: "2018-11-09".to_owned(),
            protocol: Some(SASProtocolOrString::SASProtocol(SASProtocol::HTTPS)),
            startTime: Some(DateOrString::Date(
                chrono::Utc.with_ymd_and_hms(2024, 1, 2, 3, 4, 5).unwrap(),
            )),
            expiryTime: Some(DateOrString::Date(
                chrono::Utc.with_ymd_and_hms(2024, 1, 2, 4, 5, 6).unwrap(),
            )),
            permissions: Some("raud".to_owned()),
            ipRange: None,
            tableName: "MyTable".to_owned(),
            identifier: None,
            startingPartitionKey: Some("a".to_owned()),
            startingRowKey: Some("b".to_owned()),
            endingPartitionKey: Some("y".to_owned()),
            endingRowKey: Some("z".to_owned()),
        };

        let (_, string_to_sign) = generateTableSASSignature(&values, "devstoreaccount1", b"secret");

        assert!(string_to_sign.contains("/table/devstoreaccount1/mytable"));
        assert!(string_to_sign.ends_with("a\nb\ny\nz"));
    }
}
