use std::collections::BTreeMap;

use crate::generated::i_request::{
    GeneratedHttpRequest, GeneratedReadableStream, HttpMethod, IRequest, RequestHeaderValue,
};

#[derive(Debug, Clone, Default)]
pub struct ExpressRequestAdapter {
    inner: GeneratedHttpRequest,
}

impl ExpressRequestAdapter {
    pub fn new(request: GeneratedHttpRequest) -> Self {
        Self { inner: request }
    }

    pub fn into_inner(self) -> GeneratedHttpRequest {
        self.inner
    }
}

impl IRequest for ExpressRequestAdapter {
    fn getMethod(&self) -> HttpMethod {
        self.inner.getMethod()
    }
    fn getUrl(&self) -> String {
        self.inner.getUrl()
    }
    fn getEndpoint(&self) -> String {
        self.inner.getEndpoint()
    }
    fn getPath(&self) -> String {
        self.inner.getPath()
    }
    fn getBodyStream(&self) -> GeneratedReadableStream {
        self.inner.getBodyStream()
    }
    fn setBody(&mut self, body: Option<String>) -> &mut Self {
        self.inner.setBody(body);
        self
    }
    fn getBody(&self) -> Option<String> {
        self.inner.getBody()
    }
    fn getHeader(&self, field: &str) -> Option<String> {
        self.inner.getHeader(field)
    }
    fn getHeaders(&self) -> BTreeMap<String, RequestHeaderValue> {
        self.inner.getHeaders()
    }
    fn getRawHeaders(&self) -> Vec<String> {
        self.inner.getRawHeaders()
    }
    fn getQuery(&self, key: &str) -> Option<String> {
        self.inner.getQuery(key)
    }
    fn getProtocol(&self) -> String {
        self.inner.getProtocol()
    }
}
