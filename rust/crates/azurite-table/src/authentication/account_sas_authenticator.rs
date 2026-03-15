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

use crate::context::TableStorageContext;
use crate::errors::{StorageError, StorageErrorFactory};
use crate::generated::context::Context;
use crate::generated::i_request::{GeneratedHttpRequest, IRequest};

use super::i_authenticator::IAuthenticator;
use super::operation_account_sas_permission::OPERATION_ACCOUNT_SAS_PERMISSIONS;

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
        let protocol = decodeIfExist(req.getQuery("spr").as_deref());
        let startTime = decodeIfExist(req.getQuery("st").as_deref());
        let expiryTime = decodeIfExist(req.getQuery("se").as_deref())?;
        let ipRange = decodeIfExist(req.getQuery("sip").as_deref());
        let permissions = decodeIfExist(req.getQuery("sp").as_deref())?;
        let signature = decodeIfExist(req.getQuery("sig").as_deref())?;

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
        content: &Context,
    ) -> Result<Option<bool>, StorageError> {
        let tableContext = TableStorageContext::new(content);
        let account = tableContext.account().unwrap_or_default();

        let accountProperties = self
            .accountDataStore
            .getAccount(&account)
            .ok_or_else(|| StorageErrorFactory::ResourceNotFoundXml(content))?;

        let signature = decodeIfExist(req.getQuery("sig").as_deref());
        let values = match self.getAccountSASSignatureValuesFromRequest(req) {
            Some(values) => values,
            None => return Ok(Some(false)),
        };

        let (sig1, _) = generateAccountSASSignature(&values, &account, &accountProperties.key1);
        let sig1Pass = Some(sig1) == signature;
        if let Some(key2) = accountProperties.key2.as_deref() {
            let (sig2, _) = generateAccountSASSignature(&values, &account, key2);
            if !sig1Pass && Some(sig2) != signature {
                return Ok(Some(false));
            }
        } else if !sig1Pass {
            return Ok(Some(false));
        }

        if !self.validateTime(values.expiryTime.clone(), values.startTime.clone()) {
            return Err(StorageErrorFactory::getAuthorizationFailure(content));
        }
        if !self.validateIPRange() {
            return Err(StorageErrorFactory::getAuthorizationSourceIPMismatch(
                content,
            ));
        }
        if !self.validateProtocol(values.protocol.clone(), &req.getProtocol()) {
            return Err(StorageErrorFactory::getAuthorizationProtocolMismatch(
                content,
            ));
        }

        let operation = tableContext.operation().unwrap_or_else(|| {
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

        if !accountSASPermission.validateServices(values.services.toString()) {
            return Err(StorageErrorFactory::getAuthorizationServiceMismatch(
                content,
            ));
        }
        if !accountSASPermission.validateResourceTypes(values.resourceTypes.toString()) {
            return Err(StorageErrorFactory::getAuthorizationResourceTypeMismatch(
                content,
            ));
        }
        if !accountSASPermission.validatePermissions(values.permissions.toString()) {
            return Err(StorageErrorFactory::getAuthorizationPermissionMismatch(
                content,
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
