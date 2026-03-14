use std::sync::Arc;

use async_trait::async_trait;
use azurite_common::authentication::{
    generateAccountSASSignature, AccountSASPermissionsOrString, AccountSASResourceTypesOrString,
    AccountSASServicesOrString, DateOrString, IAccountSASSignatureValues, SASProtocolOrString,
    SasIPRangeOrString,
};
use azurite_common::i_account_data_store::IAccountDataStore;
use azurite_common::i_logger::ILogger;

use crate::context::BlobStorageContext;
use crate::errors::{StorageError, StorageErrorFactory, StrictModelNotSupportedError};
use crate::generated::artifacts::operation::Operation;
use crate::generated::context::Context;
use crate::generated::i_request::{GeneratedHttpRequest, IRequest};
use crate::persistence::IBlobMetadataStore;

use super::i_authenticator::IAuthenticator;
use super::operation_account_sas_permission::OPERATION_ACCOUNT_SAS_PERMISSIONS;

const AUTHENTICATION_BEARERTOKEN_REQUIRED: &str = "Only authentication scheme Bearer is supported";
const BLOCK_BLOB: &str = "BlockBlob";

pub struct AccountSASAuthenticator {
    accountDataStore: Arc<dyn IAccountDataStore + Send + Sync>,
    blobMetadataStore: Arc<dyn IBlobMetadataStore + Send + Sync>,
    logger: Arc<dyn ILogger + Send + Sync>,
}

impl AccountSASAuthenticator {
    pub fn new(
        accountDataStore: Arc<dyn IAccountDataStore + Send + Sync>,
        blobMetadataStore: Arc<dyn IBlobMetadataStore + Send + Sync>,
        logger: Arc<dyn ILogger + Send + Sync>,
    ) -> Self {
        Self {
            accountDataStore,
            blobMetadataStore,
            logger,
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
        let encryptionScope = decodeIfExist(req.getQuery("ses").as_deref());

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
            encryptionScope,
        })
    }

    fn validateTime(&self, expiry: DateOrString, start: Option<DateOrString>) -> bool {
        let expiryTime = parse_date_or_string(&expiry);
        let now = chrono::Utc::now();
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

    fn validateProtocol(&self, sasProtocol: Option<SASProtocolOrString>, requestProtocol: &str) -> bool {
        let sasProtocol = sasProtocol.map(|value| value.toString()).unwrap_or_else(|| String::from("https,http"));
        if sasProtocol.contains(',') {
            true
        } else {
            sasProtocol.to_ascii_lowercase() == requestProtocol
        }
    }

    async fn blobExist(&self, account: &str, container: &str, blob: &str) -> Result<bool, StorageError> {
        let blobModel = self.blobMetadataStore.getBlobType(account, container, blob, None).await?;
        if blobModel.is_none() {
            return Ok(false);
        }
        let blobModel = blobModel.unwrap();
        if blobModel.blobType.as_deref() == Some(BLOCK_BLOB) && !blobModel.isCommitted {
            return Ok(false);
        }
        Ok(true)
    }
}

#[async_trait]
impl IAuthenticator for AccountSASAuthenticator {
    async fn validate(
        &self,
        req: &GeneratedHttpRequest,
        content: &Context,
    ) -> Result<Option<bool>, StorageError> {
        let blobContext = BlobStorageContext::new(content);
        let account = blobContext.account().unwrap_or_default();
        let containerName = blobContext.container();
        let blobName = blobContext.blob();

        let accountProperties = self.accountDataStore.getAccount(&account);
        if accountProperties.is_none() {
            return Err(StorageErrorFactory::ResourceNotFound(
                blobContext.contextId().as_deref(),
            ));
        }
        let accountProperties = accountProperties.unwrap();

        let signature = decodeIfExist(req.getQuery("sig").as_deref());
        let values = self.getAccountSASSignatureValuesFromRequest(req);
        if values.is_none() {
            self.logger.info(
                "AccountSASAuthenticator:validate() Failed to get valid account SAS values from request.",
                blobContext.contextId().as_deref(),
            );
            return Ok(Some(false));
        }
        let values = values.unwrap();

        if !blobContext.loose().unwrap_or(false) && values.encryptionScope.is_some() {
            return Err(StrictModelNotSupportedError::new(
                "SAS Encryption Scope 'ses'",
                blobContext.contextId().as_deref(),
            )
            .into());
        }

        let (sig1, _) = generateAccountSASSignature(&values, &account, &accountProperties.key1);
        let sig1Pass = Some(sig1.clone()) == signature;
        if let Some(key2) = accountProperties.key2.as_deref() {
            let (sig2, _) = generateAccountSASSignature(&values, &account, key2);
            let sig2Pass = Some(sig2.clone()) == signature;
            if !sig1Pass && !sig2Pass {
                return Ok(Some(false));
            }
        } else if !sig1Pass {
            return Ok(Some(false));
        }

        if !self.validateTime(values.expiryTime.clone(), values.startTime.clone()) {
            return Err(StorageErrorFactory::getAuthorizationFailure(
                blobContext.contextId().as_deref().unwrap_or(""),
            ));
        }
        if !self.validateIPRange() {
            return Err(StorageErrorFactory::getAuthorizationSourceIPMismatch(
                blobContext.contextId().as_deref().unwrap_or(""),
            ));
        }
        if !self.validateProtocol(values.protocol.clone(), &req.getProtocol()) {
            return Err(StorageErrorFactory::getAuthorizationProtocolMismatch(
                blobContext.contextId().as_deref().unwrap_or(""),
            ));
        }

        let operation = blobContext.operation().unwrap_or_else(|| {
            panic!(
                "AccountSASAuthenticator:validate() operation shouldn't be undefined. Please make sure DispatchMiddleware is hooked before authentication related middleware."
            )
        });
        if operation == Operation::Service_GetUserDelegationKey {
            return Err(StorageErrorFactory::getAuthenticationFailed(
                blobContext.contextId().as_deref(),
                AUTHENTICATION_BEARERTOKEN_REQUIRED,
            ));
        }

        let accountSASPermission = OPERATION_ACCOUNT_SAS_PERMISSIONS.get(&operation).unwrap_or_else(|| {
            panic!(
                "AccountSASAuthenticator:validate() OPERATION_ACCOUNT_SAS_PERMISSIONS doesn't have configuration for operation {}'s account SAS permission.",
                operation.as_str()
            )
        });
        if !accountSASPermission.validateServices(values.services.toString()) {
            return Err(StorageErrorFactory::getAuthorizationServiceMismatch(
                blobContext.contextId().as_deref().unwrap_or(""),
            ));
        }
        if !accountSASPermission.validateResourceTypes(values.resourceTypes.toString()) {
            return Err(StorageErrorFactory::getAuthorizationResourceTypeMismatch(
                blobContext.contextId().as_deref().unwrap_or(""),
            ));
        }
        if !accountSASPermission.validatePermissions(values.permissions.toString()) {
            return Err(StorageErrorFactory::getAuthorizationPermissionMismatch(
                blobContext.contextId().as_deref().unwrap_or(""),
            ));
        }

        if matches!(
            operation,
            Operation::BlockBlob_Upload
                | Operation::PageBlob_Create
                | Operation::AppendBlob_Create
                | Operation::Blob_StartCopyFromURL
                | Operation::Blob_CopyFromURL
        ) {
            if let (Some(containerName), Some(blobName)) = (containerName.as_deref(), blobName.as_deref()) {
                if self.blobExist(&account, containerName, blobName).await?
                    && !values.permissions.toString().contains('w')
                {
                    return Err(StorageErrorFactory::getAuthorizationPermissionMismatch(
                        blobContext.contextId().as_deref().unwrap_or(""),
                    ));
                }
            }
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
            if let (Some(high), Some(low)) = (from_hex(bytes[index + 1]), from_hex(bytes[index + 2])) {
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
