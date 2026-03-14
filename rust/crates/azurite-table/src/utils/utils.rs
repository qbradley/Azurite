use std::{collections::BTreeMap, sync::LazyLock};

use azurite_common::utils::utils::truncatedISO8061Date;
use chrono::{DateTime, Utc};
use regex::Regex;
use url::Url;

use crate::{
    errors::{StorageError, StorageErrorFactory},
    generated::{
        artifacts::models::{GeneratedBody, GeneratedValue},
        context::Context,
        i_request::{IRequest, RequestHeaderValue},
    },
};

use super::constants::{
    HeaderConstants, FULL_METADATA_ACCEPT, MINIMAL_METADATA_ACCEPT, XML_METADATA,
};

static TABLE_NAME_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z][A-Za-z0-9]{2,62}$").unwrap());
static WEAK_ETAG_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^[wW]/"([^"]|\\")*"$"#).unwrap());

#[allow(non_snake_case)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ODataAnnotations {
    pub odatametadata: String,
    pub odatatype: String,
    pub odataid: String,
    pub odataeditLink: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ODataAnnotationsOptional {
    pub odatametadata: Option<String>,
    pub odatatype: Option<String>,
    pub odataid: Option<String>,
    pub odataeditLink: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableResponseProperties {
    pub tableName: String,
    pub odatatype: Option<String>,
    pub odataid: Option<String>,
    pub odataeditLink: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableODataDocument {
    pub table: String,
    pub odatametadata: Option<String>,
    pub odatatype: Option<String>,
    pub odataid: Option<String>,
    pub odataeditLink: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntityODataDocument {
    pub PartitionKey: String,
    pub RowKey: String,
    pub odatametadata: Option<String>,
    pub odatatype: Option<String>,
    pub odataid: Option<String>,
    pub odataeditLink: Option<String>,
}

pub fn get_table_odata_annotations_for_request(account: &str, table: &str) -> ODataAnnotations {
    get_odata_annotations(account, "", Some(table), None, None)
}

pub fn get_table_odata_annotations_for_response(
    account: &str,
    table: &str,
    url_prefix: &str,
) -> ODataAnnotations {
    get_odata_annotations(account, url_prefix, Some(table), None, None)
}

pub fn update_table_odata_annotations_for_response(
    mut table: TableODataDocument,
    account: &str,
    url_prefix: &str,
    accept: Option<&str>,
) -> TableODataDocument {
    let annotation = get_table_odata_annotations_for_response(account, &table.table, url_prefix);

    if matches!(accept, Some(MINIMAL_METADATA_ACCEPT | FULL_METADATA_ACCEPT)) {
        table.odatametadata = Some(annotation.odatametadata);
    }

    if matches!(accept, Some(FULL_METADATA_ACCEPT)) {
        table.odatatype = Some(annotation.odatatype);
        table.odataid = Some(annotation.odataid);
        table.odataeditLink = Some(annotation.odataeditLink);
    }

    table
}

pub fn get_table_properties_odata_annotations_for_response(
    table_name: &str,
    account: &str,
    url_prefix: &str,
    accept: Option<&str>,
) -> TableResponseProperties {
    let mut table = TableResponseProperties {
        tableName: table_name.to_string(),
        ..TableResponseProperties::default()
    };
    let annotation = get_table_odata_annotations_for_response(account, table_name, url_prefix);

    if matches!(accept, Some(FULL_METADATA_ACCEPT)) {
        table.odatatype = Some(annotation.odatatype);
        table.odataid = Some(annotation.odataid);
        table.odataeditLink = Some(annotation.odataeditLink);
    }

    table
}

pub fn update_table_optional_odata_annotations_for_response(
    mut table_like: ODataAnnotationsOptional,
    account: &str,
    table: &str,
    url_prefix: &str,
    accept: Option<&str>,
) -> ODataAnnotationsOptional {
    let annotation = get_table_odata_annotations_for_response(account, table, url_prefix);

    if matches!(accept, Some(MINIMAL_METADATA_ACCEPT | FULL_METADATA_ACCEPT)) {
        table_like.odatametadata = Some(annotation.odatametadata);
    }

    if matches!(accept, Some(FULL_METADATA_ACCEPT)) {
        table_like.odatatype = Some(annotation.odatatype);
        table_like.odataid = Some(annotation.odataid);
        table_like.odataeditLink = Some(annotation.odataeditLink);
    }

    table_like
}

pub fn get_entity_odata_annotations_for_request(
    account: &str,
    table: &str,
    partition_key: Option<&str>,
    row_key: Option<&str>,
) -> ODataAnnotations {
    get_odata_annotations(account, "", Some(table), partition_key, row_key)
}

pub fn get_entity_odata_annotations_for_response(
    account: &str,
    table: &str,
    url_prefix: &str,
    partition_key: Option<&str>,
    row_key: Option<&str>,
    accept: Option<&str>,
) -> ODataAnnotationsOptional {
    let annotation =
        get_odata_annotations(account, url_prefix, Some(table), partition_key, row_key);
    let mut response = ODataAnnotationsOptional::default();

    if matches!(accept, Some(MINIMAL_METADATA_ACCEPT | FULL_METADATA_ACCEPT)) {
        response.odatametadata = Some(annotation.odatametadata);
    }

    if matches!(accept, Some(FULL_METADATA_ACCEPT)) {
        response.odatatype = Some(annotation.odatatype);
        response.odataid = Some(annotation.odataid);
        response.odataeditLink = Some(annotation.odataeditLink);
    }

    response
}

pub fn update_entity_odata_annotations_for_response(
    mut entity: EntityODataDocument,
    account: &str,
    table: &str,
    url_prefix: &str,
    accept: Option<&str>,
) -> EntityODataDocument {
    let annotation = get_odata_annotations(
        account,
        url_prefix,
        Some(table),
        Some(&entity.PartitionKey),
        Some(&entity.RowKey),
    );

    if matches!(accept, Some(MINIMAL_METADATA_ACCEPT | FULL_METADATA_ACCEPT)) {
        entity.odatametadata = Some(annotation.odatametadata);
    }

    if matches!(accept, Some(FULL_METADATA_ACCEPT)) {
        entity.odatatype = Some(annotation.odatatype);
        entity.odataid = Some(annotation.odataid);
        entity.odataeditLink = Some(annotation.odataeditLink);
    }

    entity
}

pub fn get_odata_annotations(
    account: &str,
    url_prefix: &str,
    table: Option<&str>,
    partition_key: Option<&str>,
    row_key: Option<&str>,
) -> ODataAnnotations {
    let mut url_prefix_end_with_slash = url_prefix.to_string();
    if !url_prefix_end_with_slash.ends_with('/') {
        url_prefix_end_with_slash.push('/');
    }

    if let (Some(table), Some(partition_key), Some(row_key)) = (table, partition_key, row_key) {
        return ODataAnnotations {
            odatametadata: format!("{}$metadata#{}/@Element", url_prefix_end_with_slash, table),
            odatatype: format!("{}.{}", account, table),
            odataid: format!(
                "{}{table}(PartitionKey='{partition_key}',RowKey='{row_key}')",
                url_prefix_end_with_slash
            ),
            odataeditLink: format!("{table}(PartitionKey='{partition_key}',RowKey='{row_key}')"),
        };
    }

    let table = table.unwrap_or_default();
    let metadata_suffix = if table.is_empty() { "" } else { "/@Element" };
    ODataAnnotations {
        odatametadata: format!(
            "{}$metadata#Tables{}",
            url_prefix_end_with_slash, metadata_suffix
        ),
        odatatype: format!("{}.Tables", account),
        odataid: format!("{}Tables('{table}')", url_prefix_end_with_slash),
        odataeditLink: format!("Tables('{table}')"),
    }
}

pub fn check_api_version(
    input_api_version: &str,
    valid_api_versions: &[&str],
    context: &Context,
) -> Result<(), StorageError> {
    if !valid_api_versions.contains(&input_api_version) {
        return Err(StorageErrorFactory::getInvalidAPIVersion(
            context,
            Some(input_api_version),
        ));
    }

    Ok(())
}

pub fn get_timestamp_string(date: DateTime<Utc>) -> String {
    truncatedISO8061Date(date, true, false)
}

pub fn get_payload_format(context: &Context) -> String {
    let mut format = context
        .request()
        .and_then(|request| request.getHeader(HeaderConstants.ACCEPT));

    if let Some(format_parameter) = context
        .request()
        .and_then(|request| request.getQuery("$format"))
    {
        format = Some(format_parameter);
    }

    let mut format = format.unwrap_or_else(|| XML_METADATA.to_string());
    if format.is_empty() {
        format = XML_METADATA.to_string();
    }
    if format == HeaderConstants.APPLICATION_JSON {
        format = MINIMAL_METADATA_ACCEPT.to_string();
    }

    format.retain(|ch| !ch.is_whitespace());
    format
}

pub fn validate_table_name(context: &Context, table_name: &str) -> Result<(), StorageError> {
    if !table_name.is_empty() && (table_name.len() < 3 || table_name.len() > 63) {
        return Err(StorageErrorFactory::getOutOfRangeName(context));
    }

    if !TABLE_NAME_REGEX.is_match(table_name) && !table_name.starts_with("$Metric") {
        return Err(StorageErrorFactory::getInvalidResourceName(context));
    }

    if table_name.eq_ignore_ascii_case("tables") {
        return Err(StorageErrorFactory::getInvalidResourceName(context));
    }

    Ok(())
}

pub fn new_table_entity_etag(high_pres_mod_time: &str) -> String {
    format!("W/\"datetime'{}'\"", high_pres_mod_time.replace(':', "%3A"))
}

pub fn new_high_precision_timestamp(start_time: DateTime<Utc>) -> String {
    truncatedISO8061Date(start_time, true, true)
}

/// Preserves TypeScript parity: returns `true` when the supplied value fails weak-ETag validation.
pub fn is_etag_valid(etag: &str) -> bool {
    !WEAK_ETAG_REGEX.is_match(etag)
}

pub fn get_utf8_byte_size(text: &str) -> usize {
    text.len()
}

pub fn generated_body_to_string(body: Option<&GeneratedBody>) -> Option<String> {
    match body {
        Some(GeneratedBody::Text(value)) => Some(value.clone()),
        Some(GeneratedBody::Value(value)) => Some(match value {
            GeneratedValue::String(value) => value.clone(),
            _ => serde_json::to_string(&value.to_json_value()).unwrap_or_default(),
        }),
        Some(GeneratedBody::Stream(stream)) => Some(stream.read_to_string()),
        None => None,
    }
}

pub fn parse_header_block(header_block: &str) -> BTreeMap<String, RequestHeaderValue> {
    let mut headers = BTreeMap::new();

    for line in header_block
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let key = name.trim().to_ascii_lowercase();
        let value = value.trim().to_string();
        match headers.remove(&key) {
            Some(RequestHeaderValue::Single(existing)) => {
                headers.insert(key, RequestHeaderValue::Multi(vec![existing, value]));
            }
            Some(RequestHeaderValue::Multi(mut existing)) => {
                existing.push(value);
                headers.insert(key, RequestHeaderValue::Multi(existing));
            }
            None => {
                headers.insert(key, RequestHeaderValue::Single(value));
            }
        }
    }

    headers
}

pub fn extract_path_from_uri(uri: &str) -> Option<String> {
    Url::parse(uri)
        .ok()
        .map(|url| url.path().to_string())
        .or_else(|| {
            uri.split_once("://")
                .and_then(|(_, rest)| rest.find('/').map(|index| rest[index..].to_string()))
        })
}

pub fn query_map_from_uri(uri: &str) -> BTreeMap<String, String> {
    if let Ok(url) = Url::parse(uri) {
        return url.query_pairs().into_owned().collect();
    }

    uri.split_once('?')
        .map(|(_, query)| {
            url::form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .collect()
        })
        .unwrap_or_default()
}

pub fn extract_entity_keys_from_url_or_body(
    uri: &str,
    body: Option<&str>,
) -> (Option<String>, Option<String>) {
    let decoded_uri = percent_decode_string(uri);
    let partition_key = extract_key_from_uri(&decoded_uri, "PartitionKey='", "',")
        .map(decode_entity_key)
        .or_else(|| extract_entity_key_from_body(body, "PartitionKey"));
    let row_key = extract_key_from_uri(&decoded_uri, "RowKey='", "')")
        .map(decode_entity_key)
        .or_else(|| extract_entity_key_from_body(body, "RowKey"));

    (partition_key, row_key)
}

fn extract_key_from_uri(uri: &str, prefix: &str, suffix: &str) -> Option<String> {
    let start = uri.find(prefix)? + prefix.len();
    let remainder = &uri[start..];
    let end = remainder.find(suffix)?;
    Some(remainder[..end].to_string())
}

fn extract_entity_key_from_body(body: Option<&str>, key: &str) -> Option<String> {
    body.and_then(|body| serde_json::from_str::<serde_json::Value>(body).ok())
        .and_then(|value| {
            value
                .get(key)
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
        })
}

fn decode_entity_key(value: String) -> String {
    value.replace("''", "'")
}

fn percent_decode_string(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hi = (bytes[index + 1] as char).to_digit(16);
            let lo = (bytes[index + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                output.push(((hi << 4) | lo) as u8);
                index += 3;
                continue;
            }
        }
        output.push(bytes[index]);
        index += 1;
    }

    String::from_utf8(output).unwrap_or_else(|_| value.to_string())
}

pub use check_api_version as checkApiVersion;
pub use extract_entity_keys_from_url_or_body as extractEntityKeysFromUrlOrBody;
pub use extract_path_from_uri as extractPathFromUri;
pub use generated_body_to_string as generatedBodyToString;
pub use get_entity_odata_annotations_for_request as getEntityOdataAnnotationsForRequest;
pub use get_entity_odata_annotations_for_response as getEntityOdataAnnotationsForResponse;
pub use get_odata_annotations as getOdataAnnotations;
pub use get_payload_format as getPayloadFormat;
pub use get_table_odata_annotations_for_request as getTableOdataAnnotationsForRequest;
pub use get_table_odata_annotations_for_response as getTableOdataAnnotationsForResponse;
pub use get_table_properties_odata_annotations_for_response as getTablePropertiesOdataAnnotationsForResponse;
pub use get_timestamp_string as getTimestampString;
pub use get_utf8_byte_size as getUTF8ByteSize;
pub use is_etag_valid as isEtagValid;
pub use new_high_precision_timestamp as newHighPrecisionTimeStamp;
pub use new_table_entity_etag as newTableEntityEtag;
pub use parse_header_block as parseHeaderBlock;
pub use query_map_from_uri as queryMapFromUri;
pub use update_entity_odata_annotations_for_response as updateEntityOdataAnnotationsForResponse;
pub use update_table_odata_annotations_for_response as updateTableOdataAnnotationsForResponse;
pub use update_table_optional_odata_annotations_for_response as updateTableOptionalOdataAnnotationsForResponse;
pub use validate_table_name as validateTableName;

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use crate::generated::{
        context::Context,
        i_request::{GeneratedHttpRequest, RequestHeaderValue},
    };

    use crate::utils::constants::{FULL_METADATA_ACCEPT, MINIMAL_METADATA_ACCEPT, XML_METADATA};

    use super::{
        get_payload_format, get_table_odata_annotations_for_response, get_utf8_byte_size,
        is_etag_valid, new_high_precision_timestamp, new_table_entity_etag, validate_table_name,
    };

    #[test]
    fn get_payload_format_prefers_query_parameter() {
        let context = Context::default();
        let mut request = GeneratedHttpRequest::new();
        request.headers.insert(
            "accept".to_string(),
            RequestHeaderValue::Single(" application/json ".to_string()),
        );
        request
            .query
            .insert("$format".to_string(), FULL_METADATA_ACCEPT.to_string());
        context.setRequest(Some(request));

        assert_eq!(get_payload_format(&context), FULL_METADATA_ACCEPT);
    }

    #[test]
    fn get_payload_format_defaults_to_xml() {
        assert_eq!(get_payload_format(&Context::default()), XML_METADATA);
    }

    #[test]
    fn get_payload_format_normalizes_application_json() {
        let context = Context::default();
        let mut request = GeneratedHttpRequest::new();
        request.headers.insert(
            "accept".to_string(),
            RequestHeaderValue::Single("application/json".to_string()),
        );
        context.setRequest(Some(request));

        assert_eq!(get_payload_format(&context), MINIMAL_METADATA_ACCEPT);
    }

    #[test]
    fn validate_table_name_accepts_metric_tables() {
        validate_table_name(&Context::default(), "$MetricsCapacityBlob").unwrap();
    }

    #[test]
    fn validate_table_name_rejects_reserved_tables_name() {
        assert!(validate_table_name(&Context::default(), "Tables").is_err());
    }

    #[test]
    fn new_table_entity_etag_encodes_colons() {
        assert_eq!(
            new_table_entity_etag("2025-03-14T10:09:08.1234Z"),
            "W/\"datetime'2025-03-14T10%3A09%3A08.1234Z'\""
        );
    }

    #[test]
    fn new_high_precision_timestamp_preserves_expected_shape() {
        let timestamp = new_high_precision_timestamp(
            Utc.with_ymd_and_hms(2025, 3, 14, 10, 9, 8)
                .single()
                .unwrap(),
        );
        assert!(timestamp.starts_with("2025-03-14T10:09:08.000"));
        assert!(timestamp.ends_with('Z'));
    }

    #[test]
    fn is_etag_valid_matches_legacy_parity() {
        assert!(!is_etag_valid(
            "W/\"datetime'2025-03-14T10%3A09%3A08.1234Z'\""
        ));
        assert!(is_etag_valid("not-an-etag"));
    }

    #[test]
    fn get_utf8_byte_size_counts_multibyte_text() {
        assert_eq!(get_utf8_byte_size("Aß你"), 6);
    }

    #[test]
    fn get_table_odata_annotations_for_response_builds_expected_urls() {
        let annotation = get_table_odata_annotations_for_response(
            "devstoreaccount1",
            "Customers",
            "http://127.0.0.1:10002/devstoreaccount1",
        );
        assert_eq!(
            annotation.odatametadata,
            "http://127.0.0.1:10002/devstoreaccount1/$metadata#Tables/@Element"
        );
        assert_eq!(
            annotation.odataid,
            "http://127.0.0.1:10002/devstoreaccount1/Tables('Customers')"
        );
    }
}
