use std::collections::BTreeMap;

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use bytes::Bytes;
use chrono::Utc;

use azurite_common::persistence::i_extent_store::{
    ExtentDataInput, IExtentChunk as CommonExtentChunk,
};
use azurite_common::utils::utils::{convertRawHeadersToMetadata, getMD5FromStream, newEtag};

use crate::context::blob_storage_context::BlobStorageContext;
use crate::errors::{NotImplementedError, StorageError, StorageErrorFactory};
use crate::generated::artifacts::models::{self, GeneratedResponse, GeneratedValue};
use crate::generated::context::Context;
use crate::generated::handlers::i_append_blob_handler::IAppendBlobHandler;
use crate::generated::i_request::{GeneratedReadableStream, IRequest};
use crate::handlers::base_handler::BaseHandler;
use crate::persistence::{BlobModel, BlockModel, IExtentChunk};

// ── Constants ────────────────────────────────────────────────────────────────

const BLOB_API_VERSION: &str = "2025-11-05";
/// Maximum size in bytes for a single append block: 100 MB.
const MAX_APPEND_BLOB_BLOCK_SIZE: i64 = 100 * 1024 * 1024;
/// Maximum number of committed blocks per append blob.
const MAX_APPEND_BLOB_BLOCK_COUNT: usize = 50_000;
/// HTTP header name for Content-Length.
const HEADER_CONTENT_LENGTH: &str = "content-length";
/// HTTP header name for Content-MD5.
const HEADER_CONTENT_MD5: &str = "content-md5";

// ── Handler struct ───────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppendBlobHandler {
    pub base: BaseHandler,
}

impl AppendBlobHandler {
    pub fn new(base: BaseHandler) -> Self {
        Self { base }
    }
}

// ── Private helpers ──────────────────────────────────────────────────────────

fn to_blob_extent(c: CommonExtentChunk) -> IExtentChunk {
    IExtentChunk {
        id: c.id,
        offset: c.offset as i64,
        count: c.count as i64,
    }
}

fn to_common_extent(b: &IExtentChunk) -> CommonExtentChunk {
    CommonExtentChunk {
        id: b.id.clone(),
        offset: b.offset as u64,
        count: b.count as u64,
    }
}

fn map_common_err(
    e: azurite_common::storage_error::StorageError,
    context_id: Option<&str>,
) -> StorageError {
    StorageErrorFactory::getInvalidOperation(context_id, Some(&e.message))
}

fn to_generated_metadata(m: std::collections::HashMap<String, String>) -> models::BlobMetadata {
    m.into_iter()
        .map(|(k, v)| (k, GeneratedValue::String(v)))
        .collect()
}

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

// ── Trait implementation ─────────────────────────────────────────────────────

#[allow(non_snake_case)]
#[async_trait]
impl IAppendBlobHandler for AppendBlobHandler {
    async fn create(
        &self,
        contentLength: f64,
        options: models::AppendBlobCreateOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::AppendBlobCreateResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account_name = blobCtx.account().unwrap_or_default();
        let container_name = blobCtx.container().unwrap_or_default();
        let blob_name = blobCtx.blob().unwrap_or_default();
        let date = context.startTime().unwrap_or_else(Utc::now);
        let context_id = context.contextId();
        let context_id_str = context_id.as_deref().unwrap_or("");
        let etag = newEtag();

        // Strict mode: Content-Length must be 0; loose mode allows non-zero.
        if contentLength as u64 != 0 && !self.base.loose {
            return Err(Box::new(StorageErrorFactory::getInvalidOperation(
                context_id.as_deref(),
                Some("Content-Length must be 0 for Create Append Blob request."),
            )));
        }

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

        let raw_headers = request.getRawHeaders();
        let metadata_map = convertRawHeadersToMetadata(&raw_headers, context_id_str)
            .map_err(|e| map_common_err(e, context_id.as_deref()))?;

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
        properties.insert("contentLength".to_string(), GeneratedValue::Number(0.0));
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
        if let Some(v) = blob_http_headers
            .get("blobContentMD5")
            .and_then(GeneratedValue::as_string)
        {
            properties.insert("contentMD5".to_string(), GeneratedValue::String(v));
        }
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
            GeneratedValue::String("AppendBlob".to_string()),
        );
        properties.insert(
            "leaseStatus".to_string(),
            GeneratedValue::String("Unlocked".to_string()),
        );
        properties.insert(
            "leaseState".to_string(),
            GeneratedValue::String("Available".to_string()),
        );
        properties.insert("serverEncrypted".to_string(), GeneratedValue::Bool(true));
        // Append blobs start unsealed
        properties.insert("isSealed".to_string(), GeneratedValue::Bool(false));

        let blob_tags = options
            .get("blobTagsString")
            .and_then(GeneratedValue::as_string)
            .as_deref()
            .and_then(|s| get_tags_from_string(s, context_id_str));

        let content_md5 = properties
            .get("contentMD5")
            .and_then(GeneratedValue::as_string);

        let blob = BlobModel {
            deleted: Some(false),
            metadata: metadata_map.map(to_generated_metadata),
            accountName: account_name,
            containerName: container_name,
            name: Some(blob_name),
            properties,
            snapshot: Some(String::new()),
            isCommitted: Some(true),
            committedBlocksInOrder: Some(Vec::new()),
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
        if let Some(md5) = content_md5 {
            response.insert_field("contentMD5", GeneratedValue::String(md5));
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
        response.insert_field("isServerEncrypted", GeneratedValue::Bool(true));
        if let Some(crid) = client_request_id {
            response.insert_field("clientRequestId", GeneratedValue::String(crid));
        }

        Ok(response)
    }

    async fn appendBlock(
        &self,
        body: GeneratedReadableStream,
        contentLength: f64,
        options: models::AppendBlobAppendBlockOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::AppendBlobAppendBlockResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account_name = blobCtx.account().unwrap_or_default();
        let container_name = blobCtx.container().unwrap_or_default();
        let blob_name = blobCtx.blob().unwrap_or_default();
        let date = context.startTime().unwrap_or_else(Utc::now);
        let context_id = context.contextId();
        let context_id_str = context_id.as_deref().unwrap_or("");

        let content_length_i64 = contentLength as i64;

        if content_length_i64 > MAX_APPEND_BLOB_BLOCK_SIZE {
            return Err(Box::new(StorageErrorFactory::getRequestEntityTooLarge(
                context_id.as_deref(),
            )));
        }

        if content_length_i64 == 0 {
            let mut extra = BTreeMap::new();
            extra.insert("HeaderName".to_string(), HEADER_CONTENT_LENGTH.to_string());
            extra.insert("HeaderValue".to_string(), "0".to_string());
            return Err(Box::new(StorageErrorFactory::getInvalidHeaderValue(
                context_id.as_deref(),
                Some(extra),
            )));
        }

        // Fetch current blob to get existing block count and current content length.
        // Note: no handler-level lease validation here — delegated to store methods (as in TS).
        let blob = self
            .base
            .metadataStore
            .downloadBlob(
                &context,
                &account_name,
                &container_name,
                &blob_name,
                None,
                None,
                None,
            )
            .await?;

        let blob_type = blob
            .properties
            .get("blobType")
            .and_then(GeneratedValue::as_string)
            .unwrap_or_default();
        if blob_type != "AppendBlob" {
            return Err(Box::new(StorageErrorFactory::getBlobInvalidBlobType(
                context_id.as_deref(),
            )));
        }

        let committed_block_count = blob
            .committedBlocksInOrder
            .as_ref()
            .map(|b| b.len())
            .unwrap_or(0);
        if committed_block_count >= MAX_APPEND_BLOB_BLOCK_COUNT {
            return Err(Box::new(StorageErrorFactory::getBlockCountExceedsLimit(
                context_id.as_deref(),
            )));
        }

        // Persist content before validating MD5
        let raw_data = body.read_to_vec();
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
        if common_chunk.count != content_length_i64 as u64 {
            return Err(Box::new(StorageErrorFactory::getInvalidOperation(
                context_id.as_deref(),
                Some(&format!(
                    "The size of the request body {} mismatches the content-length {}.",
                    common_chunk.count, content_length_i64
                )),
            )));
        }
        let blob_extent = to_blob_extent(common_chunk);

        // Optional MD5 validation — only the standard content-md5 header is checked
        let request = context.request().unwrap_or_default();
        let content_md5_header = request.getHeader(HEADER_CONTENT_MD5);

        let content_md5_bytes: Option<Vec<u8>> = if let Some(ref md5_b64) = content_md5_header {
            let expected_bytes = STANDARD.decode(md5_b64).unwrap_or_default();

            let calculated: Vec<u8> = {
                let store = self.base.extentStore.lock().await;
                let stream = store
                    .readExtent(Some(&to_common_extent(&blob_extent)), Some(context_id_str))
                    .await
                    .map_err(|e| map_common_err(e, context_id.as_deref()))?;
                getMD5FromStream(stream).await?
            };

            let expected_b64 = md5_b64.clone();
            let calculated_b64 = STANDARD.encode(&calculated);
            if expected_b64 != calculated_b64 {
                return Err(Box::new(StorageErrorFactory::getMd5Mismatch(
                    context_id.as_deref(),
                    &expected_b64,
                    &calculated_b64,
                )));
            }

            Some(expected_bytes)
        } else {
            None
        };

        let origin_offset = blob
            .properties
            .get("contentLength")
            .and_then(GeneratedValue::as_number)
            .unwrap_or(0.0) as i64;

        let lease_access_conditions = options
            .get("leaseAccessConditions")
            .and_then(GeneratedValue::as_object)
            .cloned();
        let modified_access_conditions = options
            .get("modifiedAccessConditions")
            .and_then(GeneratedValue::as_object)
            .cloned();
        let append_position_access_conditions = options
            .get("appendPositionAccessConditions")
            .and_then(GeneratedValue::as_object)
            .cloned();

        let block = BlockModel {
            accountName: account_name,
            containerName: container_name,
            blobName: blob_name,
            isCommitted: true,
            name: Some(String::new()), // No block ID for append blocks
            size: Some(blob_extent.count),
            persistency: blob_extent,
        };

        let properties = self
            .base
            .metadataStore
            .appendBlock(
                &context,
                block,
                lease_access_conditions.as_ref(),
                modified_access_conditions.as_ref(),
                append_position_access_conditions.as_ref(),
            )
            .await?;

        let etag = properties.get("etag").and_then(GeneratedValue::as_string);
        let last_modified = properties
            .get("lastModified")
            .and_then(GeneratedValue::as_string);

        let client_request_id = options.get("requestId").and_then(GeneratedValue::as_string);

        let mut response = GeneratedResponse::new(201);
        response.insert_field(
            "requestId",
            GeneratedValue::String(context_id_str.to_string()),
        );
        if let Some(v) = etag {
            response.insert_field("eTag", GeneratedValue::String(v));
        }
        if let Some(v) = last_modified {
            response.insert_field("lastModified", GeneratedValue::String(v));
        } else {
            response.insert_field("lastModified", GeneratedValue::String(date.to_rfc3339()));
        }
        if let Some(md5_bytes) = content_md5_bytes {
            response.insert_field(
                "contentMD5",
                GeneratedValue::String(STANDARD.encode(&md5_bytes)),
            );
        }
        if let Some(crid) = client_request_id {
            response.insert_field("clientRequestId", GeneratedValue::String(crid));
        }
        response.insert_field(
            "version",
            GeneratedValue::String(BLOB_API_VERSION.to_string()),
        );
        response.insert_field("date", GeneratedValue::String(date.to_rfc3339()));
        // blobAppendOffset is the pre-append contentLength, returned as string
        response.insert_field(
            "blobAppendOffset",
            GeneratedValue::String(origin_offset.to_string()),
        );
        // blobCommittedBlockCount is committed_block_count + 1 after successful append
        response.insert_field(
            "blobCommittedBlockCount",
            GeneratedValue::Number((committed_block_count + 1) as f64),
        );
        response.insert_field("isServerEncrypted", GeneratedValue::Bool(true));

        Ok(response)
    }

    async fn appendBlockFromUrl(
        &self,
        _sourceUrl: String,
        _contentLength: f64,
        _options: models::AppendBlobAppendBlockFromUrlOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::AppendBlobAppendBlockFromUrlResponse> {
        Err(Box::new(NotImplementedError::new(
            context.contextId().as_deref(),
        )))
    }

    async fn seal(
        &self,
        options: models::AppendBlobSealOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::AppendBlobSealResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account_name = blobCtx.account().unwrap_or_default();
        let container_name = blobCtx.container().unwrap_or_default();
        let blob_name = blobCtx.blob().unwrap_or_default();
        let date = context.startTime().unwrap_or_else(Utc::now);
        let context_id = context.contextId();
        let context_id_str = context_id.as_deref().unwrap_or("");

        // Extract clientRequestId before options is consumed by sealBlob
        let client_request_id = options.get("requestId").and_then(GeneratedValue::as_string);

        let properties = self
            .base
            .metadataStore
            .sealBlob(
                &context,
                &account_name,
                &container_name,
                &blob_name,
                None, // snapshot = undefined per TS
                options,
            )
            .await?;

        let etag = properties.get("etag").and_then(GeneratedValue::as_string);
        let last_modified = properties
            .get("lastModified")
            .and_then(GeneratedValue::as_string);
        let is_sealed = properties.get("isSealed").and_then(GeneratedValue::as_bool);

        let mut response = GeneratedResponse::new(200);
        response.insert_field(
            "requestId",
            GeneratedValue::String(context_id_str.to_string()),
        );
        if let Some(v) = etag {
            response.insert_field("eTag", GeneratedValue::String(v));
        }
        if let Some(v) = last_modified {
            response.insert_field("lastModified", GeneratedValue::String(v));
        } else {
            response.insert_field("lastModified", GeneratedValue::String(date.to_rfc3339()));
        }
        if let Some(crid) = client_request_id {
            response.insert_field("clientRequestId", GeneratedValue::String(crid));
        }
        response.insert_field(
            "version",
            GeneratedValue::String(BLOB_API_VERSION.to_string()),
        );
        response.insert_field("date", GeneratedValue::String(date.to_rfc3339()));
        if let Some(sealed) = is_sealed {
            response.insert_field("isSealed", GeneratedValue::Bool(sealed));
        }

        Ok(response)
    }
}
