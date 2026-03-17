use std::collections::BTreeMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::context::queue_storage_context::QueueStorageContext;
use crate::errors::StorageErrorFactory;
use crate::generated::artifacts::models::{
    GeneratedObject, GeneratedResponse, GeneratedValue, ServiceGetPropertiesOptionalParams,
    ServiceGetPropertiesResponse, ServiceGetStatisticsOptionalParams, ServiceGetStatisticsResponse,
    ServiceListQueuesSegmentOptionalParams, ServiceListQueuesSegmentResponse,
    ServiceSetPropertiesOptionalParams, ServiceSetPropertiesResponse, StorageServiceProperties,
};
use crate::generated::context::Context;
use crate::generated::handlers::i_service_handler::IServiceHandler;
use crate::generated::i_request::IRequest;
use crate::generated::utils::xml::parseXML;
use crate::persistence::{QueueModel, ServicePropertiesModel};
use crate::utils::constants::{
    LIST_QUEUE_MAXRESULTS_MAX, LIST_QUEUE_MAXRESULTS_MIN, QUEUE_API_VERSION,
};

use super::base_handler::{
    get_i32, get_string, get_string_array, json_value, string_value, BaseHandler,
};
use azurite_common::utils::utils::formatRfc1123;

const LIST_QUEUES_MAX_RESULTS_DEFAULT: i32 = 5000;

#[derive(Clone)]
pub struct ServiceHandler {
    pub base: BaseHandler,
}

impl ServiceHandler {
    pub fn new(base: BaseHandler) -> Self {
        Self { base }
    }

    fn default_service_properties() -> StorageServiceProperties {
        let retention_policy =
            GeneratedObject::from([(String::from("enabled"), GeneratedValue::Bool(false))]);
        let hour_metrics = GeneratedObject::from([
            (String::from("enabled"), GeneratedValue::Bool(false)),
            (
                String::from("retentionPolicy"),
                GeneratedValue::Object(retention_policy.clone()),
            ),
            (String::from("version"), string_value("1.0")),
        ]);
        let logging = GeneratedObject::from([
            (String::from("deleteProperty"), GeneratedValue::Bool(true)),
            (String::from("read"), GeneratedValue::Bool(true)),
            (
                String::from("retentionPolicy"),
                GeneratedValue::Object(retention_policy.clone()),
            ),
            (String::from("version"), string_value("1.0")),
            (String::from("write"), GeneratedValue::Bool(true)),
        ]);
        let minute_metrics = GeneratedObject::from([
            (String::from("enabled"), GeneratedValue::Bool(false)),
            (
                String::from("retentionPolicy"),
                GeneratedValue::Object(retention_policy),
            ),
            (String::from("version"), string_value("1.0")),
        ]);
        let static_website =
            GeneratedObject::from([(String::from("enabled"), GeneratedValue::Bool(false))]);

        GeneratedObject::from([
            (String::from("cors"), GeneratedValue::Array(Vec::new())),
            (
                String::from("defaultServiceVersion"),
                string_value(QUEUE_API_VERSION),
            ),
            (
                String::from("hourMetrics"),
                GeneratedValue::Object(hour_metrics),
            ),
            (String::from("logging"), GeneratedValue::Object(logging)),
            (
                String::from("minuteMetrics"),
                GeneratedValue::Object(minute_metrics),
            ),
            (
                String::from("staticWebsite"),
                GeneratedValue::Object(static_website),
            ),
        ])
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
        let queue_ctx = QueueStorageContext::new(&context);
        let account_name = queue_ctx.account().unwrap_or_default();

        normalize_service_properties(&mut storageServiceProperties);

        if let Some(body) = context.request().and_then(|request| request.getBody()) {
            let parsed_body = parseXML(&body, false).unwrap_or(Value::Null);
            if parsed_body.get("cors").is_none() && parsed_body.get("Cors").is_none() {
                storageServiceProperties.shift_remove("cors");
                storageServiceProperties.shift_remove("Cors");
            }
        }

        if let Some(GeneratedValue::Array(cors_rules)) = storageServiceProperties.get_mut("cors") {
            for rule in cors_rules {
                if let GeneratedValue::Object(rule) = rule {
                    normalize_cors_rule(rule);
                    rule.entry(String::from("allowedHeaders"))
                        .or_insert_with(|| string_value(""));
                    rule.entry(String::from("exposedHeaders"))
                        .or_insert_with(|| string_value(""));
                }
            }
        }

        self.base
            .metadataStore
            .updateServiceProperties(ServicePropertiesModel {
                accountName: account_name,
                properties: storageServiceProperties,
            })
            .await?;

        let mut response = GeneratedResponse::new(202);
        self.base
            .add_response_metadata(&mut response, &options, &context, false);
        Ok(response)
    }

    async fn getProperties(
        &self,
        options: ServiceGetPropertiesOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ServiceGetPropertiesResponse> {
        let queue_ctx = QueueStorageContext::new(&context);
        let account_name = queue_ctx.account().unwrap_or_default();

        let mut properties = self
            .base
            .metadataStore
            .getServiceProperties(&account_name)
            .await?
            .map(|properties| properties.properties)
            .unwrap_or_else(Self::default_service_properties);
        normalize_service_properties(&mut properties);

        let defaults = Self::default_service_properties();
        for key in [
            "cors",
            "hourMetrics",
            "logging",
            "minuteMetrics",
            "defaultServiceVersion",
            "staticWebsite",
        ] {
            if !properties.contains_key(key) {
                if let Some(value) = defaults.get(key) {
                    properties.insert(key.to_string(), value.clone());
                }
            }
        }

        let mut response = GeneratedResponse::new(200);
        response.fields.extend(properties);
        self.base
            .add_response_metadata(&mut response, &options, &context, false);
        Ok(response)
    }

    async fn getStatistics(
        &self,
        options: ServiceGetStatisticsOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ServiceGetStatisticsResponse> {
        let queue_ctx = QueueStorageContext::new(&context);
        if queue_ctx.isSecondary() != Some(true) {
            return Err(Box::new(
                StorageErrorFactory::getInvalidQueryParameterValue(
                    context.contextId().as_deref(),
                    None,
                ),
            ));
        }

        let mut geo_replication = GeneratedObject::new();
        geo_replication.insert(String::from("status"), string_value("live"));
        geo_replication.insert(
            String::from("lastSyncTime"),
            json_value(formatRfc1123(BaseHandler::start_time(&context))),
        );

        let mut response = GeneratedResponse::new(200);
        response.insert_field("geoReplication", GeneratedValue::Object(geo_replication));
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        Ok(response)
    }

    async fn listQueuesSegment(
        &self,
        options: ServiceListQueuesSegmentOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<ServiceListQueuesSegmentResponse> {
        let queue_ctx = QueueStorageContext::new(&context);
        let account_name = queue_ctx.account().unwrap_or_default();
        let request = context.request();

        let mut max_results = LIST_QUEUES_MAX_RESULTS_DEFAULT;
        if let Some(value) = get_i32(&options, "maxresults") {
            if !(LIST_QUEUE_MAXRESULTS_MIN..=LIST_QUEUE_MAXRESULTS_MAX).contains(&value) {
                let mut details = BTreeMap::new();
                details.insert(
                    String::from("QueryParameterName"),
                    String::from("maxresults"),
                );
                details.insert(String::from("QueryParameterValue"), value.to_string());
                details.insert(
                    String::from("MinimumAllowed"),
                    LIST_QUEUE_MAXRESULTS_MIN.to_string(),
                );
                details.insert(
                    String::from("MaximumAllowed"),
                    LIST_QUEUE_MAXRESULTS_MAX.to_string(),
                );
                return Err(Box::new(
                    StorageErrorFactory::getOutOfRangeQueryParameterValue(
                        context.contextId().as_deref(),
                        Some(details),
                    ),
                ));
            }
            max_results = value;
        }

        let prefix = get_string(&options, "prefix").unwrap_or_default();
        let marker = get_string(&options, "marker")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);

        let (queues, next_marker) = self
            .base
            .metadataStore
            .listQueues(
                &account_name,
                Some(prefix.as_str()),
                Some(max_results as u64),
                Some(marker),
            )
            .await?;

        let include_metadata = get_string_array(&options, "include")
            .iter()
            .any(|item| item.eq_ignore_ascii_case("metadata"));
        let service_endpoint = request
            .as_ref()
            .map(|request| request_service_endpoint(request, &account_name))
            .unwrap_or_default();
        let queue_items = queues
            .iter()
            .map(|queue| GeneratedValue::Object(queue_model_to_object(queue, include_metadata)))
            .collect();

        let mut response = GeneratedResponse::new(200);
        response.insert_field("queueItems", GeneratedValue::Array(queue_items));
        response.insert_field("maxResults", GeneratedValue::Number(max_results as f64));
        response.insert_field(
            "nextMarker",
            string_value(
                next_marker
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ),
        );
        response.insert_field("prefix", string_value(prefix));
        response.insert_field("serviceEndpoint", string_value(service_endpoint));
        self.base
            .add_response_metadata(&mut response, &options, &context, false);
        Ok(response)
    }
}

fn request_service_endpoint(
    request: &crate::generated::i_request::GeneratedHttpRequest,
    account_name: &str,
) -> String {
    let base_endpoint = request
        .getHeader("host")
        .filter(|host| !host.is_empty())
        .map(|host| format!("{}://{}", request.getProtocol(), host))
        .unwrap_or_else(|| request.getEndpoint());
    format!("{}/{}", base_endpoint, account_name)
}

fn queue_model_to_object(queue: &QueueModel, include_metadata: bool) -> GeneratedObject {
    let mut value = queue.properties.clone();
    value.insert(String::from("name"), string_value(queue.name.clone()));
    if include_metadata {
        if let Some(metadata) = &queue.metadata {
            value.insert(
                String::from("metadata"),
                GeneratedValue::Object(
                    metadata
                        .iter()
                        .map(|(key, value)| (key.clone(), string_value(value.clone())))
                        .collect(),
                ),
            );
        }
    }
    value
}

fn normalize_service_properties(properties: &mut GeneratedObject) {
    rename_key(properties, "Logging", "logging");
    rename_key(properties, "HourMetrics", "hourMetrics");
    rename_key(properties, "MinuteMetrics", "minuteMetrics");
    rename_key(properties, "Cors", "cors");
    rename_key(properties, "DefaultServiceVersion", "defaultServiceVersion");
    rename_key(properties, "StaticWebsite", "staticWebsite");

    if let Some(GeneratedValue::Object(logging)) = properties.get_mut("logging") {
        normalize_logging(logging);
    }
    if let Some(GeneratedValue::Object(metrics)) = properties.get_mut("hourMetrics") {
        normalize_metrics(metrics);
    }
    if let Some(GeneratedValue::Object(metrics)) = properties.get_mut("minuteMetrics") {
        normalize_metrics(metrics);
    }

    normalize_cors_property(properties);

    if let Some(GeneratedValue::Array(cors_rules)) = properties.get_mut("cors") {
        for rule in cors_rules {
            if let GeneratedValue::Object(rule) = rule {
                normalize_cors_rule(rule);
            }
        }
    }
}

fn normalize_logging(logging: &mut GeneratedObject) {
    rename_key(logging, "Version", "version");
    rename_key(logging, "Delete", "deleteProperty");
    rename_key(logging, "Read", "read");
    rename_key(logging, "Write", "write");
    rename_key(logging, "RetentionPolicy", "retentionPolicy");

    if let Some(GeneratedValue::Object(retention_policy)) = logging.get_mut("retentionPolicy") {
        normalize_retention_policy(retention_policy);
    }
}

fn normalize_metrics(metrics: &mut GeneratedObject) {
    rename_key(metrics, "Version", "version");
    rename_key(metrics, "Enabled", "enabled");
    rename_key(metrics, "IncludeAPIs", "includeAPIs");
    rename_key(metrics, "RetentionPolicy", "retentionPolicy");

    if let Some(GeneratedValue::Object(retention_policy)) = metrics.get_mut("retentionPolicy") {
        normalize_retention_policy(retention_policy);
    }
}

fn normalize_cors_rule(rule: &mut GeneratedObject) {
    rename_key(rule, "AllowedOrigins", "allowedOrigins");
    rename_key(rule, "AllowedMethods", "allowedMethods");
    rename_key(rule, "AllowedHeaders", "allowedHeaders");
    rename_key(rule, "ExposedHeaders", "exposedHeaders");
    rename_key(rule, "MaxAgeInSeconds", "maxAgeInSeconds");
}

/// Unwrap the XML-deserialized CORS wrapper into a flat array.
fn normalize_cors_property(properties: &mut GeneratedObject) {
    let cors_val = match properties.shift_remove("cors") {
        Some(v) => v,
        None => return,
    };

    let rules = match cors_val {
        GeneratedValue::Array(_) => cors_val,
        GeneratedValue::Object(mut obj) => {
            let inner = obj
                .shift_remove("CorsRule")
                .or_else(|| obj.shift_remove("corsRule"))
                .unwrap_or(GeneratedValue::Array(Vec::new()));
            match inner {
                GeneratedValue::Array(arr) => GeneratedValue::Array(arr),
                GeneratedValue::Object(_) => GeneratedValue::Array(vec![inner]),
                _ => GeneratedValue::Array(Vec::new()),
            }
        }
        GeneratedValue::String(s) if s.is_empty() => GeneratedValue::Array(Vec::new()),
        _ => GeneratedValue::Array(Vec::new()),
    };

    properties.insert("cors".to_string(), rules);
}

fn normalize_retention_policy(retention_policy: &mut GeneratedObject) {
    rename_key(retention_policy, "Enabled", "enabled");
    rename_key(retention_policy, "Days", "days");
}

fn rename_key(object: &mut GeneratedObject, from: &str, to: &str) {
    if object.contains_key(to) {
        return;
    }
    if let Some(value) = object.shift_remove(from) {
        object.insert(to.to_string(), value);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::generated::i_request::{GeneratedHttpRequest, HttpMethod, RequestHeaderValue};

    use super::request_service_endpoint;

    #[test]
    fn request_service_endpoint_prefers_host_header_with_port() {
        let mut request = GeneratedHttpRequest::new(
            HttpMethod::GET,
            "/devstoreaccount1?comp=list",
            "http://127.0.0.1",
            "/devstoreaccount1",
        );
        request.headers = BTreeMap::from([(
            "host".to_string(),
            RequestHeaderValue::Single("127.0.0.1:11015".to_string()),
        )]);

        assert_eq!(
            request_service_endpoint(&request, "devstoreaccount1"),
            "http://127.0.0.1:11015/devstoreaccount1"
        );
    }
}
