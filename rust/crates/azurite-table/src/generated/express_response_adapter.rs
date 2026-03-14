use std::collections::BTreeMap;

use crate::generated::i_response::{
    GeneratedHttpResponse, GeneratedWritableStream, IResponse, ResponseHeaderValue,
};

#[derive(Debug, Clone, Default)]
pub struct ExpressResponseAdapter {
    inner: GeneratedHttpResponse,
}

impl ExpressResponseAdapter {
    pub fn new(response: GeneratedHttpResponse) -> Self {
        Self { inner: response }
    }

    pub fn into_inner(self) -> GeneratedHttpResponse {
        self.inner
    }
}

impl IResponse for ExpressResponseAdapter {
    fn setStatusCode(&mut self, code: u16) -> &mut Self {
        self.inner.setStatusCode(code);
        self
    }
    fn getStatusCode(&self) -> u16 {
        self.inner.getStatusCode()
    }
    fn setStatusMessage(&mut self, message: impl Into<String>) -> &mut Self {
        self.inner.setStatusMessage(message);
        self
    }
    fn getStatusMessage(&self) -> String {
        self.inner.getStatusMessage()
    }
    fn setHeader(&mut self, field: &str, value: Option<ResponseHeaderValue>) -> &mut Self {
        self.inner.setHeader(field, value);
        self
    }
    fn getHeader(&self, field: &str) -> Option<ResponseHeaderValue> {
        self.inner.getHeader(field)
    }
    fn getHeaders(&self) -> BTreeMap<String, ResponseHeaderValue> {
        self.inner.getHeaders()
    }
    fn headersSent(&self) -> bool {
        self.inner.headersSent()
    }
    fn setContentType(&mut self, value: Option<String>) -> &mut Self {
        self.inner.setContentType(value);
        self
    }
    fn getBodyStream(&self) -> GeneratedWritableStream {
        self.inner.getBodyStream()
    }
}
