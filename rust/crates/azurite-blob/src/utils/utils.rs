use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
    sync::LazyLock,
};

use azurite_common::utils::utils::computeHMACSHA256;
use regex::Regex;
use thiserror::Error;
use tokio::{fs::File, io::AsyncRead, io::AsyncWriteExt};
use url::form_urlencoded;

use crate::{
    errors::{StorageError, StorageErrorFactory},
    generated::artifacts::models::{BlobTag, BlobTags, GeneratedValue},
    persistence::query_interpreter::query_nodes::TagContent,
    utils::constants::USERDELEGATIONKEY_BASIC_KEY,
};

pub type BlobRange = (u64, u64);

static CONTAINER_NAME_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9](?!.*--)[a-z0-9-]{1,61}[a-z0-9]$").unwrap());

#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[error("{message}")]
pub struct RangeError {
    pub message: String,
}

impl RangeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[allow(non_snake_case)]
pub fn checkApiVersion(
    inputApiVersion: &str,
    validApiVersions: &[&str],
    requestId: &str,
) -> Result<(), StorageError> {
    if !validApiVersions.contains(&inputApiVersion) {
        return Err(StorageErrorFactory::getInvalidAPIVersion(
            Some(requestId),
            Some(inputApiVersion),
        ));
    }

    Ok(())
}

#[allow(non_snake_case)]
pub async fn streamToLocalFile<R, P>(mut stream: R, path: P) -> std::io::Result<()>
where
    R: AsyncRead + Unpin,
    P: AsRef<Path>,
{
    let mut writeStream = File::create(path).await?;
    tokio::io::copy(&mut stream, &mut writeStream).await?;
    writeStream.flush().await
}

#[allow(non_snake_case)]
pub fn deserializeRangeHeader(
    rangeHeaderValue: Option<&str>,
    xMsRangeHeaderValue: Option<&str>,
) -> Result<Option<BlobRange>, RangeError> {
    let Some(range) = xMsRangeHeaderValue.or(rangeHeaderValue) else {
        return Ok(None);
    };

    let (_, rangeValue) = range.split_once('=').ok_or_else(|| {
        RangeError::new(format!(
            "deserializeRangeHeader: raw range value {range} is wrong."
        ))
    })?;
    let parts: Vec<&str> = rangeValue.split('-').collect();
    if parts.is_empty() || parts.len() > 2 {
        return Err(RangeError::new(format!(
            "deserializeRangeHeader: raw range value {range} is wrong."
        )));
    }

    let startInclusive = parts[0].parse::<u64>().map_err(|_| {
        RangeError::new(format!(
            "deserializeRangeHeader: raw range value {range} is wrong."
        ))
    })?;
    let mut endInclusive = u64::MAX;
    if let Some(end) = parts.get(1).copied().filter(|value| !value.is_empty()) {
        endInclusive = end.parse::<u64>().map_err(|_| {
            RangeError::new(format!(
                "deserializeRangeHeader: raw range value {range} is wrong."
            ))
        })?;
    }

    if startInclusive > endInclusive {
        return Err(RangeError::new(format!(
            "deserializeRangeHeader: raw range value {range} is wrong."
        )));
    }

    Ok(Some((startInclusive, endInclusive)))
}

#[allow(non_snake_case)]
pub fn deserializePageBlobRangeHeader(
    rangeHeaderValue: Option<&str>,
    xMsRangeHeaderValue: Option<&str>,
    force512boundary: bool,
) -> Result<BlobRange, RangeError> {
    let ranges = deserializeRangeHeader(rangeHeaderValue, xMsRangeHeaderValue)?;
    let startInclusive = ranges.map(|range| range.0).unwrap_or(0);
    let endInclusive = ranges.map(|range| range.1).unwrap_or(u64::MAX);

    if force512boundary && startInclusive % 512 != 0 {
        return Err(RangeError::new(format!(
            "deserializePageBlobRangeHeader: range start value {startInclusive} doesn't align with 512 boundary."
        )));
    }

    if force512boundary && endInclusive != u64::MAX && (endInclusive + 1) % 512 != 0 {
        return Err(RangeError::new(format!(
            "deserializePageBlobRangeHeader: range end value {endInclusive} doesn't align with 512 boundary."
        )));
    }

    Ok((startInclusive, endInclusive))
}

#[allow(non_snake_case)]
pub fn removeQuotationFromListBlobEtag(inputEtag: Option<&str>) -> Option<String> {
    let inputEtag = inputEtag?;
    if inputEtag.starts_with('"') && inputEtag.ends_with('"') && inputEtag.len() >= 2 {
        return Some(inputEtag[1..inputEtag.len() - 1].to_string());
    }

    Some(inputEtag.to_string())
}

#[allow(non_snake_case)]
pub fn validateContainerName(requestID: &str, containerName: &str) -> Result<(), StorageError> {
    if !containerName.is_empty() && (containerName.len() < 3 || containerName.len() > 63) {
        return Err(StorageErrorFactory::getOutOfRangeName(Some(requestID)));
    }

    if !CONTAINER_NAME_REGEX.is_match(containerName) {
        return Err(StorageErrorFactory::getInvalidResourceName(Some(requestID)));
    }

    Ok(())
}

#[allow(non_snake_case)]
pub fn getUserDelegationKeyValue(
    signedObjectid: &str,
    signedTenantid: &str,
    signedStartsOn: &str,
    signedExpiresOn: &str,
    signedVersion: &str,
) -> String {
    let stringToSign = [
        signedObjectid,
        signedTenantid,
        signedStartsOn,
        signedExpiresOn,
        "b",
        signedVersion,
    ]
    .join("\n");

    computeHMACSHA256(&stringToSign, USERDELEGATIONKEY_BASIC_KEY.as_bytes())
}

#[allow(non_snake_case)]
pub fn getBlobTagsCount(blobTags: Option<&BlobTags>) -> Option<usize> {
    let blobTagSet = getBlobTagSet(blobTags?);
    if blobTagSet.is_empty() {
        None
    } else {
        Some(blobTagSet.len())
    }
}

#[allow(non_snake_case)]
pub fn getTagsFromString(
    blobTagsString: &str,
    contextID: &str,
) -> Result<Option<BlobTags>, StorageError> {
    if blobTagsString.is_empty() {
        return Ok(None);
    }

    let mut blobTags = Vec::new();
    for rawTag in blobTagsString.split('&') {
        let (key, value) = rawTag.split_once('=').unwrap_or((rawTag, ""));
        blobTags.push(createBlobTag(
            &decodeBlobTagComponent(key),
            &decodeBlobTagComponent(value),
        ));
    }

    let tags = createBlobTags(blobTags);
    validateBlobTag(&tags, contextID)?;
    Ok(Some(tags))
}

#[allow(non_snake_case)]
pub fn validateBlobTag(tags: &BlobTags, contextID: &str) -> Result<(), StorageError> {
    let blobTagSet = getBlobTagSet(tags);
    if blobTagSet.len() > 10 {
        return Err(StorageErrorFactory::getTagsTooLarge(contextID));
    }

    for tag in blobTagSet {
        let key = getBlobTagString(&tag, "key");
        let value = getBlobTagString(&tag, "value");
        if key.is_empty() {
            return Err(StorageErrorFactory::getEmptyTagName(contextID));
        }
        if key.len() > 128 || value.len() > 256 {
            return Err(StorageErrorFactory::getTagsTooLarge(contextID));
        }
        if containsInvalidTagCharacter(&key) || containsInvalidTagCharacter(&value) {
            return Err(StorageErrorFactory::getInvalidTag(contextID));
        }
    }

    Ok(())
}

#[allow(non_snake_case)]
pub fn toBlobTags(input: &[TagContent]) -> Vec<BlobTag> {
    let mut tags: HashMap<String, String> = HashMap::new();
    for element in input {
        if element.key.as_deref() != Some("@container") {
            if let (Some(key), Some(value)) = (element.key.as_ref(), element.value.as_ref()) {
                tags.insert(key.clone(), value.clone());
            }
        }
    }

    tags.into_iter()
        .map(|(key, value)| createBlobTag(&key, &value))
        .collect()
}

#[allow(non_snake_case)]
fn decodeBlobTagComponent(component: &str) -> String {
    form_urlencoded::parse(format!("value={component}").as_bytes())
        .next()
        .map(|(_, value)| value.into_owned())
        .unwrap_or_default()
}

#[allow(non_snake_case)]
fn containsInvalidTagCharacter(s: &str) -> bool {
    for c in s.chars() {
        if !(c.is_ascii_lowercase()
            || c.is_ascii_uppercase()
            || c.is_ascii_digit()
            || matches!(c, ' ' | '+' | '-' | '.' | '/' | ':' | '=' | '_'))
        {
            return true;
        }
    }

    false
}

#[allow(non_snake_case)]
fn createBlobTag(key: &str, value: &str) -> BlobTag {
    BTreeMap::from([
        ("key".to_string(), GeneratedValue::String(key.to_string())),
        (
            "value".to_string(),
            GeneratedValue::String(value.to_string()),
        ),
    ])
}

#[allow(non_snake_case)]
fn createBlobTags(blobTagSet: Vec<BlobTag>) -> BlobTags {
    BTreeMap::from([(
        "blobTagSet".to_string(),
        GeneratedValue::Array(blobTagSet.into_iter().map(GeneratedValue::Object).collect()),
    )])
}

#[allow(non_snake_case)]
fn getBlobTagSet(tags: &BlobTags) -> Vec<BlobTag> {
    match tags.get("blobTagSet") {
        Some(GeneratedValue::Array(blobTagSet)) => blobTagSet
            .iter()
            .filter_map(|value| match value {
                GeneratedValue::Object(tag) => Some(tag.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

#[allow(non_snake_case)]
fn getBlobTagString(tag: &BlobTag, key: &str) -> String {
    tag.get(key)
        .and_then(GeneratedValue::as_string)
        .unwrap_or_default()
}
