use async_trait::async_trait;
use azurite_common::utils::utils::formatRfc1123;
use serde_json::Value;

use crate::context::TableStorageContext;
use crate::errors::StorageErrorFactory;
use crate::generated::artifacts::models::{
    GeneratedObject, GeneratedResponse, GeneratedValue, ServiceGetPropertiesOptionalParams,
    ServiceGetPropertiesResponse, ServiceGetStatisticsOptionalParams, ServiceGetStatisticsResponse,
    ServiceSetPropertiesOptionalParams, ServiceSetPropertiesResponse, StorageServiceProperties,
};
use crate::generated::context::Context;
use crate::generated::handlers::i_service_handler::IServiceHandler;
use crate::generated::i_request::IRequest;
use crate::generated::utils::xml::parseXML;
use crate::persistence::ServicePropertiesModel;
use crate::utils::constants::TABLE_API_VERSION;

use super::base_handler::{get_string, rename_key, string_value, BaseHandler};

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
            GeneratedObject::from([(String::from("Enabled"), GeneratedValue::Bool(false))]);
        let hour_metrics = GeneratedObject::from([
            (String::from("Enabled"), GeneratedValue::Bool(false)),
            (
                String::from("RetentionPolicy"),
                GeneratedValue::Object(retention_policy.clone()),
            ),
            (String::from("Version"), string_value("1.0")),
        ]);
        let logging = GeneratedObject::from([
            (String::from("Delete"), GeneratedValue::Bool(true)),
            (String::from("Read"), GeneratedValue::Bool(true)),
            (
                String::from("RetentionPolicy"),
                GeneratedValue::Object(retention_policy.clone()),
            ),
            (String::from("Version"), string_value("1.0")),
            (String::from("Write"), GeneratedValue::Bool(true)),
        ]);
        let minute_metrics = GeneratedObject::from([
            (String::from("Enabled"), GeneratedValue::Bool(false)),
            (
                String::from("RetentionPolicy"),
                GeneratedValue::Object(retention_policy),
            ),
            (String::from("Version"), string_value("1.0")),
        ]);

        GeneratedObject::from([
            (String::from("cors"), GeneratedValue::Array(Vec::new())),
            (
                String::from("defaultServiceVersion"),
                string_value(TABLE_API_VERSION),
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
    ) -> Result<ServiceSetPropertiesResponse, crate::errors::StorageError> {
        let table_ctx = TableStorageContext::new(&context);
        let account_name = table_ctx.account().unwrap_or_default();

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
                    // Keep PascalCase keys for XML output
                    rule.entry(String::from("AllowedHeaders"))
                        .or_insert_with(|| string_value(""));
                    rule.entry(String::from("ExposedHeaders"))
                        .or_insert_with(|| string_value(""));
                }
            }
        }

        self.base
            .metadataStore
            .setServiceProperties(
                &context,
                ServicePropertiesModel {
                    accountName: account_name,
                    properties: storageServiceProperties,
                },
            )
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
    ) -> Result<ServiceGetPropertiesResponse, crate::errors::StorageError> {
        let table_ctx = TableStorageContext::new(&context);
        let account_name = table_ctx.account().unwrap_or_default();

        let mut properties = self
            .base
            .metadataStore
            .getServiceProperties(&context, &account_name)
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
    ) -> Result<ServiceGetStatisticsResponse, crate::errors::StorageError> {
        let table_ctx = TableStorageContext::new(&context);
        if table_ctx.isSecondary() != Some(true) {
            return Err(StorageErrorFactory::getInvalidQueryParameterValue(
                &context, None,
            ));
        }

        let mut geo_replication = GeneratedObject::new();
        geo_replication.insert(String::from("Status"), string_value("live"));
        geo_replication.insert(
            String::from("LastSyncTime"),
            string_value(formatRfc1123(BaseHandler::start_time(&context))),
        );

        let mut response = GeneratedResponse::new(200);
        response.insert_field("geoReplication", GeneratedValue::Object(geo_replication));
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        Ok(response)
    }
}

fn normalize_service_properties(properties: &mut GeneratedObject) {
    // Only rename top-level keys to camelCase — these match the response spec's
    // modelProperties which the XML serializer uses to map back to PascalCase.
    // Inner keys stay PascalCase since the spec doesn't have nested modelProperties.
    rename_key(properties, "Logging", "logging");
    rename_key(properties, "HourMetrics", "hourMetrics");
    rename_key(properties, "MinuteMetrics", "minuteMetrics");
    rename_key(properties, "Cors", "cors");
    rename_key(properties, "DefaultServiceVersion", "defaultServiceVersion");

    // Coerce types in nested objects (keep PascalCase keys)
    if let Some(GeneratedValue::Object(logging)) = properties.get_mut("logging") {
        coerce_bool(logging, "Delete");
        coerce_bool(logging, "Read");
        coerce_bool(logging, "Write");
        if let Some(GeneratedValue::Object(retention)) = logging.get_mut("RetentionPolicy") {
            coerce_bool(retention, "Enabled");
            coerce_int(retention, "Days");
        }
    }
    if let Some(GeneratedValue::Object(metrics)) = properties.get_mut("hourMetrics") {
        normalize_metrics(metrics);
    }
    if let Some(GeneratedValue::Object(metrics)) = properties.get_mut("minuteMetrics") {
        normalize_metrics(metrics);
    }
    if let Some(GeneratedValue::Array(cors_rules)) = properties.get_mut("cors") {
        for rule in cors_rules {
            if let GeneratedValue::Object(rule) = rule {
                coerce_int(rule, "MaxAgeInSeconds");
            }
        }
    }
    // Also handle the XML wrapper form: {"cors": {"CorsRule": [...]}}
    if let Some(GeneratedValue::Object(cors_obj)) = properties.get("cors") {
        if let Some(GeneratedValue::Array(rules)) = cors_obj
            .get("CorsRule")
            .or_else(|| cors_obj.get("corsRule"))
        {
            for rule in rules {
                if let GeneratedValue::Object(_) = rule {
                    // coerce_int on CorsRule entries if nested
                    // handled below after flattening for middleware
                }
            }
        }
    }

    if get_string(properties, "defaultServiceVersion").is_none() {
        properties.insert(
            String::from("defaultServiceVersion"),
            string_value(TABLE_API_VERSION),
        );
    }
}

fn normalize_metrics(metrics: &mut GeneratedObject) {
    // Keep PascalCase keys — they'll pass through XML serializer correctly
    coerce_bool(metrics, "Enabled");
    coerce_bool(metrics, "IncludeAPIs");
    if let Some(GeneratedValue::Object(retention)) = metrics.get_mut("RetentionPolicy") {
        coerce_bool(retention, "Enabled");
        coerce_int(retention, "Days");
    }
}

fn coerce_bool(obj: &mut GeneratedObject, key: &str) {
    if let Some(GeneratedValue::String(s)) = obj.get(key) {
        let val = s.eq_ignore_ascii_case("true");
        obj.insert(key.to_string(), GeneratedValue::Bool(val));
    }
}

fn coerce_int(obj: &mut GeneratedObject, key: &str) {
    if let Some(GeneratedValue::String(s)) = obj.get(key) {
        if let Ok(n) = s.parse::<f64>() {
            obj.insert(key.to_string(), GeneratedValue::Number(n));
        }
    }
}
