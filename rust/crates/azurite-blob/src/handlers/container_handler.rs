use std::sync::Arc;

use async_trait::async_trait;
use azurite_common::i_account_data_store::IAccountDataStore;
use azurite_common::utils::utils::{convertRawHeadersToMetadata, newEtag};
use chrono::Utc;
use serde::Serialize;

use crate::context::blob_storage_context::BlobStorageContext;
use crate::errors::{NotImplementedError, StorageErrorFactory};
use crate::generated::artifacts::models::{
    ContainerAcquireLeaseOptionalParams, ContainerAcquireLeaseResponse,
    ContainerBreakLeaseOptionalParams, ContainerBreakLeaseResponse,
    ContainerChangeLeaseOptionalParams, ContainerChangeLeaseResponse,
    ContainerCreateOptionalParams, ContainerCreateResponse, ContainerDeleteMethodOptionalParams,
    ContainerDeleteResponse, ContainerFilterBlobsOptionalParams, ContainerFilterBlobsResponse,
    ContainerGetAccessPolicyOptionalParams, ContainerGetAccessPolicyResponse,
    ContainerGetAccountInfoResponse, ContainerGetPropertiesOptionalParams,
    ContainerGetPropertiesResponse, ContainerGetPropertiesWithHeadOptionalParams,
    ContainerGetPropertiesWithHeadResponse, ContainerReleaseLeaseOptionalParams,
    ContainerReleaseLeaseResponse, ContainerRenewLeaseOptionalParams, ContainerRenewLeaseResponse,
    ContainerRestoreOptionalParams, ContainerRestoreResponse,
    ContainerSetAccessPolicyOptionalParams, ContainerSetAccessPolicyResponse,
    ContainerSetMetadataOptionalParams, ContainerSetMetadataResponse,
    ContainerSubmitBatchOptionalParams, ContainerSubmitBatchResponse, GeneratedBody,
    GeneratedObject, GeneratedResponse, GeneratedValue, SignedIdentifier,
};
use crate::generated::context::Context;
use crate::generated::handlers::i_container_handler::IContainerHandler;
use crate::generated::i_request::{GeneratedReadableStream, IRequest};
use crate::persistence::{ContainerModel, SetContainerAccessPolicyOptions};

use super::base_handler::BaseHandler;
use super::batch_handlers_bundle::{
    create_batch_request, create_blob_batch_handler, parse_batch_boundary,
};
use super::service_handler::ServiceHandler;
const BLOB_API_VERSION: &str = "2025-11-05";
const DEFAULT_LIST_BLOBS_MAX_RESULTS: i64 = 5000;
const EMULATOR_ACCOUNT_SKUNAME: &str = "Standard_RAGRS";
const EMULATOR_ACCOUNT_KIND: &str = "StorageV2";
const HEADER_CLIENT_REQUEST_ID: &str = "x-ms-client-request-id";

#[derive(Clone)]
pub struct ContainerHandler {
    pub base: BaseHandler,
    pub accountDataStore: Arc<dyn IAccountDataStore + Send + Sync>,
    pub oauth: Option<String>,
    pub disableProductStyle: Option<bool>,
}

impl ContainerHandler {
    pub fn new(
        accountDataStore: Arc<dyn IAccountDataStore + Send + Sync>,
        oauth: Option<String>,
        base: BaseHandler,
        disableProductStyle: Option<bool>,
    ) -> Self {
        Self {
            base,
            accountDataStore,
            oauth,
            disableProductStyle,
        }
    }
}

#[async_trait]
impl IContainerHandler for ContainerHandler {
    async fn create(
        &self,
        options: ContainerCreateOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ContainerCreateResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let accountName = blobCtx.account().unwrap_or_default();
        let containerName = blobCtx.container().unwrap_or_default();
        let lastModified = context.startTime().unwrap_or_else(Utc::now);
        let etag = newEtag();
        let ctx_id = context.contextId().unwrap_or_default();
        let metadata = context
            .request()
            .map(|request| convertRawHeadersToMetadata(&request.getRawHeaders(), &ctx_id))
            .transpose()
            .map_err(|_| {
                Box::new(StorageErrorFactory::getInvalidMetadata(&ctx_id))
                    as Box<dyn std::error::Error + Send + Sync>
            })?
            .flatten();

        let mut properties = GeneratedObject::new();
        properties.insert("etag".into(), string_value(etag.clone()));
        properties.insert("lastModified".into(), json_value(lastModified));
        properties.insert("leaseStatus".into(), string_value("unlocked"));
        properties.insert("leaseState".into(), string_value("available"));
        if let Some(access) = get_string(&options, "access") {
            properties.insert("publicAccess".into(), string_value(access));
        }
        properties.insert("hasImmutabilityPolicy".into(), GeneratedValue::Bool(false));
        properties.insert("hasLegalHold".into(), GeneratedValue::Bool(false));

        self.base
            .metadataStore
            .createContainer(
                &context,
                ContainerModel {
                    accountName,
                    name: Some(containerName),
                    metadata,
                    properties,
                    ..Default::default()
                },
            )
            .await?;

        let mut response = GeneratedResponse::new(201);
        response.insert_field(
            "requestId",
            string_value(context.contextId().unwrap_or_default()),
        );
        if let Some(client_request_id) = get_string(&options, "requestId") {
            response.insert_field("clientRequestId", string_value(client_request_id));
        }
        response.insert_field("eTag", string_value(etag));
        response.insert_field("lastModified", json_value(lastModified));
        response.insert_field("version", string_value(BLOB_API_VERSION));
        Ok(response)
    }

    async fn getProperties(
        &self,
        options: ContainerGetPropertiesOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ContainerGetPropertiesResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let accountName = blobCtx.account().unwrap_or_default();
        let containerName = blobCtx.container().unwrap_or_default();
        let leaseAccessConditions = get_object(&options, "leaseAccessConditions");
        let containerProperties = self
            .base
            .metadataStore
            .getContainerProperties(
                &context,
                &accountName,
                &containerName,
                leaseAccessConditions.as_ref(),
            )
            .await?;

        let mut response = GeneratedResponse::new(200);
        response.insert_field(
            "requestId",
            string_value(context.contextId().unwrap_or_default()),
        );
        if let Some(client_request_id) = get_string(&options, "requestId") {
            response.insert_field("clientRequestId", string_value(client_request_id));
        }
        if let Some(etag) = containerProperties
            .get("properties")
            .and_then(GeneratedValue::as_object)
            .and_then(|props| props.get("etag"))
            .and_then(GeneratedValue::as_string)
        {
            response.insert_field("eTag", string_value(etag));
        }
        if let Some(properties) = containerProperties
            .get("properties")
            .and_then(GeneratedValue::as_object)
        {
            for (key, value) in properties {
                response.insert_field(key.clone(), value.clone());
            }
            if let Some(publicAccess) = properties.get("publicAccess") {
                response.insert_field("blobPublicAccess", publicAccess.clone());
            }
        }
        if let Some(metadata) = containerProperties.get("metadata") {
            response.insert_field("metadata", metadata.clone());
        }
        response.insert_field("version", string_value(BLOB_API_VERSION));
        Ok(response)
    }

    async fn getPropertiesWithHead(
        &self,
        options: ContainerGetPropertiesWithHeadOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ContainerGetPropertiesWithHeadResponse> {
        self.getProperties(options, context).await
    }

    async fn delete(
        &self,
        options: ContainerDeleteMethodOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ContainerDeleteResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        self.base
            .metadataStore
            .deleteContainer(
                &context,
                &blobCtx.account().unwrap_or_default(),
                &blobCtx.container().unwrap_or_default(),
                Some(&options),
            )
            .await?;

        let mut response = GeneratedResponse::new(202);
        response.insert_field(
            "requestId",
            string_value(context.contextId().unwrap_or_default()),
        );
        if let Some(client_request_id) = get_string(&options, "requestId") {
            response.insert_field("clientRequestId", string_value(client_request_id));
        }
        if let Some(start_time) = context.startTime() {
            response.insert_field("date", json_value(start_time));
        }
        response.insert_field("version", string_value(BLOB_API_VERSION));
        Ok(response)
    }

    async fn setMetadata(
        &self,
        options: ContainerSetMetadataOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ContainerSetMetadataResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let accountName = blobCtx.account().unwrap_or_default();
        let containerName = blobCtx.container().unwrap_or_default();
        let date = context.startTime().unwrap_or_else(Utc::now);
        let eTag = newEtag();
        let ctx_id = context.contextId().unwrap_or_default();
        let metadata = context
            .request()
            .map(|request| convertRawHeadersToMetadata(&request.getRawHeaders(), &ctx_id))
            .transpose()
            .map_err(|_| {
                Box::new(StorageErrorFactory::getInvalidMetadata(&ctx_id))
                    as Box<dyn std::error::Error + Send + Sync>
            })?
            .flatten();
        let leaseAccessConditions = get_object(&options, "leaseAccessConditions");
        let modifiedAccessConditions = get_object(&options, "modifiedAccessConditions");

        self.base
            .metadataStore
            .setContainerMetadata(
                &context,
                &accountName,
                &containerName,
                date,
                &eTag,
                metadata.as_ref(),
                leaseAccessConditions.as_ref(),
                modifiedAccessConditions.as_ref(),
            )
            .await?;

        let mut response = GeneratedResponse::new(200);
        response.insert_field(
            "requestId",
            string_value(context.contextId().unwrap_or_default()),
        );
        if let Some(client_request_id) = get_string(&options, "requestId") {
            response.insert_field("clientRequestId", string_value(client_request_id));
        }
        response.insert_field("date", json_value(date));
        response.insert_field("eTag", string_value(eTag));
        response.insert_field("lastModified", json_value(date));
        Ok(response)
    }

    async fn getAccessPolicy(
        &self,
        options: ContainerGetAccessPolicyOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ContainerGetAccessPolicyResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let accountName = blobCtx.account().unwrap_or_default();
        let containerName = blobCtx.container().unwrap_or_default();
        let leaseAccessConditions = get_object(&options, "leaseAccessConditions");
        let containerAcl = self
            .base
            .metadataStore
            .getContainerACL(
                &context,
                &accountName,
                &containerName,
                leaseAccessConditions.as_ref(),
            )
            .await?
            .unwrap_or_default();

        let mut response = GeneratedResponse::new(200);
        response.body = Some(GeneratedBody::Value(GeneratedValue::Array(
            containerAcl
                .containerAcl
                .unwrap_or_default()
                .into_iter()
                .map(GeneratedValue::Object)
                .collect(),
        )));
        response.insert_field(
            "requestId",
            string_value(context.contextId().unwrap_or_default()),
        );
        response.insert_field("version", string_value(BLOB_API_VERSION));
        if let Some(client_request_id) = get_string(&options, "requestId") {
            response.insert_field("clientRequestId", string_value(client_request_id));
        }
        if let Some(last_modified) = containerAcl.properties.get("lastModified") {
            response.insert_field("date", last_modified.clone());
            response.insert_field("lastModified", last_modified.clone());
        }
        if let Some(public_access) = containerAcl.properties.get("publicAccess") {
            response.insert_field("blobPublicAccess", public_access.clone());
        }
        if let Some(etag) = containerAcl.properties.get("etag") {
            response.insert_field("eTag", etag.clone());
        }
        Ok(response)
    }

    async fn setAccessPolicy(
        &self,
        options: ContainerSetAccessPolicyOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ContainerSetAccessPolicyResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let accountName = blobCtx.account().unwrap_or_default();
        let containerName = blobCtx.container().unwrap_or_default();
        let date = context.startTime().unwrap_or_else(Utc::now);
        let eTag = newEtag();

        self.base
            .metadataStore
            .setContainerACL(
                &context,
                &accountName,
                &containerName,
                SetContainerAccessPolicyOptions {
                    lastModified: Some(date),
                    etag: Some(eTag.clone()),
                    publicAccess: get_string(&options, "access"),
                    containerAcl: get_signed_identifier_array(&options, "containerAcl"),
                    leaseAccessConditions: get_object(&options, "leaseAccessConditions"),
                    modifiedAccessConditions: get_object(&options, "modifiedAccessConditions"),
                },
            )
            .await?;

        let mut response = GeneratedResponse::new(200);
        response.insert_field("date", json_value(date));
        response.insert_field("eTag", string_value(eTag));
        response.insert_field("lastModified", json_value(date));
        response.insert_field(
            "requestId",
            string_value(context.contextId().unwrap_or_default()),
        );
        response.insert_field("version", string_value(BLOB_API_VERSION));
        if let Some(client_request_id) = get_string(&options, "requestId") {
            response.insert_field("clientRequestId", string_value(client_request_id));
        }
        Ok(response)
    }

    async fn restore(
        &self,
        _options: ContainerRestoreOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ContainerRestoreResponse> {
        Err(Box::new(NotImplementedError::new(
            context.contextId().as_deref(),
        )))
    }

    /// `ContainerHandler.submitBatch()` — delegates to `BlobBatchHandler`.
    async fn submitBatch(
        &self,
        body: GeneratedReadableStream,
        _contentLength: f64,
        multipartContentType: String,
        options: ContainerSubmitBatchOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ContainerSubmitBatchResponse> {
        let boundary = parse_batch_boundary(&multipartContentType).ok_or_else(|| {
            Box::new(StorageErrorFactory::getInvalidHeaderValue(
                context.contextId().as_deref(),
                None,
            )) as Box<dyn std::error::Error + Send + Sync>
        })?;
        let batch_request = create_batch_request(&context).ok_or_else(|| {
            Box::new(StorageErrorFactory::getInvalidHeaderValue(
                context.contextId().as_deref(),
                None,
            )) as Box<dyn std::error::Error + Send + Sync>
        })?;
        let request_path = context
            .request()
            .map(|request| request.getPath())
            .unwrap_or_default();
        let service_handler = ServiceHandler::new(
            Arc::clone(&self.accountDataStore),
            self.oauth.clone(),
            self.base.clone(),
            self.disableProductStyle,
        );
        let batch_handler = create_blob_batch_handler(
            service_handler,
            self.clone(),
            self.base.clone(),
            Arc::clone(&self.accountDataStore),
            self.oauth.clone(),
            self.disableProductStyle,
        );
        let response_body = batch_handler
            .submitBatch(
                &body.read_to_vec(),
                &boundary,
                &request_path,
                &batch_request,
                context.contextId().as_deref().unwrap_or_default(),
            )
            .await;

        let mut response = GeneratedResponse::new(202);
        response.contentType = Some(format!("multipart/mixed; boundary={boundary}"));
        response.body = Some(GeneratedBody::Text(response_body));
        response.insert_field(
            "requestId",
            string_value(context.contextId().unwrap_or_default()),
        );
        response.insert_field("version", string_value(BLOB_API_VERSION));
        if let Some(start_time) = context.startTime() {
            response.insert_field("date", json_value(start_time));
        }
        if let Some(client_request_id) = get_string(&options, "requestId") {
            response.insert_field("clientRequestId", string_value(client_request_id));
        }
        Ok(response)
    }

    async fn filterBlobs(
        &self,
        options: ContainerFilterBlobsOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ContainerFilterBlobsResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let accountName = blobCtx.account().unwrap_or_default();
        let containerName = blobCtx.container().unwrap_or_default();
        self.base
            .metadataStore
            .checkContainerExist(&context, &accountName, &containerName)
            .await?;

        let marker = get_string(&options, "marker");
        let maxresults = get_i64(&options, "maxresults")
            .map(|value| value.min(DEFAULT_LIST_BLOBS_MAX_RESULTS))
            .unwrap_or(DEFAULT_LIST_BLOBS_MAX_RESULTS);
        let where_clause = get_string(&options, "where");
        let (blobs, next_marker) = self
            .base
            .metadataStore
            .filterBlobs(
                &context,
                &accountName,
                Some(&containerName),
                where_clause.as_deref(),
                Some(maxresults),
                marker.as_deref(),
            )
            .await?;

        let serviceEndpoint = context
            .request()
            .map(|request| format!("{}/{}", request.getEndpoint(), accountName))
            .unwrap_or_default();
        let mut response = GeneratedResponse::new(200);
        response.insert_field(
            "requestId",
            string_value(context.contextId().unwrap_or_default()),
        );
        response.insert_field("version", string_value(BLOB_API_VERSION));
        if let Some(start_time) = context.startTime() {
            response.insert_field("date", json_value(start_time));
        }
        response.insert_field("serviceEndpoint", string_value(serviceEndpoint));
        if let Some(where_clause) = where_clause {
            response.insert_field("where", string_value(where_clause));
        }
        response.insert_field(
            "blobs",
            GeneratedValue::Array(
                blobs
                    .into_iter()
                    .map(|blob| GeneratedValue::Object(filter_blob_to_obj(&blob)))
                    .collect(),
            ),
        );
        if let Some(client_request_id) = get_string(&options, "requestId") {
            response.insert_field("clientRequestId", string_value(client_request_id));
        }
        response.insert_field("nextMarker", string_value(next_marker.unwrap_or_default()));
        Ok(response)
    }

    async fn acquireLease(
        &self,
        options: ContainerAcquireLeaseOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ContainerAcquireLeaseResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let res = self
            .base
            .metadataStore
            .acquireContainerLease(
                &context,
                &blobCtx.account().unwrap_or_default(),
                &blobCtx.container().unwrap_or_default(),
                Some(&options),
            )
            .await?;
        let mut response = GeneratedResponse::new(201);
        response.insert_field(
            "requestId",
            string_value(context.contextId().unwrap_or_default()),
        );
        if let Some(client_request_id) = get_string(&options, "requestId") {
            response.insert_field("clientRequestId", string_value(client_request_id));
        }
        if let Some(start_time) = context.startTime() {
            response.insert_field("date", json_value(start_time));
        }
        if let Some(etag) = res.properties.get("etag") {
            response.insert_field("eTag", etag.clone());
        }
        if let Some(last_modified) = res.properties.get("lastModified") {
            response.insert_field("lastModified", last_modified.clone());
        }
        if let Some(leaseId) = res.leaseId {
            response.insert_field("leaseId", string_value(leaseId));
        }
        response.insert_field("version", string_value(BLOB_API_VERSION));
        Ok(response)
    }

    async fn releaseLease(
        &self,
        leaseId: String,
        options: ContainerReleaseLeaseOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ContainerReleaseLeaseResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let res = self
            .base
            .metadataStore
            .releaseContainerLease(
                &context,
                &blobCtx.account().unwrap_or_default(),
                &blobCtx.container().unwrap_or_default(),
                &leaseId,
                Some(&options),
            )
            .await?;
        let mut response = GeneratedResponse::new(200);
        response.insert_field(
            "requestId",
            string_value(context.contextId().unwrap_or_default()),
        );
        if let Some(client_request_id) = get_string(&options, "requestId") {
            response.insert_field("clientRequestId", string_value(client_request_id));
        }
        if let Some(start_time) = context.startTime() {
            response.insert_field("date", json_value(start_time));
        }
        if let Some(etag) = res.get("etag") {
            response.insert_field("eTag", etag.clone());
        }
        if let Some(last_modified) = res.get("lastModified") {
            response.insert_field("lastModified", last_modified.clone());
        }
        response.insert_field("version", string_value(BLOB_API_VERSION));
        Ok(response)
    }

    async fn renewLease(
        &self,
        leaseId: String,
        options: ContainerRenewLeaseOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ContainerRenewLeaseResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let res = self
            .base
            .metadataStore
            .renewContainerLease(
                &context,
                &blobCtx.account().unwrap_or_default(),
                &blobCtx.container().unwrap_or_default(),
                &leaseId,
                Some(&options),
            )
            .await?;
        let mut response = GeneratedResponse::new(200);
        response.insert_field(
            "requestId",
            string_value(context.contextId().unwrap_or_default()),
        );
        if let Some(client_request_id) = get_string(&options, "requestId") {
            response.insert_field("clientRequestId", string_value(client_request_id));
        }
        if let Some(start_time) = context.startTime() {
            response.insert_field("date", json_value(start_time));
        }
        if let Some(leaseId) = res.leaseId {
            response.insert_field("leaseId", string_value(leaseId));
        }
        if let Some(etag) = res.properties.get("etag") {
            response.insert_field("eTag", etag.clone());
        }
        if let Some(last_modified) = res.properties.get("lastModified") {
            response.insert_field("lastModified", last_modified.clone());
        }
        response.insert_field("version", string_value(BLOB_API_VERSION));
        Ok(response)
    }

    async fn breakLease(
        &self,
        options: ContainerBreakLeaseOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ContainerBreakLeaseResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let res = self
            .base
            .metadataStore
            .breakContainerLease(
                &context,
                &blobCtx.account().unwrap_or_default(),
                &blobCtx.container().unwrap_or_default(),
                get_i64(&options, "breakPeriod"),
                Some(&options),
            )
            .await?;
        let mut response = GeneratedResponse::new(202);
        response.insert_field(
            "requestId",
            string_value(context.contextId().unwrap_or_default()),
        );
        if let Some(client_request_id) = get_string(&options, "requestId") {
            response.insert_field("clientRequestId", string_value(client_request_id));
        }
        if let Some(start_time) = context.startTime() {
            response.insert_field("date", json_value(start_time));
        }
        if let Some(etag) = res.properties.get("etag") {
            response.insert_field("eTag", etag.clone());
        }
        if let Some(last_modified) = res.properties.get("lastModified") {
            response.insert_field("lastModified", last_modified.clone());
        }
        if let Some(leaseTime) = res.leaseTime {
            response.insert_field("leaseTime", json_value(leaseTime));
        }
        response.insert_field("version", string_value(BLOB_API_VERSION));
        Ok(response)
    }

    async fn changeLease(
        &self,
        leaseId: String,
        proposedLeaseId: String,
        options: ContainerChangeLeaseOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ContainerChangeLeaseResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let res = self
            .base
            .metadataStore
            .changeContainerLease(
                &context,
                &blobCtx.account().unwrap_or_default(),
                &blobCtx.container().unwrap_or_default(),
                &leaseId,
                &proposedLeaseId,
                Some(&options),
            )
            .await?;
        let mut response = GeneratedResponse::new(200);
        response.insert_field(
            "requestId",
            string_value(context.contextId().unwrap_or_default()),
        );
        if let Some(client_request_id) = get_string(&options, "requestId") {
            response.insert_field("clientRequestId", string_value(client_request_id));
        }
        if let Some(start_time) = context.startTime() {
            response.insert_field("date", json_value(start_time));
        }
        if let Some(etag) = res.properties.get("etag") {
            response.insert_field("eTag", etag.clone());
        }
        if let Some(last_modified) = res.properties.get("lastModified") {
            response.insert_field("lastModified", last_modified.clone());
        }
        if let Some(leaseId) = res.leaseId {
            response.insert_field("leaseId", string_value(leaseId));
        }
        response.insert_field("version", string_value(BLOB_API_VERSION));
        Ok(response)
    }

    async fn listBlobFlatSegment(
        &self,
        options: crate::generated::artifacts::models::ContainerListBlobFlatSegmentOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<
        crate::generated::artifacts::models::ContainerListBlobFlatSegmentResponse,
    > {
        list_blobs(self, None, options, context).await
    }

    async fn listBlobHierarchySegment(
        &self,
        delimiter: String,
        options: crate::generated::artifacts::models::ContainerListBlobHierarchySegmentOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<
        crate::generated::artifacts::models::ContainerListBlobHierarchySegmentResponse,
    > {
        list_blobs(self, Some(delimiter), options, context).await
    }

    async fn getAccountInfo(
        &self,
        context: Context,
    ) -> crate::generated::GeneratedResult<ContainerGetAccountInfoResponse> {
        let mut response = GeneratedResponse::new(200);
        response.insert_field(
            "requestId",
            string_value(context.contextId().unwrap_or_default()),
        );
        if let Some(client_request_id) = context
            .request()
            .and_then(|request| request.getHeader(HEADER_CLIENT_REQUEST_ID))
        {
            response.insert_field("clientRequestId", string_value(client_request_id));
        }
        response.insert_field("skuName", string_value(EMULATOR_ACCOUNT_SKUNAME));
        response.insert_field("accountKind", string_value(EMULATOR_ACCOUNT_KIND));
        if let Some(start_time) = context.startTime() {
            response.insert_field("date", json_value(start_time));
        }
        response.insert_field("version", string_value(BLOB_API_VERSION));
        Ok(response)
    }
}

async fn list_blobs(
    handler: &ContainerHandler,
    delimiter: Option<String>,
    options: GeneratedObject,
    context: Context,
) -> crate::generated::GeneratedResult<GeneratedResponse> {
    let blobCtx = BlobStorageContext::new(&context);
    let accountName = blobCtx.account().unwrap_or_default();
    let containerName = blobCtx.container().unwrap_or_default();
    handler
        .base
        .metadataStore
        .checkContainerExist(&context, &accountName, &containerName)
        .await?;

    let marker = get_string(&options, "marker");
    let prefix = get_string(&options, "prefix").unwrap_or_default();
    let include = get_string_array(&options, "include");
    let includeSnapshots = include
        .iter()
        .any(|value| value.eq_ignore_ascii_case("snapshots"));
    let includeUncommittedBlobs = include
        .iter()
        .any(|value| value.eq_ignore_ascii_case("uncommittedblobs"));
    let includeTags = include
        .iter()
        .any(|value| value.eq_ignore_ascii_case("tags"));
    let includeMetadata = include
        .iter()
        .any(|value| value.eq_ignore_ascii_case("metadata"));
    let maxresults = get_i64(&options, "maxresults")
        .map(|value| value.min(DEFAULT_LIST_BLOBS_MAX_RESULTS))
        .unwrap_or(DEFAULT_LIST_BLOBS_MAX_RESULTS);

    let (blobItems, blobPrefixes, nextMarker) = handler
        .base
        .metadataStore
        .listBlobs(
            &context,
            &accountName,
            &containerName,
            delimiter.as_deref().filter(|value| !value.is_empty()),
            None,
            Some(prefix.as_str()),
            Some(maxresults),
            marker.as_deref(),
            Some(includeSnapshots),
            Some(includeUncommittedBlobs),
        )
        .await?;

    let serviceEndpoint = context
        .request()
        .map(|request| format!("{}/{}", request.getEndpoint(), accountName))
        .unwrap_or_default();
    let mut response = GeneratedResponse::new(200);
    response.contentType = Some("application/xml".into());
    response.insert_field(
        "requestId",
        string_value(context.contextId().unwrap_or_default()),
    );
    response.insert_field("version", string_value(BLOB_API_VERSION));
    if let Some(start_time) = context.startTime() {
        response.insert_field("date", json_value(start_time));
    }
    response.insert_field("serviceEndpoint", string_value(serviceEndpoint));
    response.insert_field("containerName", string_value(containerName));
    response.insert_field("prefix", string_value(prefix));
    response.insert_field(
        "marker",
        string_value(get_string(&options, "marker").unwrap_or_default()),
    );
    response.insert_field("maxResults", json_value(maxresults));
    if let Some(delimiter) = delimiter.clone() {
        response.insert_field("delimiter", string_value(delimiter));
    }

    let mapped_blobs = blobItems
        .into_iter()
        .map(|item| {
            let mut value = blob_model_to_obj(&item);
            let tag_count = value
                .get("blobTags")
                .and_then(GeneratedValue::as_object)
                .map(|tags| tags.len() as i64)
                .unwrap_or(0);
            if !includeTags {
                value.remove("blobTags");
            }
            if let Some(GeneratedValue::Object(properties)) = value.get_mut("properties") {
                if let Some(etag) = properties.get("etag").and_then(GeneratedValue::as_string) {
                    properties.insert("etag".into(), string_value(etag.trim_matches('"')));
                }
                if includeTags {
                    properties.insert("tagCount".into(), json_value(tag_count));
                }
                let access_tier_inferred = properties
                    .get("accessTierInferred")
                    .and_then(GeneratedValue::as_bool)
                    .unwrap_or(false);
                if !access_tier_inferred {
                    properties.remove("accessTierInferred");
                }
            }
            if !includeMetadata {
                value.remove("metadata");
            }
            if let Some(deleted) = value.get("deleted").and_then(GeneratedValue::as_bool) {
                if !deleted {
                    value.remove("deleted");
                }
            }
            if value
                .get("snapshot")
                .and_then(GeneratedValue::as_string)
                .unwrap_or_default()
                .is_empty()
            {
                value.remove("snapshot");
            }
            GeneratedValue::Object(value)
        })
        .collect::<Vec<_>>();

    let mut segment = GeneratedObject::new();
    segment.insert("blobItems".into(), GeneratedValue::Array(mapped_blobs));
    if delimiter.is_some() {
        segment.insert(
            "blobPrefixes".into(),
            GeneratedValue::Array(
                blobPrefixes
                    .into_iter()
                    .map(|prefix| GeneratedValue::Object(blob_prefix_to_obj(&prefix)))
                    .collect(),
            ),
        );
    }
    response.insert_field("segment", GeneratedValue::Object(segment));
    if let Some(client_request_id) = get_string(&options, "requestId") {
        response.insert_field("clientRequestId", string_value(client_request_id));
    }
    response.insert_field("nextMarker", string_value(nextMarker.unwrap_or_default()));
    Ok(response)
}

fn string_value(value: impl Into<String>) -> GeneratedValue {
    GeneratedValue::String(value.into())
}

fn json_value<T: Serialize>(value: T) -> GeneratedValue {
    GeneratedValue::from(serde_json::to_value(value).unwrap_or(serde_json::Value::Null))
}

fn blob_model_to_obj(item: &crate::persistence::BlobModel) -> GeneratedObject {
    let mut obj: GeneratedObject = Default::default();
    if let Some(name) = &item.name {
        obj.insert("name".to_string(), GeneratedValue::String(name.clone()));
    }
    if let Some(snapshot) = &item.snapshot {
        obj.insert(
            "snapshot".to_string(),
            GeneratedValue::String(snapshot.clone()),
        );
    }
    if let Some(deleted) = item.deleted {
        obj.insert("deleted".to_string(), GeneratedValue::Bool(deleted));
    }
    obj.insert(
        "properties".to_string(),
        GeneratedValue::Object(item.properties.clone()),
    );
    if let Some(metadata) = &item.metadata {
        obj.insert(
            "metadata".to_string(),
            GeneratedValue::Object(metadata.clone()),
        );
    }
    if let Some(blob_tags) = &item.blobTags {
        obj.insert(
            "blobTags".to_string(),
            GeneratedValue::Object(blob_tags.clone()),
        );
    }
    obj
}

fn blob_prefix_to_obj(prefix: &crate::persistence::BlobPrefixModel) -> GeneratedObject {
    let mut obj = GeneratedObject::new();
    obj.insert(
        "name".to_string(),
        GeneratedValue::String(prefix.name.clone()),
    );
    obj
}

fn filter_blob_to_obj(item: &crate::persistence::FilterBlobModel) -> GeneratedObject {
    let mut obj = GeneratedObject::new();
    obj.insert(
        "name".to_string(),
        GeneratedValue::String(item.name.clone()),
    );
    obj.insert(
        "containerName".to_string(),
        GeneratedValue::String(item.containerName.clone()),
    );
    if let Some(tags) = &item.tags {
        obj.insert("tags".to_string(), GeneratedValue::Object(tags.clone()));
    }
    obj
}

fn get_string(map: &GeneratedObject, key: &str) -> Option<String> {
    map.get(key).and_then(GeneratedValue::as_string)
}

fn get_i64(map: &GeneratedObject, key: &str) -> Option<i64> {
    map.get(key)
        .and_then(GeneratedValue::as_number)
        .map(|value| value as i64)
}

fn get_string_array(map: &GeneratedObject, key: &str) -> Vec<String> {
    match map.get(key) {
        Some(GeneratedValue::Array(values)) => values
            .iter()
            .filter_map(GeneratedValue::as_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn get_object(map: &GeneratedObject, key: &str) -> Option<GeneratedObject> {
    map.get(key).and_then(GeneratedValue::as_object).cloned()
}

fn get_signed_identifier_array(map: &GeneratedObject, key: &str) -> Option<Vec<SignedIdentifier>> {
    match map.get(key) {
        Some(GeneratedValue::Array(values)) => Some(
            values
                .iter()
                .filter_map(GeneratedValue::as_object)
                .cloned()
                .collect(),
        ),
        _ => None,
    }
}
