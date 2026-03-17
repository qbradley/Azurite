use std::collections::BTreeMap;

use quick_xml::escape::escape;
use serde_json::{Map, Value};
use thiserror::Error;

use crate::generated::artifacts::models::GeneratedValue;
use crate::generated::context::Context;
use crate::generated::i_response::ResponseHeaderValue;
use crate::utils::constants::{
    FULL_METADATA_ACCEPT, MINIMAL_METADATA_ACCEPT, NO_METADATA_ACCEPT, TABLE_API_VERSION,
};
use crate::utils::utils::get_payload_format;

#[allow(non_snake_case)]
#[derive(Debug, Clone, Error)]
#[error("{storageErrorMessage}")]
pub struct StorageError {
    pub statusCode: u16,
    pub message: String,
    pub statusMessage: Option<String>,
    pub headers: Option<BTreeMap<String, ResponseHeaderValue>>,
    pub body: Option<GeneratedValue>,
    pub contentType: Option<String>,
    pub storageErrorCode: String,
    pub storageErrorMessage: String,
    pub storageRequestID: String,
}

#[allow(non_snake_case)]
impl StorageError {
    pub fn new(
        statusCode: u16,
        storageErrorCode: impl Into<String>,
        storageErrorMessage: impl Into<String>,
        storageRequestID: impl Into<String>,
        storageAdditionalErrorMessages: BTreeMap<String, String>,
        context: &Context,
    ) -> Self {
        let storageErrorCode = storageErrorCode.into();
        let storageErrorMessage = storageErrorMessage.into();
        let storageRequestID = storageRequestID.into();
        let payload = get_payload_format(context);
        let isJSON = is_json_payload(&payload);
        let message = format_error_message(&storageErrorMessage, &storageRequestID);

        let body = if isJSON {
            build_body_json(&storageErrorCode, &message, &storageAdditionalErrorMessages)
        } else {
            build_body_xml(&storageErrorCode, &message, &storageAdditionalErrorMessages)
        };

        let mut headers = BTreeMap::new();
        headers.insert(
            String::from("x-ms-error-code"),
            ResponseHeaderValue::from(storageErrorCode.clone()),
        );
        headers.insert(
            String::from("x-ms-request-id"),
            ResponseHeaderValue::from(storageRequestID.clone()),
        );
        headers.insert(
            String::from("x-ms-version"),
            ResponseHeaderValue::from(TABLE_API_VERSION),
        );

        let contentType = if isJSON {
            format!("{payload};streaming=true;charset=utf-8")
        } else {
            String::from("application/xml")
        };

        Self {
            statusCode,
            message: storageErrorMessage.clone(),
            statusMessage: Some(storageErrorMessage.clone()),
            headers: Some(headers),
            body: Some(GeneratedValue::String(body)),
            contentType: Some(contentType),
            storageErrorCode,
            storageErrorMessage,
            storageRequestID,
        }
    }

    /// Create a StorageError that always uses XML body format, regardless of Accept header.
    /// Used by authenticators to match TypeScript behavior where auth-stage errors are XML.
    pub fn new_xml(
        statusCode: u16,
        storageErrorCode: impl Into<String>,
        storageErrorMessage: impl Into<String>,
        storageRequestID: impl Into<String>,
        storageAdditionalErrorMessages: BTreeMap<String, String>,
    ) -> Self {
        let storageErrorCode = storageErrorCode.into();
        let storageErrorMessage = storageErrorMessage.into();
        let storageRequestID = storageRequestID.into();
        let message = format_error_message(&storageErrorMessage, &storageRequestID);

        let body = build_body_xml(&storageErrorCode, &message, &storageAdditionalErrorMessages);

        let mut headers = BTreeMap::new();
        headers.insert(
            String::from("x-ms-error-code"),
            ResponseHeaderValue::from(storageErrorCode.clone()),
        );
        headers.insert(
            String::from("x-ms-request-id"),
            ResponseHeaderValue::from(storageRequestID.clone()),
        );
        headers.insert(
            String::from("x-ms-version"),
            ResponseHeaderValue::from(TABLE_API_VERSION),
        );

        Self {
            statusCode,
            message: storageErrorMessage.clone(),
            statusMessage: Some(storageErrorMessage.clone()),
            headers: Some(headers),
            body: Some(GeneratedValue::String(body)),
            contentType: Some(String::from("application/xml")),
            storageErrorCode,
            storageErrorMessage,
            storageRequestID,
        }
    }

    pub fn empty_extra() -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    pub fn headers_mut(&mut self) -> &mut BTreeMap<String, ResponseHeaderValue> {
        self.headers.get_or_insert_with(BTreeMap::new)
    }
}

fn is_json_payload(payload: &str) -> bool {
    matches!(
        payload,
        NO_METADATA_ACCEPT | MINIMAL_METADATA_ACCEPT | FULL_METADATA_ACCEPT
    )
}

fn format_error_message(storageErrorMessage: &str, storageRequestID: &str) -> String {
    format!(
        "{}\nRequestId:{}\nTime:{}",
        storageErrorMessage,
        storageRequestID,
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    )
}

fn build_body_json(
    storageErrorCode: &str,
    message: &str,
    storageAdditionalErrorMessages: &BTreeMap<String, String>,
) -> String {
    let mut bodyInJson = Map::new();
    bodyInJson.insert(
        String::from("code"),
        Value::String(storageErrorCode.to_string()),
    );

    let mut messageObject = Map::new();
    messageObject.insert(String::from("lang"), Value::String(String::from("en-US")));
    messageObject.insert(String::from("value"), Value::String(message.to_string()));
    bodyInJson.insert(String::from("message"), Value::Object(messageObject));

    for (key, value) in storageAdditionalErrorMessages {
        bodyInJson.insert(key.clone(), Value::String(value.clone()));
    }

    let mut root = Map::new();
    root.insert(String::from("odata.error"), Value::Object(bodyInJson));
    Value::Object(root).to_string()
}

fn build_body_xml(
    storageErrorCode: &str,
    message: &str,
    storageAdditionalErrorMessages: &BTreeMap<String, String>,
) -> String {
    // L-XML2JS-Declaration-Parity: xml2js.Builder emits XML declaration by default
    let mut bodyInXML =
        String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Error>");
    bodyInXML.push_str("<Code>");
    bodyInXML.push_str(&escape(storageErrorCode));
    bodyInXML.push_str("</Code>");
    bodyInXML.push_str("<Message>");
    bodyInXML.push_str(&escape(message));
    bodyInXML.push_str("</Message>");

    for (key, value) in storageAdditionalErrorMessages {
        bodyInXML.push('<');
        bodyInXML.push_str(key);
        bodyInXML.push('>');
        bodyInXML.push_str(&escape(value));
        bodyInXML.push_str("</");
        bodyInXML.push_str(key);
        bodyInXML.push('>');
    }

    bodyInXML.push_str("</Error>");
    bodyInXML
}
