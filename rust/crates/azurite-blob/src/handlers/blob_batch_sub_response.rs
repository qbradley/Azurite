use std::collections::BTreeMap;

use axum::http::StatusCode;

use crate::generated::i_response::{GeneratedWritableStream, IResponse, ResponseHeaderValue};

use super::sub_response_text_body_stream::SubResponseTextBodyStream;

#[derive(Debug, Clone)]
pub struct BlobBatchSubResponse {
    pub content_id: Option<u32>,
    pub protocolWithVersion: String,
    statusCode: u16,
    statusMessage: String,
    headers: BTreeMap<String, ResponseHeaderValue>,
    bodyStream: SubResponseTextBodyStream,
}

impl BlobBatchSubResponse {
    pub fn new(content_id: Option<u32>, protocolWithVersion: String) -> Self {
        Self {
            content_id,
            protocolWithVersion,
            statusCode: 0,
            statusMessage: String::new(),
            headers: BTreeMap::new(),
            bodyStream: SubResponseTextBodyStream::new(),
        }
    }

    pub fn getBodyContent(&self) -> String {
        self.bodyStream.getBodyContent()
    }

    pub fn end(&mut self) {
        if self.statusMessage.is_empty() {
            self.statusMessage = StatusCode::from_u16(self.statusCode)
                .ok()
                .and_then(|status| status.canonical_reason().map(str::to_string))
                .unwrap_or_else(|| "unknown".to_string());
        }
        self.bodyStream.end();
    }
}

impl IResponse for BlobBatchSubResponse {
    fn setStatusCode(&mut self, code: u16) -> &mut Self {
        self.statusCode = code;
        self
    }

    fn getStatusCode(&self) -> u16 {
        self.statusCode
    }

    fn setStatusMessage(&mut self, message: impl Into<String>) -> &mut Self {
        self.statusMessage = message.into();
        self
    }

    fn getStatusMessage(&self) -> String {
        self.statusMessage.clone()
    }

    fn setHeader(&mut self, field: &str, value: Option<ResponseHeaderValue>) -> &mut Self {
        if let Some(value) = value {
            self.headers.insert(field.to_string(), value);
        }
        self
    }

    fn getHeader(&self, field: &str) -> Option<ResponseHeaderValue> {
        self.headers.get(field).cloned()
    }

    fn getHeaders(&self) -> BTreeMap<String, ResponseHeaderValue> {
        self.headers.clone()
    }

    fn headersSent(&self) -> bool {
        false
    }

    fn setContentType(&mut self, value: Option<String>) -> &mut Self {
        if let Some(value) = value {
            self.setHeader("content-type", Some(ResponseHeaderValue::Single(value)));
        }
        self
    }

    fn getBodyStream(&self) -> GeneratedWritableStream {
        self.bodyStream.stream()
    }
}
