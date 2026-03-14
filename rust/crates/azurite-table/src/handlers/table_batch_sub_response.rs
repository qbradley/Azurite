use std::collections::BTreeMap;

use crate::generated::i_response::{
    GeneratedHttpResponse, GeneratedWritableStream, IResponse, ResponseHeaderValue,
};

#[derive(Debug, Clone, Default)]
pub struct TableBatchSubResponse {
    pub response: GeneratedHttpResponse,
}

impl TableBatchSubResponse {
    pub fn into_inner(self) -> GeneratedHttpResponse {
        self.response
    }
}

impl IResponse for TableBatchSubResponse {
    fn setStatusCode(&mut self, code: u16) -> &mut Self {
        self.response.setStatusCode(code);
        self
    }

    fn getStatusCode(&self) -> u16 {
        self.response.getStatusCode()
    }

    fn setStatusMessage(&mut self, message: impl Into<String>) -> &mut Self {
        self.response.setStatusMessage(message);
        self
    }

    fn getStatusMessage(&self) -> String {
        self.response.getStatusMessage()
    }

    fn setHeader(&mut self, field: &str, value: Option<ResponseHeaderValue>) -> &mut Self {
        self.response.setHeader(field, value);
        self
    }

    fn getHeader(&self, field: &str) -> Option<ResponseHeaderValue> {
        self.response.getHeader(field)
    }

    fn getHeaders(&self) -> BTreeMap<String, ResponseHeaderValue> {
        self.response.getHeaders()
    }

    fn headersSent(&self) -> bool {
        self.response.headersSent()
    }

    fn setContentType(&mut self, value: Option<String>) -> &mut Self {
        self.response.setContentType(value);
        self
    }

    fn getBodyStream(&self) -> GeneratedWritableStream {
        self.response.getBodyStream()
    }
}
