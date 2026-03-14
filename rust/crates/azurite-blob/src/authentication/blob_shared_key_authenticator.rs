use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use azurite_common::i_account_data_store::IAccountDataStore;
use azurite_common::i_logger::ILogger;
use azurite_common::utils::utils::{computeHMACSHA256, getURLQueries};

use crate::context::BlobStorageContext;
use crate::errors::{StorageError, StorageErrorFactory};
use crate::generated::artifacts::operation::Operation;
use crate::generated::context::Context;
use crate::generated::i_request::{GeneratedHttpRequest, IRequest, RequestHeaderValue};

use super::i_authenticator::IAuthenticator;

const AUTHORIZATION: &str = "authorization";
const CONTENT_ENCODING: &str = "content-encoding";
const CONTENT_LANGUAGE: &str = "content-language";
const CONTENT_LENGTH: &str = "content-length";
const CONTENT_MD5: &str = "content-md5";
const CONTENT_TYPE: &str = "content-type";
const DATE: &str = "date";
const IF_MODIFIED_SINCE: &str = "if-modified-since";
const IF_MATCH: &str = "if-match";
const IF_NONE_MATCH: &str = "if-none-match";
const IF_UNMODIFIED_SINCE: &str = "if-unmodified-since";
const RANGE: &str = "Range";
const X_MS_PREFIX: &str = "x-ms-";
const AUTHENTICATION_BEARERTOKEN_REQUIRED: &str = "Only authentication scheme Bearer is supported";
const SECONDARY_SUFFIX: &str = "-secondary";

pub struct BlobSharedKeyAuthenticator {
    dataStore: Arc<dyn IAccountDataStore + Send + Sync>,
    logger: Arc<dyn ILogger + Send + Sync>,
}

impl BlobSharedKeyAuthenticator {
    pub fn new(
        dataStore: Arc<dyn IAccountDataStore + Send + Sync>,
        logger: Arc<dyn ILogger + Send + Sync>,
    ) -> Self {
        Self { dataStore, logger }
    }

    fn getHeaderValueToSign(&self, request: &GeneratedHttpRequest, headerName: &str) -> String {
        let value = request.getHeader(headerName);
        if value.is_none() {
            return String::new();
        }
        let value = value.unwrap_or_default();
        if headerName == CONTENT_LENGTH && value == "0" {
            return String::new();
        }
        value
    }

    fn getCanonicalizedHeadersString(&self, request: &GeneratedHttpRequest) -> String {
        let mut headers: Vec<(String, String)> = Vec::new();
        for (name, value) in request.getHeaders() {
            let value = match value {
                RequestHeaderValue::Single(value) => value,
                RequestHeaderValue::Multi(values) => values.join(","),
            };
            headers.push((name, value));
        }

        headers.retain(|(name, _)| name.to_ascii_lowercase().starts_with(X_MS_PREFIX));
        headers.sort_by_key(|a| a.0.to_ascii_lowercase());

        let mut canonicalizedHeadersStringToSign = String::new();
        for (name, value) in headers {
            canonicalizedHeadersStringToSign.push_str(&format!(
                "{}:{}\n",
                name.to_ascii_lowercase().trim_end(),
                value.trim_start()
            ));
        }
        canonicalizedHeadersStringToSign
    }

    fn getCanonicalizedResourceString(
        &self,
        request: &GeneratedHttpRequest,
        account: &str,
        authenticationPath: Option<&str>,
    ) -> String {
        let mut path = request.getPath();
        if path.is_empty() {
            path = String::from("/");
        }
        if let Some(authenticationPath) = authenticationPath {
            path = String::from(authenticationPath);
        }

        let mut canonicalizedResourceString = format!("/{account}{path}");

        let queries = getURLQueries(&request.getUrl());
        let mut lowercaseQueries = BTreeMap::new();
        for (key, value) in queries {
            lowercaseQueries.insert(key.to_ascii_lowercase(), value);
        }

        for (key, value) in lowercaseQueries {
            canonicalizedResourceString.push_str(&format!(
                "\n{key}:{}",
                decode_uri_component(&value.replace('+', "%20"))
            ));
        }

        canonicalizedResourceString
    }

    fn build_string_to_sign(
        &self,
        req: &GeneratedHttpRequest,
        account: &str,
        authenticationPath: Option<&str>,
    ) -> String {
        [
            req.getMethod().to_string(),
            self.getHeaderValueToSign(req, CONTENT_ENCODING),
            self.getHeaderValueToSign(req, CONTENT_LANGUAGE),
            self.getHeaderValueToSign(req, CONTENT_LENGTH),
            self.getHeaderValueToSign(req, CONTENT_MD5),
            self.getHeaderValueToSign(req, CONTENT_TYPE),
            self.getHeaderValueToSign(req, DATE),
            self.getHeaderValueToSign(req, IF_MODIFIED_SINCE),
            self.getHeaderValueToSign(req, IF_MATCH),
            self.getHeaderValueToSign(req, IF_NONE_MATCH),
            self.getHeaderValueToSign(req, IF_UNMODIFIED_SINCE),
            self.getHeaderValueToSign(req, RANGE),
        ]
        .join("\n")
            + "\n"
            + &self.getCanonicalizedHeadersString(req)
            + &self.getCanonicalizedResourceString(req, account, authenticationPath)
    }
}

#[async_trait]
impl IAuthenticator for BlobSharedKeyAuthenticator {
    async fn validate(
        &self,
        req: &GeneratedHttpRequest,
        content: &Context,
    ) -> Result<Option<bool>, StorageError> {
        let blobContext = BlobStorageContext::new(content);
        let account = blobContext.account().unwrap_or_default();

        self.logger.info(
            "BlobSharedKeyAuthenticator:validate() Start validation against account shared key authentication.",
            blobContext.contextId().as_deref(),
        );

        let authHeaderValue = req.getHeader(AUTHORIZATION);
        if authHeaderValue.is_none() {
            self.logger.info(
                "BlobSharedKeyAuthenticator:validate() Request doesn't include valid authentication header. Skip shared key authentication.",
                blobContext.contextId().as_deref(),
            );
            return Ok(None);
        }
        let authHeaderValue = authHeaderValue.unwrap_or_default();
        if !authHeaderValue.starts_with("SharedKey") {
            self.logger.info(
                "BlobSharedKeyAuthenticator:validate() Request doesn't include shared key authentication.",
                blobContext.contextId().as_deref(),
            );
            return Ok(None);
        }

        let accountProperties = self.dataStore.getAccount(&account);
        if accountProperties.is_none() {
            self.logger.error(
                &format!(
                    "BlobSharedKeyAuthenticator:validate() Invalid storage account {account}."
                ),
                blobContext.contextId().as_deref(),
            );
            return Err(StorageErrorFactory::ResourceNotFound(
                blobContext.contextId().as_deref(),
            ));
        }
        let accountProperties = accountProperties.unwrap();

        let operation = blobContext.operation().unwrap_or_else(|| {
            panic!(
                "BlobSharedKeyAuthenticator:validate() Operation shouldn't be undefined. Please make sure DispatchMiddleware is hooked before authentication related middleware."
            )
        });
        if operation == Operation::Service_GetUserDelegationKey {
            self.logger.info(
                "BlobSharedKeyAuthenticator:validate() Service_GetUserDelegationKey requires OAuth credentials.",
                blobContext.contextId().as_deref(),
            );
            return Err(StorageErrorFactory::getAuthenticationFailed(
                blobContext.contextId().as_deref(),
                AUTHENTICATION_BEARERTOKEN_REQUIRED,
            ));
        }

        let stringToSign =
            self.build_string_to_sign(req, &account, blobContext.authenticationPath().as_deref());

        let signature1 = computeHMACSHA256(&stringToSign, &accountProperties.key1);
        let authValue1 = format!("SharedKey {account}:{signature1}");
        if authHeaderValue == authValue1 {
            return Ok(Some(true));
        }

        if let Some(key2) = accountProperties.key2.as_deref() {
            let signature2 = computeHMACSHA256(&stringToSign, key2);
            let authValue2 = format!("SharedKey {account}:{signature2}");
            if authHeaderValue == authValue2 {
                return Ok(Some(true));
            }
        }

        if blobContext.isSecondary().unwrap_or(false)
            && blobContext
                .authenticationPath()
                .as_deref()
                .map(|value| value.find(&account) == Some(1))
                .unwrap_or(false)
        {
            let secondaryPath = blobContext
                .authenticationPath()
                .unwrap_or_default()
                .replacen(&account, &format!("{account}{SECONDARY_SUFFIX}"), 1);
            let stringToSign_secondary =
                self.build_string_to_sign(req, &account, Some(&secondaryPath));
            let signature1_secondary =
                computeHMACSHA256(&stringToSign_secondary, &accountProperties.key1);
            let authValue1_secondary = format!("SharedKey {account}:{signature1_secondary}");
            if authHeaderValue == authValue1_secondary {
                return Ok(Some(true));
            }

            if let Some(key2) = accountProperties.key2.as_deref() {
                let signature2_secondary = computeHMACSHA256(&stringToSign_secondary, key2);
                let authValue2_secondary = format!("SharedKey {account}:{signature2_secondary}");
                if authHeaderValue == authValue2_secondary {
                    return Ok(Some(true));
                }
            }
        }

        Ok(Some(false))
    }
}

fn decode_uri_component(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut index = 0;
    let mut result = Vec::with_capacity(bytes.len());
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) =
                (from_hex(bytes[index + 1]), from_hex(bytes[index + 2]))
            {
                result.push(high * 16 + low);
                index += 3;
                continue;
            }
        }
        result.push(bytes[index]);
        index += 1;
    }
    String::from_utf8(result).unwrap_or_default()
}

fn from_hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}
