use std::sync::Arc;

use async_trait::async_trait;
use azurite_common::authentication::{
    generateAccountSASSignature, AccountSASPermissionsOrString, AccountSASResourceTypesOrString,
    AccountSASServicesOrString, DateOrString, IAccountSASSignatureValues, SASProtocolOrString,
    SasIPRangeOrString,
};
use azurite_common::i_account_data_store::IAccountDataStore;
use azurite_common::i_logger::ILogger;
use chrono::Utc;

use crate::context::QueueStorageContext;
use crate::errors::{StorageError, StorageErrorFactory};
use crate::generated::context::Context;
use crate::generated::i_request::{GeneratedHttpRequest, IRequest};

use super::i_authenticator::IAuthenticator;
use super::operation_account_sas_permission::OPERATION_ACCOUNT_SAS_PERMISSIONS;

const DEFAULT_CONTEXT_ID: &str = "DefaultID";

pub struct AccountSASAuthenticator {
    accountDataStore: Arc<dyn IAccountDataStore + Send + Sync>,
    _logger: Arc<dyn ILogger + Send + Sync>,
}

impl AccountSASAuthenticator {
    pub fn new(
        accountDataStore: Arc<dyn IAccountDataStore + Send + Sync>,
        logger: Arc<dyn ILogger + Send + Sync>,
    ) -> Self {
        Self {
            accountDataStore,
            _logger: logger,
        }
    }

    fn getAccountSASSignatureValuesFromRequest(
        &self,
        req: &GeneratedHttpRequest,
    ) -> Option<IAccountSASSignatureValues> {
        let version = decodeIfExist(req.getQuery("sv").as_deref())?;
        let services = decodeIfExist(req.getQuery("ss").as_deref())?;
        let resourceTypes = decodeIfExist(req.getQuery("srt").as_deref())?;
        let expiryTime = decodeIfExist(req.getQuery("se").as_deref())?;
        let permissions = decodeIfExist(req.getQuery("sp").as_deref())?;
        let signature = decodeIfExist(req.getQuery("sig").as_deref())?;
        let protocol = decodeIfExist(req.getQuery("spr").as_deref());
        let startTime = decodeIfExist(req.getQuery("st").as_deref());
        let ipRange = decodeIfExist(req.getQuery("sip").as_deref());

        let _ = signature;

        Some(IAccountSASSignatureValues {
            version,
            protocol: protocol.map(SASProtocolOrString::String),
            startTime: startTime.map(DateOrString::String),
            expiryTime: DateOrString::String(expiryTime),
            permissions: AccountSASPermissionsOrString::String(permissions),
            ipRange: ipRange.map(SasIPRangeOrString::String),
            services: AccountSASServicesOrString::String(services),
            resourceTypes: AccountSASResourceTypesOrString::String(resourceTypes),
            encryptionScope: None,
        })
    }

    fn validateTime(&self, expiry: DateOrString, start: Option<DateOrString>) -> bool {
        let expiryTime = parse_date_or_string(&expiry);
        let now = Utc::now();
        if now > expiryTime {
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
impl IAuthenticator for AccountSASAuthenticator {
    async fn validate(
        &self,
        req: &GeneratedHttpRequest,
        context: &Context,
    ) -> Result<Option<bool>, StorageError> {
        let queueContext = QueueStorageContext::new(context);
        let account = queueContext.account().unwrap_or_default();

        let accountProperties = self.accountDataStore.getAccount(&account).ok_or_else(|| {
            StorageErrorFactory::ResourceNotFound(
                queueContext
                    .contextId()
                    .as_deref()
                    .unwrap_or(DEFAULT_CONTEXT_ID),
            )
        })?;

        let signature = decodeIfExist(req.getQuery("sig").as_deref());
        let values = match self.getAccountSASSignatureValuesFromRequest(req) {
            Some(values) => values,
            None => return Ok(Some(false)),
        };

        let (sig1, _) = generateAccountSASSignature(&values, &account, &accountProperties.key1);
        let sig1Pass = Some(sig1.clone()) == signature;
        if let Some(key2) = accountProperties.key2.as_deref() {
            let (sig2, _) = generateAccountSASSignature(&values, &account, key2);
            if !sig1Pass && Some(sig2) != signature {
                return Ok(Some(false));
            }
        } else if !sig1Pass {
            return Ok(Some(false));
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
                "AccountSASAuthenticator:validate() operation shouldn't be undefined. Please make sure DispatchMiddleware is hooked before authentication related middleware."
            )
        });
        let accountSASPermission = OPERATION_ACCOUNT_SAS_PERMISSIONS.get(&operation).unwrap_or_else(|| {
            panic!(
                "AccountSASAuthenticator:validate() OPERATION_ACCOUNT_SAS_PERMISSIONS doesn't have configuration for operation {}'s account SAS permission.",
                operation.as_str()
            )
        });

        if !accountSASPermission.validateServices(&values.services.toString()) {
            return Err(StorageErrorFactory::getAuthorizationServiceMismatch(
                queueContext
                    .contextId()
                    .as_deref()
                    .unwrap_or(DEFAULT_CONTEXT_ID),
            ));
        }
        if !accountSASPermission.validateResourceTypes(&values.resourceTypes.toString()) {
            return Err(StorageErrorFactory::getAuthorizationResourceTypeMismatch(
                queueContext
                    .contextId()
                    .as_deref()
                    .unwrap_or(DEFAULT_CONTEXT_ID),
            ));
        }
        if !accountSASPermission.validatePermissions(&values.permissions.toString()) {
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
    use azurite_common::authentication::{
        AccountSASPermissions, AccountSASResourceTypes, AccountSASServices,
    };
    use azurite_common::i_account_data_store::{IAccountDataStore, IAccountProperties};
    use azurite_common::i_cleaner::ICleaner;
    use azurite_common::i_data_store::IDataStore;
    use azurite_common::i_logger::ILogger;
    use azurite_common::storage_error::StorageError as CommonStorageError;
    use chrono::{Duration, Utc};

    use crate::context::QueueStorageContext;
    use crate::generated::artifacts::operation::Operation;
    use crate::generated::context::Context;
    use crate::generated::i_request::{GeneratedHttpRequest, HttpMethod};

    use super::super::i_authenticator::IAuthenticator;
    use super::AccountSASAuthenticator;

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

    #[tokio::test]
    async fn account_sas_authenticator_accepts_valid_queue_request() {
        let account = IAccountProperties {
            name: String::from("devstoreaccount1"),
            key1: b"secret".to_vec(),
            key2: None,
        };
        let authenticator = AccountSASAuthenticator::new(
            Arc::new(TestAccountStore {
                account: account.clone(),
            }),
            Arc::new(TestLogger),
        );

        let start = Utc::now() - Duration::minutes(5);
        let end = Utc::now() + Duration::hours(1);
        let sas = azurite_common::authentication::generateAccountSASSignature(
            &azurite_common::authentication::IAccountSASSignatureValues {
                version: String::from("2020-08-04"),
                protocol: None,
                startTime: Some(azurite_common::authentication::DateOrString::String(
                    start.to_rfc3339(),
                )),
                expiryTime: azurite_common::authentication::DateOrString::String(end.to_rfc3339()),
                permissions: azurite_common::authentication::AccountSASPermissionsOrString::AccountSASPermissions(
                    AccountSASPermissions::parse("rwdlacup").unwrap(),
                ),
                ipRange: None,
                services: azurite_common::authentication::AccountSASServicesOrString::AccountSASServices(
                    AccountSASServices::parse("q").unwrap(),
                ),
                resourceTypes: azurite_common::authentication::AccountSASResourceTypesOrString::AccountSASResourceTypes(
                    AccountSASResourceTypes::parse("co").unwrap(),
                ),
                encryptionScope: None,
            },
            &account.name,
            &account.key1,
        )
        .0;

        let mut request = GeneratedHttpRequest::new(
            HttpMethod::GET,
            format!(
                "http://127.0.0.1/devstoreaccount1/queue/messages?sv=2020-08-04&ss=q&srt=co&st={}&se={}&sp=rwdlacup&sig={}",
                start.to_rfc3339(),
                end.to_rfc3339(),
                sas
            ),
            "http://127.0.0.1:10001",
            "/queue/messages",
        );
        request.protocol = String::from("http");
        request
            .query
            .insert(String::from("sv"), String::from("2020-08-04"));
        request.query.insert(String::from("ss"), String::from("q"));
        request
            .query
            .insert(String::from("srt"), String::from("co"));
        request.query.insert(String::from("st"), start.to_rfc3339());
        request.query.insert(String::from("se"), end.to_rfc3339());
        request
            .query
            .insert(String::from("sp"), String::from("rwdlacup"));
        request.query.insert(String::from("sig"), sas);

        let context = Context::default();
        context.setOperation(Some(Operation::Messages_Dequeue));
        let queue_context = QueueStorageContext::new(&context);
        queue_context.setAccount(Some(String::from("devstoreaccount1")));

        assert_eq!(
            authenticator.validate(&request, &context).await.unwrap(),
            Some(true)
        );
    }
}
