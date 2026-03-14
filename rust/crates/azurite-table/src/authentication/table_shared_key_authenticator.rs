use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use azurite_common::i_account_data_store::IAccountDataStore;
use azurite_common::i_logger::ILogger;
use azurite_common::utils::utils::{computeHMACSHA256, getURLQueries};

use crate::context::TableStorageContext;
use crate::errors::{StorageError, StorageErrorFactory};
use crate::generated::context::Context;
use crate::generated::i_request::{GeneratedHttpRequest, IRequest};

use super::i_authenticator::IAuthenticator;

const AUTHORIZATION: &str = "authorization";
const CONTENT_MD5: &str = "content-md5";
const CONTENT_TYPE: &str = "content-type";
const CONTENT_LENGTH: &str = "content-length";
const DATE: &str = "date";
const X_MS_DATE: &str = "x-ms-date";
const SECONDARY_SUFFIX: &str = "-secondary";

pub struct TableSharedKeyAuthenticator {
    dataStore: Arc<dyn IAccountDataStore + Send + Sync>,
    _logger: Arc<dyn ILogger + Send + Sync>,
}

impl TableSharedKeyAuthenticator {
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
        if headerName == CONTENT_LENGTH && value == "0" {
            return String::new();
        }
        value
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
            path = authenticationPath.to_owned();
        }

        let mut canonicalizedResourceString = format!("/{account}{path}");
        let queries = getURLQueries(&request.getUrl());
        let mut lowercaseQueries = BTreeMap::new();
        for (key, value) in queries {
            lowercaseQueries.insert(key.to_ascii_lowercase(), value);
        }
        if let Some(comp) = lowercaseQueries.get("comp") {
            canonicalizedResourceString.push_str(&format!("?comp={comp}"));
        }
        canonicalizedResourceString
    }

    fn date_value_for_signature(&self, req: &GeneratedHttpRequest) -> String {
        let date = self.getHeaderValueToSign(req, DATE);
        if date.is_empty() {
            self.getHeaderValueToSign(req, X_MS_DATE)
        } else {
            date
        }
    }

    fn build_string_to_sign(
        &self,
        req: &GeneratedHttpRequest,
        account: &str,
        authenticationPath: Option<&str>,
    ) -> String {
        [
            req.getMethod().to_string(),
            self.getHeaderValueToSign(req, CONTENT_MD5),
            self.getHeaderValueToSign(req, CONTENT_TYPE),
            self.date_value_for_signature(req),
        ]
        .join("\n")
            + "\n"
            + &self.getCanonicalizedResourceString(req, account, authenticationPath)
    }
}

#[async_trait]
impl IAuthenticator for TableSharedKeyAuthenticator {
    async fn validate(
        &self,
        req: &GeneratedHttpRequest,
        content: &Context,
    ) -> Result<Option<bool>, StorageError> {
        let tableContext = TableStorageContext::new(content);
        let account = tableContext.account().unwrap_or_default();

        let authHeaderValue = match req.getHeader(AUTHORIZATION) {
            Some(value) => value,
            None => return Ok(None),
        };

        let accountProperties = self
            .dataStore
            .getAccount(&account)
            .ok_or_else(|| StorageErrorFactory::ResourceNotFound(content))?;

        let stringToSign =
            self.build_string_to_sign(req, &account, tableContext.authenticationPath().as_deref());
        let signature1 = computeHMACSHA256(&stringToSign, &accountProperties.key1);
        if authHeaderValue == format!("SharedKey {account}:{signature1}") {
            return Ok(Some(true));
        }

        if let Some(key2) = accountProperties.key2.as_deref() {
            let signature2 = computeHMACSHA256(&stringToSign, key2);
            if authHeaderValue == format!("SharedKey {account}:{signature2}") {
                return Ok(Some(true));
            }
        }

        if tableContext.isSecondary().unwrap_or(false)
            && tableContext
                .authenticationPath()
                .as_deref()
                .map(|value| value.find(&account) == Some(1))
                .unwrap_or(false)
        {
            let secondaryPath = tableContext
                .authenticationPath()
                .unwrap_or_default()
                .replacen(&account, &format!("{account}{SECONDARY_SUFFIX}"), 1);
            let stringToSign_secondary =
                self.build_string_to_sign(req, &account, Some(&secondaryPath));
            let signature1_secondary =
                computeHMACSHA256(&stringToSign_secondary, &accountProperties.key1);
            if authHeaderValue == format!("SharedKey {account}:{signature1_secondary}") {
                return Ok(Some(true));
            }

            if let Some(key2) = accountProperties.key2.as_deref() {
                let signature2_secondary = computeHMACSHA256(&stringToSign_secondary, key2);
                if authHeaderValue == format!("SharedKey {account}:{signature2_secondary}") {
                    return Ok(Some(true));
                }
            }
        }

        Ok(Some(false))
    }
}
