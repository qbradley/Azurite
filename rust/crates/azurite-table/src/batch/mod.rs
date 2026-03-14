use std::str::FromStr;

use chrono::{SecondsFormat, Utc};
use serde_json::json;
use url::Url;

use crate::{
    errors::StorageError,
    generated::{
        artifacts::models::{GeneratedResponse, GeneratedValue},
        i_request::{
            GeneratedHttpRequest, GeneratedReadableStream, HttpMethod, IRequest, RequestHeaderValue,
        },
    },
    utils::{
        constants::{MINIMAL_METADATA_ACCEPT, NO_METADATA_ACCEPT},
        utils::{
            extract_entity_keys_from_url_or_body, extract_path_from_uri, generated_body_to_string,
            parse_header_block, query_map_from_uri,
        },
    },
};

const CRLF: &str = "\r\n";
const SUB_RESPONSE_PREAMBLE: &str =
    "\r\nContent-Type: application/http\r\nContent-Transfer-Encoding: binary\r\n\r\n";
pub const MAX_BATCH_OPERATIONS: usize = 100;
pub const TOO_MANY_OPERATIONS_ERROR: &str =
    "0:The batch request operation exceeds the maximum 100 changes per change set.";
pub const NO_PARTITION_KEY_ERROR: &str = "Partition key not found in request.";

#[derive(Debug, Clone, Default)]
pub struct TableBatchModule;

#[derive(Debug, Clone)]
pub struct BatchEnvelope {
    pub batch_boundary: String,
    pub changeset_boundary: String,
    pub requests: Vec<BatchRequestPart>,
}

#[derive(Debug, Clone)]
pub struct BatchRequestPart {
    pub content_id: Option<String>,
    pub http_version: String,
    pub request: GeneratedHttpRequest,
}

pub fn parse_batch_request(body: &str) -> Result<BatchEnvelope, String> {
    let line_ending = if body.contains(CRLF) { CRLF } else { "\n" };
    let batch_boundary = extract_batch_boundary(body)
        .ok_or_else(|| String::from("Batch request is missing a batch boundary."))?;
    let changeset_boundary =
        extract_changeset_boundary(body).unwrap_or_else(|| batch_boundary.clone());
    let request_boundary = format!("--{changeset_boundary}");

    let mut requests = Vec::new();
    for raw_part in body.split(&request_boundary).skip(1) {
        let trimmed = raw_part.trim_matches(|ch| matches!(ch, '\r' | '\n' | ' ' | '\t'));
        if trimmed.is_empty() || trimmed == "--" || !trimmed.contains("HTTP/") {
            continue;
        }
        requests.push(parse_request_part(trimmed, line_ending)?);
    }

    Ok(BatchEnvelope {
        batch_boundary,
        changeset_boundary,
        requests,
    })
}

pub fn serialize_batch_response(envelope: &BatchEnvelope, response_parts: &[String]) -> String {
    let batch_response_boundary =
        response_boundary(&envelope.batch_boundary, "batch", "batchresponse");
    let changeset_response_boundary = if envelope.changeset_boundary == envelope.batch_boundary {
        envelope.changeset_boundary.clone()
    } else {
        response_boundary(
            &envelope.changeset_boundary,
            "changeset",
            "changesetresponse",
        )
    };

    let mut output = String::new();
    output.push_str(&format!(
        "--{batch_response_boundary}{CRLF}Content-Type: multipart/mixed; boundary={changeset_response_boundary}{CRLF}{CRLF}"
    ));

    for part in response_parts {
        output.push_str(&format!("--{changeset_response_boundary}{CRLF}"));
        output.push_str(part);
        output.push_str(CRLF);
        output.push_str(CRLF);
    }

    output.push_str(&format!("--{changeset_response_boundary}--{CRLF}"));
    output.push_str(&format!("--{batch_response_boundary}--{CRLF}"));
    output
}

pub fn serialize_response_part(request: &BatchRequestPart, response: &GeneratedResponse) -> String {
    let mut output = String::from(SUB_RESPONSE_PREAMBLE);
    output.push_str(&format!(
        "HTTP/1.1 {} {}{CRLF}",
        response.statusCode,
        status_message(response.statusCode)
    ));

    if let Some(content_id) = request.content_id.as_deref() {
        output.push_str(&format!("Content-ID: {content_id}{CRLF}"));
    }

    output.push_str("X-Content-Type-Options: nosniff\r\n");
    output.push_str("Cache-Control: no-cache\r\n");
    output.push_str(&format!(
        "DataServiceVersion: {};{CRLF}",
        data_service_version(request)
    ));

    if let Some(preference_applied) = response
        .get_field("preferenceApplied")
        .and_then(GeneratedValue::as_string)
    {
        output.push_str(&format!("Preference-Applied: {preference_applied}{CRLF}"));
    }

    if request.request.getMethod() == HttpMethod::POST {
        if let Some(location) = entity_location(&request.request) {
            output.push_str(&format!("Location: {location}{CRLF}"));
            output.push_str(&format!("DataServiceId: {location}{CRLF}"));
        }
    }

    if let Some(etag) = response
        .get_field("eTag")
        .and_then(GeneratedValue::as_string)
    {
        output.push_str(&format!("ETag: {etag}{CRLF}"));
    }

    if let Some(body) = generated_body_to_string(response.body.as_ref()) {
        let content_type = response_content_type(response, &request.request);
        output.push_str(&format!("Content-Type: {content_type}{CRLF}"));
        output.push_str(CRLF);
        output.push_str(&body);
    } else {
        output.push_str(CRLF);
    }

    output
}

pub fn serialize_storage_error_part(operation_index: usize, error: &StorageError) -> String {
    let mut output = String::from(SUB_RESPONSE_PREAMBLE);
    output.push_str(&format!(
        "HTTP/1.1 {} {}{CRLF}",
        error.statusCode,
        status_message(error.statusCode)
    ));
    output.push_str(&format!("Content-ID: {operation_index}{CRLF}"));
    output.push_str(&format!("DataServiceVersion: 3.0;{CRLF}"));
    output.push_str(&format!(
        "Content-Type: {MINIMAL_METADATA_ACCEPT};charset=utf-8{CRLF}{CRLF}"
    ));
    output.push_str(&storage_error_body(
        error,
        operation_index.saturating_sub(1),
    ));
    output
}

pub fn serialize_general_error_part(message: &str, request_id: Option<&str>) -> String {
    let mut value = message.to_string();
    if let Some(request_id) = request_id.filter(|value| !value.is_empty()) {
        value.push_str(&format!("\nRequestId:{request_id}"));
    }
    value.push_str(&format!(
        "\nTime:{}",
        Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true)
    ));

    let mut output = String::from(SUB_RESPONSE_PREAMBLE);
    output.push_str(&format!("HTTP/1.1 400 Bad Request{CRLF}"));
    output.push_str("X-Content-Type-Options: nosniff\r\n");
    output.push_str(&format!("DataServiceVersion: 3.0;{CRLF}"));
    output.push_str(&format!(
        "Content-Type: {MINIMAL_METADATA_ACCEPT};charset=utf-8{CRLF}{CRLF}"
    ));
    output.push_str(
        &json!({
            "odata.error": {
                "code": "InvalidInput",
                "message": {
                    "lang": "en-US",
                    "value": value,
                }
            }
        })
        .to_string(),
    );
    output
}

fn parse_request_part(raw_part: &str, line_ending: &str) -> Result<BatchRequestPart, String> {
    let raw_part = raw_part.trim_end_matches("--").trim();
    let (mime_headers_block, request_block) = split_header_block(raw_part, line_ending)
        .ok_or_else(|| String::from("Batch request part is missing MIME headers."))?;
    let mime_headers = parse_header_block(mime_headers_block);
    let content_id = mime_headers
        .get("content-id")
        .and_then(RequestHeaderValue::first);

    let (request_head, request_body) = split_header_block(request_block.trim(), line_ending)
        .map(|(head, body)| (head, Some(body)))
        .unwrap_or((request_block.trim(), None));

    let mut request_lines = request_head.lines().filter(|line| !line.trim().is_empty());
    let request_line = request_lines
        .next()
        .ok_or_else(|| String::from("Batch request part is missing an HTTP request line."))?;
    let mut request_line_parts = request_line.split_whitespace();
    let method = request_line_parts
        .next()
        .and_then(|value| HttpMethod::from_str(value).ok())
        .ok_or_else(|| format!("Unable to parse HTTP method from '{request_line}'."))?;
    let uri = request_line_parts
        .next()
        .ok_or_else(|| format!("Unable to parse request URI from '{request_line}'."))?;
    let http_version = request_line_parts
        .next()
        .unwrap_or("HTTP/1.1")
        .trim_start_matches("HTTP/")
        .to_string();
    let header_lines = request_lines.collect::<Vec<_>>();
    let mut headers = parse_header_block(&header_lines.join(CRLF));
    headers
        .entry(String::from("accept"))
        .or_insert_with(|| RequestHeaderValue::Single(NO_METADATA_ACCEPT.to_string()));

    let path = extract_path_from_uri(uri)
        .ok_or_else(|| format!("Unable to parse request path from '{uri}'."))?;
    let query = query_map_from_uri(uri);
    let (endpoint, protocol) = endpoint_from_uri(uri);
    let body = request_body
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    let mut request = GeneratedHttpRequest::with_details(method, uri.to_string(), endpoint, path);
    request.protocol = protocol;
    request.query = query;
    request.headers = headers;
    request.rawHeaders = header_lines.into_iter().map(ToOwned::to_owned).collect();
    request.body = body.clone();
    request.bodyStream = GeneratedReadableStream::from_string(body.unwrap_or_default());

    Ok(BatchRequestPart {
        content_id,
        http_version,
        request,
    })
}

fn split_header_block<'a>(value: &'a str, line_ending: &str) -> Option<(&'a str, &'a str)> {
    let separator = format!("{line_ending}{line_ending}");
    value
        .split_once(&separator)
        .or_else(|| value.split_once("\r\n\r\n"))
        .or_else(|| value.split_once("\n\n"))
}

fn extract_batch_boundary(body: &str) -> Option<String> {
    body.lines().find_map(|line| {
        let trimmed = line.trim();
        trimmed
            .strip_prefix("--")
            .map(|value| value.trim_end_matches("--").trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn extract_changeset_boundary(body: &str) -> Option<String> {
    let marker = "boundary=";
    let start = body.find(marker)? + marker.len();
    let remainder = &body[start..];
    let end = remainder
        .find(['\r', '\n', ';', ' ', '\t'])
        .unwrap_or(remainder.len());
    let boundary = remainder[..end].trim_matches('"').trim();
    (!boundary.is_empty()).then(|| boundary.to_string())
}

fn endpoint_from_uri(uri: &str) -> (String, String) {
    if let Ok(url) = Url::parse(uri) {
        let mut endpoint = format!("{}://{}", url.scheme(), url.host_str().unwrap_or_default());
        if let Some(port) = url.port() {
            endpoint.push_str(&format!(":{port}"));
        }
        return (endpoint, url.scheme().to_string());
    }

    (String::new(), String::from("http"))
}

fn response_boundary(boundary: &str, request_token: &str, response_token: &str) -> String {
    if boundary.starts_with(request_token) {
        boundary.replacen(request_token, response_token, 1)
    } else {
        boundary.to_string()
    }
}

fn data_service_version(request: &BatchRequestPart) -> &'static str {
    if request.request.getMethod() == HttpMethod::DELETE {
        "1.0"
    } else {
        "3.0"
    }
}

fn response_content_type(response: &GeneratedResponse, request: &GeneratedHttpRequest) -> String {
    let base = response
        .contentType
        .clone()
        .or_else(|| request.getHeader("accept"))
        .unwrap_or_else(|| NO_METADATA_ACCEPT.to_string());

    if base.contains("charset=") {
        base
    } else {
        format!("{base};streaming=true;charset=utf-8")
    }
}

fn entity_location(request: &GeneratedHttpRequest) -> Option<String> {
    let url = request.getUrl();
    let (partition_key, row_key) =
        extract_entity_keys_from_url_or_body(&url, request.getBody().as_deref());
    let partition_key = partition_key?;
    let row_key = row_key?;
    let base_uri = url
        .split('?')
        .next()
        .unwrap_or(url.as_str())
        .split('(')
        .next()
        .unwrap_or(url.as_str())
        .to_string();
    let partition_key =
        url::form_urlencoded::byte_serialize(partition_key.as_bytes()).collect::<String>();
    let row_key = url::form_urlencoded::byte_serialize(row_key.as_bytes()).collect::<String>();
    Some(format!(
        "{base_uri}(PartitionKey='{partition_key}',RowKey='{row_key}')"
    ))
}

fn storage_error_body(error: &StorageError, zero_based_index: usize) -> String {
    let body = error
        .body
        .as_ref()
        .map(|value| match value {
            GeneratedValue::String(value) => value.clone(),
            _ => serde_json::to_string(&value.to_json_value()).unwrap_or_default(),
        })
        .unwrap_or_else(|| {
            json!({
                "odata.error": {
                    "code": error.storageErrorCode.clone(),
                    "message": {
                        "lang": "en-US",
                        "value": error.message.clone(),
                    }
                }
            })
            .to_string()
        });

    if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&body) {
        let current_message = json
            .pointer("/odata.error/message/value")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned);
        if let Some(current_message) = current_message {
            if let Some(message_target) = json.pointer_mut("/odata.error/message/value") {
                *message_target =
                    serde_json::Value::String(format!("{zero_based_index}:{current_message}"));
                return json.to_string();
            }
        }
    }

    body
}

fn status_message(status_code: u16) -> &'static str {
    match status_code {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        409 => "Conflict",
        412 => "Precondition Failed",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        501 => "Not Implemented",
        _ => "STATUS_CODE_NOT_IMPLEMENTED",
    }
}

#[cfg(test)]
mod tests {
    use super::parse_batch_request;
    use crate::generated::i_request::IRequest;

    #[test]
    fn parses_insert_batch_request() {
        let body = "--batch_test\r\nContent-Type: multipart/mixed; boundary=changeset_test\r\n\r\n--changeset_test\r\nContent-Type: application/http\r\nContent-Transfer-Encoding: binary\r\n\r\nPOST http://127.0.0.1:10002/devstoreaccount1/Customers HTTP/1.1\r\nAccept: application/json;odata=nometadata\r\nContent-Type: application/json\r\n\r\n{\"PartitionKey\":\"pk\",\"RowKey\":\"rk\"}\r\n--changeset_test--\r\n--batch_test--\r\n";

        let parsed = parse_batch_request(body).unwrap();
        assert_eq!(parsed.batch_boundary, "batch_test");
        assert_eq!(parsed.changeset_boundary, "changeset_test");
        assert_eq!(parsed.requests.len(), 1);
        assert_eq!(parsed.requests[0].request.getMethod().to_string(), "POST");
        assert_eq!(
            parsed.requests[0].request.getPath(),
            "/devstoreaccount1/Customers"
        );
        assert_eq!(
            parsed.requests[0].request.getHeader("accept").as_deref(),
            Some("application/json;odata=nometadata")
        );
    }
}
