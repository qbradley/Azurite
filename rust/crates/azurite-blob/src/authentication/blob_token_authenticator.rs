use std::sync::{Arc, LazyLock};

use async_trait::async_trait;
use azurite_common::i_account_data_store::IAccountDataStore;
use azurite_common::i_logger::ILogger;
use azurite_common::models::OAuthLevel;
use azurite_common::utils::constants::{BEARER_TOKEN_PREFIX, HTTPS, VALID_ISSUE_PREFIXES};
use base64::{engine::general_purpose::URL_SAFE, engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Utc;
use regex::Regex;
use serde_json::Value;

use crate::context::BlobStorageContext;
use crate::errors::{StorageError, StorageErrorFactory};
use crate::generated::artifacts::operation::Operation;
use crate::generated::context::Context;
use crate::generated::i_request::{GeneratedHttpRequest, IRequest};

use super::i_authenticator::IAuthenticator;

const AUTHORIZATION: &str = "authorization";

static VALID_BLOB_AUDIENCES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"^https://storage\.azure\.com[/]?$" ).unwrap(),
        Regex::new(r"^e406a681-f3d4-42a8-90b6-c2b029497af1$" ).unwrap(),
        Regex::new(r"^https://(.*)\.blob\.core\.windows\.net[/]?$" ).unwrap(),
        Regex::new(r"^https://(.*)\.blob\.core\.chinacloudapi\.cn[/]?$" ).unwrap(),
        Regex::new(r"^https://(.*)\.blob\.core\.usgovcloudapi\.net[/]?$" ).unwrap(),
        Regex::new(r"^https://(.*)\.blob\.core\.cloudapi\.de[/]?$" ).unwrap(),
    ]
});

pub struct BlobTokenAuthenticator {
    dataStore: Arc<dyn IAccountDataStore + Send + Sync>,
    oauth: OAuthLevel,
    logger: Arc<dyn ILogger + Send + Sync>,
}

impl BlobTokenAuthenticator {
    pub fn new(
        dataStore: Arc<dyn IAccountDataStore + Send + Sync>,
        oauth: OAuthLevel,
        logger: Arc<dyn ILogger + Send + Sync>,
    ) -> Self {
        Self {
            dataStore,
            oauth,
            logger,
        }
    }

    async fn authenticateBasic(&self, token: &str, context: &Context) -> Result<bool, StorageError> {
        let decoded = decode_jwt_payload(token).ok_or_else(|| {
            StorageErrorFactory::getAuthenticationFailed(
                context.contextId().as_deref(),
                "Authentication scheme Bearer is not supported.",
            )
        })?;

        let nbf = decoded.get("nbf").and_then(|value| value.as_i64());
        let exp = decoded.get("exp").and_then(|value| value.as_i64());
        let iat = decoded.get("iat").and_then(|value| value.as_i64());
        if nbf.is_none() || exp.is_none() || iat.is_none() {
            return Err(StorageErrorFactory::getAuthenticationFailed(
                context.contextId().as_deref(),
                "Authentication scheme Bearer is not supported.",
            ));
        }

        let now = context
            .startTime()
            .unwrap_or_else(Utc::now)
            .timestamp_millis();
        let nbf = nbf.unwrap_or_default() * 1000;
        let exp = exp.unwrap_or_default() * 1000;
        if now < nbf {
            return Err(StorageErrorFactory::getAuthenticationFailed(
                context.contextId().as_deref(),
                "Lifetime validation failed.",
            ));
        }
        if now > exp {
            return Err(StorageErrorFactory::getAuthenticationFailed(
                context.contextId().as_deref(),
                "Lifetime validation failed. The token is expired.",
            ));
        }

        let iss = decoded.get("iss").and_then(|value| value.as_str()).ok_or_else(|| {
            StorageErrorFactory::getAuthenticationFailed(
                context.contextId().as_deref(),
                "Authentication scheme Bearer is not supported.",
            )
        })?;
        if !VALID_ISSUE_PREFIXES.iter().any(|prefix| iss.starts_with(prefix)) {
            return Err(StorageErrorFactory::getAuthenticationFailed(
                context.contextId().as_deref(),
                "Invalid token issuer.",
            ));
        }

        let aud = decoded.get("aud").and_then(|value| value.as_str()).ok_or_else(|| {
            StorageErrorFactory::getAuthenticationFailed(
                context.contextId().as_deref(),
                "Authentication scheme Bearer is not supported.",
            )
        })?;
        let blobContext = BlobStorageContext::new(context);
        let mut audMatch = false;
        for regex in VALID_BLOB_AUDIENCES.iter() {
            if let Some(captures) = regex.captures(aud) {
                if let Some(matched) = captures.get(0) {
                    if matched.as_str() == aud {
                        if let Some(account) = captures.get(1) {
                            if Some(account.as_str().to_owned()) != blobContext.account() {
                                break;
                            }
                        }
                        audMatch = true;
                        break;
                    }
                }
            }
        }
        if !audMatch {
            return Err(StorageErrorFactory::getAuthenticationFailed(
                context.contextId().as_deref(),
                "Invalid token audience.",
            ));
        }

        Ok(true)
    }
}

#[async_trait]
impl IAuthenticator for BlobTokenAuthenticator {
    async fn validate(
        &self,
        req: &GeneratedHttpRequest,
        content: &Context,
    ) -> Result<Option<bool>, StorageError> {
        let blobContext = BlobStorageContext::new(content);
        let account = blobContext.account().unwrap_or_default();

        let accountProperties = self.dataStore.getAccount(&account);
        if accountProperties.is_none() {
            return Err(StorageErrorFactory::ResourceNotFound(
                blobContext.contextId().as_deref(),
            ));
        }

        let authHeaderValue = req.getHeader(AUTHORIZATION);
        if authHeaderValue.is_none() {
            return Ok(None);
        }
        let authHeaderValue = authHeaderValue.unwrap_or_default();

        if matches!(
            blobContext.operation(),
            Some(Operation::Container_GetAccessPolicy | Operation::Container_SetAccessPolicy)
        ) {
            return Ok(None);
        }

        if !authHeaderValue.starts_with(BEARER_TOKEN_PREFIX) {
            return Err(StorageErrorFactory::getInvalidAuthenticationInfo(
                blobContext.contextId().as_deref(),
            ));
        }

        if req.getProtocol().to_ascii_lowercase() != HTTPS {
            return Err(StorageErrorFactory::getAuthenticationFailed(
                blobContext.contextId().as_deref(),
                "Authentication scheme Bearer is not allowed with HTTP.",
            ));
        }

        let token = authHeaderValue[(BEARER_TOKEN_PREFIX.len() + 1)..].to_owned();
        match self.oauth {
            OAuthLevel::BASIC => Ok(Some(self.authenticateBasic(&token, content).await?)),
        }
    }
}

fn decode_jwt_payload(token: &str) -> Option<Value> {
    let payload = token.split('.').nth(1)?;
    let decoded = URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| URL_SAFE.decode(payload))
        .ok()?;
    serde_json::from_slice(&decoded).ok()
}
