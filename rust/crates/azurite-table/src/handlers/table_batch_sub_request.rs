use std::collections::BTreeMap;

use crate::generated::i_request::{
    GeneratedHttpRequest, GeneratedReadableStream, HttpMethod, IRequest, RequestHeaderValue,
};

#[derive(Debug, Clone, Default)]
pub struct TableBatchSubRequest {
    pub request: GeneratedHttpRequest,
    pub id: usize,
    pub http_version: String,
}

impl TableBatchSubRequest {
    pub fn new(
        id: usize,
        url: impl Into<String>,
        method: HttpMethod,
        http_version: impl Into<String>,
        headers: BTreeMap<String, RequestHeaderValue>,
    ) -> Self {
        let url = url.into();
        Self {
            request: GeneratedHttpRequest {
                method,
                url: url.clone(),
                endpoint: String::new(),
                path: url,
                bodyStream: GeneratedReadableStream::default(),
                body: None,
                headers,
                rawHeaders: Vec::new(),
                query: BTreeMap::new(),
                protocol: String::from("http"),
            },
            id,
            http_version: http_version.into(),
        }
    }
}

impl IRequest for TableBatchSubRequest {
    fn getMethod(&self) -> HttpMethod {
        self.request.getMethod()
    }

    fn getUrl(&self) -> String {
        self.request.getUrl()
    }

    fn getEndpoint(&self) -> String {
        self.request.getEndpoint()
    }

    fn getPath(&self) -> String {
        self.request.getPath()
    }

    fn getBodyStream(&self) -> GeneratedReadableStream {
        self.request.getBodyStream()
    }

    fn setBody(&mut self, body: Option<String>) -> &mut Self {
        self.request.setBody(body);
        self
    }

    fn getBody(&self) -> Option<String> {
        self.request.getBody()
    }

    fn getHeader(&self, field: &str) -> Option<String> {
        self.request.getHeader(field)
    }

    fn getHeaders(&self) -> BTreeMap<String, RequestHeaderValue> {
        self.request.getHeaders()
    }

    fn getRawHeaders(&self) -> Vec<String> {
        self.request.getRawHeaders()
    }

    fn getQuery(&self, key: &str) -> Option<String> {
        self.request.getQuery(key)
    }

    fn getProtocol(&self) -> String {
        self.request.getProtocol()
    }
}
