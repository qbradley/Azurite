use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::Mutex;

use azurite_common::persistence::i_extent_store::IExtentStore;

use crate::generated::artifacts::models::{GeneratedObject, GeneratedResponse, GeneratedValue};
use crate::generated::context::Context;
use crate::generated::utils::i_logger::ILogger;
use crate::persistence::IQueueMetadataStore;
use crate::utils::constants::QUEUE_API_VERSION;
use crate::utils::utils::{parseXMLwithEmpty, ParseXMLwithEmptyError};

pub type SharedExtentStore = Arc<Mutex<Box<dyn IExtentStore + Send + Sync>>>;
pub type SharedQueueMetadataStore = Arc<dyn IQueueMetadataStore + Send + Sync>;
pub type SharedLogger = Arc<dyn ILogger + Send + Sync>;

#[allow(non_snake_case)]
#[derive(Clone)]
pub struct BaseHandler {
    pub metadataStore: SharedQueueMetadataStore,
    pub extentStore: SharedExtentStore,
    pub logger: SharedLogger,
}

impl BaseHandler {
    pub fn new(
        metadataStore: SharedQueueMetadataStore,
        extentStore: SharedExtentStore,
        logger: SharedLogger,
    ) -> Self {
        Self {
            metadataStore,
            extentStore,
            logger,
        }
    }

    pub fn start_time(context: &Context) -> DateTime<Utc> {
        context.startTime().unwrap_or_else(Utc::now)
    }

    pub fn request_id(context: &Context) -> String {
        context.contextId().unwrap_or_default()
    }

    pub fn add_response_metadata(
        &self,
        response: &mut GeneratedResponse,
        options: &GeneratedObject,
        context: &Context,
        include_date: bool,
    ) {
        response.insert_field("requestId", string_value(Self::request_id(context)));
        response.insert_field("version", string_value(QUEUE_API_VERSION));
        if include_date {
            response.insert_field("date", json_value(Self::start_time(context)));
        }
        if let Some(client_request_id) = get_string(options, "requestId") {
            response.insert_field("clientRequestId", string_value(client_request_id));
        }
    }
}

pub(crate) fn string_value(value: impl Into<String>) -> GeneratedValue {
    GeneratedValue::String(value.into())
}

pub(crate) fn rfc1123_value(dt: chrono::DateTime<chrono::Utc>) -> GeneratedValue {
    GeneratedValue::String(dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string())
}

pub(crate) fn json_value<T: serde::Serialize>(value: T) -> GeneratedValue {
    GeneratedValue::from(serde_json::to_value(value).unwrap_or(serde_json::Value::Null))
}

pub(crate) fn get_string(map: &GeneratedObject, key: &str) -> Option<String> {
    map.get(key).and_then(GeneratedValue::as_string)
}

pub(crate) fn get_i32(map: &GeneratedObject, key: &str) -> Option<i32> {
    map.get(key)
        .and_then(GeneratedValue::as_number)
        .map(|value| value as i32)
}

pub(crate) fn get_string_array(map: &GeneratedObject, key: &str) -> Vec<String> {
    match map.get(key) {
        Some(GeneratedValue::Array(values)) => values
            .iter()
            .filter_map(GeneratedValue::as_string)
            .collect(),
        _ => Vec::new(),
    }
}

pub(crate) fn get_optional_object_array(
    map: &GeneratedObject,
    key: &str,
) -> Option<Vec<GeneratedObject>> {
    match map.get(key) {
        None | Some(GeneratedValue::Null) => None,
        Some(GeneratedValue::Array(values)) => Some(
            values
                .iter()
                .filter_map(|value| value.as_object().cloned())
                .collect(),
        ),
        Some(GeneratedValue::Object(value)) => Some(vec![value.clone()]),
        _ => Some(Vec::new()),
    }
}

pub(crate) fn metadata_to_object(metadata: &BTreeMap<String, String>) -> GeneratedObject {
    metadata
        .iter()
        .map(|(key, value)| (key.clone(), string_value(value.clone())))
        .collect()
}

pub(crate) fn extract_message_text(
    queue_message: &GeneratedObject,
    raw_body: Option<&str>,
) -> Result<Option<String>, ParseXMLwithEmptyError> {
    if let Some(text) = get_string(queue_message, "messageText")
        .or_else(|| get_string(queue_message, "MessageText"))
    {
        return Ok(Some(text));
    }

    let Some(raw_body) = raw_body else {
        return Ok(None);
    };

    let parsed_body = parseXMLwithEmpty(raw_body, false)?;
    let Some(values) = parsed_body.as_object() else {
        return Ok(None);
    };

    for (key, value) in values {
        if key.eq_ignore_ascii_case("messagetext") {
            return Ok(Some(value.as_str().unwrap_or_default().to_string()));
        }
    }

    Ok(None)
}
