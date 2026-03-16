use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;

use azurite_common::persistence::i_extent_store::{
    ExtentDataInput, IExtentChunk as CommonExtentChunk,
};
use azurite_common::utils::utils::{convertRawHeadersToMetadata, formatRfc1123, newEtag};

use crate::context::blob_storage_context::BlobStorageContext;
use crate::errors::{NotImplementedError, StorageError, StorageErrorFactory};
use crate::generated::artifacts::models::{self, GeneratedResponse, GeneratedValue};
use crate::generated::context::Context;
use crate::generated::handlers::i_page_blob_handler::IPageBlobHandler;
use crate::generated::i_request::{GeneratedReadableStream, IRequest};
use crate::handlers::base_handler::BaseHandler;
use crate::handlers::i_page_blob_ranges_manager::IPageBlobRangesManager;
use crate::lease::{BlobLeaseAdapter, BlobWriteLeaseValidator, ILeaseValidator};
use crate::persistence::{BlobModel, IExtentChunk};

// ── Constants ────────────────────────────────────────────────────────────────

const BLOB_API_VERSION: &str = "2025-11-05";

// ── Handler struct ───────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct PageBlobHandler {
    pub base: BaseHandler,
    pub rangesManager: Arc<dyn IPageBlobRangesManager + Send + Sync>,
}

impl PageBlobHandler {
    pub fn new(
        base: BaseHandler,
        rangesManager: Arc<dyn IPageBlobRangesManager + Send + Sync>,
    ) -> Self {
        Self {
            base,
            rangesManager,
        }
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

fn map_common_err(
    _e: azurite_common::storage_error::StorageError,
    context_id: Option<&str>,
) -> StorageError {
    StorageErrorFactory::getInvalidMetadata(context_id.unwrap_or_default())
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

/// Parse "range" / "x-ms-range" header of the form `bytes=<start>-<end>`.
/// Returns `Ok(Some((start, end)))` when a range header is present,
/// `Ok(None)` when absent, or `Err(())` when the value is malformed / misaligned.
fn deserialize_page_blob_range_header(
    range_val: Option<&str>,
    x_ms_range: Option<&str>,
    force_512: bool,
) -> Result<Option<(i64, i64)>, ()> {
    let range_str = x_ms_range.or(range_val);
    let range_str = match range_str {
        Some(r) => r,
        None => return Ok(None),
    };

    // expected format: "bytes=<start>-<end>"
    let parts: Vec<&str> = range_str.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err(());
    }
    let range_parts: Vec<&str> = parts[1].splitn(2, '-').collect();
    if range_parts.is_empty() {
        return Err(());
    }

    let start: i64 = range_parts[0].parse().map_err(|_| ())?;
    let end: i64 = if range_parts.len() > 1 && !range_parts[1].is_empty() {
        range_parts[1].parse().map_err(|_| ())?
    } else {
        i64::MAX
    };

    if start > end {
        return Err(());
    }

    if force_512 {
        if start % 512 != 0 {
            return Err(());
        }
        if end != i64::MAX && (end + 1) % 512 != 0 {
            return Err(());
        }
    }

    Ok(Some((start, end)))
}

fn make_page_range(start: i64, end: i64) -> models::PageRange {
    let mut pr = models::PageRange::new();
    pr.insert("start".to_string(), GeneratedValue::Number(start as f64));
    pr.insert("end".to_string(), GeneratedValue::Number(end as f64));
    pr
}

// ── Trait implementation ─────────────────────────────────────────────────────

#[allow(non_snake_case)]
#[async_trait]
impl IPageBlobHandler for PageBlobHandler {
    async fn uploadPagesFromURL(
        &self,
        _sourceUrl: String,
        _sourceRange: String,
        _contentLength: f64,
        _range: String,
        _options: models::PageBlobUploadPagesFromURLOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::PageBlobUploadPagesFromURLResponse> {
        Err(Box::new(NotImplementedError::new(
            context.contextId().as_deref(),
        )))
    }

    async fn create(
        &self,
        contentLength: f64,
        blobContentLength: f64,
        options: models::PageBlobCreateOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::PageBlobCreateResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account_name = blobCtx.account().unwrap_or_default();
        let container_name = blobCtx.container().unwrap_or_default();
        let blob_name = blobCtx.blob().unwrap_or_default();
        let date = context.startTime().unwrap_or_else(Utc::now);
        let context_id = context.contextId();
        let context_id_str = context_id.as_deref().unwrap_or("");

        // Explicit tier is not supported for page blobs
        if options
            .get("tier")
            .is_some_and(|v| !matches!(v, GeneratedValue::Null))
        {
            return Err(Box::new(
                StorageErrorFactory::getAccessTierNotSupportedForBlobType(context_id_str),
            ));
        }

        if contentLength as u64 != 0 {
            return Err(Box::new(StorageErrorFactory::getInvalidOperation(
                context_id.as_deref(),
                Some("Content-Length must be 0 for Create Page Blob request."),
            )));
        }

        let blob_content_length = blobContentLength as i64;
        if blob_content_length % 512 != 0 {
            return Err(Box::new(StorageErrorFactory::getInvalidOperation(
                context_id.as_deref(),
                Some("x-ms-content-length must be aligned to a 512-byte boundary."),
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

        let etag = newEtag();
        let blob_sequence_number = options
            .get("blobSequenceNumber")
            .and_then(GeneratedValue::as_number)
            .unwrap_or(0.0);

        let mut properties = models::BlobPropertiesInternal::new();
        properties.insert(
            "creationTime".to_string(),
            GeneratedValue::String(formatRfc1123(date)),
        );
        properties.insert(
            "lastModified".to_string(),
            GeneratedValue::String(formatRfc1123(date)),
        );
        properties.insert("etag".to_string(), GeneratedValue::String(etag.clone()));
        properties.insert(
            "contentLength".to_string(),
            GeneratedValue::Number(blobContentLength),
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
            "blobSequenceNumber".to_string(),
            GeneratedValue::Number(blob_sequence_number),
        );
        properties.insert(
            "blobType".to_string(),
            GeneratedValue::String("PageBlob".to_string()),
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
            properties: properties.clone(),
            snapshot: Some(String::new()),
            isCommitted: Some(true),
            pageRangesInOrder: Some(Vec::new()),
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

        let content_md5 = properties
            .get("contentMD5")
            .and_then(GeneratedValue::as_string);

        let client_request_id = options.get("requestId").and_then(GeneratedValue::as_string);

        let mut response = GeneratedResponse::new(201);
        response.insert_field("eTag", GeneratedValue::String(etag));
        response.insert_field("lastModified", GeneratedValue::String(formatRfc1123(date)));
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
        response.insert_field("date", GeneratedValue::String(formatRfc1123(date)));
        response.insert_field("isServerEncrypted", GeneratedValue::Bool(true));
        if let Some(crid) = client_request_id {
            response.insert_field("clientRequestId", GeneratedValue::String(crid));
        }

        Ok(response)
    }

    async fn uploadPages(
        &self,
        body: GeneratedReadableStream,
        contentLength: f64,
        options: models::PageBlobUploadPagesOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::PageBlobUploadPagesResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account_name = blobCtx.account().unwrap_or_default();
        let container_name = blobCtx.container().unwrap_or_default();
        let blob_name = blobCtx.blob().unwrap_or_default();
        let date = context.startTime().unwrap_or_else(Utc::now);
        let context_id = context.contextId();
        let context_id_str = context_id.as_deref().unwrap_or("");

        let content_length_i64 = contentLength as i64;
        if content_length_i64 % 512 != 0 {
            return Err(Box::new(StorageErrorFactory::getInvalidOperation(
                context_id.as_deref(),
                Some(
                    "content-length or x-ms-content-length must be aligned to a 512-byte boundary.",
                ),
            )));
        }

        let lease_access_conditions = options
            .get("leaseAccessConditions")
            .and_then(GeneratedValue::as_object)
            .cloned();
        let modified_access_conditions = options
            .get("modifiedAccessConditions")
            .and_then(GeneratedValue::as_object)
            .cloned();
        let sequence_number_access_conditions = options
            .get("sequenceNumberAccessConditions")
            .and_then(GeneratedValue::as_object)
            .cloned();

        let mut blob = self
            .base
            .metadataStore
            .downloadBlob(
                &context,
                &account_name,
                &container_name,
                &blob_name,
                None,
                lease_access_conditions.as_ref(),
                None,
            )
            .await?;

        let blob_type = blob
            .properties
            .get("blobType")
            .and_then(GeneratedValue::as_string)
            .unwrap_or_default();
        if blob_type != "PageBlob" {
            return Err(Box::new(StorageErrorFactory::getBlobInvalidBlobType(
                context_id.as_deref(),
            )));
        }

        // Handler-layer lease validation before writing
        let lease = BlobLeaseAdapter::from_blob(&mut blob);
        BlobWriteLeaseValidator::new(lease_access_conditions.clone()).validate(&lease, &context)?;

        let request = context.request().unwrap_or_default();
        let ranges = deserialize_page_blob_range_header(
            request.getHeader("range").as_deref(),
            request.getHeader("x-ms-range").as_deref(),
            true,
        )
        .map_err(|_| -> Box<dyn std::error::Error + Send + Sync> {
            Box::new(StorageErrorFactory::getInvalidPageRange(context_id_str))
        })?
        .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
            Box::new(StorageErrorFactory::getInvalidPageRange(context_id_str))
        })?;

        let start = ranges.0;
        let end = ranges.1; // inclusive

        if end - start + 1 != content_length_i64 {
            return Err(Box::new(StorageErrorFactory::getInvalidPageRange(
                context_id_str,
            )));
        }

        let blob_content_length = blob
            .properties
            .get("contentLength")
            .and_then(GeneratedValue::as_number)
            .unwrap_or(0.0) as i64;
        if start >= blob_content_length {
            return Err(Box::new(StorageErrorFactory::getInvalidPageRange(
                context_id_str,
            )));
        }

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
        if common_chunk.count != contentLength as u64 {
            return Err(Box::new(StorageErrorFactory::getInvalidOperation(
                context_id.as_deref(),
                Some(&format!(
                    "The size of the request body {} mismatches the content-length {}.",
                    common_chunk.count, contentLength as u64
                )),
            )));
        }
        let blob_extent = to_blob_extent(common_chunk);

        let res = self
            .base
            .metadataStore
            .uploadPages(
                &context,
                blob,
                start,
                end,
                blob_extent,
                lease_access_conditions.as_ref(),
                modified_access_conditions.as_ref(),
                sequence_number_access_conditions.as_ref(),
            )
            .await?;

        let etag = res.get("etag").and_then(GeneratedValue::as_string);
        let blob_sequence_number = res
            .get("blobSequenceNumber")
            .and_then(GeneratedValue::as_number);

        let client_request_id = options.get("requestId").and_then(GeneratedValue::as_string);

        let mut response = GeneratedResponse::new(201);
        if let Some(v) = etag {
            response.insert_field("eTag", GeneratedValue::String(v));
        }
        response.insert_field("lastModified", GeneratedValue::String(formatRfc1123(date)));
        // contentMD5: undefined per TS comment (TODO)
        if let Some(v) = blob_sequence_number {
            response.insert_field("blobSequenceNumber", GeneratedValue::Number(v));
        }
        response.insert_field(
            "requestId",
            GeneratedValue::String(context_id_str.to_string()),
        );
        response.insert_field(
            "version",
            GeneratedValue::String(BLOB_API_VERSION.to_string()),
        );
        response.insert_field("date", GeneratedValue::String(formatRfc1123(date)));
        response.insert_field("isServerEncrypted", GeneratedValue::Bool(true));
        if let Some(crid) = client_request_id {
            response.insert_field("clientRequestId", GeneratedValue::String(crid));
        }

        Ok(response)
    }

    async fn clearPages(
        &self,
        contentLength: f64,
        options: models::PageBlobClearPagesOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::PageBlobClearPagesResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account_name = blobCtx.account().unwrap_or_default();
        let container_name = blobCtx.container().unwrap_or_default();
        let blob_name = blobCtx.blob().unwrap_or_default();
        let date = context.startTime().unwrap_or_else(Utc::now);
        let context_id = context.contextId();
        let context_id_str = context_id.as_deref().unwrap_or("");

        if contentLength as u64 != 0 {
            return Err(Box::new(StorageErrorFactory::getInvalidOperation(
                context_id.as_deref(),
                Some("content-length or x-ms-content-length must be 0 for clear pages operation."),
            )));
        }

        let lease_access_conditions = options
            .get("leaseAccessConditions")
            .and_then(GeneratedValue::as_object)
            .cloned();
        let modified_access_conditions = options
            .get("modifiedAccessConditions")
            .and_then(GeneratedValue::as_object)
            .cloned();
        let sequence_number_access_conditions = options
            .get("sequenceNumberAccessConditions")
            .and_then(GeneratedValue::as_object)
            .cloned();

        let blob = self
            .base
            .metadataStore
            .downloadBlob(
                &context,
                &account_name,
                &container_name,
                &blob_name,
                None,
                lease_access_conditions.as_ref(),
                None,
            )
            .await?;

        let blob_type = blob
            .properties
            .get("blobType")
            .and_then(GeneratedValue::as_string)
            .unwrap_or_default();
        if blob_type != "PageBlob" {
            return Err(Box::new(StorageErrorFactory::getBlobInvalidBlobType(
                context_id.as_deref(),
            )));
        }

        let request = context.request().unwrap_or_default();
        let ranges = deserialize_page_blob_range_header(
            request.getHeader("range").as_deref(),
            request.getHeader("x-ms-range").as_deref(),
            true,
        )
        .map_err(|_| -> Box<dyn std::error::Error + Send + Sync> {
            Box::new(StorageErrorFactory::getInvalidPageRange(context_id_str))
        })?
        .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
            Box::new(StorageErrorFactory::getInvalidPageRange(context_id_str))
        })?;

        let start = ranges.0;
        let end = ranges.1;

        let blob_content_length = blob
            .properties
            .get("contentLength")
            .and_then(GeneratedValue::as_number)
            .unwrap_or(0.0) as i64;
        if start >= blob_content_length {
            return Err(Box::new(StorageErrorFactory::getInvalidPageRange(
                context_id_str,
            )));
        }

        let res = self
            .base
            .metadataStore
            .clearRange(
                &context,
                blob,
                start,
                end,
                lease_access_conditions.as_ref(),
                modified_access_conditions.as_ref(),
                sequence_number_access_conditions.as_ref(),
            )
            .await?;

        let etag = res.get("etag").and_then(GeneratedValue::as_string);
        let blob_sequence_number = res
            .get("blobSequenceNumber")
            .and_then(GeneratedValue::as_number);

        let client_request_id = options.get("requestId").and_then(GeneratedValue::as_string);

        let mut response = GeneratedResponse::new(201);
        if let Some(v) = etag {
            response.insert_field("eTag", GeneratedValue::String(v));
        }
        response.insert_field("lastModified", GeneratedValue::String(formatRfc1123(date)));
        // contentMD5: undefined per TS comment (TODO)
        if let Some(v) = blob_sequence_number {
            response.insert_field("blobSequenceNumber", GeneratedValue::Number(v));
        }
        response.insert_field(
            "requestId",
            GeneratedValue::String(context_id_str.to_string()),
        );
        response.insert_field(
            "version",
            GeneratedValue::String(BLOB_API_VERSION.to_string()),
        );
        response.insert_field("date", GeneratedValue::String(formatRfc1123(date)));
        if let Some(crid) = client_request_id {
            response.insert_field("clientRequestId", GeneratedValue::String(crid));
        }

        Ok(response)
    }

    async fn getPageRanges(
        &self,
        options: models::PageBlobGetPageRangesOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::PageBlobGetPageRangesResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account_name = blobCtx.account().unwrap_or_default();
        let container_name = blobCtx.container().unwrap_or_default();
        let blob_name = blobCtx.blob().unwrap_or_default();
        let date = context.startTime().unwrap_or_else(Utc::now);
        let context_id = context.contextId();
        let context_id_str = context_id.as_deref().unwrap_or("");

        let snapshot = options.get("snapshot").and_then(GeneratedValue::as_string);
        let lease_access_conditions = options
            .get("leaseAccessConditions")
            .and_then(GeneratedValue::as_object)
            .cloned();
        let modified_access_conditions = options
            .get("modifiedAccessConditions")
            .and_then(GeneratedValue::as_object)
            .cloned();

        let blob_result = self
            .base
            .metadataStore
            .getPageRanges(
                &context,
                &account_name,
                &container_name,
                &blob_name,
                snapshot.as_deref(),
                lease_access_conditions.as_ref(),
                modified_access_conditions.as_ref(),
            )
            .await?;

        let blob_type = blob_result
            .properties
            .get("blobType")
            .and_then(GeneratedValue::as_string)
            .unwrap_or_default();
        if blob_type != "PageBlob" {
            return Err(Box::new(StorageErrorFactory::getBlobInvalidBlobType(
                context_id.as_deref(),
            )));
        }

        let blob_content_length = blob_result
            .properties
            .get("contentLength")
            .and_then(GeneratedValue::as_number)
            .unwrap_or(0.0) as i64;

        let request = context.request().unwrap_or_default();
        let ranges = deserialize_page_blob_range_header(
            request.getHeader("range").as_deref(),
            request.getHeader("x-ms-range").as_deref(),
            false,
        )
        .map_err(|_| Box::new(StorageErrorFactory::getInvalidPageRange(context_id_str)))?;

        let (range_start, range_end) = ranges.unwrap_or((0, blob_content_length - 1));

        if range_start >= blob_content_length {
            return Err(Box::new(StorageErrorFactory::getInvalidPageRange(
                context_id_str,
            )));
        }

        let stored_ranges = blob_result.pageRangesInOrder.unwrap_or_default();
        let cut_range = make_page_range(range_start, range_end);
        let impacted_ranges = self.rangesManager.cut_ranges(&stored_ranges, cut_range);

        // Extract the plain PageRange (GeneratedObject) from each PersistencyPageRange
        let page_ranges: Vec<GeneratedValue> = impacted_ranges
            .into_iter()
            .map(|pr| GeneratedValue::Object(pr.range))
            .collect();

        let etag = blob_result
            .properties
            .get("etag")
            .and_then(GeneratedValue::as_string);

        let client_request_id = options.get("requestId").and_then(GeneratedValue::as_string);

        let mut response = GeneratedResponse::new(200);
        response.insert_field("pageRange", GeneratedValue::Array(page_ranges));
        if let Some(v) = etag {
            response.insert_field("eTag", GeneratedValue::String(v));
        }
        response.insert_field(
            "blobContentLength",
            GeneratedValue::Number(blob_content_length as f64),
        );
        response.insert_field("lastModified", GeneratedValue::String(formatRfc1123(date)));
        response.insert_field(
            "requestId",
            GeneratedValue::String(context_id_str.to_string()),
        );
        response.insert_field(
            "version",
            GeneratedValue::String(BLOB_API_VERSION.to_string()),
        );
        response.insert_field("date", GeneratedValue::String(formatRfc1123(date)));
        if let Some(crid) = client_request_id {
            response.insert_field("clientRequestId", GeneratedValue::String(crid));
        }

        Ok(response)
    }

    async fn getPageRangesDiff(
        &self,
        _options: models::PageBlobGetPageRangesDiffOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::PageBlobGetPageRangesDiffResponse> {
        Err(Box::new(NotImplementedError::new(
            context.contextId().as_deref(),
        )))
    }

    async fn resize(
        &self,
        blobContentLength: f64,
        options: models::PageBlobResizeOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::PageBlobResizeResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account_name = blobCtx.account().unwrap_or_default();
        let container_name = blobCtx.container().unwrap_or_default();
        let blob_name = blobCtx.blob().unwrap_or_default();
        let date = context.startTime().unwrap_or_else(Utc::now);
        let context_id = context.contextId();
        let context_id_str = context_id.as_deref().unwrap_or("");

        let blob_content_length_i64 = blobContentLength as i64;
        if blob_content_length_i64 % 512 != 0 {
            return Err(Box::new(StorageErrorFactory::getInvalidOperation(
                context_id.as_deref(),
                Some("x-ms-blob-content-length must be aligned to a 512-byte boundary for Page Blob Resize request."),
            )));
        }

        let lease_access_conditions = options
            .get("leaseAccessConditions")
            .and_then(GeneratedValue::as_object)
            .cloned();
        let modified_access_conditions = options
            .get("modifiedAccessConditions")
            .and_then(GeneratedValue::as_object)
            .cloned();

        let res = self
            .base
            .metadataStore
            .resizePageBlob(
                &context,
                &account_name,
                &container_name,
                &blob_name,
                blob_content_length_i64,
                lease_access_conditions.as_ref(),
                modified_access_conditions.as_ref(),
            )
            .await?;

        let etag = res.get("etag").and_then(GeneratedValue::as_string);
        let last_modified = res.get("lastModified").and_then(GeneratedValue::as_string);
        let blob_sequence_number = res
            .get("blobSequenceNumber")
            .and_then(GeneratedValue::as_number);

        let client_request_id = options.get("requestId").and_then(GeneratedValue::as_string);

        let mut response = GeneratedResponse::new(200);
        if let Some(v) = etag {
            response.insert_field("eTag", GeneratedValue::String(v));
        }
        if let Some(v) = last_modified {
            response.insert_field("lastModified", GeneratedValue::String(v));
        } else {
            response.insert_field("lastModified", GeneratedValue::String(formatRfc1123(date)));
        }
        if let Some(v) = blob_sequence_number {
            response.insert_field("blobSequenceNumber", GeneratedValue::Number(v));
        }
        response.insert_field(
            "requestId",
            GeneratedValue::String(context_id_str.to_string()),
        );
        response.insert_field(
            "version",
            GeneratedValue::String(BLOB_API_VERSION.to_string()),
        );
        response.insert_field("date", GeneratedValue::String(formatRfc1123(date)));
        if let Some(crid) = client_request_id {
            response.insert_field("clientRequestId", GeneratedValue::String(crid));
        }

        Ok(response)
    }

    async fn updateSequenceNumber(
        &self,
        sequenceNumberAction: models::SequenceNumberActionType,
        options: models::PageBlobUpdateSequenceNumberOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::PageBlobUpdateSequenceNumberResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account_name = blobCtx.account().unwrap_or_default();
        let container_name = blobCtx.container().unwrap_or_default();
        let blob_name = blobCtx.blob().unwrap_or_default();
        let date = context.startTime().unwrap_or_else(Utc::now);
        let context_id = context.contextId();
        let context_id_str = context_id.as_deref().unwrap_or("");

        // Only use blobSequenceNumber if the header was actually present in the request
        // (the deserializer fills in defaultValue=0 when absent)
        let blob_sequence_number_opt = context
            .request()
            .and_then(|r| r.getHeader("x-ms-blob-sequence-number"))
            .and_then(|v| v.parse::<i64>().ok());

        let lease_access_conditions = options
            .get("leaseAccessConditions")
            .and_then(GeneratedValue::as_object)
            .cloned();
        let modified_access_conditions = options
            .get("modifiedAccessConditions")
            .and_then(GeneratedValue::as_object)
            .cloned();

        let res = self
            .base
            .metadataStore
            .updateSequenceNumber(
                &context,
                &account_name,
                &container_name,
                &blob_name,
                &sequenceNumberAction,
                blob_sequence_number_opt,
                lease_access_conditions.as_ref(),
                modified_access_conditions.as_ref(),
            )
            .await?;

        let etag = res.get("etag").and_then(GeneratedValue::as_string);
        let last_modified = res.get("lastModified").and_then(GeneratedValue::as_string);
        let blob_sequence_number = res
            .get("blobSequenceNumber")
            .and_then(GeneratedValue::as_number);

        let client_request_id = options.get("requestId").and_then(GeneratedValue::as_string);

        let mut response = GeneratedResponse::new(200);
        if let Some(v) = etag {
            response.insert_field("eTag", GeneratedValue::String(v));
        }
        if let Some(v) = last_modified {
            response.insert_field("lastModified", GeneratedValue::String(v));
        } else {
            response.insert_field("lastModified", GeneratedValue::String(formatRfc1123(date)));
        }
        if let Some(v) = blob_sequence_number {
            response.insert_field("blobSequenceNumber", GeneratedValue::Number(v));
        }
        response.insert_field(
            "requestId",
            GeneratedValue::String(context_id_str.to_string()),
        );
        response.insert_field(
            "version",
            GeneratedValue::String(BLOB_API_VERSION.to_string()),
        );
        response.insert_field("date", GeneratedValue::String(formatRfc1123(date)));
        if let Some(crid) = client_request_id {
            response.insert_field("clientRequestId", GeneratedValue::String(crid));
        }

        Ok(response)
    }

    async fn copyIncremental(
        &self,
        _copySource: String,
        _options: models::PageBlobCopyIncrementalOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::PageBlobCopyIncrementalResponse> {
        Err(Box::new(NotImplementedError::new(
            context.contextId().as_deref(),
        )))
    }
}
