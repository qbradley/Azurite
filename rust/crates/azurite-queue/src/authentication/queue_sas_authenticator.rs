use std::sync::Arc;

use async_trait::async_trait;
use azurite_common::authentication::{DateOrString, SASProtocolOrString};
use azurite_common::i_account_data_store::IAccountDataStore;
use azurite_common::i_logger::ILogger;
use chrono::Utc;

use crate::context::QueueStorageContext;
use crate::errors::{StorageError, StorageErrorFactory};
use crate::generated::artifacts::models::{AccessPolicy, GeneratedValue};
use crate::generated::context::Context;
use crate::generated::i_request::{GeneratedHttpRequest, IRequest};

use super::i_authenticator::IAuthenticator;
use super::i_queue_sas_signature_values::{
    generateQueueSASSignature, IIPRangeOrString, IQueueSASSignatureValues,
};
use super::operation_queue_sas_permission::OPERATION_QUEUE_SAS_PERMISSIONS;

const DEFAULT_CONTEXT_ID: &str = "DefaultID";

#[async_trait]
pub trait QueueSASAccessPolicySource: Send + Sync {
    async fn get_queue_access_policy(
        &self,
        account: &str,
        queue: &str,
        identifier: &str,
        context: &Context,
    ) -> Option<AccessPolicy>;
}

pub struct QueueSASAuthenticator {
    accountDataStore: Arc<dyn IAccountDataStore + Send + Sync>,
    accessPolicySource: Arc<dyn QueueSASAccessPolicySource + Send + Sync>,
    _logger: Arc<dyn ILogger + Send + Sync>,
}

impl QueueSASAuthenticator {
    pub fn new(
        accountDataStore: Arc<dyn IAccountDataStore + Send + Sync>,
        accessPolicySource: Arc<dyn QueueSASAccessPolicySource + Send + Sync>,
        logger: Arc<dyn ILogger + Send + Sync>,
    ) -> Self {
        Self {
            accountDataStore,
            accessPolicySource,
            _logger: logger,
        }
    }

    fn getQueueSASSignatureValuesFromRequest(
        &self,
        req: &GeneratedHttpRequest,
        queueName: &str,
    ) -> Option<IQueueSASSignatureValues> {
        let version = decodeIfExist(req.getQuery("sv").as_deref())?;
        let protocol = decodeIfExist(req.getQuery("spr").as_deref());
        let startTime = decodeIfExist(req.getQuery("st").as_deref());
        let expiryTime = decodeIfExist(req.getQuery("se").as_deref());
        let permissions = decodeIfExist(req.getQuery("sp").as_deref());
        let ipRange = decodeIfExist(req.getQuery("sip").as_deref());
        let identifier = decodeIfExist(req.getQuery("si").as_deref());

        if identifier.is_none() && (permissions.is_none() || expiryTime.is_none()) {
            return None;
        }

        Some(IQueueSASSignatureValues {
            version,
            protocol: protocol.map(SASProtocolOrString::String),
            startTime: startTime.map(DateOrString::String),
            expiryTime: expiryTime.map(DateOrString::String),
            permissions,
            ipRange: ipRange.map(IIPRangeOrString::String),
            queueName: queueName.to_owned(),
            identifier,
        })
    }

    fn validateTime(&self, expiry: Option<DateOrString>, start: Option<DateOrString>) -> bool {
        let Some(expiry) = expiry else {
            return false;
        };
        let now = Utc::now();
        if now > parse_date_or_string(&expiry) {
            return false;
        }
        if let Some(start) = start {
            if now < parse_date_or_string(&start) {
                return false;
            }
        }
        true
    }

    fn validateIPRange(&self) -> bool {
        true
    }

    fn validateProtocol(
        &self,
        sasProtocol: Option<SASProtocolOrString>,
        requestProtocol: &str,
    ) -> bool {
        let sasProtocol = sasProtocol
            .map(|value| value.toString())
            .unwrap_or_else(|| String::from("https,http"));
        if sasProtocol.contains(',') {
            true
        } else {
            sasProtocol.to_ascii_lowercase() == requestProtocol
        }
    }
}

#[async_trait]
impl IAuthenticator for QueueSASAuthenticator {
    async fn validate(
        &self,
        req: &GeneratedHttpRequest,
        context: &Context,
    ) -> Result<Option<bool>, StorageError> {
        let queueContext = QueueStorageContext::new(context);
        let account = queueContext.account().unwrap_or_else(|| {
            panic!("QueueSASAuthenticator:validate() account is undefined in context.")
        });
        let queueName = match queueContext.queue() {
            Some(queueName) => queueName,
            None => return Ok(None),
        };

        let accountProperties = self.accountDataStore.getAccount(&account).ok_or_else(|| {
            StorageErrorFactory::ResourceNotFound(
                queueContext
                    .contextId()
                    .as_deref()
                    .unwrap_or(DEFAULT_CONTEXT_ID),
            )
        })?;

        let signature = match decodeIfExist(req.getQuery("sig").as_deref()) {
            Some(signature) => signature,
            None => return Ok(None),
        };
        let mut values = match self.getQueueSASSignatureValuesFromRequest(req, &queueName) {
            Some(values) => values,
            None => return Ok(None),
        };

        let (sig1, _) = generateQueueSASSignature(&values, &account, &accountProperties.key1);
        let sig1Pass = sig1 == signature;
        if let Some(key2) = accountProperties.key2.as_deref() {
            let (sig2, _) = generateQueueSASSignature(&values, &account, key2);
            if !sig1Pass && sig2 != signature {
                return Ok(Some(false));
            }
        } else if !sig1Pass {
            return Ok(Some(false));
        }

        if let Some(identifier) = values.identifier.clone() {
            if values.startTime.is_some()
                || values.expiryTime.is_some()
                || values.permissions.is_some()
            {
                return Err(StorageErrorFactory::getAuthorizationFailure(
                    queueContext.contextId().as_deref(),
                ));
            }

            let accessPolicy = self
                .accessPolicySource
                .get_queue_access_policy(&account, &queueName, &identifier, context)
                .await
                .ok_or_else(|| {
                    StorageErrorFactory::getAuthorizationFailure(
                        queueContext.contextId().as_deref(),
                    )
                })?;

            values.startTime =
                access_policy_field(&accessPolicy, "start").map(DateOrString::String);
            values.expiryTime =
                access_policy_field(&accessPolicy, "expiry").map(DateOrString::String);
            values.permissions = access_policy_field(&accessPolicy, "permission");
        }

        if !self.validateTime(values.expiryTime.clone(), values.startTime.clone()) {
            return Err(StorageErrorFactory::getAuthorizationFailure(
                queueContext.contextId().as_deref(),
            ));
        }
        if !self.validateIPRange() {
            return Err(StorageErrorFactory::getAuthorizationSourceIPMismatch(
                queueContext
                    .contextId()
                    .as_deref()
                    .unwrap_or(DEFAULT_CONTEXT_ID),
            ));
        }
        if !self.validateProtocol(values.protocol.clone(), &req.getProtocol()) {
            return Err(StorageErrorFactory::getAuthorizationProtocolMismatch(
                queueContext
                    .contextId()
                    .as_deref()
                    .unwrap_or(DEFAULT_CONTEXT_ID),
            ));
        }

        let operation = queueContext.operation().unwrap_or_else(|| {
            panic!(
                "QueueSASAuthenticator:validate() Operation shouldn't be undefined. Please make sure DispatchMiddleware is hooked before authentication related middleware."
            )
        });
        let queueSASPermission = OPERATION_QUEUE_SAS_PERMISSIONS.get(&operation).unwrap_or_else(|| {
            panic!(
                "QueueSASAuthenticator:validate() OPERATION_QUEUE_SAS_PERMISSIONS doesn't have configuration for operation {}'s queue service SAS permission.",
                operation.as_str()
            )
        });

        if !queueSASPermission
            .validatePermissions(values.permissions.as_deref().unwrap_or_default())
        {
            return Err(StorageErrorFactory::getAuthorizationPermissionMismatch(
                queueContext
                    .contextId()
                    .as_deref()
                    .unwrap_or(DEFAULT_CONTEXT_ID),
            ));
        }

        Ok(Some(true))
    }
}

fn access_policy_field(access_policy: &AccessPolicy, field: &str) -> Option<String> {
    let alternate = match field {
        "start" => Some("Start"),
        "expiry" => Some("Expiry"),
        "permission" => Some("Permission"),
        _ => None,
    };

    access_policy
        .get(field)
        .or_else(|| alternate.and_then(|alternate| access_policy.get(alternate)))
        .and_then(GeneratedValue::as_string)
}

fn parse_date_or_string(value: &DateOrString) -> chrono::DateTime<chrono::Utc> {
    match value {
        DateOrString::Date(value) => *value,
        DateOrString::String(value) => chrono::DateTime::parse_from_rfc3339(value)
            .map(|value| value.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
    }
}

fn decodeIfExist(value: Option<&str>) -> Option<String> {
    value.map(decode_uri_component)
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
    use chrono::{Duration, Utc};

    use crate::context::QueueStorageContext;
    use crate::generated::artifacts::models::{AccessPolicy, GeneratedValue};
    use crate::generated::artifacts::operation::Operation;
    use crate::generated::context::Context;
    use crate::generated::i_request::{GeneratedHttpRequest, HttpMethod};

    use super::super::i_authenticator::IAuthenticator;
    use super::{QueueSASAccessPolicySource, QueueSASAuthenticator};

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

    struct StaticAccessPolicySource;

    #[async_trait]
    impl QueueSASAccessPolicySource for StaticAccessPolicySource {
        async fn get_queue_access_policy(
            &self,
            _account: &str,
            _queue: &str,
            identifier: &str,
            _context: &Context,
        ) -> Option<AccessPolicy> {
            (identifier == "policy").then(|| {
                let mut access_policy = AccessPolicy::new();
                access_policy.insert(
                    String::from("start"),
                    GeneratedValue::String((Utc::now() - Duration::minutes(5)).to_rfc3339()),
                );
                access_policy.insert(
                    String::from("expiry"),
                    GeneratedValue::String((Utc::now() + Duration::hours(1)).to_rfc3339()),
                );
                access_policy.insert(
                    String::from("permission"),
                    GeneratedValue::String(String::from("raup")),
                );
                access_policy
            })
        }
    }

    #[tokio::test]
    async fn queue_sas_authenticator_accepts_identifier_backed_signature() {
        let account = IAccountProperties {
            name: String::from("devstoreaccount1"),
            key1: b"secret".to_vec(),
            key2: None,
        };
        let authenticator = QueueSASAuthenticator::new(
            Arc::new(TestAccountStore {
                account: account.clone(),
            }),
            Arc::new(StaticAccessPolicySource),
            Arc::new(TestLogger),
        );

        let values = super::super::i_queue_sas_signature_values::IQueueSASSignatureValues {
            version: String::from("2020-08-04"),
            protocol: None,
            startTime: None,
            expiryTime: None,
            permissions: None,
            ipRange: None,
            queueName: String::from("queue"),
            identifier: Some(String::from("policy")),
        };

        let signature = super::super::i_queue_sas_signature_values::generateQueueSASSignature(
            &values,
            &account.name,
            &account.key1,
        )
        .0;

        let mut request = GeneratedHttpRequest::new(
            HttpMethod::GET,
            format!(
                "http://127.0.0.1/devstoreaccount1/queue/messages?sv=2020-08-04&si=policy&sig={signature}"
            ),
            "http://127.0.0.1:10001",
            "/queue/messages",
        );
        request
            .query
            .insert(String::from("sv"), String::from("2020-08-04"));
        request
            .query
            .insert(String::from("si"), String::from("policy"));
        request.query.insert(String::from("sig"), signature);
        request.protocol = String::from("http");

        let context = Context::default();
        context.setOperation(Some(Operation::Messages_Peek));
        let queue_context = QueueStorageContext::new(&context);
        queue_context.setAccount(Some(String::from("devstoreaccount1")));
        queue_context.setQueue(Some(String::from("queue")));

        assert_eq!(
            authenticator.validate(&request, &context).await.unwrap(),
            Some(true)
        );
    }
}
