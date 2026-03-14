use std::sync::Arc;

use async_trait::async_trait;
use azurite_common::i_account_data_store::IAccountDataStore;

use crate::context::blob_storage_context::BlobStorageContext;
use crate::errors::StorageErrorFactory;
use crate::generated::artifacts::models::{
    GeneratedBody, GeneratedObject, GeneratedResponse, GeneratedValue, KeyInfo,
    ServiceFilterBlobsOptionalParams, ServiceFilterBlobsResponse, ServiceGetAccountInfoResponse,
    ServiceGetPropertiesOptionalParams, ServiceGetPropertiesResponse,
    ServiceGetStatisticsOptionalParams, ServiceGetStatisticsResponse,
    ServiceGetUserDelegationKeyOptionalParams, ServiceGetUserDelegationKeyResponse,
    ServiceListContainersSegmentOptionalParams, ServiceListContainersSegmentResponse,
    ServiceSetPropertiesOptionalParams, ServiceSetPropertiesResponse,
    ServiceSubmitBatchOptionalParams, ServiceSubmitBatchResponse, StorageServiceProperties,
};
use crate::generated::context::Context;
use crate::generated::handlers::i_service_handler::IServiceHandler;
use crate::generated::i_request::{GeneratedReadableStream, IRequest};
use crate::generated::utils::xml::parseXML;
use crate::persistence::ServicePropertiesModel;

use super::base_handler::BaseHandler;
use super::batch_handlers_bundle::{
    create_batch_request, create_blob_batch_handler, parse_batch_boundary,
};
use super::container_handler::ContainerHandler;

const BLOB_API_VERSION: &str = "2025-11-05";
const DEFAULT_LIST_BLOBS_MAX_RESULTS: i64 = 5000;
const DEFAULT_LIST_CONTAINERS_MAX_RESULTS: i64 = 5000;
const EMULATOR_ACCOUNT_SKUNAME: &str = "Standard_RAGRS";
const EMULATOR_ACCOUNT_KIND: &str = "StorageV2";
const EMULATOR_ACCOUNT_ISHIERARCHICALNAMESPACEENABLED: bool = false;
const HEADER_AUTHORIZATION: &str = "authorization";
const HEADER_CLIENT_REQUEST_ID: &str = "x-ms-client-request-id";
const BEARER_PREFIX: &str = "Bearer ";

#[derive(Clone)]
pub struct ServiceHandler {
    pub base: BaseHandler,
    pub accountDataStore: Arc<dyn IAccountDataStore + Send + Sync>,
    pub oauth: Option<String>,
    pub disableProductStyle: Option<bool>,
}

impl ServiceHandler {
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

    fn default_service_properties() -> StorageServiceProperties {
        let mut retention = GeneratedObject::new();
        retention.insert("enabled".into(), GeneratedValue::Bool(false));

        let mut hour_metrics = GeneratedObject::new();
        hour_metrics.insert("enabled".into(), GeneratedValue::Bool(false));
        hour_metrics.insert(
            "retentionPolicy".into(),
            GeneratedValue::Object(retention.clone()),
        );
        hour_metrics.insert("version".into(), GeneratedValue::String("1.0".into()));

        let mut logging = GeneratedObject::new();
        logging.insert("deleteProperty".into(), GeneratedValue::Bool(true));
        logging.insert("read".into(), GeneratedValue::Bool(true));
        logging.insert(
            "retentionPolicy".into(),
            GeneratedValue::Object(retention.clone()),
        );
        logging.insert("version".into(), GeneratedValue::String("1.0".into()));
        logging.insert("write".into(), GeneratedValue::Bool(true));

        let mut minute_metrics = GeneratedObject::new();
        minute_metrics.insert("enabled".into(), GeneratedValue::Bool(false));
        minute_metrics.insert("retentionPolicy".into(), GeneratedValue::Object(retention));
        minute_metrics.insert("version".into(), GeneratedValue::String("1.0".into()));

        let mut static_website = GeneratedObject::new();
        static_website.insert("enabled".into(), GeneratedValue::Bool(false));

        let mut properties = GeneratedObject::new();
        properties.insert("cors".into(), GeneratedValue::Array(Vec::new()));
        properties.insert(
            "defaultServiceVersion".into(),
            GeneratedValue::String(BLOB_API_VERSION.into()),
        );
        properties.insert("hourMetrics".into(), GeneratedValue::Object(hour_metrics));
        properties.insert("logging".into(), GeneratedValue::Object(logging));
        properties.insert(
            "minuteMetrics".into(),
            GeneratedValue::Object(minute_metrics),
        );
        properties.insert(
            "staticWebsite".into(),
            GeneratedValue::Object(static_website),
        );
        properties
    }
}

#[async_trait]
impl IServiceHandler for ServiceHandler {
    async fn setProperties(
        &self,
        mut storageServiceProperties: StorageServiceProperties,
        options: ServiceSetPropertiesOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ServiceSetPropertiesResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let accountName = blobCtx.account().unwrap_or_default();

        if let Some(body) = context.request().and_then(|request| request.getBody()) {
            let parsed_body = parseXML(&body, false).unwrap_or(serde_json::Value::Null);
            if !parsed_body.get("cors").is_some() && !parsed_body.get("Cors").is_some() {
                storageServiceProperties.remove("cors");
            }
        }

        if let Some(cors_rules) = storageServiceProperties.get_mut("cors") {
            if let GeneratedValue::Array(rules) = cors_rules {
                for rule in rules {
                    if let GeneratedValue::Object(rule) = rule {
                        rule.entry("allowedHeaders".into())
                            .or_insert_with(|| GeneratedValue::String(String::new()));
                        rule.entry("exposedHeaders".into())
                            .or_insert_with(|| GeneratedValue::String(String::new()));
                    }
                }
            }
        }

        self.base
            .metadataStore
            .setServiceProperties(
                &context,
                ServicePropertiesModel {
                    accountName,
                    properties: storageServiceProperties,
                },
            )
            .await?;

        let mut response = GeneratedResponse::new(202);
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

    async fn getProperties(
        &self,
        options: ServiceGetPropertiesOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ServiceGetPropertiesResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let accountName = blobCtx.account().unwrap_or_default();
        let mut properties = self
            .base
            .metadataStore
            .getServiceProperties(&context, &accountName)
            .await?
            .map(|value| value.properties)
            .unwrap_or_else(Self::default_service_properties);
        let default_properties = Self::default_service_properties();
        for key in [
            "cors",
            "hourMetrics",
            "logging",
            "minuteMetrics",
            "defaultServiceVersion",
            "staticWebsite",
        ] {
            if !properties.contains_key(key) {
                if let Some(value) = default_properties.get(key) {
                    properties.insert(key.to_string(), value.clone());
                }
            }
        }

        let mut response = GeneratedResponse::new(200);
        response.fields.extend(properties);
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

    async fn getStatistics(
        &self,
        options: ServiceGetStatisticsOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ServiceGetStatisticsResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        if blobCtx.isSecondary() != Some(true) {
            return Err(Box::new(
                StorageErrorFactory::getInvalidQueryParameterValue(
                    context.contextId().as_deref(),
                    None,
                    None,
                    None,
                ),
            ));
        }

        let mut geo_replication = GeneratedObject::new();
        geo_replication.insert("status".into(), string_value("live"));
        if let Some(start_time) = context.startTime() {
            geo_replication.insert("lastSyncTime".into(), json_value(start_time));
        }

        let mut response = GeneratedResponse::new(200);
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
        response.insert_field("geoReplication", GeneratedValue::Object(geo_replication));
        Ok(response)
    }

    async fn listContainersSegment(
        &self,
        mut options: ServiceListContainersSegmentOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ServiceListContainersSegmentResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let request = context.request();
        let accountName = blobCtx.account().unwrap_or_default();

        let prefix = get_string(&options, "prefix").unwrap_or_default();
        let marker = get_string(&options, "marker");
        let maxresults =
            get_i64(&options, "maxresults").unwrap_or(DEFAULT_LIST_CONTAINERS_MAX_RESULTS);
        options.insert(
            "maxresults".into(),
            GeneratedValue::Number(maxresults as f64),
        );
        options.insert("prefix".into(), string_value(prefix.clone()));

        let (containers, next_marker) = self
            .base
            .metadataStore
            .listContainers(
                &context,
                &accountName,
                Some(prefix.as_str()),
                Some(maxresults),
                marker.as_deref(),
            )
            .await?;

        let include_metadata = get_string_array(&options, "include")
            .into_iter()
            .any(|item| item.eq_ignore_ascii_case("metadata"));
        let serviceEndpoint = request
            .as_ref()
            .map(|request| format!("{}/{}", request.getEndpoint(), accountName))
            .unwrap_or_default();
        let container_items = containers
            .iter()
            .map(|item| {
                let mut value = container_model_to_obj(item);
                if !include_metadata {
                    value.remove("metadata");
                }
                GeneratedValue::Object(value)
            })
            .collect::<Vec<_>>();

        let mut response = GeneratedResponse::new(200);
        response.insert_field("containerItems", GeneratedValue::Array(container_items));
        response.insert_field("maxResults", json_value(maxresults));
        response.insert_field("nextMarker", string_value(next_marker.unwrap_or_default()));
        response.insert_field("prefix", string_value(prefix));
        response.insert_field("serviceEndpoint", string_value(serviceEndpoint));
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

    async fn getUserDelegationKey(
        &self,
        keyInfo: KeyInfo,
        _options: ServiceGetUserDelegationKeyOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ServiceGetUserDelegationKeyResponse> {
        let request = context.request();
        let token = request
            .as_ref()
            .and_then(|request| request.getHeader(HEADER_AUTHORIZATION))
            .and_then(|value| value.strip_prefix(BEARER_PREFIX).map(str::to_string))
            .unwrap_or_default();
        let claims = decode_jwt_claims(&token);
        let oid = claims
            .get("oid")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let tid = claims
            .get("tid")
            .and_then(|value| value.as_str())
            .unwrap_or_default();

        let mut response = GeneratedResponse::new(200);
        response.insert_field("signedOid", string_value(oid));
        response.insert_field("signedTid", string_value(tid));
        response.insert_field("signedService", string_value("b"));
        response.insert_field("signedVersion", string_value(BLOB_API_VERSION));
        if let Some(start) = keyInfo.get("start") {
            response.insert_field("signedStart", start.clone());
        }
        if let Some(expiry) = keyInfo.get("expiry") {
            response.insert_field("signedExpiry", expiry.clone());
        }
        response.insert_field(
            "value",
            string_value(format!("{}:{}:{}", oid, tid, BLOB_API_VERSION)),
        );
        Ok(response)
    }

    async fn getAccountInfo(
        &self,
        context: Context,
    ) -> crate::generated::GeneratedResult<ServiceGetAccountInfoResponse> {
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
        response.insert_field(
            "isHierarchicalNamespaceEnabled",
            GeneratedValue::Bool(EMULATOR_ACCOUNT_ISHIERARCHICALNAMESPACEENABLED),
        );
        response.insert_field("version", string_value(BLOB_API_VERSION));
        Ok(response)
    }

    /// `ServiceHandler.submitBatch()` — delegates to `BlobBatchHandler`.
    async fn submitBatch(
        &self,
        body: GeneratedReadableStream,
        _contentLength: f64,
        multipartContentType: String,
        options: ServiceSubmitBatchOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ServiceSubmitBatchResponse> {
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
        let container_handler = ContainerHandler::new(
            Arc::clone(&self.accountDataStore),
            self.oauth.clone(),
            self.base.clone(),
            self.disableProductStyle,
        );
        let batch_handler = create_blob_batch_handler(
            self.clone(),
            container_handler,
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
        options: ServiceFilterBlobsOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ServiceFilterBlobsResponse> {
        let blobCtx = BlobStorageContext::new(&context);
        let accountName = blobCtx.account().unwrap_or_default();
        let request = context.request();
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
                None,
                where_clause.as_deref(),
                Some(maxresults),
                marker.as_deref(),
            )
            .await?;

        let serviceEndpoint = request
            .as_ref()
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
}

fn string_value(value: impl Into<String>) -> GeneratedValue {
    GeneratedValue::String(value.into())
}

fn json_value<T: serde::Serialize>(value: T) -> GeneratedValue {
    GeneratedValue::from(serde_json::to_value(value).unwrap_or(serde_json::Value::Null))
}

/// Convert a `ContainerModel` to a flat `GeneratedObject` suitable for response serialization.
/// Spreads `item.properties` and adds `name` and (optionally already present) `metadata`.
fn container_model_to_obj(item: &crate::persistence::ContainerModel) -> GeneratedObject {
    let mut obj = item.properties.clone();
    if let Some(name) = &item.name {
        obj.insert("name".to_string(), GeneratedValue::String(name.clone()));
    }
    if let Some(meta) = &item.metadata {
        let meta_map: GeneratedObject = meta
            .iter()
            .map(|(k, v)| (k.clone(), GeneratedValue::String(v.clone())))
            .collect();
        obj.insert("metadata".to_string(), GeneratedValue::Object(meta_map));
    }
    obj
}

/// Convert a `FilterBlobModel` to a `GeneratedObject` suitable for the filterBlobs response.
fn filter_blob_to_obj(item: &crate::persistence::FilterBlobModel) -> GeneratedObject {
    let mut obj: GeneratedObject = Default::default();
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

fn decode_jwt_claims(token: &str) -> serde_json::Value {
    let payload = token.split('.').nth(1).unwrap_or_default();
    if payload.is_empty() {
        return serde_json::Value::Null;
    }
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or(serde_json::Value::Null)
}
