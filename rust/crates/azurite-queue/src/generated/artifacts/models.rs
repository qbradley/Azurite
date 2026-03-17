use std::collections::BTreeMap;

use indexmap::IndexMap;

use crate::generated::i_request::GeneratedReadableStream;
use crate::generated::i_response::ResponseHeaderValue;

#[derive(Debug, Clone)]
pub enum GeneratedValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<GeneratedValue>),
    Object(GeneratedObject),
    Stream(GeneratedReadableStream),
}

pub type GeneratedObject = IndexMap<String, GeneratedValue>;

#[derive(Debug, Clone)]
pub enum GeneratedBody {
    Text(String),
    Value(GeneratedValue),
    Stream(GeneratedReadableStream),
}

#[derive(Debug, Clone, Default)]
pub struct GeneratedResponse {
    pub statusCode: u16,
    pub statusMessage: Option<String>,
    pub headers: BTreeMap<String, ResponseHeaderValue>,
    pub contentType: Option<String>,
    pub body: Option<GeneratedBody>,
    pub fields: GeneratedObject,
}

impl Default for GeneratedValue {
    fn default() -> Self {
        Self::Null
    }
}

impl GeneratedValue {
    pub fn as_object(&self) -> Option<&GeneratedObject> {
        match self {
            GeneratedValue::Object(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<String> {
        match self {
            GeneratedValue::String(value) => Some(value.clone()),
            GeneratedValue::Number(value) => Some(value.to_string()),
            GeneratedValue::Bool(value) => Some(value.to_string()),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            GeneratedValue::Number(value) => Some(*value),
            GeneratedValue::String(value) => value.parse().ok(),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            GeneratedValue::Bool(value) => Some(*value),
            GeneratedValue::String(value) => value.parse().ok(),
            _ => None,
        }
    }

    pub fn as_stream(&self) -> Option<GeneratedReadableStream> {
        match self {
            GeneratedValue::Stream(stream) => Some(stream.clone()),
            _ => None,
        }
    }

    pub fn to_json_value(&self) -> serde_json::Value {
        match self {
            GeneratedValue::Null => serde_json::Value::Null,
            GeneratedValue::Bool(value) => serde_json::Value::Bool(*value),
            GeneratedValue::Number(value) => serde_json::Number::from_f64(*value)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            GeneratedValue::String(value) => serde_json::Value::String(value.clone()),
            GeneratedValue::Array(values) => {
                serde_json::Value::Array(values.iter().map(GeneratedValue::to_json_value).collect())
            }
            GeneratedValue::Object(values) => serde_json::Value::Object(
                values
                    .iter()
                    .map(|(key, value)| (key.clone(), value.to_json_value()))
                    .collect(),
            ),
            GeneratedValue::Stream(stream) => serde_json::Value::String(stream.read_to_string()),
        }
    }
}

impl From<serde_json::Value> for GeneratedValue {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => GeneratedValue::Null,
            serde_json::Value::Bool(value) => GeneratedValue::Bool(value),
            serde_json::Value::Number(value) => {
                GeneratedValue::Number(value.as_f64().unwrap_or_default())
            }
            serde_json::Value::String(value) => GeneratedValue::String(value),
            serde_json::Value::Array(values) => {
                GeneratedValue::Array(values.into_iter().map(GeneratedValue::from).collect())
            }
            serde_json::Value::Object(values) => GeneratedValue::Object(
                values
                    .into_iter()
                    .map(|(key, value)| (key, GeneratedValue::from(value)))
                    .collect(),
            ),
        }
    }
}

impl GeneratedResponse {
    pub fn new(statusCode: u16) -> Self {
        Self {
            statusCode,
            ..Self::default()
        }
    }

    pub fn insert_field(&mut self, key: impl Into<String>, value: GeneratedValue) {
        self.fields.insert(key.into(), value);
    }

    pub fn get_field(&self, key: &str) -> Option<&GeneratedValue> {
        self.fields.get(key)
    }

    pub fn body_value(&self) -> GeneratedValue {
        if let Some(body) = &self.body {
            match body {
                GeneratedBody::Text(value) => GeneratedValue::String(value.clone()),
                GeneratedBody::Value(value) => value.clone(),
                GeneratedBody::Stream(stream) => GeneratedValue::Stream(stream.clone()),
            }
        } else {
            GeneratedValue::Object(self.fields.clone())
        }
    }
}

pub type AccessPolicy = GeneratedObject;
pub type QueueItem = GeneratedObject;
pub type ListQueuesSegmentResponse = GeneratedObject;
pub type CorsRule = GeneratedObject;
pub type GeoReplication = GeneratedObject;
pub type RetentionPolicy = GeneratedObject;
pub type Logging = GeneratedObject;
pub type StorageError = GeneratedObject;
pub type Metrics = GeneratedObject;
pub type QueueMessage = GeneratedObject;
pub type DequeuedMessageItem = GeneratedObject;
pub type PeekedMessageItem = GeneratedObject;
pub type EnqueuedMessage = GeneratedObject;
pub type SignedIdentifier = GeneratedObject;
pub type StorageServiceProperties = GeneratedObject;
pub type StorageServiceStats = GeneratedObject;
pub type AzuriteServerQueueOptions = GeneratedObject;
pub type ServiceSetPropertiesOptionalParams = GeneratedObject;
pub type ServiceGetPropertiesOptionalParams = GeneratedObject;
pub type ServiceGetStatisticsOptionalParams = GeneratedObject;
pub type ServiceListQueuesSegmentOptionalParams = GeneratedObject;
pub type QueueCreateOptionalParams = GeneratedObject;
pub type QueueDeleteMethodOptionalParams = GeneratedObject;
pub type QueueGetPropertiesOptionalParams = GeneratedObject;
pub type QueueGetPropertiesWithHeadOptionalParams = GeneratedObject;
pub type QueueSetMetadataOptionalParams = GeneratedObject;
pub type QueueGetAccessPolicyOptionalParams = GeneratedObject;
pub type QueueGetAccessPolicyWithHeadOptionalParams = GeneratedObject;
pub type QueueSetAccessPolicyOptionalParams = GeneratedObject;
pub type MessagesDequeueOptionalParams = GeneratedObject;
pub type MessagesClearOptionalParams = GeneratedObject;
pub type MessagesEnqueueOptionalParams = GeneratedObject;
pub type MessagesPeekOptionalParams = GeneratedObject;
pub type MessageIdUpdateOptionalParams = GeneratedObject;
pub type MessageIdDeleteMethodOptionalParams = GeneratedObject;
pub type ServiceSetPropertiesHeaders = GeneratedObject;
pub type ServiceGetPropertiesHeaders = GeneratedObject;
pub type ServiceGetStatisticsHeaders = GeneratedObject;
pub type ServiceListQueuesSegmentHeaders = GeneratedObject;
pub type QueueCreateHeaders = GeneratedObject;
pub type QueueDeleteHeaders = GeneratedObject;
pub type QueueGetPropertiesHeaders = GeneratedObject;
pub type QueueGetPropertiesWithHeadHeaders = GeneratedObject;
pub type QueueSetMetadataHeaders = GeneratedObject;
pub type QueueGetAccessPolicyHeaders = GeneratedObject;
pub type QueueGetAccessPolicyWithHeadHeaders = GeneratedObject;
pub type QueueSetAccessPolicyHeaders = GeneratedObject;
pub type MessagesDequeueHeaders = GeneratedObject;
pub type MessagesClearHeaders = GeneratedObject;
pub type MessagesEnqueueHeaders = GeneratedObject;
pub type MessagesPeekHeaders = GeneratedObject;
pub type MessageIdUpdateHeaders = GeneratedObject;
pub type MessageIdDeleteHeaders = GeneratedObject;
pub type ServiceSetPropertiesResponse = GeneratedResponse;
pub type ServiceGetPropertiesResponse = GeneratedResponse;
pub type ServiceGetStatisticsResponse = GeneratedResponse;
pub type ServiceListQueuesSegmentResponse = GeneratedResponse;
pub type QueueCreateResponse = GeneratedResponse;
pub type QueueDeleteResponse = GeneratedResponse;
pub type QueueGetPropertiesResponse = GeneratedResponse;
pub type QueueGetPropertiesWithHeadResponse = GeneratedResponse;
pub type QueueSetMetadataResponse = GeneratedResponse;
pub type QueueGetAccessPolicyResponse = GeneratedResponse;
pub type QueueGetAccessPolicyWithHeadResponse = GeneratedResponse;
pub type QueueSetAccessPolicyResponse = GeneratedResponse;
pub type MessagesDequeueResponse = GeneratedResponse;
pub type MessagesClearResponse = GeneratedResponse;
pub type MessagesEnqueueResponse = GeneratedResponse;
pub type MessagesPeekResponse = GeneratedResponse;
pub type MessageIdUpdateResponse = GeneratedResponse;
pub type MessageIdDeleteResponse = GeneratedResponse;
pub type StorageErrorCode = String;
pub type GeoReplicationStatusType = String;
pub type ListQueuesIncludeType = String;
pub type Version = String;
pub type Version1 = String;
pub type Version2 = String;
pub type Version3 = String;
pub type Version4 = String;
pub type Version5 = String;
pub type Version6 = String;
pub type Version7 = String;
pub type Version8 = String;
pub type Version9 = String;
pub type Version10 = String;
pub type Version11 = String;
pub type Version12 = String;
pub type Version13 = String;
pub type Version14 = String;
pub type Version15 = String;
pub type Version16 = String;
pub type Version17 = String;
pub type Version18 = String;
