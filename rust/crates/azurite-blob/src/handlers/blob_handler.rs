use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use azurite_common::persistence::i_extent_store::IExtentChunk as CommonIExtentChunk;
use azurite_common::utils::utils::{convertRawHeadersToMetadata, formatRfc1123};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use tokio::io::AsyncReadExt;
use url::Url;

use crate::context::blob_storage_context::BlobStorageContext;
use crate::errors::{NotImplementedError, StorageErrorFactory};
use crate::generated::artifacts::models::{
    BlobAbortCopyFromURLOptionalParams, BlobAbortCopyFromURLResponse,
    BlobAcquireLeaseOptionalParams, BlobAcquireLeaseResponse, BlobBreakLeaseOptionalParams,
    BlobBreakLeaseResponse, BlobChangeLeaseOptionalParams, BlobChangeLeaseResponse,
    BlobCopyFromURLOptionalParams, BlobCopyFromURLResponse, BlobCreateSnapshotOptionalParams,
    BlobCreateSnapshotResponse, BlobDeleteImmutabilityPolicyOptionalParams,
    BlobDeleteImmutabilityPolicyResponse, BlobDeleteMethodOptionalParams, BlobDeleteResponse,
    BlobDownloadOptionalParams, BlobDownloadResponse, BlobExpiryOptions,
    BlobGetAccountInfoResponse, BlobGetPropertiesOptionalParams, BlobGetPropertiesResponse,
    BlobGetTagsOptionalParams, BlobGetTagsResponse, BlobQueryOptionalParams, BlobQueryResponse,
    BlobReleaseLeaseOptionalParams, BlobReleaseLeaseResponse, BlobRenewLeaseOptionalParams,
    BlobRenewLeaseResponse, BlobSetExpiryOptionalParams, BlobSetExpiryResponse,
    BlobSetHTTPHeadersOptionalParams, BlobSetHTTPHeadersResponse,
    BlobSetImmutabilityPolicyOptionalParams, BlobSetImmutabilityPolicyResponse,
    BlobSetLegalHoldOptionalParams, BlobSetLegalHoldResponse, BlobSetMetadataOptionalParams,
    BlobSetMetadataResponse, BlobSetTagsOptionalParams, BlobSetTagsResponse,
    BlobSetTierOptionalParams, BlobSetTierResponse, BlobStartCopyFromURLOptionalParams,
    BlobStartCopyFromURLResponse, BlobUndeleteOptionalParams, BlobUndeleteResponse, GeneratedBody,
    GeneratedObject, GeneratedResponse, GeneratedValue,
};
use crate::generated::context::Context;
use crate::generated::handlers::i_blob_handler::IBlobHandler;
use crate::generated::i_request::{GeneratedReadableStream, IRequest};
use crate::handlers::base_handler::BaseHandler;
use crate::handlers::i_page_blob_ranges_manager::IPageBlobRangesManager;
use crate::persistence::BlobId;

// ─── Local constants (mirrored from TS utils/constants.ts) ──────────────────

const BLOB_API_VERSION: &str = "2025-11-05";
const EMULATOR_ACCOUNT_SKUNAME: &str = "StandardRAGRS";
const EMULATOR_ACCOUNT_KIND: &str = "StorageV2";

// ─── Local header name constants ─────────────────────────────────────────────

const X_MS_SEQUENCE_NUMBER_ACTION: &str = "x-ms-sequence-number-action";
const X_MS_BLOB_SEQUENCE_NUMBER: &str = "x-ms-blob-sequence-number";
const CONTENT_TYPE: &str = "content-type";
const HOST: &str = "host";

// ─── BlobHandler ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct BlobHandler {
    pub base: BaseHandler,
    pub rangesManager: Arc<dyn IPageBlobRangesManager + Send + Sync>,
}

impl BlobHandler {
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

// ─── IBlobHandler impl ────────────────────────────────────────────────────────

#[async_trait]
#[allow(non_snake_case)]
impl IBlobHandler for BlobHandler {
    // ── download ──────────────────────────────────────────────────────────────

    async fn download(
        &self,
        options: BlobDownloadOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobDownloadResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let accountName = blobCtx.account().unwrap_or_default();
        let containerName = blobCtx.container().unwrap_or_default();
        let blobName = blobCtx.blob().unwrap_or_default();

        let snapshot = get_string(&options, "snapshot");
        let lease_conds = get_object(&options, "leaseAccessConditions");
        let mod_conds = get_object(&options, "modifiedAccessConditions");

        let blob = self
            .base
            .metadataStore
            .downloadBlob(
                &context,
                &accountName,
                &containerName,
                &blobName,
                snapshot.as_deref(),
                lease_conds.as_ref(),
                mod_conds.as_ref(),
            )
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        if get_string(&blob.properties, "accessTier").as_deref() == Some("Archive") {
            return Err(Box::new(StorageErrorFactory::getBlobArchived(
                context.contextId().as_deref(),
                None,
            )));
        }

        let blob_type = get_string(&blob.properties, "blobType");
        match blob_type.as_deref() {
            Some("BlockBlob") | Some("AppendBlob") => {
                self.downloadBlockBlobOrAppendBlob(options, &context, blob)
                    .await
            }
            Some("PageBlob") => self.downloadPageBlob(options, &context, blob).await,
            _ => Err(Box::new(StorageErrorFactory::getInvalidOperation(
                context.contextId().as_deref(),
                None,
            ))),
        }
    }

    // ── getProperties ─────────────────────────────────────────────────────────

    async fn getProperties(
        &self,
        options: BlobGetPropertiesOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobGetPropertiesResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account = blobCtx.account().unwrap_or_default();
        let container = blobCtx.container().unwrap_or_default();
        let blob = blobCtx.blob().unwrap_or_default();

        let snapshot = get_string(&options, "snapshot");
        let lease_conds = get_object(&options, "leaseAccessConditions");
        let mod_conds = get_object(&options, "modifiedAccessConditions");

        let res = self
            .base
            .metadataStore
            .getBlobProperties(
                &context,
                &account,
                &container,
                &blob,
                snapshot.as_deref(),
                lease_conds.as_ref(),
                mod_conds.as_ref(),
            )
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let req = context.request();
        let against_metadata =
            req.as_ref().and_then(|r| r.getQuery("comp")).as_deref() == Some("metadata");

        let ctx_id = context.contextId().unwrap_or_default();
        let start_time = context.startTime();
        let client_request_id = get_string(&options, "requestId");

        let mut response = GeneratedResponse::new(200);
        response.insert_field("requestId", string_value(&ctx_id));
        response.insert_field("version", string_value(BLOB_API_VERSION));
        if let Some(t) = start_time {
            response.insert_field("date", string_value(formatRfc1123(t)));
        }
        if let Some(cri) = client_request_id {
            response.insert_field("clientRequestId", string_value(cri));
        }
        // metadata is included in both paths
        if let Some(metadata) = &res.metadata {
            response.insert_field("metadata", GeneratedValue::Object(metadata.clone()));
        }

        if against_metadata {
            // minimal response for ?comp=metadata
            copy_prop_field(&res.properties, &mut response, "etag", "eTag");
            copy_prop_field(
                &res.properties,
                &mut response,
                "contentLength",
                "contentLength",
            );
            copy_prop_field(
                &res.properties,
                &mut response,
                "lastModified",
                "lastModified",
            );
        } else {
            // Full properties response - spread all stored properties
            for (key, value) in &res.properties {
                response.insert_field(key.clone(), value.clone());
            }

            // Remap properties keys to match response headersMapper field names
            if let Some(v) = response.fields.remove("etag") {
                response.insert_field("eTag", v);
            }

            // Override with per-request query overrides
            if let Some(req_ref) = req.as_ref() {
                override_prop_from_query(req_ref, &mut response, "rscc", "cacheControl");
                override_prop_from_query(req_ref, &mut response, "rscd", "contentDisposition");
                override_prop_from_query(req_ref, &mut response, "rsce", "contentEncoding");
                override_prop_from_query(req_ref, &mut response, "rscl", "contentLanguage");
                override_prop_from_query(req_ref, &mut response, "rsct", "contentType");
            }

            response.insert_field("acceptRanges", string_value("bytes"));
            response.insert_field("isServerEncrypted", GeneratedValue::Bool(true));
            response.insert_field(
                "isIncrementalCopy",
                res.properties
                    .get("incrementalCopy")
                    .cloned()
                    .unwrap_or(GeneratedValue::Null),
            );

            // blobCommittedBlockCount only for AppendBlob
            if get_string(&res.properties, "blobType").as_deref() == Some("AppendBlob") {
                if let Some(cnt) = res.blobCommittedBlockCount {
                    response.insert_field(
                        "blobCommittedBlockCount",
                        GeneratedValue::Number(cnt as f64),
                    );
                }
            }

            // Tag count
            if let Some(tc) = get_blob_tags_count(res.blobTags.as_ref()) {
                response.insert_field("tagCount", GeneratedValue::Number(tc as f64));
            }
        }

        Ok(response)
    }

    // ── delete ────────────────────────────────────────────────────────────────

    async fn delete(
        &self,
        options: BlobDeleteMethodOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobDeleteResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account = blobCtx.account().unwrap_or_default();
        let container = blobCtx.container().unwrap_or_default();
        let blob = blobCtx.blob().unwrap_or_default();

        // deleteBlob takes options by value; clone so we can still read it for the response
        self.base
            .metadataStore
            .deleteBlob(&context, &account, &container, &blob, options.clone())
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let mut response = GeneratedResponse::new(202);
        set_common_fields(&mut response, &context, &options, "requestId");
        response.insert_field("deleteTypePermanent", GeneratedValue::Bool(true));
        Ok(response)
    }

    // ── undelete ──────────────────────────────────────────────────────────────

    async fn undelete(
        &self,
        _options: BlobUndeleteOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobUndeleteResponse> {
        Err(Box::new(NotImplementedError::new(
            context.contextId().as_deref(),
        )))
    }

    // ── setExpiry ─────────────────────────────────────────────────────────────

    async fn setExpiry(
        &self,
        _expiryOptions: BlobExpiryOptions,
        _options: BlobSetExpiryOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobSetExpiryResponse> {
        Err(Box::new(NotImplementedError::new(
            context.contextId().as_deref(),
        )))
    }

    // ── setHTTPHeaders ────────────────────────────────────────────────────────

    async fn setHTTPHeaders(
        &self,
        options: BlobSetHTTPHeadersOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobSetHTTPHeadersResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account = blobCtx.account().unwrap_or_default();
        let container = blobCtx.container().unwrap_or_default();
        let blob = blobCtx.blob().unwrap_or_default();

        let lease_conds = get_object(&options, "leaseAccessConditions");
        let mod_conds = get_object(&options, "modifiedAccessConditions");
        let http_headers = get_object(&options, "blobHTTPHeaders");

        let res = {
            // Workaround: if x-ms-sequence-number-action is present, route to updateSequenceNumber
            // (mirrors BlobHandler.ts:245-277)
            let req = context.request();
            let seq_action = req
                .as_ref()
                .and_then(|r| r.getHeader(X_MS_SEQUENCE_NUMBER_ACTION));
            if let Some(action) = seq_action {
                let seq_number = req
                    .as_ref()
                    .and_then(|r| r.getHeader(X_MS_BLOB_SEQUENCE_NUMBER))
                    .and_then(|s| s.parse::<i64>().ok());
                self.base
                    .metadataStore
                    .updateSequenceNumber(
                        &context,
                        &account,
                        &container,
                        &blob,
                        &action.to_lowercase(),
                        seq_number,
                        lease_conds.as_ref(),
                        mod_conds.as_ref(),
                    )
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?
            } else {
                self.base
                    .metadataStore
                    .setBlobHTTPHeaders(
                        &context,
                        &account,
                        &container,
                        &blob,
                        lease_conds.as_ref(),
                        http_headers.as_ref(),
                        mod_conds.as_ref(),
                    )
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?
            }
        };

        let mut response = GeneratedResponse::new(200);
        set_common_fields(&mut response, &context, &options, "requestId");
        copy_prop_field(&res, &mut response, "etag", "eTag");
        copy_prop_field(&res, &mut response, "lastModified", "lastModified");
        copy_prop_field(
            &res,
            &mut response,
            "blobSequenceNumber",
            "blobSequenceNumber",
        );
        Ok(response)
    }

    // ── setImmutabilityPolicy ─────────────────────────────────────────────────

    async fn setImmutabilityPolicy(
        &self,
        _options: BlobSetImmutabilityPolicyOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobSetImmutabilityPolicyResponse> {
        Err(Box::new(NotImplementedError::new(
            context.contextId().as_deref(),
        )))
    }

    // ── deleteImmutabilityPolicy ──────────────────────────────────────────────

    async fn deleteImmutabilityPolicy(
        &self,
        _options: BlobDeleteImmutabilityPolicyOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobDeleteImmutabilityPolicyResponse> {
        Err(Box::new(NotImplementedError::new(
            context.contextId().as_deref(),
        )))
    }

    // ── setLegalHold ──────────────────────────────────────────────────────────

    async fn setLegalHold(
        &self,
        _legalHold: bool,
        _options: BlobSetLegalHoldOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobSetLegalHoldResponse> {
        Err(Box::new(NotImplementedError::new(
            context.contextId().as_deref(),
        )))
    }

    // ── setMetadata ───────────────────────────────────────────────────────────

    async fn setMetadata(
        &self,
        options: BlobSetMetadataOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobSetMetadataResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account = blobCtx.account().unwrap_or_default();
        let container = blobCtx.container().unwrap_or_default();
        let blob = blobCtx.blob().unwrap_or_default();

        // Preserve metadata key case from raw headers
        let ctx_id = context.contextId().unwrap_or_default();
        let raw_headers = context
            .request()
            .map(|r| r.getRawHeaders())
            .unwrap_or_default();
        let metadata_map = convertRawHeadersToMetadata(&raw_headers, &ctx_id).map_err(|_| {
            Box::new(StorageErrorFactory::getInvalidMetadata(&ctx_id))
                as Box<dyn std::error::Error + Send + Sync>
        })?;
        let metadata: Option<GeneratedObject> = metadata_map.map(|m| {
            m.into_iter()
                .map(|(k, v)| (k, GeneratedValue::String(v)))
                .collect()
        });

        let lease_conds = get_object(&options, "leaseAccessConditions");
        let mod_conds = get_object(&options, "modifiedAccessConditions");

        let res = self
            .base
            .metadataStore
            .setBlobMetadata(
                &context,
                &account,
                &container,
                &blob,
                lease_conds.as_ref(),
                metadata.as_ref(),
                mod_conds.as_ref(),
            )
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let mut response = GeneratedResponse::new(200);
        set_common_fields(&mut response, &context, &options, "requestId");
        copy_prop_field(&res, &mut response, "etag", "eTag");
        copy_prop_field(&res, &mut response, "lastModified", "lastModified");
        response.insert_field("isServerEncrypted", GeneratedValue::Bool(true));
        Ok(response)
    }

    // ── acquireLease ──────────────────────────────────────────────────────────

    async fn acquireLease(
        &self,
        options: BlobAcquireLeaseOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobAcquireLeaseResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account = blobCtx.account().unwrap_or_default();
        let container = blobCtx.container().unwrap_or_default();
        let blob = blobCtx.blob().unwrap_or_default();

        // Reject lease on snapshots (BlobHandler.ts:380-385)
        let snapshot_qs = context.request().and_then(|r| r.getQuery("snapshot"));
        if snapshot_qs
            .as_deref()
            .map(|s| !s.is_empty())
            .unwrap_or(false)
        {
            return Err(Box::new(StorageErrorFactory::getInvalidOperation(
                context.contextId().as_deref(),
                Some("A lease cannot be granted for a blob snapshot"),
            )));
        }

        let duration = get_number(&options, "duration").unwrap_or(-1.0) as i64;
        let proposed_lease_id = get_string(&options, "proposedLeaseId");

        let res = self
            .base
            .metadataStore
            .acquireBlobLease(
                &context,
                &account,
                &container,
                &blob,
                duration,
                proposed_lease_id.as_deref(),
                Some(&options),
            )
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let mut response = GeneratedResponse::new(201);
        set_common_fields(&mut response, &context, &options, "requestId");
        copy_prop_field(&res.properties, &mut response, "etag", "eTag");
        copy_prop_field(
            &res.properties,
            &mut response,
            "lastModified",
            "lastModified",
        );
        if let Some(lid) = res.leaseId {
            response.insert_field("leaseId", string_value(lid));
        }
        Ok(response)
    }

    // ── releaseLease ──────────────────────────────────────────────────────────

    async fn releaseLease(
        &self,
        leaseId: String,
        options: BlobReleaseLeaseOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobReleaseLeaseResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account = blobCtx.account().unwrap_or_default();
        let container = blobCtx.container().unwrap_or_default();
        let blob = blobCtx.blob().unwrap_or_default();

        let res = self
            .base
            .metadataStore
            .releaseBlobLease(
                &context,
                &account,
                &container,
                &blob,
                &leaseId,
                Some(&options),
            )
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let mut response = GeneratedResponse::new(200);
        set_common_fields(&mut response, &context, &options, "requestId");
        copy_prop_field(&res, &mut response, "etag", "eTag");
        copy_prop_field(&res, &mut response, "lastModified", "lastModified");
        Ok(response)
    }

    // ── renewLease ────────────────────────────────────────────────────────────

    async fn renewLease(
        &self,
        leaseId: String,
        options: BlobRenewLeaseOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobRenewLeaseResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account = blobCtx.account().unwrap_or_default();
        let container = blobCtx.container().unwrap_or_default();
        let blob = blobCtx.blob().unwrap_or_default();

        let res = self
            .base
            .metadataStore
            .renewBlobLease(
                &context,
                &account,
                &container,
                &blob,
                &leaseId,
                Some(&options),
            )
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let mut response = GeneratedResponse::new(200);
        set_common_fields(&mut response, &context, &options, "requestId");
        copy_prop_field(&res.properties, &mut response, "etag", "eTag");
        copy_prop_field(
            &res.properties,
            &mut response,
            "lastModified",
            "lastModified",
        );
        if let Some(lid) = res.leaseId {
            response.insert_field("leaseId", string_value(lid));
        }
        Ok(response)
    }

    // ── changeLease ───────────────────────────────────────────────────────────

    async fn changeLease(
        &self,
        leaseId: String,
        proposedLeaseId: String,
        options: BlobChangeLeaseOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobChangeLeaseResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account = blobCtx.account().unwrap_or_default();
        let container = blobCtx.container().unwrap_or_default();
        let blob = blobCtx.blob().unwrap_or_default();

        let res = self
            .base
            .metadataStore
            .changeBlobLease(
                &context,
                &account,
                &container,
                &blob,
                &leaseId,
                &proposedLeaseId,
                Some(&options),
            )
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let mut response = GeneratedResponse::new(200);
        set_common_fields(&mut response, &context, &options, "requestId");
        copy_prop_field(&res.properties, &mut response, "etag", "eTag");
        copy_prop_field(
            &res.properties,
            &mut response,
            "lastModified",
            "lastModified",
        );
        if let Some(lid) = res.leaseId {
            response.insert_field("leaseId", string_value(lid));
        }
        Ok(response)
    }

    // ── breakLease ────────────────────────────────────────────────────────────

    async fn breakLease(
        &self,
        options: BlobBreakLeaseOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobBreakLeaseResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account = blobCtx.account().unwrap_or_default();
        let container = blobCtx.container().unwrap_or_default();
        let blob = blobCtx.blob().unwrap_or_default();

        let break_period = get_number(&options, "breakPeriod").map(|n| n as i64);

        let res = self
            .base
            .metadataStore
            .breakBlobLease(
                &context,
                &account,
                &container,
                &blob,
                break_period,
                Some(&options),
            )
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let mut response = GeneratedResponse::new(202);
        set_common_fields(&mut response, &context, &options, "requestId");
        copy_prop_field(&res.properties, &mut response, "etag", "eTag");
        copy_prop_field(
            &res.properties,
            &mut response,
            "lastModified",
            "lastModified",
        );
        response.insert_field(
            "leaseTime",
            GeneratedValue::Number(res.leaseTime.unwrap_or(0) as f64),
        );
        Ok(response)
    }

    // ── createSnapshot ────────────────────────────────────────────────────────

    async fn createSnapshot(
        &self,
        options: BlobCreateSnapshotOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobCreateSnapshotResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account = blobCtx.account().unwrap_or_default();
        let container = blobCtx.container().unwrap_or_default();
        let blob = blobCtx.blob().unwrap_or_default();

        // Preserve metadata key case from raw headers
        let ctx_id = context.contextId().unwrap_or_default();
        let raw_headers = context
            .request()
            .map(|r| r.getRawHeaders())
            .unwrap_or_default();
        let metadata_map = convertRawHeadersToMetadata(&raw_headers, &ctx_id).map_err(|_| {
            Box::new(StorageErrorFactory::getInvalidMetadata(&ctx_id))
                as Box<dyn std::error::Error + Send + Sync>
        })?;
        let metadata: Option<GeneratedObject> = metadata_map.map(|m| {
            m.into_iter()
                .map(|(k, v)| (k, GeneratedValue::String(v)))
                .collect()
        });

        // BlobHandler.ts:594-608: only forward metadata when non-empty
        let metadata_to_store = if metadata.as_ref().map(|m| m.is_empty()).unwrap_or(true) {
            None
        } else {
            metadata.as_ref()
        };
        // Also respect options.metadata field: if options has explicit metadata,
        // the caller sets it but TS code only sends it when non-empty.
        let opts_metadata = get_object(&options, "metadata");
        let final_metadata = opts_metadata
            .as_ref()
            .and_then(|m| if m.is_empty() { None } else { Some(m) })
            .or(metadata_to_store);

        let lease_conds = get_object(&options, "leaseAccessConditions");
        let mod_conds = get_object(&options, "modifiedAccessConditions");

        let res = self
            .base
            .metadataStore
            .createSnapshot(
                &context,
                &account,
                &container,
                &blob,
                lease_conds.as_ref(),
                final_metadata,
                mod_conds.as_ref(),
            )
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let mut response = GeneratedResponse::new(201);
        set_common_fields(&mut response, &context, &options, "requestId");
        copy_prop_field(&res.properties, &mut response, "etag", "eTag");
        copy_prop_field(
            &res.properties,
            &mut response,
            "lastModified",
            "lastModified",
        );
        response.insert_field("snapshot", string_value(&res.snapshot));
        Ok(response)
    }

    // ── startCopyFromURL ──────────────────────────────────────────────────────

    async fn startCopyFromURL(
        &self,
        copySource: String,
        options: BlobStartCopyFromURLOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobStartCopyFromURLResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account = blobCtx.account().unwrap_or_default();
        let container = blobCtx.container().unwrap_or_default();
        let blob = blobCtx.blob().unwrap_or_default();
        let disable_product_style = blobCtx.disableProductStyleUrl();

        let url = new_uri_from_copy_source(&copySource, &context)?;
        let hostname = url.host_str().unwrap_or("").to_string();
        let pathname = url.path().to_string();
        let snapshot_qs = url
            .query_pairs()
            .find(|(k, _)| k == "snapshot")
            .map(|(_, v)| v.to_string())
            .unwrap_or_default();

        let (source_account, source_container, source_blob) =
            extract_storage_parts_from_path(&hostname, &pathname, disable_product_style);

        let (source_account, source_container, source_blob) =
            match (source_account, source_container, source_blob) {
                (Some(a), Some(c), Some(b)) => (a, c, b),
                _ => {
                    return Err(Box::new(StorageErrorFactory::getBlobNotFound(
                        context.contextId().as_deref(),
                    )))
                }
            };

        // BlobHandler.ts:644-664: validate when cross-account OR sig present
        let sig = url
            .query_pairs()
            .find(|(k, _)| k == "sig")
            .map(|(_, v)| v.to_string());
        if source_account != account || sig.is_some() {
            self.validateCopySource(&copySource, &source_account, &context)
                .await?;
        }

        let ctx_id = context.contextId().unwrap_or_default();
        let raw_headers = context
            .request()
            .map(|r| r.getRawHeaders())
            .unwrap_or_default();
        let metadata_map = convertRawHeadersToMetadata(&raw_headers, &ctx_id).map_err(|_| {
            Box::new(StorageErrorFactory::getInvalidMetadata(&ctx_id))
                as Box<dyn std::error::Error + Send + Sync>
        })?;
        let metadata: Option<GeneratedObject> = metadata_map.map(|m| {
            m.into_iter()
                .map(|(k, v)| (k, GeneratedValue::String(v)))
                .collect()
        });

        let tier = get_string(&options, "tier");

        let res = self
            .base
            .metadataStore
            .startCopyFromURL(
                &context,
                BlobId {
                    account: source_account,
                    container: source_container,
                    blob: source_blob,
                    snapshot: Some(snapshot_qs),
                },
                BlobId {
                    account,
                    container,
                    blob,
                    snapshot: None,
                },
                &copySource,
                metadata.as_ref(),
                tier.as_deref(),
                Some(&options),
            )
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let mut response = GeneratedResponse::new(202);
        set_common_fields(&mut response, &context, &options, "requestId");
        copy_prop_field(&res, &mut response, "etag", "eTag");
        copy_prop_field(&res, &mut response, "lastModified", "lastModified");
        copy_prop_field(&res, &mut response, "copyId", "copyId");
        copy_prop_field(&res, &mut response, "copyStatus", "copyStatus");
        Ok(response)
    }

    // ── copyFromURL ───────────────────────────────────────────────────────────

    async fn copyFromURL(
        &self,
        copySource: String,
        options: BlobCopyFromURLOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobCopyFromURLResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account = blobCtx.account().unwrap_or_default();
        let container = blobCtx.container().unwrap_or_default();
        let blob = blobCtx.blob().unwrap_or_default();
        let disable_product_style = blobCtx.disableProductStyleUrl();

        let url = new_uri_from_copy_source(&copySource, &context)?;
        let hostname = url.host_str().unwrap_or("").to_string();
        let pathname = url.path().to_string();
        let snapshot_qs = url
            .query_pairs()
            .find(|(k, _)| k == "snapshot")
            .map(|(_, v)| v.to_string())
            .unwrap_or_default();

        let (source_account, source_container, source_blob) =
            extract_storage_parts_from_path(&hostname, &pathname, disable_product_style);

        let (source_account, source_container, source_blob) =
            match (source_account, source_container, source_blob) {
                (Some(a), Some(c), Some(b)) => (a, c, b),
                _ => {
                    return Err(Box::new(StorageErrorFactory::getBlobNotFound(
                        context.contextId().as_deref(),
                    )))
                }
            };

        // BlobHandler.ts:860-862: only validate when source account differs (stricter than startCopy)
        if source_account != account {
            self.validateCopySource(&copySource, &source_account, &context)
                .await?;
        }

        // BlobHandler.ts:864-867: COPY tag-option and explicit tags cannot coexist
        let copy_source_tags = get_string(&options, "copySourceTags");
        let blob_tags_string = get_string(&options, "blobTagsString");
        if copy_source_tags.as_deref() == Some("COPY") && blob_tags_string.is_some() {
            return Err(Box::new(
                StorageErrorFactory::getBothUserTagsAndSourceTagsCopyPresentException(
                    context.contextId().as_deref().unwrap_or(""),
                ),
            ));
        }

        let ctx_id = context.contextId().unwrap_or_default();
        let raw_headers = context
            .request()
            .map(|r| r.getRawHeaders())
            .unwrap_or_default();
        let metadata_map = convertRawHeadersToMetadata(&raw_headers, &ctx_id).map_err(|_| {
            Box::new(StorageErrorFactory::getInvalidMetadata(&ctx_id))
                as Box<dyn std::error::Error + Send + Sync>
        })?;
        let metadata: Option<GeneratedObject> = metadata_map.map(|m| {
            m.into_iter()
                .map(|(k, v)| (k, GeneratedValue::String(v)))
                .collect()
        });

        let tier = get_string(&options, "tier");

        let res = self
            .base
            .metadataStore
            .copyFromURL(
                &context,
                BlobId {
                    account: source_account,
                    container: source_container,
                    blob: source_blob,
                    snapshot: Some(snapshot_qs),
                },
                BlobId {
                    account,
                    container,
                    blob,
                    snapshot: None,
                },
                &copySource,
                metadata.as_ref(),
                tier.as_deref(),
                Some(&options),
            )
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        // BlobHandler.ts:889-899: synchronous copy must end in Success
        let copy_status_str = get_string(&res, "copyStatus");
        if let Some(ref cs) = copy_status_str {
            if cs != "success" {
                return Err(Box::new(StorageErrorFactory::getUnexpectedSyncCopyStatus(
                    context.contextId().as_deref().unwrap_or(""),
                    cs,
                )));
            }
        }

        let mut response = GeneratedResponse::new(202);
        set_common_fields(&mut response, &context, &options, "requestId");
        copy_prop_field(&res, &mut response, "etag", "eTag");
        copy_prop_field(&res, &mut response, "lastModified", "lastModified");
        copy_prop_field(&res, &mut response, "copyId", "copyId");
        // copyStatus for CopyFromURL uses SyncCopyStatusType ("success")
        if copy_status_str.is_some() {
            response.insert_field("copyStatus", string_value("success"));
        }
        Ok(response)
    }

    // ── abortCopyFromURL ──────────────────────────────────────────────────────

    async fn abortCopyFromURL(
        &self,
        copyId: String,
        options: BlobAbortCopyFromURLOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobAbortCopyFromURLResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account = blobCtx.account().unwrap_or_default();
        let container = blobCtx.container().unwrap_or_default();
        let blob_name = blobCtx.blob().unwrap_or_default();

        let lease_conds = get_object(&options, "leaseAccessConditions");

        let blob = self
            .base
            .metadataStore
            .downloadBlob(
                &context,
                &account,
                &container,
                &blob_name,
                None,
                lease_conds.as_ref(),
                None,
            )
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let stored_copy_id = get_string(&blob.properties, "copyId");
        if stored_copy_id.as_deref() != Some(&copyId) {
            return Err(Box::new(StorageErrorFactory::getCopyIdMismatch(
                context.contextId().as_deref().unwrap_or(""),
            )));
        }

        let copy_status = get_string(&blob.properties, "copyStatus");
        if copy_status.as_deref() == Some("success") {
            return Err(Box::new(StorageErrorFactory::getNoPendingCopyOperation(
                context.contextId().as_deref().unwrap_or(""),
            )));
        }

        let mut response = GeneratedResponse::new(204);
        set_common_fields(&mut response, &context, &options, "requestId");
        Ok(response)
    }

    // ── setTier ───────────────────────────────────────────────────────────────

    async fn setTier(
        &self,
        tier: String,
        options: BlobSetTierOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobSetTierResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account = blobCtx.account().unwrap_or_default();
        let container = blobCtx.container().unwrap_or_default();
        let blob = blobCtx.blob().unwrap_or_default();

        let lease_conds = get_object(&options, "leaseAccessConditions");

        let status_code = self
            .base
            .metadataStore
            .setTier(
                &context,
                &account,
                &container,
                &blob,
                &tier,
                lease_conds.as_ref(),
            )
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let mut response = GeneratedResponse::new(status_code);
        let ctx_id = context.contextId().unwrap_or_default();
        let client_request_id = get_string(&options, "requestId");
        response.insert_field("requestId", string_value(&ctx_id));
        response.insert_field("version", string_value(BLOB_API_VERSION));
        if let Some(cri) = client_request_id {
            response.insert_field("clientRequestId", string_value(cri));
        }
        Ok(response)
    }

    // ── getAccountInfo ────────────────────────────────────────────────────────

    async fn getAccountInfo(
        &self,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobGetAccountInfoResponse> {
        let ctx_id = context.contextId().unwrap_or_default();
        let client_request_id = context
            .request()
            .and_then(|r| r.getHeader("x-ms-client-request-id"));
        let start_time = context.startTime();

        let mut response = GeneratedResponse::new(200);
        response.insert_field("requestId", string_value(&ctx_id));
        response.insert_field("version", string_value(BLOB_API_VERSION));
        response.insert_field("skuName", string_value(EMULATOR_ACCOUNT_SKUNAME));
        response.insert_field("accountKind", string_value(EMULATOR_ACCOUNT_KIND));
        if let Some(cri) = client_request_id {
            response.insert_field("clientRequestId", string_value(cri));
        }
        if let Some(t) = start_time {
            response.insert_field("date", string_value(formatRfc1123(t)));
        }
        Ok(response)
    }

    // ── query (not implemented) ───────────────────────────────────────────────

    async fn query(
        &self,
        _options: BlobQueryOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobQueryResponse> {
        Err(Box::new(NotImplementedError::new(
            context.contextId().as_deref(),
        )))
    }

    // ── getTags ───────────────────────────────────────────────────────────────

    async fn getTags(
        &self,
        options: BlobGetTagsOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobGetTagsResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account = blobCtx.account().unwrap_or_default();
        let container = blobCtx.container().unwrap_or_default();
        let blob = blobCtx.blob().unwrap_or_default();

        let snapshot = get_string(&options, "snapshot");
        let lease_conds = get_object(&options, "leaseAccessConditions");
        let mod_conds = get_object(&options, "modifiedAccessConditions");

        let tags = self
            .base
            .metadataStore
            .getBlobTag(
                &context,
                &account,
                &container,
                &blob,
                snapshot.as_deref(),
                lease_conds.as_ref(),
                mod_conds.as_ref(),
            )
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        // Tags are stored in XML-parsed format: { "TagSet": { "Tag": [...] } }
        // Normalize to model format: [ { "key": "k", "value": "v" }, ... ]
        let blob_tag_set = normalize_blob_tags_for_response(tags.as_ref());

        let mut response = GeneratedResponse::new(200);
        set_common_fields(&mut response, &context, &options, "requestId");
        response.insert_field("blobTagSet", blob_tag_set);
        Ok(response)
    }

    // ── setTags ───────────────────────────────────────────────────────────────

    async fn setTags(
        &self,
        options: BlobSetTagsOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<BlobSetTagsResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let account = blobCtx.account().unwrap_or_default();
        let container = blobCtx.container().unwrap_or_default();
        let blob = blobCtx.blob().unwrap_or_default();

        let tags = get_object(&options, "tags");

        // Normalize tags from XML-parsed format to model format
        let tags = tags.map(|t| normalize_tags_to_model(&t));

        // Validate tags (BlobHandler.ts:1317-1319)
        if let Some(ref t) = tags {
            validate_blob_tags(t, context.contextId().as_deref().unwrap_or(""))?;
        }

        // BlobHandler.ts:1311-1323: snapshot is read directly from query string
        // because swagger doesn't model it
        let snapshot = context.request().and_then(|r| r.getQuery("snapshot"));

        let lease_conds = get_object(&options, "leaseAccessConditions");
        let mod_conds = get_object(&options, "modifiedAccessConditions");

        self.base
            .metadataStore
            .setBlobTag(
                &context,
                &account,
                &container,
                &blob,
                snapshot.as_deref(),
                lease_conds.as_ref(),
                tags.as_ref(),
                mod_conds.as_ref(),
            )
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let mut response = GeneratedResponse::new(204);
        set_common_fields(&mut response, &context, &options, "requestId");
        Ok(response)
    }
}

// ─── Private methods ──────────────────────────────────────────────────────────

impl BlobHandler {
    /// Download a block blob or append blob.
    /// Mirrors BlobHandler.ts:downloadBlockBlobOrAppendBlob (lines ~1000-1140).
    async fn downloadBlockBlobOrAppendBlob(
        &self,
        options: BlobDownloadOptionalParams,
        context: &Context,
        blob: crate::persistence::BlobModel,
    ) -> crate::generated::GeneratedResult<BlobDownloadResponse> {
        if blob.isCommitted == Some(false) {
            return Err(Box::new(StorageErrorFactory::getBlobNotFound(
                context.contextId().as_deref(),
            )));
        }

        let req = context.request();

        // BlobHandler.ts:1007-1019: ignore malformed range headers per RFC 9110
        let range_parts = {
            let range_hdr = req.as_ref().and_then(|r| r.getHeader("range"));
            let x_ms_range = req.as_ref().and_then(|r| r.getHeader("x-ms-range"));
            deserialize_range_header(range_hdr.as_deref(), x_ms_range.as_deref())
                .unwrap_or_default()
        };

        let content_length_stored =
            get_number(&blob.properties, "contentLength").unwrap_or(0.0) as i64;

        let range_start = range_parts.map(|(s, _)| s).unwrap_or(0);
        let range_end_raw = range_parts.map(|(_, e)| e);

        // Start Range bigger than blob length
        if range_start > content_length_stored {
            return Err(Box::new(StorageErrorFactory::getInvalidPageRange2(
                context.contextId().as_deref().unwrap_or(""),
                Some(&format!("bytes */{}", content_length_stored)),
            )));
        }

        // Clamp rangeEnd to blob length; handle zero-length blob edge case
        let range_end = match range_end_raw {
            Some(e) if e >= content_length_stored || e == i64::MAX => {
                if content_length_stored == 0 && e != 0 {
                    return Err(Box::new(StorageErrorFactory::getInvalidPageRange2(
                        context.contextId().as_deref().unwrap_or(""),
                        Some(&format!("bytes */{}", content_length_stored)),
                    )));
                }
                content_length_stored - 1
            }
            Some(e) => e,
            None => content_length_stored - 1,
        };

        let content_length = range_end - range_start + 1;
        let partial_read = content_length != content_length_stored;

        // Gather extent chunks and read bytes
        let ctx_id_opt = context.contextId();
        let ctx_id = ctx_id_opt.as_deref();

        let body_bytes: Vec<u8> = {
            let blocks = &blob.committedBlocksInOrder;
            let guard = self.base.extentStore.lock().await;

            if blocks.as_ref().map(|b| b.is_empty()).unwrap_or(true) {
                // Single-chunk (or no-persistency) read
                let stream = if let Some(ref persistency) = blob.persistency {
                    let chunk = CommonIExtentChunk {
                        id: persistency.id.clone(),
                        offset: (persistency.offset + range_start) as u64,
                        count: std::cmp::min(persistency.count, content_length) as u64,
                    };
                    guard
                        .readExtent(Some(&chunk), ctx_id)
                        .await
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?
                } else {
                    guard
                        .readExtent(None, ctx_id)
                        .await
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?
                };
                read_stream_to_bytes(stream).await?
            } else {
                let chunks: Vec<CommonIExtentChunk> = blocks
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(|b| to_common_chunk(&b.persistency))
                    .collect();
                let stream = guard
                    .readExtents(
                        &chunks,
                        range_start as u64,
                        (range_end + 1 - range_start) as u64,
                        ctx_id,
                    )
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                read_stream_to_bytes(stream).await?
            }
        };

        // Content range header
        let content_range = if range_parts.is_some() {
            Some(format!(
                "bytes {}-{}/{}",
                range_start, range_end, content_length_stored
            ))
        } else {
            None
        };

        // MD5: use stored value when full blob read; compute if needed and body small enough
        let stored_md5 = get_string(&blob.properties, "contentMD5");
        let mut content_md5: Option<String> = if !partial_read {
            stored_md5.clone()
        } else {
            None
        };

        if content_length <= 4 * 1024 * 1024 && content_md5.is_none() {
            let md5_bytes = compute_md5(&body_bytes);
            content_md5 = Some(STANDARD.encode(&md5_bytes));
        }

        // BlobHandler.ts:1114: only return per-range MD5 when range response AND caller asked for it
        let include_range_md5 = req
            .as_ref()
            .and_then(|r| r.getHeader("x-ms-range-get-content-md5"))
            .is_some();
        let response_content_md5 = if content_range.is_some() {
            if include_range_md5 {
                content_md5
            } else {
                None
            }
        } else {
            content_md5
        };

        let blob_content_md5 = stored_md5;

        // Build response
        let status_code = if content_range.is_some() { 206u16 } else { 200 };
        let mut response = GeneratedResponse::new(status_code);
        set_common_fields(&mut response, context, &options, "requestId");

        // Spread blob properties
        for (key, value) in &blob.properties {
            response.insert_field(key.clone(), value.clone());
        }

        // Remap properties keys to match response headersMapper field names
        if let Some(v) = response.fields.remove("etag") {
            response.insert_field("eTag", v);
        }

        // Apply per-request override query params (rscc, rscd, rsce, rscl, rsct)
        if let Some(req_ref) = req.as_ref() {
            override_prop_from_query(req_ref, &mut response, "rscc", "cacheControl");
            override_prop_from_query(req_ref, &mut response, "rscd", "contentDisposition");
            override_prop_from_query(req_ref, &mut response, "rsce", "contentEncoding");
            override_prop_from_query(req_ref, &mut response, "rscl", "contentLanguage");
            override_prop_from_query(req_ref, &mut response, "rsct", "contentType");
        }

        response.insert_field(
            "contentLength",
            GeneratedValue::Number(content_length as f64),
        );
        if let Some(cr) = &content_range {
            response.insert_field("contentRange", string_value(cr));
        }
        // Remove the contentMD5 that was spread from blob properties —
        // we only return it conditionally based on range/header logic above.
        response.fields.remove("contentMD5");
        if let Some(md5) = response_content_md5 {
            response.insert_field("contentMD5", string_value(md5));
        }
        if let Some(blob_md5) = blob_content_md5 {
            response.insert_field("blobContentMD5", string_value(blob_md5));
        }
        response.insert_field("acceptRanges", string_value("bytes"));
        response.insert_field("isServerEncrypted", GeneratedValue::Bool(true));

        // Metadata
        if let Some(metadata) = &blob.metadata {
            response.insert_field("metadata", GeneratedValue::Object(metadata.clone()));
        }

        // Tag count
        if let Some(tc) = get_blob_tags_count(blob.blobTags.as_ref()) {
            response.insert_field("tagCount", GeneratedValue::Number(tc as f64));
        }

        // blobCommittedBlockCount for AppendBlob
        if get_string(&blob.properties, "blobType").as_deref() == Some("AppendBlob") {
            let count = blob
                .committedBlocksInOrder
                .as_ref()
                .map(|b| b.len())
                .unwrap_or(0);
            response.insert_field(
                "blobCommittedBlockCount",
                GeneratedValue::Number(count as f64),
            );
        }

        let gen_stream = GeneratedReadableStream::from_bytes(body_bytes);
        response.body = Some(GeneratedBody::Stream(gen_stream));

        Ok(response)
    }

    /// Download a page blob.
    /// Mirrors BlobHandler.ts:downloadPageBlob (lines ~1150-1260).
    async fn downloadPageBlob(
        &self,
        options: BlobDownloadOptionalParams,
        context: &Context,
        mut blob: crate::persistence::BlobModel,
    ) -> crate::generated::GeneratedResult<BlobDownloadResponse> {
        let req = context.request();

        // Page blob range header (no 512-boundary enforcement on download, force512boundary=false)
        let range_header = req.as_ref().and_then(|r| r.getHeader("range"));
        let x_ms_range = req.as_ref().and_then(|r| r.getHeader("x-ms-range"));
        let (range_start, range_end_raw) = deserialize_page_blob_range_header(
            range_header.as_deref(),
            x_ms_range.as_deref(),
            false,
        )?;

        let content_length_stored =
            get_number(&blob.properties, "contentLength").unwrap_or(0.0) as i64;

        if range_start > content_length_stored {
            return Err(Box::new(StorageErrorFactory::getInvalidPageRange2(
                context.contextId().as_deref().unwrap_or(""),
                Some(&format!("bytes */{}", content_length_stored)),
            )));
        }

        let range_end = if range_end_raw == i64::MAX || range_end_raw + 1 >= content_length_stored {
            if content_length_stored == 0 && range_end_raw != 0 && range_end_raw != i64::MAX {
                return Err(Box::new(StorageErrorFactory::getInvalidPageRange2(
                    context.contextId().as_deref().unwrap_or(""),
                    Some(&format!("bytes */{}", content_length_stored)),
                )));
            }
            content_length_stored - 1
        } else {
            range_end_raw
        };

        let content_length = range_end - range_start + 1;
        let partial_read = content_length != content_length_stored;

        // Fill zero ranges for page blob holes (BlobHandler.ts:1192-1199)
        let page_ranges = blob.pageRangesInOrder.take().unwrap_or_default();

        let ranges = if content_length <= 0 {
            vec![]
        } else {
            let mut range_map = GeneratedObject::new();
            range_map.insert(
                "start".to_string(),
                GeneratedValue::Number(range_start as f64),
            );
            range_map.insert("end".to_string(), GeneratedValue::Number(range_end as f64));
            self.rangesManager.fill_zero_ranges(&page_ranges, range_map)
        };

        let ctx_id_opt = context.contextId();
        let ctx_id = ctx_id_opt.as_deref();

        let body_bytes: Vec<u8> = {
            let chunks: Vec<CommonIExtentChunk> = ranges
                .iter()
                .map(|r| to_common_chunk(&r.persistency))
                .collect();
            let guard = self.base.extentStore.lock().await;
            let stream = guard
                .readExtents(&chunks, 0, content_length as u64, ctx_id)
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            read_stream_to_bytes(stream).await?
        };

        // Content range header: present when any range was requested
        let has_range_request = range_header.is_some() || x_ms_range.is_some();
        let content_range = if has_range_request {
            Some(format!(
                "bytes {}-{}/{}",
                range_start, range_end, content_length_stored
            ))
        } else {
            None
        };

        // MD5 for small non-partial reads or on request
        let stored_md5 = get_string(&blob.properties, "contentMD5");
        let mut content_md5: Option<String> = if !partial_read {
            stored_md5.clone()
        } else {
            None
        };
        if content_length <= 4 * 1024 * 1024 && content_md5.is_none() {
            let md5_bytes = compute_md5(&body_bytes);
            content_md5 = Some(STANDARD.encode(&md5_bytes));
        }

        let include_range_md5 = req
            .as_ref()
            .and_then(|r| r.getHeader("x-ms-range-get-content-md5"))
            .is_some();
        let response_content_md5 = if content_range.is_some() {
            if include_range_md5 {
                content_md5
            } else {
                None
            }
        } else {
            content_md5
        };

        // Status code: 200 when no explicit range was requested
        let is_full = range_start == 0 && range_end_raw == i64::MAX;
        let status_code = if is_full { 200u16 } else { 206 };

        let mut response = GeneratedResponse::new(status_code);
        set_common_fields(&mut response, context, &options, "requestId");

        for (key, value) in &blob.properties {
            response.insert_field(key.clone(), value.clone());
        }

        // Remap properties keys to match response headersMapper field names
        if let Some(v) = response.fields.remove("etag") {
            response.insert_field("eTag", v);
        }

        if let Some(req_ref) = req.as_ref() {
            override_prop_from_query(req_ref, &mut response, "rscc", "cacheControl");
            override_prop_from_query(req_ref, &mut response, "rscd", "contentDisposition");
            override_prop_from_query(req_ref, &mut response, "rsce", "contentEncoding");
            override_prop_from_query(req_ref, &mut response, "rscl", "contentLanguage");
            override_prop_from_query(req_ref, &mut response, "rsct", "contentType");
        }

        response.insert_field(
            "contentLength",
            GeneratedValue::Number(content_length as f64),
        );
        if let Some(cr) = &content_range {
            response.insert_field("contentRange", string_value(cr));
        }
        // Remove the contentMD5 that was spread from blob properties —
        // we only return it conditionally based on range/header logic above.
        response.fields.remove("contentMD5");
        if let Some(md5) = response_content_md5 {
            response.insert_field("contentMD5", string_value(md5));
        }
        if let Some(blob_md5) = stored_md5 {
            response.insert_field("blobContentMD5", string_value(blob_md5));
        }
        response.insert_field("isServerEncrypted", GeneratedValue::Bool(true));

        if let Some(metadata) = &blob.metadata {
            response.insert_field("metadata", GeneratedValue::Object(metadata.clone()));
        }
        if let Some(tc) = get_blob_tags_count(blob.blobTags.as_ref()) {
            response.insert_field("tagCount", GeneratedValue::Number(tc as f64));
        }

        let gen_stream = GeneratedReadableStream::from_bytes(body_bytes);
        response.body = Some(GeneratedBody::Stream(gen_stream));

        Ok(response)
    }

    /// Validate copy source by issuing a GET ?comp=metadata request.
    /// Mirrors BlobHandler.ts:validateCopySource (lines ~701-775).
    ///
    /// # Residual risk
    /// The HTTP client here is created per-call (no connection pool reuse).
    /// Error message parsing of XML `<Message>` element is a best-effort approach.
    async fn validateCopySource(
        &self,
        copySource: &str,
        _sourceAccount: &str,
        context: &Context,
    ) -> crate::generated::GeneratedResult<()> {
        let _blobCtx = BlobStorageContext::new(context);
        let current_server = context
            .request()
            .and_then(|r| r.getHeader(HOST))
            .unwrap_or_default();

        let url = new_uri_from_copy_source(copySource, context)?;
        let url_host = url.host_str().unwrap_or("").to_string();
        let url_port = url.port().map(|p| format!(":{}", p)).unwrap_or_default();
        let url_host_with_port = format!("{}{}", url_host, url_port);

        // BlobHandler.ts:706-719: source host must equal current Host header
        if current_server != url_host_with_port {
            return Err(Box::new(StorageErrorFactory::getCannotVerifyCopySource(
                context.contextId().as_deref().unwrap_or(""),
                404,
                "The specified resource does not exist",
                None,
            )));
        }

        let metadata_url = build_copy_source_metadata_url(copySource);

        // Issue GET request and validate (BlobHandler.ts:725-775)
        let client = reqwest::Client::new();
        let validation_response = client
            .get(&metadata_url)
            .send()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let status = validation_response.status().as_u16();

        if status == 200 {
            return Ok(());
        }

        let content_type = validation_response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let body_text = validation_response.text().await.unwrap_or_default();

        if status == 404 {
            return Err(Box::new(StorageErrorFactory::getCannotVerifyCopySource(
                context.contextId().as_deref().unwrap_or(""),
                404,
                "The specified resource does not exist",
                None,
            )));
        }

        // BlobHandler.ts:757-771: attempt to extract <Message> from XML error body
        let mut message = "Could not verify the copy source within the specified time.".to_string();
        if content_type == "application/xml" {
            if let Ok(parsed) = crate::generated::utils::xml::parseXML(&body_text, false) {
                if let Some(msg) = parsed.get("Message").and_then(|v| v.as_str()) {
                    message = msg.replace('\n', "");
                }
            }
        }

        Err(Box::new(StorageErrorFactory::getCannotVerifyCopySource(
            context.contextId().as_deref().unwrap_or(""),
            status,
            &message,
            None,
        )))
    }
}

// ─── Local helper functions ───────────────────────────────────────────────────

fn build_copy_source_metadata_url(copy_source: &str) -> String {
    let Some((base, query)) = copy_source.split_once('?') else {
        return format!("{copy_source}?comp=metadata");
    };

    let mut query_parts = query
        .split('&')
        .filter(|part| !part.is_empty() && !part.starts_with("comp="))
        .collect::<Vec<_>>();
    query_parts.push("comp=metadata");

    format!("{base}?{}", query_parts.join("&"))
}

/// Parse `new URL(copySource)`, throwing `InvalidHeaderValue` for `x-ms-copy-source` on failure.
/// Mirrors BlobHandler.ts:NewUriFromCopySource (lines ~1336-1348).
fn new_uri_from_copy_source(
    copySource: &str,
    context: &Context,
) -> crate::generated::GeneratedResult<Url> {
    Url::parse(copySource).map_err(|_| {
        let mut extra = BTreeMap::new();
        extra.insert("HeaderName".to_string(), "x-ms-copy-source".to_string());
        extra.insert("HeaderValue".to_string(), copySource.to_string());
        Box::new(StorageErrorFactory::getInvalidHeaderValue(
            context.contextId().as_deref(),
            Some(extra),
        )) as _
    })
}

/// Extract (account, container, blob) from a URL path.
/// Mirrors extractStoragePartsFromPath from blobStorageContext.middleware.ts.
fn extract_storage_parts_from_path(
    hostname: &str,
    path: &str,
    disable_product_style_url: Option<bool>,
) -> (Option<String>, Option<String>, Option<String>) {
    const SECONDARY_SUFFIX: &str = "-secondary";
    const HOST_DOCKER_INTERNAL: &str = "host.docker.internal";

    let decoded = percent_decode(path);
    let normalized = decoded.strip_prefix('/').unwrap_or(&decoded);
    let parts: Vec<&str> = normalized.split('/').collect();

    let is_ip =
        hostname.split('.').count() == 4 && hostname.split('.').all(|s| s.parse::<u8>().is_ok());
    let is_no_account_host = hostname.eq_ignore_ascii_case(HOST_DOCKER_INTERNAL)
        || hostname.eq_ignore_ascii_case("localhost");
    let first_dot = hostname.find('.');

    let mut idx: usize = 0;
    let mut account: Option<String>;

    if !disable_product_style_url.unwrap_or(false)
        && !is_ip
        && !is_no_account_host
        && first_dot.map(|i| i > 0).unwrap_or(false)
    {
        account = first_dot.map(|i| hostname[..i].to_owned());
    } else {
        account = parts.get(idx).map(|s| s.to_string());
        idx += 1;
    }

    let container = parts.get(idx).map(|s| s.to_string());
    idx += 1;

    let blob = if idx < parts.len() {
        let joined = parts[idx..].join("/").replace('\\', "/");
        if joined.is_empty() {
            None
        } else {
            Some(joined)
        }
    } else {
        None
    };

    if let Some(ref acc) = account {
        if acc.ends_with(SECONDARY_SUFFIX) {
            account = Some(acc[..acc.len() - SECONDARY_SUFFIX.len()].to_owned());
        }
    }

    (account, container, blob)
}

/// Minimal percent-decode for ASCII-safe characters in blob/container names.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            ) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_owned())
}

/// Deserialize range header into `(start, end)`.
/// `end` is `None` when open-ended. Returns `Ok(None)` when no range is present.
/// Mirrors deserializeRangeHeader in blob/utils/utils.ts.
fn deserialize_range_header(
    range: Option<&str>,
    x_ms_range: Option<&str>,
) -> Result<Option<(i64, i64)>, String> {
    let raw = x_ms_range.or(range);
    let raw = match raw {
        Some(r) if !r.is_empty() => r,
        _ => return Ok(None),
    };

    let parts: Vec<&str> = raw.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err(format!("deserialize_range_header: wrong range {raw}"));
    }
    let sub_parts: Vec<&str> = parts[1].splitn(2, '-').collect();
    if sub_parts.is_empty() || sub_parts.len() > 2 {
        return Err(format!("deserialize_range_header: wrong range {raw}"));
    }

    let start: i64 = sub_parts[0]
        .parse()
        .map_err(|_| format!("deserialize_range_header: bad start in {raw}"))?;
    let end: i64 = if sub_parts.len() > 1 && !sub_parts[1].is_empty() {
        sub_parts[1]
            .parse()
            .map_err(|_| format!("deserialize_range_header: bad end in {raw}"))?
    } else {
        i64::MAX
    };

    if start > end {
        return Err(format!("deserialize_range_header: start > end in {raw}"));
    }

    Ok(Some((start, end)))
}

/// Deserialize page-blob range header; returns `(start, end)`.
/// `end == i64::MAX` means open-ended. `force_512` may validate 512-byte alignment.
/// Mirrors deserializePageBlobRangeHeader in blob/utils/utils.ts.
fn deserialize_page_blob_range_header(
    range: Option<&str>,
    x_ms_range: Option<&str>,
    force_512: bool,
) -> crate::generated::GeneratedResult<(i64, i64)> {
    let parts = deserialize_range_header(range, x_ms_range).map_err(|e| {
        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
            as Box<dyn std::error::Error + Send + Sync>
    })?;
    let (start, end) = parts.unwrap_or((0, i64::MAX));

    if force_512 {
        if start % 512 != 0 {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("deserialize_page_blob_range_header: start {start} not 512-aligned"),
            )));
        }
        if end != i64::MAX && (end + 1) % 512 != 0 {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("deserialize_page_blob_range_header: end {end} not 512-aligned"),
            )));
        }
    }

    Ok((start, end))
}

/// Count tags in a `BlobTags` (GeneratedObject). Returns None when no tags or empty.
fn get_blob_tags_count(tags: Option<&GeneratedObject>) -> Option<i64> {
    let arr = tags?.get("blobTagSet")?;
    if let GeneratedValue::Array(ref items) = arr {
        if items.is_empty() {
            None
        } else {
            Some(items.len() as i64)
        }
    } else {
        None
    }
}

/// Normalize tags from XML-parsed format to model format for storage.
/// XML: { "TagSet": { "Tag": [/single { "Key": {"$text":"k"}, "Value": {"$text":"v"} }] } }
/// Model: { "blobTagSet": [{ "key": "k", "value": "v" }] }
fn normalize_tags_to_model(tags: &GeneratedObject) -> GeneratedObject {
    // If already in model format, return as-is
    if tags.contains_key("blobTagSet") {
        return tags.clone();
    }
    let normalized = normalize_blob_tags_for_response(Some(tags));
    let mut result = BTreeMap::new();
    result.insert("blobTagSet".to_string(), normalized);
    result
}

/// Normalize blob tags from XML-parsed format to model format for response serialization.
/// XML-parsed: { "TagSet": { "Tag": [{ "Key": {"$text": "k"}, "Value": {"$text": "v"} }] } }
/// or single tag (non-array): { "TagSet": { "Tag": { "Key": {"$text": "k"}, "Value": {"$text": "v"} } } }
/// Model format: [{ "key": "k", "value": "v" }]
fn normalize_blob_tags_for_response(tags: Option<&GeneratedObject>) -> GeneratedValue {
    let tags = match tags {
        Some(t) => t,
        None => return GeneratedValue::Array(vec![]),
    };

    // Try model name first (blobTagSet), then XML name (TagSet)
    let tag_set = tags.get("blobTagSet").or_else(|| tags.get("TagSet"));

    let tag_set = match tag_set {
        Some(ts) => ts,
        None => return GeneratedValue::Array(vec![]),
    };

    // Get the "Tag" array (or single object) from the TagSet
    let tag_items = match tag_set {
        GeneratedValue::Array(arr) => arr.clone(),
        GeneratedValue::Object(obj) => {
            if let Some(tag_val) = obj.get("Tag") {
                match tag_val {
                    GeneratedValue::Array(arr) => arr.clone(),
                    GeneratedValue::Object(_) => vec![tag_val.clone()],
                    _ => vec![],
                }
            } else {
                // Already in model format array
                return GeneratedValue::Object(obj.clone());
            }
        }
        _ => return GeneratedValue::Array(vec![]),
    };

    // Convert each tag from XML format to model format
    let result: Vec<GeneratedValue> = tag_items
        .iter()
        .filter_map(|item| {
            let obj = item.as_object()?;
            // Extract key - may be { "Key": { "$text": "k" } } or { "key": "k" }
            let key = extract_text_value(obj, "Key").or_else(|| extract_text_value(obj, "key"));
            let value =
                extract_text_value(obj, "Value").or_else(|| extract_text_value(obj, "value"));
            let mut tag = BTreeMap::new();
            tag.insert("key".to_string(), GeneratedValue::String(key?));
            tag.insert(
                "value".to_string(),
                GeneratedValue::String(value.unwrap_or_default()),
            );
            Some(GeneratedValue::Object(tag))
        })
        .collect();

    GeneratedValue::Array(result)
}

/// Extract text value from an object field that may be either a plain string
/// or a `{ "$text": "val" }` wrapper (from quick_xml parsing).
fn extract_text_value(obj: &GeneratedObject, key: &str) -> Option<String> {
    let val = obj.get(key)?;
    match val {
        GeneratedValue::String(s) => Some(s.clone()),
        GeneratedValue::Object(inner) => inner.get("$text").and_then(|v| v.as_string()),
        _ => None,
    }
}

/// Validate blob tags against Azure Storage rules.
/// Mirrors validateBlobTag from blob/utils/utils.ts.
pub(crate) fn validate_blob_tags(
    tags: &GeneratedObject,
    context_id: &str,
) -> crate::generated::GeneratedResult<()> {
    let tag_set = match tags.get("blobTagSet") {
        Some(GeneratedValue::Array(arr)) => arr.clone(),
        _ => return Ok(()),
    };

    if tag_set.len() > 10 {
        return Err(Box::new(StorageErrorFactory::getTagsTooLarge(context_id)));
    }

    for tag in &tag_set {
        let key = match tag {
            GeneratedValue::Object(ref m) => m
                .get("key")
                .and_then(GeneratedValue::as_string)
                .unwrap_or_default(),
            _ => String::new(),
        };
        let value = match tag {
            GeneratedValue::Object(ref m) => m
                .get("value")
                .and_then(GeneratedValue::as_string)
                .unwrap_or_default(),
            _ => String::new(),
        };

        if key.is_empty() {
            // TS xml2js fails to deserialize <Key/> (empty self-closing tag),
            // producing a bare 400 with no body (DeserializationError).
            // Match that behavior instead of returning StorageError with XML body.
            return Err(Box::new(
                crate::generated::errors::middleware_error::MiddlewareError::new(
                    400,
                    "Empty tag key",
                ),
            ));
        }
        if key.len() > 128 || value.len() > 256 {
            return Err(Box::new(StorageErrorFactory::getTagsTooLarge(context_id)));
        }
        if contains_invalid_tag_character(&key) || contains_invalid_tag_character(&value) {
            return Err(Box::new(StorageErrorFactory::getInvalidTag(context_id)));
        }
    }

    Ok(())
}

/// Allowed characters in blob tag keys/values.
/// Matches TS ContainsInvalidTagCharacter: a-z A-Z 0-9 space + - . / : = _
fn contains_invalid_tag_character(s: &str) -> bool {
    !s.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || c == ' '
            || c == '+'
            || c == '-'
            || c == '.'
            || c == '/'
            || c == ':'
            || c == '='
            || c == '_'
    })
}

/// Read all bytes from an async stream.
async fn read_stream_to_bytes(
    mut stream: azurite_common::persistence::i_extent_store::ReadableStream,
) -> crate::generated::GeneratedResult<Vec<u8>> {
    let mut bytes = Vec::new();
    stream
        .read_to_end(&mut bytes)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    Ok(bytes)
}

/// Compute MD5 hash and return raw bytes.
fn compute_md5(data: &[u8]) -> Vec<u8> {
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// Convert from blob-store `IExtentChunk` (i64 fields) to the common extent chunk (u64 fields).
fn to_common_chunk(c: &crate::persistence::IExtentChunk) -> CommonIExtentChunk {
    CommonIExtentChunk {
        id: c.id.clone(),
        offset: c.offset as u64,
        count: c.count as u64,
    }
}

// ─── GeneratedValue accessor helpers ─────────────────────────────────────────

fn get_string(map: &GeneratedObject, key: &str) -> Option<String> {
    map.get(key).and_then(GeneratedValue::as_string)
}

fn get_number(map: &GeneratedObject, key: &str) -> Option<f64> {
    map.get(key).and_then(GeneratedValue::as_number)
}

fn get_object(map: &GeneratedObject, key: &str) -> Option<GeneratedObject> {
    map.get(key).and_then(GeneratedValue::as_object).cloned()
}

fn string_value(value: impl Into<String>) -> GeneratedValue {
    GeneratedValue::String(value.into())
}

/// Copy a field from a properties map to a response with a possibly different field name.
/// Used to map e.g. `etag` → `eTag`.
fn copy_prop_field(
    props: &GeneratedObject,
    response: &mut GeneratedResponse,
    src_key: &str,
    dst_key: &str,
) {
    if let Some(value) = props.get(src_key).cloned() {
        response.insert_field(dst_key, value);
    }
}

/// Apply a query parameter as an override for a response field (rscc, rscd, etc.).
fn override_prop_from_query<R: IRequest>(
    req: &R,
    response: &mut GeneratedResponse,
    query_param: &str,
    field_name: &str,
) {
    if let Some(value) = req.getQuery(query_param) {
        response.insert_field(field_name, string_value(value));
    }
}

/// Set common response fields: requestId, version, date, clientRequestId.
fn set_common_fields(
    response: &mut GeneratedResponse,
    context: &Context,
    options: &GeneratedObject,
    request_id_field: &str,
) {
    let ctx_id = context.contextId().unwrap_or_default();
    let start_time = context.startTime();
    let client_request_id = get_string(options, request_id_field);

    response.insert_field("requestId", string_value(&ctx_id));
    response.insert_field("version", string_value(BLOB_API_VERSION));
    if let Some(t) = start_time {
        response.insert_field("date", string_value(formatRfc1123(t)));
    }
    if let Some(cri) = client_request_id {
        response.insert_field("clientRequestId", string_value(cri));
    }
}
