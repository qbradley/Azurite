use std::collections::HashSet;
use std::sync::Arc;

use azurite_common::utils::utils::formatRfc1123;
use chrono::{DateTime, Utc};

use crate::authentication::IAuthenticator;
use crate::generated::artifacts::models::{
    GeneratedBody, GeneratedObject, GeneratedResponse, GeneratedValue,
};
use crate::generated::context::Context;
use crate::generated::i_request::IRequest;
use crate::generated::utils::i_logger::ILogger;
use crate::persistence::ITableMetadataStore;
use crate::utils::constants::{
    DEFAULT_TABLE_LISTENING_PORT, DEFAULT_TABLE_SERVER_HOST_NAME, FULL_METADATA_ACCEPT,
    MINIMAL_METADATA_ACCEPT, NO_METADATA_ACCEPT, RETURN_CONTENT, RETURN_NO_CONTENT,
    TABLE_API_VERSION,
};
use crate::utils::utils::get_payload_format;

pub type SharedTableMetadataStore = Arc<dyn ITableMetadataStore + Send + Sync>;
pub type SharedLogger = Arc<dyn ILogger + Send + Sync>;
pub type SharedAuthenticator = Arc<dyn IAuthenticator + Send + Sync>;

#[allow(non_snake_case)]
#[derive(Clone)]
pub struct BaseHandler {
    pub metadataStore: SharedTableMetadataStore,
    pub logger: SharedLogger,
    pub authenticator: Option<SharedAuthenticator>,
}

impl BaseHandler {
    pub fn new(metadataStore: SharedTableMetadataStore, logger: SharedLogger) -> Self {
        Self {
            metadataStore,
            logger,
            authenticator: None,
        }
    }

    pub fn with_authenticator(mut self, authenticator: SharedAuthenticator) -> Self {
        self.authenticator = Some(authenticator);
        self
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
        response.insert_field("version", string_value(TABLE_API_VERSION));
        if include_date {
            response.insert_field(
                "date",
                string_value(formatRfc1123(Self::start_time(context))),
            );
        }
        if let Some(client_request_id) = get_string(options, "requestId") {
            response.insert_field("clientRequestId", string_value(client_request_id));
        }
    }

    pub fn get_and_check_payload_format(
        &self,
        context: &Context,
    ) -> Result<String, crate::errors::StorageError> {
        let format = get_payload_format(context);
        if matches!(
            format.as_str(),
            NO_METADATA_ACCEPT | MINIMAL_METADATA_ACCEPT | FULL_METADATA_ACCEPT
        ) {
            Ok(format)
        } else {
            Err(crate::errors::StorageErrorFactory::getAtomFormatNotSupported(context))
        }
    }

    pub fn get_prefer_header(&self, context: &Context) -> Option<String> {
        context
            .request()
            .and_then(|request| request.getHeader("Prefer"))
    }

    pub fn update_response_prefer(&self, response: &mut GeneratedResponse, context: &Context) {
        match self.get_prefer_header(context).as_deref() {
            Some(RETURN_NO_CONTENT) => {
                response.statusCode = 204;
                response.insert_field("preferenceApplied", string_value(RETURN_NO_CONTENT));
                response.body = None;
            }
            Some(RETURN_CONTENT) | None => {
                response.statusCode = 201;
                response.insert_field("preferenceApplied", string_value(RETURN_CONTENT));
            }
            Some(_) => {}
        }
    }

    pub fn set_response_content_type(
        &self,
        response: &mut GeneratedResponse,
        accept: Option<&str>,
    ) {
        if let Some(accept) = accept {
            response.contentType = Some(accept.to_string());
        }
    }

    pub fn odata_annotation_url_prefix(&self, context: &Context, account: &str) -> String {
        let mut protocol = String::from("http");
        let mut host = format!(
            "{}:{}/{}",
            DEFAULT_TABLE_SERVER_HOST_NAME, DEFAULT_TABLE_LISTENING_PORT, account
        );
        if let Some(request) = context.request() {
            if let Some(request_host) = request.getHeader("host") {
                host = format!("{request_host}/{account}");
            }
            let request_protocol = request.getProtocol();
            if !request_protocol.is_empty() {
                protocol = request_protocol;
            }
        }
        format!("{protocol}://{host}")
    }

    pub fn stream_body(text: impl Into<String>) -> GeneratedBody {
        GeneratedBody::Stream(
            crate::generated::i_request::GeneratedReadableStream::from_string(text.into()),
        )
    }
}

pub(crate) fn string_value(value: impl Into<String>) -> GeneratedValue {
    GeneratedValue::String(value.into())
}

pub(crate) fn get_string(map: &GeneratedObject, key: &str) -> Option<String> {
    map.get(key).and_then(GeneratedValue::as_string)
}

pub(crate) fn generated_object_to_json_map(
    properties: &GeneratedObject,
) -> std::collections::HashMap<String, serde_json::Value> {
    properties
        .iter()
        .map(|(key, value)| (key.clone(), value.to_json_value()))
        .collect()
}

pub(crate) fn parse_select_set(options: &GeneratedObject) -> Option<HashSet<String>> {
    get_string(options, "select")
        .or_else(|| {
            options
                .get("queryOptions")
                .and_then(GeneratedValue::as_object)
                .and_then(|query| get_string(query, "select"))
        })
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned)
                .collect::<HashSet<_>>()
        })
        .filter(|value| !value.is_empty())
}

pub(crate) fn get_query_options(
    options: &GeneratedObject,
) -> crate::generated::artifacts::models::QueryOptions {
    options
        .get("queryOptions")
        .and_then(GeneratedValue::as_object)
        .map(query_options_from_object)
        .unwrap_or_default()
}

pub(crate) fn query_options_from_object(
    object: &GeneratedObject,
) -> crate::generated::artifacts::models::QueryOptions {
    crate::generated::artifacts::models::QueryOptions {
        format: get_string(object, "format"),
        top: object
            .get("top")
            .and_then(GeneratedValue::as_number)
            .map(|value| value as usize),
        select: get_string(object, "select"),
        filter: get_string(object, "filter"),
    }
}

pub(crate) fn rename_key(object: &mut GeneratedObject, from: &str, to: &str) {
    if !object.contains_key(to) {
        if let Some(value) = object.remove(from) {
            object.insert(to.to_string(), value);
        }
    }
}

#[allow(dead_code)]
pub(crate) fn normalize_cors_rule(rule: &mut GeneratedObject) {
    rename_key(rule, "AllowedOrigins", "allowedOrigins");
    rename_key(rule, "AllowedMethods", "allowedMethods");
    rename_key(rule, "AllowedHeaders", "allowedHeaders");
    rename_key(rule, "ExposedHeaders", "exposedHeaders");
    rename_key(rule, "MaxAgeInSeconds", "maxAgeInSeconds");
}

pub(crate) fn add_optional_string_field(
    response: &mut GeneratedResponse,
    key: &str,
    value: Option<String>,
) {
    if let Some(value) = value {
        response.insert_field(key, string_value(value));
    }
}
