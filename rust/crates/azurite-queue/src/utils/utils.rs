use std::{future::poll_fn, task::Poll};

use azurite_common::{persistence::i_extent_store::ReadableStream, StorageError};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, Utc};
use regex::Regex;
use serde_json::Value;
use tokio::io::{AsyncRead, ReadBuf};
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RangeError {
    pub message: String,
}

impl RangeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for RangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RangeError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseXMLwithEmptyError {
    pub message: String,
}

impl ParseXMLwithEmptyError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ParseXMLwithEmptyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseXMLwithEmptyError {}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum nameValidateCode {
    valid = 0,
    outOfRange = 1,
    invalidUri = 2,
    invalidName = 3,
}

pub fn checkApiVersion(
    inputApiVersion: &str,
    validApiVersions: &[&str],
    requestId: &str,
) -> Result<(), StorageError> {
    let _ = requestId;
    if !validApiVersions.contains(&inputApiVersion) {
        return Err(StorageError::new(format!(
            "The API version {inputApiVersion} is not supported by Azurite. Please upgrade Azurite to latest version and retry. If you are using Azurite in Visual Studio, please check you have installed latest Visual Studio patch. Azurite command line parameter \"--skipApiVersionCheck\" or Visual Studio Code configuration \"Skip Api Version Check\" can skip this error. "
        )));
    }

    Ok(())
}

/**
 * Default range value [0, Infinite] will be returned if all parameters not provided.
 *
 * @export
 * @param {string} [rangeHeaderValue]
 * @param {string} [xMsRangeHeaderValue]
 * @returns {[number, number]}
 */
pub fn deserializeRangeHeader(
    rangeHeaderValue: Option<&str>,
    xMsRangeHeaderValue: Option<&str>,
) -> Result<(f64, f64), RangeError> {
    let range = xMsRangeHeaderValue.or(rangeHeaderValue);
    let Some(range) = range else {
        return Ok((0.0, f64::INFINITY));
    };

    let parts: Vec<&str> = range.split('=').collect();
    if parts.len() != 2 {
        return Err(RangeError::new(format!(
            "deserializeRangeHeader: raw range value {range} is wrong."
        )));
    }

    let parts: Vec<&str> = parts[1].split('-').collect();
    if parts.is_empty() || parts.len() > 2 {
        return Err(RangeError::new(format!(
            "deserializeRangeHeader: raw range value {range} is wrong."
        )));
    }

    let startInclusive = parse_js_int(parts[0]);
    let mut endInclusive = f64::INFINITY;

    if parts.len() > 1 && !parts[1].is_empty() {
        endInclusive = parse_js_int(parts[1]);
    }

    if startInclusive > endInclusive {
        return Err(RangeError::new(format!(
            "deserializeRangeHeader: raw range value {range} is wrong."
        )));
    }

    Ok((startInclusive, endInclusive))
}

/**
 * validate the input name for queue or container.
 * @see https://docs.microsoft.com/en-us/rest/api/storageservices/naming-queues-and-metadata
 *
 * @export
 * @param {string} name
 * @returns {nameValidateCode} //0 for valid, 1 for outOfRange, 2 for invalid.
 */
pub fn isValidName(name: &str) -> nameValidateCode {
    if name.is_empty() {
        return nameValidateCode::invalidName;
    }
    if name.len() < 3 || name.len() > 63 {
        return nameValidateCode::outOfRange;
    }
    if name.split('-').any(str::is_empty) {
        return nameValidateCode::invalidName;
    }

    let reg = Regex::new(r"^[0-9|a-z|-]*$").unwrap();
    if reg.is_match(name) {
        return nameValidateCode::valid;
    }

    nameValidateCode::invalidName
}

/**
 * Generate a random code with given length
 *
 * @public
 * @param {number} len
 * @returns {string}
 * @memberof LokiQueueDataStore
 */
pub fn randomValueHex(len: usize) -> String {
    let mut value = String::new();
    while value.len() < len {
        value.push_str(&Uuid::new_v4().simple().to_string());
    }
    value.truncate(len);
    value
}

/**
 * Generate the popreceipt for a get messages request.
 *
 * @public
 * @param {Date} requestDate
 * @returns {string}
 * @memberof LokiQueueDataStore
 */
pub fn getPopReceipt(requestDate: DateTime<Utc>) -> String {
    let encodedStr = format!(
        "{}{}",
        requestDate.format("%d%b%Y%H:%M:%S"),
        randomValueHex(4)
    );
    STANDARD.encode(encodedStr)
}

/**
 * Read the text from a readStream to a string.
 *
 * @export
 * @param {NodeJS.ReadableStream} data
 * @returns {Promise<string>}
 */
pub async fn readStreamToString(mut data: ReadableStream) -> std::io::Result<String> {
    let mut res = Vec::new();
    let mut chunk = [0u8; 8192];

    loop {
        let read = poll_fn(|cx| {
            let mut readBuf = ReadBuf::new(&mut chunk);
            match AsyncRead::poll_read(data.as_mut(), cx, &mut readBuf) {
                Poll::Ready(Ok(())) => Poll::Ready(Ok(readBuf.filled().len())),
                Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
                Poll::Pending => Poll::Pending,
            }
        })
        .await?;

        if read == 0 {
            break;
        }

        res.extend_from_slice(&chunk[..read]);
    }

    Ok(String::from_utf8_lossy(&res).into_owned())
}

/**
 * Get the byte size of a string in UTF8.
 *
 * @public
 * @param {Date} requestDate
 * @returns {string}
 * @memberof LokiQueueDataStore
 */
pub fn getUTF8ByteSize(text: &str) -> usize {
    text.as_bytes().len()
}

/**
 * Retrieve the value from XML body without ignoring the empty characters.
 *
 * @export
 * @param {string} param
 * @param {boolean} [explicitChildrenWithOrder=false]
 * @returns {Promise<any>}
 */
pub fn parseXMLwithEmpty(
    param: &str,
    explicitChildrenWithOrder: bool,
) -> Result<Value, ParseXMLwithEmptyError> {
    use quick_xml::{escape::unescape, events::Event, Reader};

    let _ = explicitChildrenWithOrder;
    let mut reader = Reader::from_str(param);
    reader.config_mut().trim_text(false);

    let mut stack: Vec<String> = Vec::new();
    let mut values = serde_json::Map::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                let tag = String::from_utf8_lossy(event.name().as_ref()).to_string();
                if stack.len() == 1 {
                    values.insert(tag.clone(), Value::String(String::new()));
                }
                stack.push(tag);
            }
            Ok(Event::Empty(event)) => {
                let tag = String::from_utf8_lossy(event.name().as_ref()).to_string();
                if stack.len() == 1 {
                    values.insert(tag, Value::String(String::new()));
                }
            }
            Ok(Event::Text(event)) => {
                if stack.len() == 2 {
                    let text = unescape(&String::from_utf8_lossy(event.as_ref()))
                        .map_err(|error| ParseXMLwithEmptyError::new(error.to_string()))?
                        .into_owned();
                    if let Some(tag) = stack.last() {
                        values.insert(tag.clone(), Value::String(text));
                    }
                }
            }
            Ok(Event::End(_)) => {
                stack.pop();
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => return Err(ParseXMLwithEmptyError::new(error.to_string())),
        }
    }

    Ok(Value::Object(values))
}

fn parse_js_int(value: &str) -> f64 {
    let trimmed = value.trim_start();
    let mut chars = trimmed.chars().peekable();
    let mut sign = 1.0;

    if matches!(chars.peek(), Some('+')) {
        chars.next();
    } else if matches!(chars.peek(), Some('-')) {
        chars.next();
        sign = -1.0;
    }

    let digits: String = chars.take_while(|ch| ch.is_ascii_digit()).collect();
    if digits.is_empty() {
        return f64::NAN;
    }

    sign * digits.parse::<f64>().unwrap()
}

#[cfg(test)]
mod tests {
    use azurite_common::persistence::i_extent_store::ReadableStream;
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use chrono::TimeZone;
    use serde_json::json;
    use tokio::io::AsyncWriteExt;

    use super::{
        deserializeRangeHeader, getPopReceipt, getUTF8ByteSize, isValidName, nameValidateCode,
        parseXMLwithEmpty, randomValueHex, readStreamToString,
    };

    #[test]
    fn deserializeRangeHeader_returns_default_range() {
        let (start, end) = deserializeRangeHeader(None, None).unwrap();
        assert_eq!(start, 0.0);
        assert!(end.is_infinite());
    }

    #[test]
    fn deserializeRangeHeader_parses_explicit_range() {
        let (start, end) = deserializeRangeHeader(Some("bytes=0-1023"), None).unwrap();
        assert_eq!(start, 0.0);
        assert_eq!(end, 1023.0);
    }

    #[test]
    fn isValidName_matches_typescript_rules() {
        assert_eq!(isValidName("abc"), nameValidateCode::valid);
        assert_eq!(isValidName("ab"), nameValidateCode::outOfRange);
        assert_eq!(isValidName("ab--cd"), nameValidateCode::invalidName);
    }

    #[test]
    fn randomValueHex_returns_requested_length() {
        assert_eq!(randomValueHex(7).len(), 7);
    }

    #[test]
    fn getPopReceipt_encodes_timestamp_prefix() {
        let requestDate = chrono::Utc
            .with_ymd_and_hms(2024, 5, 4, 12, 30, 15)
            .unwrap();
        let decoded =
            String::from_utf8(STANDARD.decode(getPopReceipt(requestDate)).unwrap()).unwrap();
        assert!(decoded.starts_with("04May202412:30:15"));
        assert_eq!(decoded.len(), "04May202412:30:15".len() + 4);
    }

    #[tokio::test]
    async fn readStreamToString_reads_the_full_stream() {
        let (mut writer, reader) = tokio::io::duplex(64);
        tokio::spawn(async move {
            writer.write_all(b"hello world").await.unwrap();
        });
        let stream: ReadableStream = Box::pin(reader);
        let content = readStreamToString(stream).await.unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn getUTF8ByteSize_counts_utf8_bytes() {
        assert_eq!(getUTF8ByteSize("hé"), 3);
    }

    #[test]
    fn parseXMLwithEmpty_keeps_empty_message_text() {
        let parsed = parseXMLwithEmpty(
            r#"<QueueMessage><MessageText></MessageText></QueueMessage>"#,
            false,
        )
        .unwrap();
        assert_eq!(parsed, json!({ "MessageText": "" }));
    }
}
