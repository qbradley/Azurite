use std::sync::Arc;

use async_trait::async_trait;
use azurite_common::i_account_data_store::IAccountDataStore;
use azurite_common::i_logger::ILogger;
use chrono::Utc;

use crate::context::TableStorageContext;
use crate::errors::{StorageError, StorageErrorFactory};
use crate::generated::context::Context;
use crate::generated::i_request::{GeneratedHttpRequest, IRequest};
use crate::persistence::{AccessPolicy, ITableMetadataStore};

use super::i_authenticator::IAuthenticator;
use super::i_table_sas_signature_values::{
    generateTableSASSignature, DateOrString, IIPRangeOrString, ITableSASSignatureValues,
    SASProtocolOrString,
};
use super::operation_table_sas_permission::OPERATION_TABLE_SAS_TABLE_PERMISSIONS;

pub struct TableSASAuthenticator {
    accountDataStore: Arc<dyn IAccountDataStore + Send + Sync>,
    tableMetadataStore: Arc<dyn ITableMetadataStore + Send + Sync>,
    _logger: Arc<dyn ILogger + Send + Sync>,
}

impl TableSASAuthenticator {
    pub fn new(
        accountDataStore: Arc<dyn IAccountDataStore + Send + Sync>,
        tableMetadataStore: Arc<dyn ITableMetadataStore + Send + Sync>,
        logger: Arc<dyn ILogger + Send + Sync>,
    ) -> Self {
        Self {
            accountDataStore,
            tableMetadataStore,
            _logger: logger,
        }
    }

    fn getTableSASSignatureValuesFromRequest(
        &self,
        req: &GeneratedHttpRequest,
        tableName: &str,
    ) -> Option<ITableSASSignatureValues> {
        let version = decodeIfExist(req.getQuery("sv").as_deref())?;
        let protocol = decodeIfExist(req.getQuery("spr").as_deref());
        let startTime = decodeIfExist(req.getQuery("st").as_deref());
        let expiryTime = decodeIfExist(req.getQuery("se").as_deref());
        let permissions = decodeIfExist(req.getQuery("sp").as_deref());
        let ipRange = decodeIfExist(req.getQuery("sip").as_deref());
        let identifier = decodeIfExist(req.getQuery("si").as_deref());
        let startingPartitionKey = decodeIfExist(req.getQuery("spk").as_deref());
        let startingRowKey = decodeIfExist(req.getQuery("srk").as_deref());
        let endingPartitionKey = decodeIfExist(req.getQuery("epk").as_deref());
        let endingRowKey = decodeIfExist(req.getQuery("erk").as_deref());

        if identifier.is_none() && (permissions.is_none() || expiryTime.is_none()) {
            return None;
        }

        Some(ITableSASSignatureValues {
            version,
            protocol: protocol.map(SASProtocolOrString::String),
            startTime: startTime.map(DateOrString::String),
            expiryTime: expiryTime.map(DateOrString::String),
            permissions,
            ipRange: ipRange.map(IIPRangeOrString::String),
            tableName: tableName.to_owned(),
            identifier,
            startingPartitionKey,
            startingRowKey,
            endingPartitionKey,
            endingRowKey,
        })
    }

    fn validateTime(&self, expiry: Option<DateOrString>, start: Option<DateOrString>) -> bool {
        if expiry.is_none() && start.is_none() {
            return true;
        }

        let now = Utc::now();
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

    async fn getTableAccessPolicyByIdentifier(
        &self,
        account: &str,
        table: &str,
        id: &str,
        context: &Context,
    ) -> Option<AccessPolicy> {
        let tableModel = self
            .tableMetadataStore
            .getTable(account, table, context)
            .await
            .ok()??;
        let tableAcl = tableModel.tableAcl?;
        tableAcl
            .into_iter()
            .find(|acl| acl.id == id)
            .map(|acl| acl.accessPolicy)
    }
}

#[async_trait]
impl IAuthenticator for TableSASAuthenticator {
    async fn validate(
        &self,
        req: &GeneratedHttpRequest,
        content: &Context,
    ) -> Result<Option<bool>, StorageError> {
        let tableContext = TableStorageContext::new(content);
        let account = tableContext
            .account()
            .ok_or_else(|| StorageErrorFactory::getAccountNameEmpty(content))?;
        let tableName = match tableContext.tableName() {
            Some(table_name) => table_name,
            None => return Ok(None),
        };

        let accountProperties = self
            .accountDataStore
            .getAccount(&account)
            .ok_or_else(|| StorageErrorFactory::ResourceNotFoundXml(content))?;

        let signature = decodeIfExist(req.getQuery("sig").as_deref());
        if signature.is_none() {
            return Ok(None);
        }
        let values = match self.getTableSASSignatureValuesFromRequest(req, &tableName) {
            Some(values) => values,
            None => return Ok(None),
        };

        let (sig1, _) = generateTableSASSignature(&values, &account, &accountProperties.key1);
        let sig1Pass = Some(sig1) == signature;
        if let Some(key2) = accountProperties.key2.as_deref() {
            let (sig2, _) = generateTableSASSignature(&values, &account, key2);
            if !sig1Pass && Some(sig2) != signature {
                return Ok(Some(false));
            }
        } else if !sig1Pass {
            return Ok(Some(false));
        }

        let mut effectiveValues = values;
        if let Some(identifier) = effectiveValues.identifier.clone() {
            let accessPolicy = self
                .getTableAccessPolicyByIdentifier(&account, &tableName, &identifier, content)
                .await
                .ok_or_else(|| StorageErrorFactory::getAuthorizationFailure(content))?;
            effectiveValues.startTime = Some(DateOrString::String(accessPolicy.start));
            effectiveValues.expiryTime = Some(DateOrString::String(accessPolicy.expiry));
            effectiveValues.permissions = Some(accessPolicy.permission);
        }

        if !self.validateTime(
            effectiveValues.expiryTime.clone(),
            effectiveValues.startTime.clone(),
        ) {
            return Err(StorageErrorFactory::getAuthorizationFailure(content));
        }
        if !self.validateIPRange() {
            return Err(StorageErrorFactory::getAuthorizationSourceIPMismatch(
                content,
            ));
        }
        if !self.validateProtocol(effectiveValues.protocol.clone(), &req.getProtocol()) {
            return Err(StorageErrorFactory::getAuthorizationProtocolMismatch(
                content,
            ));
        }

        let operation = tableContext.operation().unwrap_or_else(|| {
            panic!(
                "TableSASAuthenticator:validate() Operation shouldn't be undefined. Please make sure DispatchMiddleware is hooked before authentication related middleware."
            )
        });
        let tableSASPermission = OPERATION_TABLE_SAS_TABLE_PERMISSIONS.get(&operation).unwrap_or_else(|| {
            panic!(
                "TableSASAuthenticator:validate() OPERATION_TABLE_SAS_TABLE_PERMISSIONS doesn't have configuration for operation {}'s table service SAS permission.",
                operation.as_str()
            )
        });

        if !tableSASPermission
            .validatePermissions(effectiveValues.permissions.clone().unwrap_or_default())
        {
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
