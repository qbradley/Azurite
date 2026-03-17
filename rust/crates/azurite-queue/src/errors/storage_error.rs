use std::collections::BTreeMap;

use quick_xml::escape::escape;
use thiserror::Error;

use crate::generated::artifacts::models::GeneratedValue;
use crate::generated::i_response::ResponseHeaderValue;
use crate::utils::constants::QUEUE_API_VERSION;

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
    ) -> Self {
        let storageErrorCode = storageErrorCode.into();
        let storageErrorMessage = storageErrorMessage.into();
        let storageRequestID = storageRequestID.into();
        let bodyInXML = build_body_xml(
            &storageErrorCode,
            &storageErrorMessage,
            &storageRequestID,
            &storageAdditionalErrorMessages,
        );

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
            ResponseHeaderValue::from(QUEUE_API_VERSION),
        );

        Self {
            statusCode,
            message: storageErrorMessage.clone(),
            statusMessage: Some(storageErrorMessage.clone()),
            headers: Some(headers),
            body: Some(GeneratedValue::String(bodyInXML)),
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

fn build_body_xml(
    storageErrorCode: &str,
    storageErrorMessage: &str,
    storageRequestID: &str,
    storageAdditionalErrorMessages: &BTreeMap<String, String>,
) -> String {
    let message = format!(
        "{}\nRequestId:{}\nTime:{}",
        storageErrorMessage,
        storageRequestID,
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    );

    // L-XML2JS-Declaration-Parity: xml2js.Builder emits XML declaration by default
    let mut bodyInXML =
        String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Error>");
    bodyInXML.push_str("<Code>");
    bodyInXML.push_str(&escape(storageErrorCode));
    bodyInXML.push_str("</Code>");
    bodyInXML.push_str("<Message>");
    bodyInXML.push_str(&escape(&message));
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
