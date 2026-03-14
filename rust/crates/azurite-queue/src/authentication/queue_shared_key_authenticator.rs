use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use azurite_common::i_account_data_store::IAccountDataStore;
use azurite_common::i_logger::ILogger;
use azurite_common::utils::utils::{computeHMACSHA256, getURLQueries};

use crate::context::QueueStorageContext;
use crate::errors::{StorageError, StorageErrorFactory};
use crate::generated::context::Context;
use crate::generated::i_request::{GeneratedHttpRequest, IRequest, RequestHeaderValue};
use crate::utils::constants::{HeaderConstants, SECONDARY_SUFFIX};

use super::i_authenticator::IAuthenticator;

const DEFAULT_CONTEXT_ID: &str = "DefaultID";

pub struct QueueSharedKeyAuthenticator {
    dataStore: Arc<dyn IAccountDataStore + Send + Sync>,
    _logger: Arc<dyn ILogger + Send + Sync>,
}

impl QueueSharedKeyAuthenticator {
    pub fn new(
        dataStore: Arc<dyn IAccountDataStore + Send + Sync>,
        logger: Arc<dyn ILogger + Send + Sync>,
    ) -> Self {
        Self {
            dataStore,
            _logger: logger,
        }
    }

    fn getHeaderValueToSign(&self, request: &GeneratedHttpRequest, headerName: &str) -> String {
        let value = request.getHeader(headerName);
        if value.is_none() {
            return String::new();
        }
        let value = value.unwrap_or_default();
        if headerName == HeaderConstants.CONTENT_LENGTH && value == "0" {
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

        headers.retain(|(name, _)| {
            name.to_ascii_lowercase()
                .starts_with(HeaderConstants.PREFIX_FOR_STORAGE)
        });
        headers.sort_by_key(|(name, _)| name.to_ascii_lowercase());

        let mut canonicalized_headers = String::new();
        for (name, value) in headers {
            canonicalized_headers.push_str(&format!(
                "{}:{}\n",
                name.to_ascii_lowercase().trim_end(),
                value.trim_start()
            ));
        }

        canonicalized_headers
    }

    fn getCanonicalizedResourceString(
        &self,
        auth_type: &str,
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

        let mut canonicalized_resource = format!("/{account}{path}");
        let queries = getURLQueries(&request.getUrl());

        if auth_type == "SharedKey" {
            let mut lowercase_queries = BTreeMap::new();
            for (key, value) in queries {
                lowercase_queries.insert(key.to_ascii_lowercase(), value);
            }
            for (key, value) in lowercase_queries {
                canonicalized_resource.push_str(&format!(
                    "\n{key}:{}",
                    decode_uri_component(&value.replace('+', "%20"))
                ));
            }
        } else if auth_type == "SharedKeyLite" {
            for (key, value) in queries {
                if key.eq_ignore_ascii_case("comp") {
                    canonicalized_resource.push_str(&format!(
                        "?comp={}",
                        decode_uri_component(&value.replace('+', "%20"))
                    ));
                }
            }
        }

        canonicalized_resource
    }

    fn getHeadersToSign(&self, auth_type: &str, req: &GeneratedHttpRequest) -> String {
        if auth_type == "SharedKey" {
            return [
                req.getMethod().to_string(),
                self.getHeaderValueToSign(req, HeaderConstants.CONTENT_LANGUAGE),
                self.getHeaderValueToSign(req, HeaderConstants.CONTENT_ENCODING),
                self.getHeaderValueToSign(req, HeaderConstants.CONTENT_LENGTH),
                self.getHeaderValueToSign(req, HeaderConstants.CONTENT_MD5),
                self.getHeaderValueToSign(req, HeaderConstants.CONTENT_TYPE),
                self.getHeaderValueToSign(req, HeaderConstants.DATE),
                self.getHeaderValueToSign(req, HeaderConstants.IF_MODIFIED_SINCE),
                self.getHeaderValueToSign(req, HeaderConstants.IF_MATCH),
                self.getHeaderValueToSign(req, HeaderConstants.IF_NONE_MATCH),
                self.getHeaderValueToSign(req, HeaderConstants.IF_UNMODIFIED_SINCE),
                self.getHeaderValueToSign(req, HeaderConstants.RANGE),
            ]
            .join("\n")
                + "\n"
                + &self.getCanonicalizedHeadersString(req);
        }

        if auth_type == "SharedKeyLite" {
            return [
                req.getMethod().to_string(),
                self.getHeaderValueToSign(req, HeaderConstants.CONTENT_MD5),
                self.getHeaderValueToSign(req, HeaderConstants.CONTENT_TYPE),
                self.getHeaderValueToSign(req, HeaderConstants.DATE),
            ]
            .join("\n")
                + "\n"
                + &self.getCanonicalizedHeadersString(req);
        }

        String::new()
    }

    fn build_string_to_sign(
        &self,
        auth_type: &str,
        req: &GeneratedHttpRequest,
        account: &str,
        authenticationPath: Option<&str>,
    ) -> String {
        self.getHeadersToSign(auth_type, req)
            + &self.getCanonicalizedResourceString(auth_type, req, account, authenticationPath)
    }
}

#[async_trait]
impl IAuthenticator for QueueSharedKeyAuthenticator {
    async fn validate(
        &self,
        req: &GeneratedHttpRequest,
        context: &Context,
    ) -> Result<Option<bool>, StorageError> {
        let queueContext = QueueStorageContext::new(context);
        let account = queueContext.account().unwrap_or_default();

        let authHeaderValue = match req.getHeader(HeaderConstants.AUTHORIZATION) {
            Some(value) => value,
            None => return Ok(None),
        };

        let accountProperties = self.dataStore.getAccount(&account).ok_or_else(|| {
            StorageErrorFactory::ResourceNotFound(
                queueContext
                    .contextId()
                    .as_deref()
                    .unwrap_or(DEFAULT_CONTEXT_ID),
            )
        })?;

        let mut split = authHeaderValue.splitn(2, ' ');
        let authType = split.next().unwrap_or_default();
        let authValue = split.next().unwrap_or_default();
        let stringToSign = self.build_string_to_sign(
            authType,
            req,
            &account,
            queueContext.authenticationPath().as_deref(),
        );

        let signature1 = computeHMACSHA256(&stringToSign, &accountProperties.key1);
        if authValue == format!("{account}:{signature1}") {
            return Ok(Some(true));
        }

        if let Some(key2) = accountProperties.key2.as_deref() {
            let signature2 = computeHMACSHA256(&stringToSign, key2);
            if authValue == format!("{account}:{signature2}") {
                return Ok(Some(true));
            }
        }

        if queueContext.isSecondary().unwrap_or(false)
            && queueContext
                .authenticationPath()
                .as_deref()
                .map(|value| value.find(&account) == Some(1))
                .unwrap_or(false)
        {
            let secondaryPath = queueContext
                .authenticationPath()
                .unwrap_or_default()
                .replacen(&account, &format!("{account}{SECONDARY_SUFFIX}"), 1);
            let stringToSign_secondary =
                self.build_string_to_sign(authType, req, &account, Some(&secondaryPath));

            let signature1_secondary =
                computeHMACSHA256(&stringToSign_secondary, &accountProperties.key1);
            if authValue == format!("{account}:{signature1_secondary}") {
                return Ok(Some(true));
            }

            if let Some(key2) = accountProperties.key2.as_deref() {
                let signature2_secondary = computeHMACSHA256(&stringToSign_secondary, key2);
                if authValue == format!("{account}:{signature2_secondary}") {
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;

    use azurite_common::i_account_data_store::{IAccountDataStore, IAccountProperties};
    use azurite_common::i_cleaner::ICleaner;
    use azurite_common::i_data_store::IDataStore;
    use azurite_common::i_logger::ILogger;
    use azurite_common::storage_error::StorageError as CommonStorageError;

    use crate::generated::i_request::{GeneratedHttpRequest, HttpMethod, RequestHeaderValue};

    use super::QueueSharedKeyAuthenticator;

    #[derive(Default)]
    struct TestAccountStore {
        account: Option<IAccountProperties>,
    }

    #[async_trait]
    impl IDataStore for TestAccountStore {
        async fn init(&mut self) -> Result<(), CommonStorageError> {
            Ok(())
        }

        fn isInitialized(&self) -> bool {
            true
        }

        async fn close(&mut self) -> Result<(), CommonStorageError> {
            Ok(())
        }

        fn isClosed(&self) -> bool {
            false
        }
    }

    #[async_trait]
    impl ICleaner for TestAccountStore {
        async fn clean(&mut self) -> Result<(), CommonStorageError> {
            Ok(())
        }
    }

    #[async_trait]
    impl IAccountDataStore for TestAccountStore {
        fn getAccount(&self, name: &str) -> Option<IAccountProperties> {
            self.account
                .as_ref()
                .filter(|account| account.name == name)
                .cloned()
        }
    }

    #[derive(Default)]
    struct TestLogger;

    impl ILogger for TestLogger {
        fn error(&self, _message: &str, _contextID: Option<&str>) {}
        fn warn(&self, _message: &str, _contextID: Option<&str>) {}
        fn info(&self, _message: &str, _contextID: Option<&str>) {}
        fn verbose(&self, _message: &str, _contextID: Option<&str>) {}
        fn debug(&self, _message: &str, _contextID: Option<&str>) {}
    }

    #[test]
    fn shared_key_signing_uses_queue_header_order() {
        let authenticator = QueueSharedKeyAuthenticator::new(
            Arc::new(TestAccountStore::default()),
            Arc::new(TestLogger),
        );

        let mut request = GeneratedHttpRequest::new(
            HttpMethod::PUT,
            "http://127.0.0.1/devstoreaccount1/queue?comp=metadata",
            "http://127.0.0.1:10001",
            "/queue",
        );
        request.headers.insert(
            String::from("content-language"),
            RequestHeaderValue::Single(String::from("en-US")),
        );
        request.headers.insert(
            String::from("content-encoding"),
            RequestHeaderValue::Single(String::from("gzip")),
        );
        request.headers.insert(
            String::from("content-length"),
            RequestHeaderValue::Single(String::from("0")),
        );
        request.headers.insert(
            String::from("x-ms-date"),
            RequestHeaderValue::Single(String::from("Mon, 01 Jan 2024 00:00:00 GMT")),
        );

        let string_to_sign = authenticator.build_string_to_sign(
            "SharedKey",
            &request,
            "devstoreaccount1",
            Some("/queue"),
        );

        assert_eq!(
            string_to_sign,
            "PUT\nen-US\ngzip\n\n\n\n\n\n\n\n\n\nx-ms-date:Mon, 01 Jan 2024 00:00:00 GMT\n/devstoreaccount1/queue\ncomp:metadata"
        );
    }
}
