use std::collections::BTreeMap;

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

pub type GeneratedObject = BTreeMap<String, GeneratedValue>;

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

pub type SignedIdentifier = GeneratedObject;
pub type StorageServiceProperties = GeneratedObject;
pub type StorageServiceStats = GeneratedObject;
pub type TableProperties = GeneratedObject;
pub type TableEntity = GeneratedObject;
pub type TableAcl = GeneratedObject;

#[derive(Debug, Clone, Default)]
pub struct QueryOptions {
    pub format: Option<String>,
    pub top: Option<usize>,
    pub select: Option<String>,
    pub filter: Option<String>,
}

pub type ServiceSetPropertiesOptionalParams = GeneratedObject;
pub type ServiceGetPropertiesOptionalParams = GeneratedObject;
pub type ServiceGetStatisticsOptionalParams = GeneratedObject;
pub type TableQueryOptionalParams = GeneratedObject;
pub type TableCreateOptionalParams = GeneratedObject;
pub type TableBatchOptionalParams = GeneratedObject;
pub type TableDeleteMethodOptionalParams = GeneratedObject;
pub type TableQueryEntitiesOptionalParams = GeneratedObject;
pub type TableQueryEntitiesWithPartitionAndRowKeyOptionalParams = GeneratedObject;
pub type TableUpdateEntityOptionalParams = GeneratedObject;
pub type TableMergeEntityOptionalParams = GeneratedObject;
pub type TableDeleteEntityOptionalParams = GeneratedObject;
pub type TableInsertEntityOptionalParams = GeneratedObject;
pub type TableGetAccessPolicyOptionalParams = GeneratedObject;
pub type TableSetAccessPolicyOptionalParams = GeneratedObject;
pub type ServiceSetPropertiesResponse = GeneratedResponse;
pub type ServiceGetPropertiesResponse = GeneratedResponse;
pub type ServiceGetStatisticsResponse = GeneratedResponse;
pub type TableQueryResponse = GeneratedResponse;
pub type TableCreateResponse = GeneratedResponse;
pub type TableBatchResponse = GeneratedResponse;
pub type TableDeleteResponse = GeneratedResponse;
pub type TableQueryEntitiesResponse = GeneratedResponse;
pub type TableQueryEntitiesWithPartitionAndRowKeyResponse = GeneratedResponse;
pub type TableUpdateEntityResponse = GeneratedResponse;
pub type TableMergeEntityResponse = GeneratedResponse;
pub type TableMergeEntityWithMergeResponse = GeneratedResponse;
pub type TableDeleteEntityResponse = GeneratedResponse;
pub type TableInsertEntityResponse = GeneratedResponse;
pub type TableGetAccessPolicyResponse = GeneratedResponse;
pub type TableSetAccessPolicyResponse = GeneratedResponse;
