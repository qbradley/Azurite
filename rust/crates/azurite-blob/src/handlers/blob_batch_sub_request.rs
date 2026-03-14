use std::collections::BTreeMap;
use std::str::FromStr;

use reqwest::Url;

use crate::generated::i_request::{
    GeneratedReadableStream, HttpMethod, IRequest, RequestHeaderValue,
};

#[derive(Debug, Clone)]
pub struct BlobBatchSubRequest {
    pub content_id: u32,
    url: String,
    method: HttpMethod,
    pub protocolWithVersion: String,
    headers: BTreeMap<String, RequestHeaderValue>,
    urlbuilder: Url,
}

impl BlobBatchSubRequest {
    pub fn new(
        content_id: u32,
        url: String,
        method: HttpMethod,
        protocolWithVersion: String,
        headers: BTreeMap<String, RequestHeaderValue>,
    ) -> Self {
        let normalized_headers = headers
            .into_iter()
            .map(|(key, value)| (key.to_ascii_lowercase(), value))
            .collect::<BTreeMap<_, _>>();
        let urlbuilder =
            Url::parse(&url).unwrap_or_else(|_| Url::parse("http://localhost/").unwrap());
        Self {
            content_id,
            url,
            method,
            protocolWithVersion,
            headers: normalized_headers,
            urlbuilder,
        }
    }

    pub fn setHeader(&mut self, key: String, value: Option<RequestHeaderValue>) {
        if let Some(value) = value {
            self.headers.insert(key.to_ascii_lowercase(), value);
        }
    }
}

impl IRequest for BlobBatchSubRequest {
    fn getMethod(&self) -> HttpMethod {
        self.method
    }

    fn getUrl(&self) -> String {
        self.url.clone()
    }

    fn getEndpoint(&self) -> String {
        match (self.urlbuilder.scheme(), self.urlbuilder.host_str()) {
            (scheme, Some(host)) => format!("{scheme}://{host}"),
            _ => String::new(),
        }
    }

    fn getPath(&self) -> String {
        self.urlbuilder.path().to_string()
    }

    fn getBodyStream(&self) -> GeneratedReadableStream {
        GeneratedReadableStream::default()
    }

    fn setBody(&mut self, _body: Option<String>) -> &mut Self {
        self
    }

    fn getBody(&self) -> Option<String> {
        None
    }

    fn getHeader(&self, field: &str) -> Option<String> {
        self.headers
            .get(&field.to_ascii_lowercase())
            .and_then(RequestHeaderValue::first)
    }

    fn getHeaders(&self) -> BTreeMap<String, RequestHeaderValue> {
        self.headers.clone()
    }

    fn getRawHeaders(&self) -> Vec<String> {
        Vec::new()
    }

    fn getQuery(&self, key: &str) -> Option<String> {
        self.urlbuilder
            .query_pairs()
            .find_map(|(query_key, value)| (query_key == key).then(|| value.into_owned()))
    }

    fn getProtocol(&self) -> String {
        self.urlbuilder.scheme().to_string()
    }
}

impl FromStr for BlobBatchSubRequest {
    type Err = Box<dyn std::error::Error + Send + Sync>;

    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        Err("BlobBatchSubRequest::from_str is not implemented".into())
    }
}
