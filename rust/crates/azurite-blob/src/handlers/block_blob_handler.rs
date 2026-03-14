use std::collections::BTreeMap;

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use bytes::Bytes;
use chrono::Utc;

use azurite_common::persistence::i_extent_store::{
    ExtentDataInput, IExtentChunk as CommonExtentChunk,
};
use azurite_common::utils::utils::{
    convertRawHeadersToMetadata, getMD5FromStream, getMD5FromString, newEtag,
};

use crate::context::blob_storage_context::BlobStorageContext;
use crate::errors::{NotImplementedError, StorageError, StorageErrorFactory};
use crate::generated::artifacts::models::{self, GeneratedResponse, GeneratedValue};
use crate::generated::context::Context;
use crate::generated::handlers::i_block_blob_handler::IBlockBlobHandler;
use crate::generated::i_request::{GeneratedReadableStream, IRequest};
use crate::handlers::base_handler::BaseHandler;
use crate::persistence::{BlobModel, BlockListEntry, BlockModel, IExtentChunk};

// ── Constants ────────────────────────────────────────────────────────────────

const BLOB_API_VERSION: &str = "2025-11-05";
const ACCESS_TIER_HOT: &str = "Hot";
const ACCESS_TIER_COOL: &str = "Cool";
const ACCESS_TIER_ARCHIVE: &str = "Archive";
const ACCESS_TIER_COLD: &str = "Cold";

// ── Handler struct ───────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct BlockBlobHandler {
    pub base: BaseHandler,
}

impl BlockBlobHandler {
    pub fn new(base: BaseHandler) -> Self {
        Self { base }
    }

    fn parse_tier(tier: &str) -> Option<String> {
        match tier.to_lowercase().as_str() {
            "hot" => Some(ACCESS_TIER_HOT.to_string()),
            "cool" => Some(ACCESS_TIER_COOL.to_string()),
            "archive" => Some(ACCESS_TIER_ARCHIVE.to_string()),
            "cold" => Some(ACCESS_TIER_COLD.to_string()),
            _ => None,
        }
    }

    fn validate_block_id(block_id: &str, context_id: Option<&str>) -> Result<(), StorageError> {
        let decoded = STANDARD.decode(block_id).map_err(|_| {
            StorageErrorFactory::getInvalidQueryParameterValue(
                context_id,
                Some("blockid"),
                Some(block_id),
                Some("Not a valid base64 string."),
            )
        })?;

        // Canonical base64 round-trip check
        if STANDARD.encode(&decoded) != block_id {
            return Err(StorageErrorFactory::getInvalidQueryParameterValue(
                context_id,
                Some("blockid"),
                Some(block_id),
                Some("Not a valid base64 string."),
            ));
        }

        if decoded.len() > 64 {
            return Err(StorageErrorFactory::getOutOfRangeInput(
                context_id,
                Some("blockid"),
                Some(block_id),
                Some("Block ID length cannot exceed 64."),
            ));
        }

        Ok(())
    }
}

// ── Private helpers ──────────────────────────────────────────────────────────

/// Convert azurite-common IExtentChunk (u64 fields) → azurite-blob IExtentChunk (i64 fields).
fn to_blob_extent(c: CommonExtentChunk) -> IExtentChunk {
    IExtentChunk {
        id: c.id,
        offset: c.offset as i64,
        count: c.count as i64,
    }
}

/// Convert azurite-blob IExtentChunk back to the common form needed by readExtent.
fn to_common_extent(b: &IExtentChunk) -> CommonExtentChunk {
    CommonExtentChunk {
        id: b.id.clone(),
        offset: b.offset as u64,
        count: b.count as u64,
    }
}

/// Map common StorageError to blob StorageError.
fn map_common_err(
    _e: azurite_common::storage_error::StorageError,
    context_id: Option<&str>,
) -> StorageError {
    StorageErrorFactory::getInvalidMetadata(context_id.unwrap_or_default())
}

/// Convert a raw metadata HashMap to a GeneratedObject (BTreeMap<String, GeneratedValue>).
fn to_generated_metadata(m: std::collections::HashMap<String, String>) -> models::BlobMetadata {
    m.into_iter()
        .map(|(k, v)| (k, GeneratedValue::String(v)))
        .collect()
}

/// Parse URL-encoded tag string "k1=v1&k2=v2" into a `BlobTags` GeneratedObject.
fn get_tags_from_string(tags_string: &str, _context_id: &str) -> Option<GeneratedValue> {
    if tags_string.is_empty() {
        return None;
    }
    let mut tags_array: Vec<GeneratedValue> = Vec::new();
    for raw_tag in tags_string.split('&') {
        let mut parts = raw_tag.splitn(2, '=');
        let key_enc = parts.next().unwrap_or("");
        let val_enc = parts.next().unwrap_or("");
        let key = decode_uri_component(key_enc);
        let val = decode_uri_component(val_enc);
        let mut tag = models::BlobTags::new();
        tag.insert("key".to_string(), GeneratedValue::String(key));
        tag.insert("value".to_string(), GeneratedValue::String(val));
        tags_array.push(GeneratedValue::Object(tag));
    }
    let mut tags_obj = models::BlobTags::new();
    tags_obj.insert("blobTagSet".to_string(), GeneratedValue::Array(tags_array));
    Some(GeneratedValue::Object(tags_obj))
}

fn decode_uri_component(s: &str) -> String {
    let s = s.replace('+', " ");
    let mut result = String::new();
    let mut bytes = s.as_bytes().iter().peekable();
    while let Some(&b) = bytes.next() {
        if b == b'%' {
            let h1 = bytes.next().copied().unwrap_or(0);
            let h2 = bytes.next().copied().unwrap_or(0);
            let hex = [h1, h2];
            if let Ok(s) = std::str::from_utf8(&hex) {
                if let Ok(byte) = u8::from_str_radix(s, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push('%');
            result.push(h1 as char);
            result.push(h2 as char);
        } else {
            result.push(b as char);
        }
    }
    result
}

/// Parse the raw XML block list body into ordered `BlockListEntry` items.
/// Uses quick_xml event-based parsing to preserve element order and handle
/// multiple elements of the same type (Committed/Uncommitted/Latest).
fn parse_commit_block_list_xml(xml_str: &str) -> Result<Vec<BlockListEntry>, ()> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml_str);
    reader.config_mut().trim_text(true);

    let mut entries: Vec<BlockListEntry> = Vec::new();
    let mut current_tag: Option<String> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match tag.as_str() {
                    "Committed" | "Uncommitted" | "Latest" => {
                        current_tag = Some(tag);
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                if let Some(tag) = current_tag.take() {
                    let text = e.unescape().map_err(|_| ())?.to_string();
                    entries.push(BlockListEntry {
                        blockName: text,
                        blockCommitType: tag,
                    });
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => return Err(()),
            _ => {}
        }
    }
    Ok(entries)
}

// ── Trait implementation ─────────────────────────────────────────────────────

#[allow(non_snake_case)]
#[async_trait]
impl IBlockBlobHandler for BlockBlobHandler {
    async fn upload(
        &self,
        body: GeneratedReadableStream,
        contentLength: f64,
        options: models::BlockBlobUploadOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlockBlobUploadResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account_name = blobCtx.account().unwrap_or_default();
        let container_name = blobCtx.container().unwrap_or_default();
        let blob_name = blobCtx.blob().unwrap_or_default();
        let date = context.startTime().unwrap_or_else(Utc::now);
        let context_id = context.contextId();
        let context_id_str = context_id.as_deref().unwrap_or("");

        let request = context.request().unwrap_or_default();

        let blob_http_headers = options
            .get("blobHTTPHeaders")
            .and_then(GeneratedValue::as_object)
            .cloned()
            .unwrap_or_default();

        let content_type = blob_http_headers
            .get("blobContentType")
            .and_then(GeneratedValue::as_string)
            .or_else(|| request.getHeader("content-type"))
            .unwrap_or_else(|| "application/octet-stream".to_string());

        // Determine expected contentMD5: only if content-md5 or x-ms-blob-content-md5 header present
        let has_md5_header = request.getHeader("content-md5").is_some()
            || request.getHeader("x-ms-blob-content-md5").is_some();
        let expected_md5: Option<String> = if has_md5_header {
            blob_http_headers
                .get("blobContentMD5")
                .and_then(GeneratedValue::as_string)
                .or_else(|| request.getHeader("content-md5"))
        } else {
            None
        };

        self.base
            .metadataStore
            .checkContainerExist(&context, &account_name, &container_name)
            .await?;

        let raw_data = body.read_to_vec();
        let content_length_u64 = contentLength as u64;
        let common_chunk = {
            let mut store = self.base.extentStore.lock().await;
            store
                .appendExtent(
                    ExtentDataInput::Buffer(Bytes::from(raw_data)),
                    Some(context_id_str),
                )
                .await
                .map_err(|e| map_common_err(e, context_id.as_deref()))?
        };
        if common_chunk.count != content_length_u64 {
            return Err(Box::new(StorageErrorFactory::getInvalidOperation(
                context_id.as_deref(),
                Some(&format!(
                    "The size of the request body {} mismatches the content-length {}.",
                    common_chunk.count, content_length_u64
                )),
            )));
        }
        let blob_extent = to_blob_extent(common_chunk);

        // Calculate MD5 from stored extent
        let calculated_md5: Vec<u8> = {
            let store = self.base.extentStore.lock().await;
            let stream = store
                .readExtent(Some(&to_common_extent(&blob_extent)), context_id.as_deref())
                .await
                .map_err(|e| map_common_err(e, context_id.as_deref()))?;
            getMD5FromStream(stream).await?
        };

        if let Some(expected) = expected_md5 {
            let calculated_b64 = STANDARD.encode(&calculated_md5);
            if expected != calculated_b64 {
                return Err(Box::new(StorageErrorFactory::getInvalidOperation(
                    context_id.as_deref(),
                    Some("Provided contentMD5 doesn't match."),
                )));
            }
        }

        let raw_headers = request.getRawHeaders();
        let metadata_map = convertRawHeadersToMetadata(&raw_headers, context_id_str)
            .map_err(|e| map_common_err(e, context_id.as_deref()))?;

        let etag = newEtag();
        let calculated_md5_b64 = STANDARD.encode(&calculated_md5);

        let mut properties = models::BlobPropertiesInternal::new();
        properties.insert(
            "creationTime".to_string(),
            GeneratedValue::String(date.to_rfc3339()),
        );
        properties.insert(
            "lastModified".to_string(),
            GeneratedValue::String(date.to_rfc3339()),
        );
        properties.insert("etag".to_string(), GeneratedValue::String(etag.clone()));
        properties.insert(
            "contentLength".to_string(),
            GeneratedValue::Number(contentLength),
        );
        properties.insert(
            "contentType".to_string(),
            GeneratedValue::String(content_type),
        );
        if let Some(v) = blob_http_headers
            .get("blobContentEncoding")
            .and_then(GeneratedValue::as_string)
        {
            properties.insert("contentEncoding".to_string(), GeneratedValue::String(v));
        }
        if let Some(v) = blob_http_headers
            .get("blobContentLanguage")
            .and_then(GeneratedValue::as_string)
        {
            properties.insert("contentLanguage".to_string(), GeneratedValue::String(v));
        }
        properties.insert(
            "contentMD5".to_string(),
            GeneratedValue::String(calculated_md5_b64.clone()),
        );
        if let Some(v) = blob_http_headers
            .get("blobContentDisposition")
            .and_then(GeneratedValue::as_string)
        {
            properties.insert("contentDisposition".to_string(), GeneratedValue::String(v));
        }
        if let Some(v) = blob_http_headers
            .get("blobCacheControl")
            .and_then(GeneratedValue::as_string)
        {
            properties.insert("cacheControl".to_string(), GeneratedValue::String(v));
        }
        properties.insert(
            "blobType".to_string(),
            GeneratedValue::String("BlockBlob".to_string()),
        );
        properties.insert(
            "leaseStatus".to_string(),
            GeneratedValue::String("unlocked".to_string()),
        );
        properties.insert(
            "leaseState".to_string(),
            GeneratedValue::String("available".to_string()),
        );
        properties.insert("serverEncrypted".to_string(), GeneratedValue::Bool(true));
        properties.insert(
            "accessTierChangeTime".to_string(),
            GeneratedValue::String(date.to_rfc3339()),
        );

        // Access tier handling
        let access_tier =
            if let Some(tier_val) = options.get("tier").and_then(GeneratedValue::as_string) {
                let parsed = Self::parse_tier(&tier_val);
                if parsed.is_none() {
                    let mut extra = BTreeMap::new();
                    extra.insert("HeaderName".to_string(), "x-ms-access-tier".to_string());
                    extra.insert("HeaderValue".to_string(), tier_val.clone());
                    return Err(Box::new(StorageErrorFactory::getInvalidHeaderValue(
                        context_id.as_deref(),
                        Some(extra),
                    )));
                }
                properties.insert(
                    "accessTierInferred".to_string(),
                    GeneratedValue::Bool(false),
                );
                parsed.unwrap()
            } else {
                properties.insert("accessTierInferred".to_string(), GeneratedValue::Bool(true));
                ACCESS_TIER_HOT.to_string()
            };
        properties.insert(
            "accessTier".to_string(),
            GeneratedValue::String(access_tier),
        );

        let blob_tags = options
            .get("blobTagsString")
            .and_then(GeneratedValue::as_string)
            .as_deref()
            .and_then(|s| get_tags_from_string(s, context_id_str));

        let blob = BlobModel {
            deleted: Some(false),
            metadata: metadata_map.map(to_generated_metadata),
            accountName: account_name,
            containerName: container_name,
            name: Some(blob_name),
            properties,
            snapshot: Some(String::new()),
            isCommitted: Some(true),
            persistency: Some(blob_extent),
            blobTags: blob_tags.and_then(|v| v.as_object().cloned()),
            ..Default::default()
        };

        let lease_access_conditions = options
            .get("leaseAccessConditions")
            .and_then(GeneratedValue::as_object);
        let modified_access_conditions = options
            .get("modifiedAccessConditions")
            .and_then(GeneratedValue::as_object);

        self.base
            .metadataStore
            .createBlob(
                &context,
                blob,
                lease_access_conditions,
                modified_access_conditions,
            )
            .await?;

        let client_request_id = options.get("requestId").and_then(GeneratedValue::as_string);

        let mut response = GeneratedResponse::new(201);
        response.insert_field("eTag", GeneratedValue::String(etag));
        response.insert_field("lastModified", GeneratedValue::String(date.to_rfc3339()));
        response.insert_field("contentMD5", GeneratedValue::String(calculated_md5_b64));
        response.insert_field(
            "requestId",
            GeneratedValue::String(context_id_str.to_string()),
        );
        response.insert_field(
            "version",
            GeneratedValue::String(BLOB_API_VERSION.to_string()),
        );
        response.insert_field("date", GeneratedValue::String(date.to_rfc3339()));
        response.insert_field("isServerEncrypted", GeneratedValue::Bool(true));
        if let Some(crid) = client_request_id {
            response.insert_field("clientRequestId", GeneratedValue::String(crid));
        }

        Ok(response)
    }

    async fn putBlobFromUrl(
        &self,
        _contentLength: f64,
        _copySource: String,
        _options: models::BlockBlobPutBlobFromUrlOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlockBlobPutBlobFromUrlResponse> {
        Err(Box::new(NotImplementedError::new(
            context.contextId().as_deref(),
        )))
    }

    async fn stageBlock(
        &self,
        blockId: String,
        contentLength: f64,
        body: GeneratedReadableStream,
        options: models::BlockBlobStageBlockOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlockBlobStageBlockResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account_name = blobCtx.account().unwrap_or_default();
        let container_name = blobCtx.container().unwrap_or_default();
        let blob_name = blobCtx.blob().unwrap_or_default();
        let date = context.startTime().unwrap_or_else(Utc::now);
        let context_id = context.contextId();
        let context_id_str = context_id.as_deref().unwrap_or("");

        let request = context.request().unwrap_or_default();

        let has_md5_header = request.getHeader("content-md5").is_some()
            || request.getHeader("x-ms-blob-content-md5").is_some();
        let expected_md5: Option<String> = if has_md5_header {
            options
                .get("transactionalContentMD5")
                .and_then(GeneratedValue::as_string)
                .or_else(|| request.getHeader("content-md5"))
        } else {
            None
        };

        Self::validate_block_id(&blockId, context_id.as_deref())?;

        self.base
            .metadataStore
            .checkContainerExist(&context, &account_name, &container_name)
            .await?;

        let content_length_u64 = contentLength as u64;
        let common_chunk = {
            let mut store = self.base.extentStore.lock().await;
            store
                .appendExtent(
                    ExtentDataInput::Buffer(Bytes::from(body.read_to_vec())),
                    Some(context_id_str),
                )
                .await
                .map_err(|e| map_common_err(e, context_id.as_deref()))?
        };
        if common_chunk.count != content_length_u64 {
            return Err(Box::new(StorageErrorFactory::getInvalidOperation(
                context_id.as_deref(),
                Some(&format!(
                    "The size of the request body {} mismatches the content-length {}.",
                    common_chunk.count, content_length_u64
                )),
            )));
        }
        let blob_extent = to_blob_extent(common_chunk);

        // Validate MD5 if provided
        if let Some(expected) = expected_md5 {
            let calculated_md5: Vec<u8> = {
                let store = self.base.extentStore.lock().await;
                let stream = store
                    .readExtent(Some(&to_common_extent(&blob_extent)), context_id.as_deref())
                    .await
                    .map_err(|e| map_common_err(e, context_id.as_deref()))?;
                getMD5FromStream(stream).await?
            };
            let calculated_b64 = STANDARD.encode(&calculated_md5);
            if expected != calculated_b64 {
                return Err(Box::new(StorageErrorFactory::getInvalidOperation(
                    context_id.as_deref(),
                    Some("Provided contentMD5 doesn't match."),
                )));
            }
        }

        let block = BlockModel {
            accountName: account_name,
            containerName: container_name,
            blobName: blob_name,
            isCommitted: false,
            name: Some(blockId),
            size: Some(contentLength as i64),
            persistency: blob_extent,
        };

        let lease_access_conditions = options
            .get("leaseAccessConditions")
            .and_then(GeneratedValue::as_object);

        self.base
            .metadataStore
            .stageBlock(&context, block, lease_access_conditions)
            .await?;

        let client_request_id = options.get("requestId").and_then(GeneratedValue::as_string);

        let mut response = GeneratedResponse::new(201);
        // contentMD5: undefined per TS comment (TODO: Block content MD5)
        response.insert_field(
            "requestId",
            GeneratedValue::String(context_id_str.to_string()),
        );
        response.insert_field(
            "version",
            GeneratedValue::String(BLOB_API_VERSION.to_string()),
        );
        response.insert_field("date", GeneratedValue::String(date.to_rfc3339()));
        response.insert_field("isServerEncrypted", GeneratedValue::Bool(true));
        if let Some(crid) = client_request_id {
            response.insert_field("clientRequestId", GeneratedValue::String(crid));
        }

        Ok(response)
    }

    async fn stageBlockFromURL(
        &self,
        _blockId: String,
        _contentLength: f64,
        _sourceUrl: String,
        _options: models::BlockBlobStageBlockFromURLOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlockBlobStageBlockFromURLResponse> {
        Err(Box::new(NotImplementedError::new(
            context.contextId().as_deref(),
        )))
    }

    async fn commitBlockList(
        &self,
        _blocks: models::BlockLookupList,
        options: models::BlockBlobCommitBlockListOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlockBlobCommitBlockListResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account_name = blobCtx.account().unwrap_or_default();
        let container_name = blobCtx.container().unwrap_or_default();
        let blob_name = blobCtx.blob().unwrap_or_default();
        let context_id = context.contextId();
        let context_id_str = context_id.as_deref().unwrap_or("");

        let request = context.request().unwrap_or_default();

        let blob_http_headers = options
            .get("blobHTTPHeaders")
            .and_then(GeneratedValue::as_object)
            .cloned()
            .unwrap_or_default();

        let content_type = blob_http_headers
            .get("blobContentType")
            .and_then(GeneratedValue::as_string)
            .unwrap_or_else(|| "application/octet-stream".to_string());

        // Re-parse the raw XML body to preserve element ordering (the deserialized
        // `blocks` parameter loses sequence information).
        let raw_body =
            request
                .getBody()
                .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                    Box::new(StorageErrorFactory::getInvalidOperation(
                        context_id.as_deref(),
                        None,
                    ))
                })?;

        let commit_block_list = parse_commit_block_list_xml(&raw_body).map_err(
            |_| -> Box<dyn std::error::Error + Send + Sync> {
                Box::new(StorageErrorFactory::getInvalidXmlDocument(
                    context_id.as_deref(),
                ))
            },
        )?;

        let date = context.startTime().unwrap_or_else(Utc::now);

        let raw_headers = request.getRawHeaders();
        let metadata_map = convertRawHeadersToMetadata(&raw_headers, context_id_str)
            .map_err(|e| map_common_err(e, context_id.as_deref()))?;

        let etag = newEtag();

        let mut properties = models::BlobPropertiesInternal::new();
        properties.insert(
            "lastModified".to_string(),
            GeneratedValue::String(date.to_rfc3339()),
        );
        properties.insert(
            "creationTime".to_string(),
            GeneratedValue::String(date.to_rfc3339()),
        );
        properties.insert("etag".to_string(), GeneratedValue::String(etag.clone()));
        properties.insert(
            "blobType".to_string(),
            GeneratedValue::String("BlockBlob".to_string()),
        );
        properties.insert(
            "contentType".to_string(),
            GeneratedValue::String(content_type),
        );
        if let Some(v) = blob_http_headers
            .get("blobContentMD5")
            .and_then(GeneratedValue::as_string)
        {
            properties.insert("contentMD5".to_string(), GeneratedValue::String(v));
        }
        if let Some(v) = blob_http_headers
            .get("blobCacheControl")
            .and_then(GeneratedValue::as_string)
        {
            properties.insert("cacheControl".to_string(), GeneratedValue::String(v));
        }
        if let Some(v) = blob_http_headers
            .get("blobContentEncoding")
            .and_then(GeneratedValue::as_string)
        {
            properties.insert("contentEncoding".to_string(), GeneratedValue::String(v));
        }
        if let Some(v) = blob_http_headers
            .get("blobContentLanguage")
            .and_then(GeneratedValue::as_string)
        {
            properties.insert("contentLanguage".to_string(), GeneratedValue::String(v));
        }
        if let Some(v) = blob_http_headers
            .get("blobContentDisposition")
            .and_then(GeneratedValue::as_string)
        {
            properties.insert("contentDisposition".to_string(), GeneratedValue::String(v));
        }

        // Access tier
        let access_tier =
            if let Some(tier_val) = options.get("tier").and_then(GeneratedValue::as_string) {
                let parsed = Self::parse_tier(&tier_val);
                if parsed.is_none() {
                    let mut extra = BTreeMap::new();
                    extra.insert("HeaderName".to_string(), "x-ms-access-tier".to_string());
                    extra.insert("HeaderValue".to_string(), tier_val.clone());
                    return Err(Box::new(StorageErrorFactory::getInvalidHeaderValue(
                        context_id.as_deref(),
                        Some(extra),
                    )));
                }
                properties.insert(
                    "accessTierInferred".to_string(),
                    GeneratedValue::Bool(false),
                );
                parsed.unwrap()
            } else {
                properties.insert("accessTierInferred".to_string(), GeneratedValue::Bool(true));
                ACCESS_TIER_HOT.to_string()
            };
        properties.insert(
            "accessTier".to_string(),
            GeneratedValue::String(access_tier),
        );

        let blob_tags = options
            .get("blobTagsString")
            .and_then(GeneratedValue::as_string)
            .as_deref()
            .and_then(|s| get_tags_from_string(s, context_id_str));

        let blob = BlobModel {
            accountName: account_name,
            containerName: container_name,
            name: Some(blob_name),
            snapshot: Some(String::new()),
            properties,
            isCommitted: Some(true),
            metadata: metadata_map.map(to_generated_metadata),
            blobTags: blob_tags.and_then(|v| v.as_object().cloned()),
            ..Default::default()
        };

        let lease_access_conditions = options
            .get("leaseAccessConditions")
            .and_then(GeneratedValue::as_object);
        let modified_access_conditions = options
            .get("modifiedAccessConditions")
            .and_then(GeneratedValue::as_object);

        self.base
            .metadataStore
            .commitBlockList(
                &context,
                blob,
                commit_block_list,
                lease_access_conditions,
                modified_access_conditions,
            )
            .await?;

        // MD5 is computed from the raw XML block-list body, not from committed blob content.
        let content_md5 = getMD5FromString(&raw_body).await;
        let content_md5_b64 = STANDARD.encode(&content_md5);

        let client_request_id = options.get("requestId").and_then(GeneratedValue::as_string);

        let mut response = GeneratedResponse::new(201);
        response.insert_field("eTag", GeneratedValue::String(etag));
        response.insert_field("lastModified", GeneratedValue::String(date.to_rfc3339()));
        response.insert_field("contentMD5", GeneratedValue::String(content_md5_b64));
        response.insert_field(
            "requestId",
            GeneratedValue::String(context_id_str.to_string()),
        );
        response.insert_field(
            "version",
            GeneratedValue::String(BLOB_API_VERSION.to_string()),
        );
        response.insert_field("date", GeneratedValue::String(date.to_rfc3339()));
        response.insert_field("isServerEncrypted", GeneratedValue::Bool(true));
        if let Some(crid) = client_request_id {
            response.insert_field("clientRequestId", GeneratedValue::String(crid));
        }

        Ok(response)
    }

    async fn getBlockList(
        &self,
        options: models::BlockBlobGetBlockListOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::BlockBlobGetBlockListResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account_name = blobCtx.account().unwrap_or_default();
        let container_name = blobCtx.container().unwrap_or_default();
        let blob_name = blobCtx.blob().unwrap_or_default();
        let date = context.startTime().unwrap_or_else(Utc::now);
        let context_id = context.contextId();
        let context_id_str = context_id.as_deref().unwrap_or("");

        let snapshot = options.get("snapshot").and_then(GeneratedValue::as_string);
        let list_type = options.get("listType").and_then(GeneratedValue::as_string);
        let lease_access_conditions = options
            .get("leaseAccessConditions")
            .and_then(GeneratedValue::as_object);
        let modified_access_conditions = options
            .get("modifiedAccessConditions")
            .and_then(GeneratedValue::as_object);

        let res = self
            .base
            .metadataStore
            .getBlockList(
                &context,
                &account_name,
                &container_name,
                &blob_name,
                snapshot.as_deref(),
                None,
                lease_access_conditions,
                modified_access_conditions,
            )
            .await?;

        let last_modified = res
            .properties
            .get("lastModified")
            .and_then(GeneratedValue::as_string);
        let etag = res
            .properties
            .get("etag")
            .and_then(GeneratedValue::as_string);
        let content_type = res
            .properties
            .get("contentType")
            .and_then(GeneratedValue::as_string);
        let content_length = res
            .properties
            .get("contentLength")
            .and_then(GeneratedValue::as_number);

        let list_type_lower = list_type.as_deref().map(str::to_lowercase);
        let include_uncommitted = list_type_lower
            .as_deref()
            .map(|t| t == "all" || t == "uncommitted")
            .unwrap_or(false);
        let include_committed = list_type_lower
            .as_deref()
            .map(|t| t == "all" || t == "committed")
            .unwrap_or(true); // default includes committed

        let committed_blocks: Vec<GeneratedValue> = if include_committed {
            res.committedBlocks
                .into_iter()
                .map(GeneratedValue::Object)
                .collect()
        } else {
            Vec::new()
        };
        let uncommitted_blocks: Vec<GeneratedValue> = if include_uncommitted {
            res.uncommittedBlocks
                .into_iter()
                .map(GeneratedValue::Object)
                .collect()
        } else {
            Vec::new()
        };

        let client_request_id = options.get("requestId").and_then(GeneratedValue::as_string);

        let mut response = GeneratedResponse::new(200);
        if let Some(v) = last_modified {
            response.insert_field("lastModified", GeneratedValue::String(v));
        }
        if let Some(v) = etag {
            response.insert_field("eTag", GeneratedValue::String(v));
        }
        if let Some(v) = content_type {
            response.insert_field("contentType", GeneratedValue::String(v));
        }
        if let Some(v) = content_length {
            response.insert_field("blobContentLength", GeneratedValue::Number(v));
        }
        response.insert_field(
            "requestId",
            GeneratedValue::String(context_id_str.to_string()),
        );
        response.insert_field(
            "version",
            GeneratedValue::String(BLOB_API_VERSION.to_string()),
        );
        response.insert_field("date", GeneratedValue::String(date.to_rfc3339()));
        response.insert_field("committedBlocks", GeneratedValue::Array(committed_blocks));
        response.insert_field(
            "uncommittedBlocks",
            GeneratedValue::Array(uncommitted_blocks),
        );
        if let Some(crid) = client_request_id {
            response.insert_field("clientRequestId", GeneratedValue::String(crid));
        }

        Ok(response)
    }
}
