use std::sync::Arc;

use async_trait::async_trait;
use azurite_common::i_account_data_store::IAccountDataStore;
use azurite_common::i_logger::ILogger;
use azurite_common::models::OAuthLevel;
use azurite_common::utils::constants::{BEARER_TOKEN_PREFIX, HTTPS, VALID_ISSUE_PREFIXES};
use base64::{
    engine::general_purpose::URL_SAFE, engine::general_purpose::URL_SAFE_NO_PAD, Engine as _,
};
use serde_json::Value;

use crate::context::QueueStorageContext;
use crate::errors::{StorageError, StorageErrorFactory};
use crate::generated::artifacts::operation::Operation;
use crate::generated::context::Context;
use crate::generated::i_request::{GeneratedHttpRequest, IRequest};
use crate::utils::constants::{HeaderConstants, VALID_QUEUE_AUDIENCES};

use super::i_authenticator::IAuthenticator;

const DEFAULT_CONTEXT_ID: &str = "DefaultID";

pub struct QueueTokenAuthenticator {
    dataStore: Arc<dyn IAccountDataStore + Send + Sync>,
    oauth: OAuthLevel,
    _logger: Arc<dyn ILogger + Send + Sync>,
}

impl QueueTokenAuthenticator {
    pub fn new(
        dataStore: Arc<dyn IAccountDataStore + Send + Sync>,
        oauth: OAuthLevel,
        logger: Arc<dyn ILogger + Send + Sync>,
    ) -> Self {
        Self {
            dataStore,
            oauth,
            _logger: logger,
        }
    }

    async fn authenticateBasic(
        &self,
        token: &str,
        context: &Context,
    ) -> Result<bool, StorageError> {
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
            .unwrap_or_else(chrono::Utc::now)
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

        let iss = decoded
            .get("iss")
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                StorageErrorFactory::getAuthenticationFailed(
                    context.contextId().as_deref(),
                    "Authentication scheme Bearer is not supported.",
                )
            })?;
        if !VALID_ISSUE_PREFIXES
            .iter()
            .any(|prefix| iss.starts_with(prefix))
        {
            return Err(StorageErrorFactory::getAuthenticationFailed(
                context.contextId().as_deref(),
                "Invalid token issuer.",
            ));
        }

        let aud = decoded
            .get("aud")
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                StorageErrorFactory::getAuthenticationFailed(
                    context.contextId().as_deref(),
                    "Authentication scheme Bearer is not supported.",
                )
            })?;

        let queueContext = QueueStorageContext::new(context);
        let mut audMatch = false;
        for regex in VALID_QUEUE_AUDIENCES.iter() {
            if let Some(captures) = regex.captures(aud) {
                if let Some(matched) = captures.get(0) {
                    if matched.as_str() == aud {
                        if let Some(account) = captures.get(1) {
                            if Some(account.as_str().to_owned()) != queueContext.account() {
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
impl IAuthenticator for QueueTokenAuthenticator {
    async fn validate(
        &self,
        req: &GeneratedHttpRequest,
        context: &Context,
    ) -> Result<Option<bool>, StorageError> {
        let queueContext = QueueStorageContext::new(context);
        let account = queueContext.account().unwrap_or_default();

        self.dataStore.getAccount(&account).ok_or_else(|| {
            StorageErrorFactory::ResourceNotFound(
                queueContext
                    .contextId()
                    .as_deref()
                    .unwrap_or(DEFAULT_CONTEXT_ID),
            )
        })?;

        let authHeaderValue = match req.getHeader(HeaderConstants.AUTHORIZATION) {
            Some(value) => value,
            None => return Ok(None),
        };

        if matches!(
            queueContext.operation(),
            Some(Operation::Queue_GetAccessPolicy | Operation::Queue_SetAccessPolicy)
        ) {
            return Ok(None);
        }

        if !authHeaderValue.starts_with(BEARER_TOKEN_PREFIX) {
            return Err(StorageErrorFactory::getInvalidAuthenticationInfo(
                queueContext.contextId().as_deref(),
            ));
        }
        if req.getProtocol().to_ascii_lowercase() != HTTPS {
            return Err(StorageErrorFactory::getAuthenticationFailed(
                queueContext.contextId().as_deref(),
                "Authentication scheme Bearer is not allowed with HTTP.",
            ));
        }

        let token = authHeaderValue[(BEARER_TOKEN_PREFIX.len() + 1)..].to_owned();
        match self.oauth {
            OAuthLevel::BASIC => Ok(Some(self.authenticateBasic(&token, context).await?)),
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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, LazyLock};

    use async_trait::async_trait;
    use azurite_common::i_account_data_store::{IAccountDataStore, IAccountProperties};
    use azurite_common::i_cleaner::ICleaner;
    use azurite_common::i_data_store::IDataStore;
    use azurite_common::i_logger::ILogger;
    use azurite_common::storage_error::StorageError as CommonStorageError;
    use chrono::{Duration, Utc};

    use crate::context::QueueStorageContext;
    use crate::generated::artifacts::operation::Operation;
    use crate::generated::context::Context;
    use crate::generated::i_request::{GeneratedHttpRequest, HttpMethod, RequestHeaderValue};
    use base64::Engine as _;

    use super::super::i_authenticator::IAuthenticator;
    use super::QueueTokenAuthenticator;

    #[derive(Default)]
    struct TestLogger;

    impl ILogger for TestLogger {
        fn error(&self, _message: &str, _contextID: Option<&str>) {}
        fn warn(&self, _message: &str, _contextID: Option<&str>) {}
        fn info(&self, _message: &str, _contextID: Option<&str>) {}
        fn verbose(&self, _message: &str, _contextID: Option<&str>) {}
        fn debug(&self, _message: &str, _contextID: Option<&str>) {}
    }

    struct TestAccountStore {
        account: IAccountProperties,
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
            (self.account.name == name).then(|| self.account.clone())
        }
    }

    static JWT_HEADER: LazyLock<String> = LazyLock::new(|| {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"JWT"}"#)
    });

    #[tokio::test]
    async fn queue_token_authenticator_accepts_valid_basic_token() {
        let account = IAccountProperties {
            name: String::from("devstoreaccount1"),
            key1: b"secret".to_vec(),
            key2: None,
        };
        let authenticator = QueueTokenAuthenticator::new(
            Arc::new(TestAccountStore {
                account: account.clone(),
            }),
            azurite_common::models::OAuthLevel::BASIC,
            Arc::new(TestLogger),
        );

        let now = Utc::now();
        let payload = serde_json::json!({
            "aud": "https://devstoreaccount1.queue.core.windows.net",
            "iss": "https://sts.windows.net/tenant/",
            "iat": now.timestamp(),
            "nbf": (now - Duration::minutes(1)).timestamp(),
            "exp": (now + Duration::minutes(5)).timestamp(),
        });
        let jwt_payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&payload).unwrap());
        let token = format!("{}.{}.", *JWT_HEADER, jwt_payload);

        let mut request = GeneratedHttpRequest::new(
            HttpMethod::GET,
            "https://devstoreaccount1.queue.core.windows.net/queue/messages",
            "https://devstoreaccount1.queue.core.windows.net",
            "/queue/messages",
        );
        request.protocol = String::from("https");
        request.headers.insert(
            String::from("authorization"),
            RequestHeaderValue::Single(format!("Bearer {token}")),
        );

        let context = Context::default();
        context.setStartTime(Some(now));
        context.setOperation(Some(Operation::Messages_Peek));
        let queue_context = QueueStorageContext::new(&context);
        queue_context.setAccount(Some(account.name.clone()));

        assert_eq!(
            authenticator.validate(&request, &context).await.unwrap(),
            Some(true)
        );
    }
}
