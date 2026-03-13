use std::{
    collections::HashMap,
    io,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, SecondsFormat, Utc};
use hmac::{Hmac, Mac};
use md5::{Digest, Md5};
use sha2::Sha256;
use tokio::{fs, io::AsyncRead, io::AsyncReadExt};

use crate::{storage_error::StorageError, utils::constants::VALID_CSHARP_IDENTIFIER_REGEX};

#[allow(non_upper_case_globals)]
pub const lfsa: &str = "lokijs/src/loki-fs-structured-adapter.js";

#[allow(non_snake_case)]
pub async fn rimrafAsync<P>(path: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    match fs::metadata(path.as_ref()).await {
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(path).await,
        Ok(_) => fs::remove_file(path).await,
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[allow(non_snake_case)]
pub fn minDate(date1: DateTime<Utc>, date2: DateTime<Utc>) -> DateTime<Utc> {
    if date1 > date2 {
        date2
    } else {
        date1
    }
}

#[allow(non_snake_case)]
pub fn convertDateTimeStringMsTo7Digital(dateTimeString: &str) -> String {
    dateTimeString.replace('Z', "0000Z")
}

#[allow(non_snake_case)]
pub fn convertRawHeadersToMetadata(
    rawHeaders: &[String],
    contextId: &str,
) -> Result<Option<HashMap<String, String>>, StorageError> {
    let metadataPrefix = "x-ms-meta-";
    let mut res = HashMap::new();
    let mut isEmpty = true;
    let mut index = 0;

    while index < rawHeaders.len() {
        let header = &rawHeaders[index];
        if header.to_ascii_lowercase().starts_with(metadataPrefix)
            && header.len() > metadataPrefix.len()
        {
            let key = header[metadataPrefix.len()..].to_string();
            if !VALID_CSHARP_IDENTIFIER_REGEX.is_match(&key) {
                return Err(StorageError::invalid_metadata(contextId));
            }
            let mut value = rawHeaders.get(index + 1).cloned().unwrap_or_default();
            if let Some(existing) = res.get(&key) {
                value = format!("{existing},{value}");
            }
            res.insert(key, value);
            isEmpty = false;
        }
        index += 2;
    }

    if isEmpty {
        Ok(None)
    } else {
        Ok(Some(res))
    }
}

#[allow(non_snake_case)]
pub fn newEtag() -> String {
    let now = Utc::now().timestamp_millis() as u64;
    let random = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    let multiplier = random % 30001 + 70000;
    format!("\"0x{:X}\"", now.saturating_mul(multiplier))
}

#[allow(non_snake_case)]
pub fn computeHMACSHA256(stringToSign: &str, key: &[u8]) -> String {
    let mut hmac = Hmac::<Sha256>::new_from_slice(key).expect("sha256 accepts any key length");
    hmac.update(stringToSign.as_bytes());
    STANDARD.encode(hmac.finalize().into_bytes())
}

#[allow(non_snake_case)]
pub fn truncatedISO8061Date(
    date: DateTime<Utc>,
    withMilliseconds: bool,
    hrtimePrecision: bool,
) -> String {
    let dateString = date.to_rfc3339_opts(SecondsFormat::Millis, true);
    if hrtimePrecision {
        let hrtime = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos()
            .to_string();
        let padded = format!("{:0>4}", hrtime);
        return format!("{}{}Z", &dateString[..dateString.len() - 1], &padded[..4]);
    }

    if withMilliseconds {
        format!("{}0000Z", &dateString[..dateString.len() - 1])
    } else {
        format!("{}Z", &dateString[..dateString.len() - 5])
    }
}

#[allow(non_snake_case)]
pub fn getURLQueries(url: &str) -> HashMap<String, String> {
    let mut queries = HashMap::new();
    let Some((_, queryString)) = url.split_once('?') else {
        return queries;
    };

    let queryString = queryString.trim();
    let queryString = queryString.strip_prefix('?').unwrap_or(queryString);
    for querySubString in queryString.split('&') {
        let indexOfEqual = querySubString.find('=');
        let lastIndexOfEqual = querySubString.rfind('=');
        if let (Some(indexOfEqual), Some(lastIndexOfEqual)) = (indexOfEqual, lastIndexOfEqual) {
            if indexOfEqual > 0 && indexOfEqual == lastIndexOfEqual {
                let mut splitResults = querySubString.splitn(2, '=');
                let key = splitResults.next().unwrap_or_default();
                let value = splitResults.next().unwrap_or_default();
                queries.insert(key.to_string(), value.to_string());
            }
        }
    }

    queries
}

#[allow(non_snake_case)]
pub async fn getMD5FromString(text: &str) -> Vec<u8> {
    let mut hash = Md5::new();
    hash.update(text.as_bytes());
    hash.finalize().to_vec()
}

#[allow(non_snake_case)]
pub async fn getMD5FromStream<R>(mut stream: R) -> io::Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    let mut hash = Md5::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = stream.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(hash.finalize().to_vec())
}
