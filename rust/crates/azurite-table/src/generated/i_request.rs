use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum HttpMethod {
    GET,
    HEAD,
    POST,
    PUT,
    DELETE,
    CONNECT,
    OPTIONS,
    TRACE,
    MERGE,
    PATCH,
}

impl Default for HttpMethod {
    fn default() -> Self {
        Self::GET
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            HttpMethod::GET => "GET",
            HttpMethod::HEAD => "HEAD",
            HttpMethod::POST => "POST",
            HttpMethod::PUT => "PUT",
            HttpMethod::DELETE => "DELETE",
            HttpMethod::CONNECT => "CONNECT",
            HttpMethod::OPTIONS => "OPTIONS",
            HttpMethod::TRACE => "TRACE",
            HttpMethod::MERGE => "MERGE",
            HttpMethod::PATCH => "PATCH",
        };
        formatter.write_str(value)
    }
}

impl FromStr for HttpMethod {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_uppercase().as_str() {
            "GET" => Ok(Self::GET),
            "HEAD" => Ok(Self::HEAD),
            "POST" => Ok(Self::POST),
            "PUT" => Ok(Self::PUT),
            "DELETE" => Ok(Self::DELETE),
            "CONNECT" => Ok(Self::CONNECT),
            "OPTIONS" => Ok(Self::OPTIONS),
            "TRACE" => Ok(Self::TRACE),
            "MERGE" => Ok(Self::MERGE),
            "PATCH" => Ok(Self::PATCH),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestHeaderValue {
    Single(String),
    Multi(Vec<String>),
}

impl RequestHeaderValue {
    pub fn first(&self) -> Option<String> {
        match self {
            RequestHeaderValue::Single(value) => Some(value.clone()),
            RequestHeaderValue::Multi(values) => values.first().cloned(),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct ReadableState {
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct GeneratedReadableStream {
    inner: Arc<Mutex<ReadableState>>,
}

impl GeneratedReadableStream {
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ReadableState {
                bytes: bytes.into(),
            })),
        }
    }

    pub fn from_string(value: impl Into<String>) -> Self {
        Self::from_bytes(value.into().into_bytes())
    }

    pub fn read_to_vec(&self) -> Vec<u8> {
        self.inner.lock().unwrap().bytes.clone()
    }

    pub fn read_to_string(&self) -> String {
        // Use lossy conversion to match Node.js Buffer.toString('utf-8') behavior:
        // invalid bytes become U+FFFD replacement characters instead of failing.
        String::from_utf8_lossy(&self.read_to_vec()).into_owned()
    }
}

#[allow(non_snake_case)]
pub trait IRequest: Clone + Send + Sync {
    fn getMethod(&self) -> HttpMethod;
    fn getUrl(&self) -> String;
    fn getEndpoint(&self) -> String;
    fn getPath(&self) -> String;
    fn getBodyStream(&self) -> GeneratedReadableStream;
    fn setBody(&mut self, body: Option<String>) -> &mut Self;
    fn getBody(&self) -> Option<String>;
    fn getHeader(&self, field: &str) -> Option<String>;
    fn getHeaders(&self) -> BTreeMap<String, RequestHeaderValue>;
    fn getRawHeaders(&self) -> Vec<String>;
    fn getQuery(&self, key: &str) -> Option<String>;
    fn getProtocol(&self) -> String;
}

#[derive(Debug, Clone, Default)]
pub struct GeneratedHttpRequest {
    pub method: HttpMethod,
    pub url: String,
    pub endpoint: String,
    pub path: String,
    pub bodyStream: GeneratedReadableStream,
    pub body: Option<String>,
    pub headers: BTreeMap<String, RequestHeaderValue>,
    pub rawHeaders: Vec<String>,
    pub query: BTreeMap<String, String>,
    pub protocol: String,
}

impl GeneratedHttpRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_details(
        method: HttpMethod,
        url: impl Into<String>,
        endpoint: impl Into<String>,
        path: impl Into<String>,
    ) -> Self {
        Self {
            method,
            url: url.into(),
            endpoint: endpoint.into(),
            path: path.into(),
            protocol: String::from("http"),
            ..Self::default()
        }
    }
}

impl IRequest for GeneratedHttpRequest {
    fn getMethod(&self) -> HttpMethod {
        self.method
    }

    fn getUrl(&self) -> String {
        self.url.clone()
    }

    fn getEndpoint(&self) -> String {
        self.endpoint.clone()
    }

    fn getPath(&self) -> String {
        self.path.clone()
    }

    fn getBodyStream(&self) -> GeneratedReadableStream {
        self.bodyStream.clone()
    }

    fn setBody(&mut self, body: Option<String>) -> &mut Self {
        self.body = body;
        self
    }

    fn getBody(&self) -> Option<String> {
        self.body.clone()
    }

    fn getHeader(&self, field: &str) -> Option<String> {
        self.headers
            .get(&field.to_ascii_lowercase())
            .or_else(|| self.headers.get(field))
            .and_then(RequestHeaderValue::first)
    }

    fn getHeaders(&self) -> BTreeMap<String, RequestHeaderValue> {
        self.headers.clone()
    }

    fn getRawHeaders(&self) -> Vec<String> {
        self.rawHeaders.clone()
    }

    fn getQuery(&self, key: &str) -> Option<String> {
        self.query.get(key).cloned()
    }

    fn getProtocol(&self) -> String {
        self.protocol.clone()
    }
}
