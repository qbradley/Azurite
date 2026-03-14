use std::sync::{Arc, LazyLock};

use async_trait::async_trait;
use azurite_common::authentication::{DateOrString, SASProtocolOrString};
use azurite_common::i_account_data_store::IAccountDataStore;
use azurite_common::i_logger::ILogger;
use azurite_common::utils::utils::computeHMACSHA256;
use base64::{engine::general_purpose::STANDARD, Engine as _};

use crate::context::BlobStorageContext;
use crate::errors::{StorageError, StorageErrorFactory, StrictModelNotSupportedError};
use crate::generated::artifacts::models::AccessPolicy;
use crate::generated::artifacts::operation::Operation;
use crate::generated::context::Context;
use crate::generated::i_request::{GeneratedHttpRequest, IRequest};
use crate::persistence::{
    access_policy_field, signed_identifier_access_policy, signed_identifier_id, IBlobMetadataStore,
};

use super::blob_sas_permissions::BlobSASPermission;
use super::blob_sas_resource_type::BlobSASResourceType;
use super::i_authenticator::IAuthenticator;
use super::i_blob_sas_signature_values::{
    generateBlobSASSignature, generateBlobSASSignatureWithUDK, DateOrString as BlobDateOrString,
    IBlobSASSignatureValues, IIPRangeOrString,
};
use super::operation_blob_sas_permission::{
    OPERATION_BLOB_SAS_BLOB_PERMISSIONS, OPERATION_BLOB_SAS_CONTAINER_PERMISSIONS,
};

const AUTHENTICATION_BEARERTOKEN_REQUIRED: &str = "Only authentication scheme Bearer is supported";
const USERDELEGATIONKEY_BASIC_KEY: &str =
    "I17GKLvcJUossaebtsEDZZ2RJ8GNLwLH4m7hRMxbVbkx6wNIRAABj4Rtw0FBhFuEAgmbL4gFMzUw+AStz9Sqdg==";
const BLOCK_BLOB: &str = "BlockBlob";

static USERDELEGATIONKEY_BASIC_KEY_BYTES: LazyLock<Vec<u8>> = LazyLock::new(|| {
    STANDARD
        .decode(USERDELEGATIONKEY_BASIC_KEY)
        .unwrap_or_default()
});

pub struct BlobSASAuthenticator {
    accountDataStore: Arc<dyn IAccountDataStore + Send + Sync>,
    blobMetadataStore: Arc<dyn IBlobMetadataStore + Send + Sync>,
    logger: Arc<dyn ILogger + Send + Sync>,
}

impl BlobSASAuthenticator {
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

    fn getBlobSASSignatureValuesFromRequest(
        &self,
        req: &GeneratedHttpRequest,
        containerName: &str,
        blobName: Option<&str>,
        context: Option<&Context>,
    ) -> Option<IBlobSASSignatureValues> {
        let version = decodeIfExist(req.getQuery("sv").as_deref())?;
        let protocol = decodeIfExist(req.getQuery("spr").as_deref());
        let startTime = decodeIfExist(req.getQuery("st").as_deref());
        let expiryTime = decodeIfExist(req.getQuery("se").as_deref());
        let permissions = decodeIfExist(req.getQuery("sp").as_deref());
        let ipRange = decodeIfExist(req.getQuery("sip").as_deref());
        let identifier = decodeIfExist(req.getQuery("si").as_deref());
        let cacheControl = req.getQuery("rscc");
        let contentDisposition = req.getQuery("rscd");
        let contentEncoding = req.getQuery("rsce");
        let contentLanguage = req.getQuery("rscl");
        let contentType = req.getQuery("rsct");
        let signedResource = decodeIfExist(req.getQuery("sr").as_deref());
        let snapshot = decodeIfExist(req.getQuery("snapshot").as_deref());
        let encryptionScope = decodeIfExist(req.getQuery("ses").as_deref());
        let signedObjectId = decodeIfExist(req.getQuery("skoid").as_deref());
        let signedTenantId = decodeIfExist(req.getQuery("sktid").as_deref());
        let signedStartsOn = decodeIfExist(req.getQuery("skt").as_deref());
        let signedExpiresOn = decodeIfExist(req.getQuery("ske").as_deref());
        let signedVersion = decodeIfExist(req.getQuery("skv").as_deref());
        let signedService = decodeIfExist(req.getQuery("sks").as_deref());

        if identifier.is_none() && (permissions.is_none() || expiryTime.is_none()) {
            self.logger.warn(
                "BlobSASAuthenticator:generateBlobSASSignature(): Must provide 'permissions' and 'expiryTime' for Blob SAS generation when 'identifier' is not provided.",
                context.and_then(|context| context.contextId()).as_deref(),
            );
            return None;
        }

        Some(IBlobSASSignatureValues {
            version,
            protocol: protocol.map(SASProtocolOrString::String),
            startTime: startTime.map(BlobDateOrString::String),
            expiryTime: expiryTime.map(BlobDateOrString::String),
            permissions,
            ipRange: ipRange.map(IIPRangeOrString::String),
            containerName: String::from(containerName),
            blobName: blobName.map(String::from),
            identifier,
            encryptionScope,
            cacheControl,
            contentDisposition,
            contentEncoding,
            contentLanguage,
            contentType,
            signedResource,
            snapshot,
            signedObjectId,
            signedTenantId,
            signedService,
            signedVersion,
            signedStartsOn,
            signedExpiresOn,
            delegatedUserObjectId: None,
            delegatedUserTenantId: None,
        })
    }

    fn validateTime(&self, expiry: Option<DateOrString>, start: Option<DateOrString>) -> bool {
        if expiry.is_none() && start.is_none() {
            return true;
        }
        let now = chrono::Utc::now();
        if let Some(expiry) = expiry {
            if now > parse_date_or_string(&expiry) {
                return false;
            }
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

    async fn getContainerAccessPolicyByIdentifier(
        &self,
        account: &str,
        container: &str,
        id: &str,
        context: &Context,
    ) -> Option<AccessPolicy> {
        let containerModel = self
            .blobMetadataStore
            .getContainerACL(context, account, container, None)
            .await
            .ok()??;
        let containerAcl = containerModel.containerAcl?;
        for acl in containerAcl {
            if signed_identifier_id(&acl).as_deref() == Some(id) {
                return signed_identifier_access_policy(&acl);
            }
        }
        None
    }

    async fn blobExist(
        &self,
        account: &str,
        container: &str,
        blob: &str,
    ) -> Result<bool, StorageError> {
        let blobModel = self
            .blobMetadataStore
            .getBlobType(account, container, blob, None)
            .await?;
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
impl IAuthenticator for BlobSASAuthenticator {
    async fn validate(
        &self,
        req: &GeneratedHttpRequest,
        content: &Context,
    ) -> Result<Option<bool>, StorageError> {
        let blobContext = BlobStorageContext::new(content);
        let account = blobContext.account().ok_or_else(|| {
            StorageErrorFactory::ResourceNotFound(blobContext.contextId().as_deref())
        })?;
        let containerName = match blobContext.container() {
            Some(containerName) => containerName,
            None => return Ok(None),
        };
        let blobName = blobContext.blob();

        let accountProperties = self.accountDataStore.getAccount(&account);
        if accountProperties.is_none() {
            return Err(StorageErrorFactory::ResourceNotFound(
                blobContext.contextId().as_deref(),
            ));
        }
        let accountProperties = accountProperties.unwrap();

        let signature = match decodeIfExist(req.getQuery("sig").as_deref()) {
            Some(signature) => signature,
            None => return Ok(None),
        };
        let resource = match decodeIfExist(req.getQuery("sr").as_deref()).as_deref() {
            Some("c") => BlobSASResourceType::Container,
            Some("b") => BlobSASResourceType::Blob,
            Some("bs") => BlobSASResourceType::BlobSnapshot,
            _ => return Ok(None),
        };

        let mut values = match self.getBlobSASSignatureValuesFromRequest(
            req,
            &containerName,
            blobName.as_deref(),
            Some(content),
        ) {
            Some(values) => values,
            None => return Ok(None),
        };

        if !blobContext.loose().unwrap_or(false) && values.encryptionScope.is_some() {
            return Err(StrictModelNotSupportedError::new(
                "SAS Encryption Scope 'ses'",
                blobContext.contextId().as_deref(),
            )
            .into());
        }

        if values.signedObjectId.is_some()
            || values.signedTenantId.is_some()
            || values.signedService.is_some()
            || values.signedVersion.is_some()
            || values.signedStartsOn.is_some()
            || values.signedExpiresOn.is_some()
        {
            if values.signedObjectId.is_none()
                || values.signedTenantId.is_none()
                || values.signedStartsOn.is_none()
                || values.signedExpiresOn.is_none()
                || values.signedService.is_none()
                || values.signedVersion.is_none()
                || values.signedService.as_deref() != Some("b")
            {
                return Err(StorageErrorFactory::getAuthorizationFailure(
                    blobContext.contextId().as_deref().unwrap_or(""),
                ));
            }

            if decodeIfExist(req.getQuery("si").as_deref()).is_some() {
                return Err(StorageErrorFactory::getAuthorizationFailure(
                    blobContext.contextId().as_deref().unwrap_or(""),
                ));
            }

            if !self.validateTime(
                values.signedExpiresOn.clone().map(DateOrString::String),
                values.signedStartsOn.clone().map(DateOrString::String),
            ) {
                return Err(StorageErrorFactory::getAuthorizationFailure(
                    blobContext.contextId().as_deref().unwrap_or(""),
                ));
            }

            let keyValue = getUserDelegationKeyValue(
                values.signedObjectId.as_deref().unwrap_or_default(),
                values.signedTenantId.as_deref().unwrap_or_default(),
                values.signedStartsOn.as_deref().unwrap_or_default(),
                values.signedExpiresOn.as_deref().unwrap_or_default(),
                values.signedVersion.as_deref().unwrap_or_default(),
            );
            let keyValue = STANDARD.decode(keyValue).unwrap_or_default();
            let (sig, _) = generateBlobSASSignatureWithUDK(&values, resource, &account, &keyValue);
            if sig != signature {
                return Ok(Some(false));
            }
        } else {
            let (sig1, _) =
                generateBlobSASSignature(&values, resource, &account, &accountProperties.key1);
            let sig1Pass = sig1 == signature;
            if let Some(key2) = accountProperties.key2.as_deref() {
                let (sig2, _) = generateBlobSASSignature(&values, resource, &account, key2);
                let sig2Pass = sig2 == signature;
                if !sig1Pass && !sig2Pass {
                    return Ok(Some(false));
                }
            } else if !sig1Pass {
                return Ok(Some(false));
            }
        }

        if let Some(identifier) = values.identifier.clone() {
            let accessPolicy = self
                .getContainerAccessPolicyByIdentifier(
                    &account,
                    &containerName,
                    &identifier,
                    content,
                )
                .await;
            if accessPolicy.is_none() {
                return Err(StorageErrorFactory::getAuthorizationFailure(
                    blobContext.contextId().as_deref().unwrap_or(""),
                ));
            }
            let accessPolicy = accessPolicy.unwrap();
            values.startTime =
                access_policy_field(&accessPolicy, "start").map(BlobDateOrString::String);
            values.expiryTime =
                access_policy_field(&accessPolicy, "expiry").map(BlobDateOrString::String);
            values.permissions = access_policy_field(&accessPolicy, "permission");
        }

        if !self.validateTime(
            values.expiryTime.clone().map(into_common_date),
            values.startTime.clone().map(into_common_date),
        ) {
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
                "BlobSASAuthenticator:validate() Operation shouldn't be undefined. Please make sure DispatchMiddleware is hooked before authentication related middleware."
            )
        });
        if operation == Operation::Service_GetUserDelegationKey {
            return Err(StorageErrorFactory::getAuthenticationFailed(
                blobContext.contextId().as_deref(),
                AUTHENTICATION_BEARERTOKEN_REQUIRED,
            ));
        }

        let blobSASPermission = if resource == BlobSASResourceType::Blob {
            OPERATION_BLOB_SAS_BLOB_PERMISSIONS.get(&operation)
        } else {
            OPERATION_BLOB_SAS_CONTAINER_PERMISSIONS.get(&operation)
        }
        .unwrap_or_else(|| {
            panic!(
                "BlobSASAuthenticator:validate() permission table doesn't have configuration for operation {}.",
                operation.as_str()
            )
        });

        if !blobSASPermission.validatePermissions(values.permissions.as_deref().unwrap_or_default())
        {
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
            if let Some(blobName) = blobName.as_deref() {
                if self.blobExist(&account, &containerName, blobName).await?
                    && !values
                        .permissions
                        .as_deref()
                        .unwrap_or_default()
                        .contains(BlobSASPermission::Write.as_str())
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

fn into_common_date(value: BlobDateOrString) -> DateOrString {
    match value {
        BlobDateOrString::Date(value) => DateOrString::Date(value),
        BlobDateOrString::String(value) => DateOrString::String(value),
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

fn getUserDelegationKeyValue(
    signedObjectid: &str,
    signedTenantid: &str,
    signedStartsOn: &str,
    signedExpiresOn: &str,
    signedVersion: &str,
) -> String {
    let stringToSign = [
        signedObjectid,
        signedTenantid,
        signedStartsOn,
        signedExpiresOn,
        "b",
        signedVersion,
    ]
    .join("\n");
    computeHMACSHA256(&stringToSign, &USERDELEGATIONKEY_BASIC_KEY_BYTES)
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
