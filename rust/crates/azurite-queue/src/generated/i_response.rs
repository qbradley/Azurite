use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseHeaderValue {
    Single(String),
    Multi(Vec<String>),
}

impl ResponseHeaderValue {
    pub fn as_single(&self) -> Option<String> {
        match self {
            ResponseHeaderValue::Single(value) => Some(value.clone()),
            ResponseHeaderValue::Multi(values) => values.first().cloned(),
        }
    }
}

impl From<String> for ResponseHeaderValue {
    fn from(value: String) -> Self {
        Self::Single(value)
    }
}
impl From<&str> for ResponseHeaderValue {
    fn from(value: &str) -> Self {
        Self::Single(value.to_owned())
    }
}
impl From<f64> for ResponseHeaderValue {
    fn from(value: f64) -> Self {
        Self::Single(value.to_string())
    }
}
impl From<bool> for ResponseHeaderValue {
    fn from(value: bool) -> Self {
        Self::Single(value.to_string())
    }
}
impl From<Vec<String>> for ResponseHeaderValue {
    fn from(value: Vec<String>) -> Self {
        Self::Multi(value)
    }
}

#[derive(Debug, Clone, Default)]
struct WritableState {
    bytes: Vec<u8>,
    ended: bool,
    wrote: bool,
}

#[derive(Debug, Clone, Default)]
pub struct GeneratedWritableStream {
    inner: Arc<Mutex<WritableState>>,
}

impl GeneratedWritableStream {
    pub fn write_text(&self, value: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.bytes.extend_from_slice(value.as_bytes());
        inner.wrote = true;
    }

    pub fn write_bytes(&self, value: &[u8]) {
        let mut inner = self.inner.lock().unwrap();
        inner.bytes.extend_from_slice(value);
        inner.wrote = true;
    }

    pub fn end(&self) {
        self.inner.lock().unwrap().ended = true;
    }

    pub fn bytes(&self) -> Vec<u8> {
        self.inner.lock().unwrap().bytes.clone()
    }

    pub fn text(&self) -> String {
        String::from_utf8(self.bytes()).unwrap_or_default()
    }

    pub fn has_written(&self) -> bool {
        self.inner.lock().unwrap().wrote
    }

    pub fn is_ended(&self) -> bool {
        self.inner.lock().unwrap().ended
    }
}

#[allow(non_snake_case)]
pub trait IResponse: Clone + Send + Sync {
    fn setStatusCode(&mut self, code: u16) -> &mut Self;
    fn getStatusCode(&self) -> u16;
    fn setStatusMessage(&mut self, message: impl Into<String>) -> &mut Self;
    fn getStatusMessage(&self) -> String;
    fn setHeader(&mut self, field: &str, value: Option<ResponseHeaderValue>) -> &mut Self;
    fn getHeader(&self, field: &str) -> Option<ResponseHeaderValue>;
    fn getHeaders(&self) -> BTreeMap<String, ResponseHeaderValue>;
    fn headersSent(&self) -> bool;
    fn setContentType(&mut self, value: Option<String>) -> &mut Self;
    fn getBodyStream(&self) -> GeneratedWritableStream;
}

#[derive(Debug, Clone, Default)]
pub struct GeneratedHttpResponse {
    pub statusCode: u16,
    pub statusMessage: String,
    pub headers: BTreeMap<String, ResponseHeaderValue>,
    pub bodyStream: GeneratedWritableStream,
    pub headersAlreadySent: bool,
}

impl IResponse for GeneratedHttpResponse {
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
            self.headers.insert(field.to_owned(), value);
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
        self.headersAlreadySent || self.bodyStream.has_written() || self.bodyStream.is_ended()
    }

    fn setContentType(&mut self, value: Option<String>) -> &mut Self {
        if let Some(value) = value {
            self.headers.insert(
                String::from("content-type"),
                ResponseHeaderValue::Single(value),
            );
        }
        self
    }

    fn getBodyStream(&self) -> GeneratedWritableStream {
        self.bodyStream.clone()
    }
}
