use std::collections::{BTreeMap, BTreeSet};
use std::io::ErrorKind;
use std::path::Path;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use azurite_common::i_cleaner::ICleaner;
use azurite_common::storage_error::StorageError as CommonStorageError;
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::context::TableStorageContext;
use crate::errors::{StorageError, StorageErrorFactory};
use crate::generated::{
    artifacts::models::{self, GeneratedBody, GeneratedObject, GeneratedResponse, GeneratedValue},
    context::Context,
};
use crate::utils::{
    constants::{ODATA_TYPE, QUERY_RESULT_MAX_NUM},
    utils::{new_high_precision_timestamp, new_table_entity_etag},
};

use super::{
    AccessPolicy, Entity, ITableMetadataStore, LokiTableStoreQueryGenerator,
    ServicePropertiesModel, SignedIdentifier, Table, TableACL,
};

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
struct EntityKey {
    partition_key: String,
    row_key: String,
}

impl EntityKey {
    fn new(partition_key: impl Into<String>, row_key: impl Into<String>) -> Self {
        Self {
            partition_key: partition_key.into(),
            row_key: row_key.into(),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct TransactionState {
    collection_key: Option<String>,
    rollback_entities: BTreeMap<EntityKey, Entity>,
    inserted_entities: BTreeSet<EntityKey>,
}

#[derive(Debug, Clone, Default)]
struct LokiDatabase {
    initialized: bool,
    closed: bool,
    tables: BTreeMap<String, Table>,
    services: BTreeMap<String, ServicePropertiesModel>,
    entity_collections: BTreeMap<String, BTreeMap<EntityKey, Entity>>,
    transactions: BTreeMap<String, TransactionState>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct LokiDatabaseFile {
    tables: Vec<TableDocument>,
    services: Vec<ServicePropertiesDocument>,
    entity_collections: Vec<EntityCollectionDocument>,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccessPolicyDocument {
    start: String,
    expiry: String,
    permission: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SignedIdentifierDocument {
    id: String,
    accessPolicy: AccessPolicyDocument,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TableDocument {
    key: String,
    account: String,
    table: String,
    tableAcl: Option<Vec<SignedIdentifierDocument>>,
    odatametadata: Option<String>,
    odatatype: Option<String>,
    odataid: Option<String>,
    odataeditLink: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServicePropertiesDocument {
    accountName: String,
    properties: BTreeMap<String, serde_json::Value>,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EntityDocument {
    PartitionKey: String,
    RowKey: String,
    eTag: String,
    lastModifiedTime: String,
    properties: BTreeMap<String, serde_json::Value>,
    odatametadata: Option<String>,
    odatatype: Option<String>,
    odataid: Option<String>,
    odataeditLink: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EntityCollectionDocument {
    key: String,
    entities: Vec<EntityDocument>,
}

#[derive(Debug, Clone)]
pub struct LokiTableMetadataStore {
    pub lokiDBPath: String,
    pub inMemory: bool,
    db: Arc<Mutex<LokiDatabase>>,
}

impl Default for LokiTableMetadataStore {
    fn default() -> Self {
        Self::new("azurite-table.db", true)
    }
}

#[allow(non_snake_case)]
impl LokiTableMetadataStore {
    pub fn new(lokiDBPath: impl Into<String>, inMemory: bool) -> Self {
        Self {
            lokiDBPath: lokiDBPath.into(),
            inMemory,
            db: Arc::new(Mutex::new(LokiDatabase::default())),
        }
    }

    pub fn isInitialized(&self) -> bool {
        self.db.lock().unwrap().initialized
    }

    pub fn isClosed(&self) -> bool {
        self.db.lock().unwrap().closed
    }

    fn table_collection_key(account: &str, table: &str) -> String {
        format!("{account}${}", table.to_ascii_lowercase())
    }

    fn entity_key(partition_key: &str, row_key: &str) -> EntityKey {
        EntityKey::new(partition_key, row_key)
    }

    fn normalize_entity_for_storage(&self, context: &Context, entity: &Entity) -> Entity {
        let mut normalized = entity.clone();
        if normalized.lastModifiedTime.is_empty() {
            normalized.lastModifiedTime =
                new_high_precision_timestamp(context.startTime().unwrap_or_else(Utc::now));
        }
        if normalized.eTag.is_empty() {
            normalized.eTag = new_table_entity_etag(&normalized.lastModifiedTime);
        }
        normalized.properties.insert(
            "Timestamp".to_string(),
            GeneratedValue::String(normalized.lastModifiedTime.clone()),
        );
        normalized.properties.insert(
            format!("Timestamp{ODATA_TYPE}"),
            GeneratedValue::String("Edm.DateTime".to_string()),
        );
        normalized
    }

    fn table_collection<'a>(
        &self,
        state: &'a LokiDatabase,
        account: &str,
        table: &str,
    ) -> Option<&'a Table> {
        state
            .tables
            .get(&Self::table_collection_key(account, table))
    }

    fn table_collection_mut<'a>(
        &self,
        state: &'a mut LokiDatabase,
        account: &str,
        table: &str,
    ) -> Option<&'a mut Table> {
        state
            .tables
            .get_mut(&Self::table_collection_key(account, table))
    }

    fn require_entity_collection<'a>(
        &self,
        state: &'a LokiDatabase,
        account: &str,
        table: &str,
        context: &Context,
    ) -> Result<&'a BTreeMap<EntityKey, Entity>, StorageError> {
        let key = Self::table_collection_key(account, table);
        state
            .entity_collections
            .get(&key)
            .ok_or_else(|| StorageErrorFactory::getTableNotExist(context))
    }

    fn require_entity_collection_mut<'a>(
        &self,
        state: &'a mut LokiDatabase,
        account: &str,
        table: &str,
        context: &Context,
    ) -> Result<&'a mut BTreeMap<EntityKey, Entity>, StorageError> {
        let key = Self::table_collection_key(account, table);
        if !state.tables.contains_key(&key) {
            return Err(StorageErrorFactory::getTableNotExist(context));
        }
        Ok(state.entity_collections.entry(key).or_default())
    }

    fn ensure_transaction<'a>(
        state: &'a mut LokiDatabase,
        batchID: Option<&str>,
        collection_key: &str,
    ) -> Option<&'a mut TransactionState> {
        let batchID = batchID.filter(|value| !value.is_empty())?;
        let transaction = state.transactions.entry(batchID.to_string()).or_default();
        if transaction.collection_key.is_none() {
            transaction.collection_key = Some(collection_key.to_string());
        }
        Some(transaction)
    }

    fn record_existing_entity(
        transaction: &mut TransactionState,
        key: &EntityKey,
        existing: &Entity,
    ) {
        if transaction.inserted_entities.contains(key)
            || transaction.rollback_entities.contains_key(key)
        {
            return;
        }
        transaction
            .rollback_entities
            .insert(key.clone(), existing.clone());
    }

    fn record_inserted_entity(transaction: &mut TransactionState, key: &EntityKey) {
        if !transaction.rollback_entities.contains_key(key) {
            transaction.inserted_entities.insert(key.clone());
        }
    }

    fn matches_if_match(&self, etag: &str, ifMatch: Option<&str>) -> bool {
        let Some(ifMatch) = ifMatch else {
            return true;
        };
        if ifMatch == "*" {
            return true;
        }

        Self::encode_if_match(etag) == Self::encode_if_match(ifMatch)
    }

    fn encode_if_match(value: &str) -> String {
        value.replace(':', "%3A")
    }

    fn decode_continuation_header(input: Option<&str>) -> Option<String> {
        input
            .and_then(|value| BASE64_STANDARD.decode(value).ok())
            .and_then(|bytes| String::from_utf8(bytes).ok())
    }

    fn encode_continuation_header(input: Option<&str>) -> Option<String> {
        input.map(|value| BASE64_STANDARD.encode(value.as_bytes()))
    }

    fn getMaxResultsOption(queryOptions: &models::QueryOptions) -> usize {
        match queryOptions.top {
            Some(top) if top <= QUERY_RESULT_MAX_NUM => top,
            _ => QUERY_RESULT_MAX_NUM,
        }
    }

    fn adjustQueryResultforTop(
        result: &mut Vec<Entity>,
        maxResults: usize,
    ) -> (Option<String>, Option<String>) {
        if result.len() > maxResults {
            if let Some(tail) = result.pop() {
                return (
                    Self::encode_continuation_header(Some(&tail.PartitionKey)),
                    Self::encode_continuation_header(Some(&tail.RowKey)),
                );
            }
        }

        (None, None)
    }

    fn signed_identifier_to_value(identifier: &SignedIdentifier) -> GeneratedValue {
        let mut access_policy = GeneratedObject::new();
        access_policy.insert(
            "start".to_string(),
            GeneratedValue::String(identifier.accessPolicy.start.clone()),
        );
        access_policy.insert(
            "expiry".to_string(),
            GeneratedValue::String(identifier.accessPolicy.expiry.clone()),
        );
        access_policy.insert(
            "permission".to_string(),
            GeneratedValue::String(identifier.accessPolicy.permission.clone()),
        );

        let mut object = GeneratedObject::new();
        object.insert(
            "id".to_string(),
            GeneratedValue::String(identifier.id.clone()),
        );
        object.insert(
            "accessPolicy".to_string(),
            GeneratedValue::Object(access_policy),
        );
        GeneratedValue::Object(object)
    }

    fn parse_signed_identifier(value: &GeneratedValue) -> Option<SignedIdentifier> {
        let object = value.as_object()?;
        let id = object.get("id")?.as_string()?;
        let accessPolicy = object.get("accessPolicy")?.as_object()?;
        Some(SignedIdentifier {
            id,
            accessPolicy: AccessPolicy {
                start: accessPolicy.get("start")?.as_string()?,
                expiry: accessPolicy.get("expiry")?.as_string()?,
                permission: accessPolicy.get("permission")?.as_string()?,
            },
        })
    }

    fn parse_signed_identifiers(options: &GeneratedObject) -> Option<Vec<SignedIdentifier>> {
        ["signedIdentifiers", "SignedIdentifiers", "tableAcl"]
            .iter()
            .find_map(|key| match options.get(*key) {
                Some(GeneratedValue::Array(values)) => values
                    .iter()
                    .map(Self::parse_signed_identifier)
                    .collect::<Option<Vec<_>>>(),
                _ => None,
            })
    }

    fn merge_service_properties(
        current: &ServicePropertiesModel,
        update: &ServicePropertiesModel,
    ) -> ServicePropertiesModel {
        let mut merged = current.clone();
        for (key, value) in &update.properties {
            merged.properties.insert(key.clone(), value.clone());
        }
        merged
    }

    fn internal_error(message: impl Into<String>) -> StorageError {
        StorageError::new(
            500,
            "InternalError",
            message.into(),
            "DefaultID",
            BTreeMap::new(),
            &Context::default(),
        )
    }

    fn common_error(message: impl Into<String>) -> CommonStorageError {
        CommonStorageError::new(message.into())
    }

    fn generated_object_to_json(object: &GeneratedObject) -> BTreeMap<String, serde_json::Value> {
        object
            .iter()
            .map(|(key, value)| (key.clone(), value.to_json_value()))
            .collect()
    }

    fn generated_object_from_json(object: BTreeMap<String, serde_json::Value>) -> GeneratedObject {
        object
            .into_iter()
            .map(|(key, value)| (key, GeneratedValue::from(value)))
            .collect()
    }

    fn to_table_document(key: String, table: &Table) -> TableDocument {
        TableDocument {
            key,
            account: table.account.clone(),
            table: table.table.clone(),
            tableAcl: table.tableAcl.clone().map(|acl| {
                acl.into_iter()
                    .map(|identifier| SignedIdentifierDocument {
                        id: identifier.id,
                        accessPolicy: AccessPolicyDocument {
                            start: identifier.accessPolicy.start,
                            expiry: identifier.accessPolicy.expiry,
                            permission: identifier.accessPolicy.permission,
                        },
                    })
                    .collect()
            }),
            odatametadata: table.odatametadata.clone(),
            odatatype: table.odatatype.clone(),
            odataid: table.odataid.clone(),
            odataeditLink: table.odataeditLink.clone(),
        }
    }

    fn from_table_document(document: TableDocument) -> (String, Table) {
        (
            document.key,
            Table {
                account: document.account,
                table: document.table,
                tableAcl: document.tableAcl.map(|acl| {
                    acl.into_iter()
                        .map(|identifier| SignedIdentifier {
                            id: identifier.id,
                            accessPolicy: AccessPolicy {
                                start: identifier.accessPolicy.start,
                                expiry: identifier.accessPolicy.expiry,
                                permission: identifier.accessPolicy.permission,
                            },
                        })
                        .collect()
                }),
                odatametadata: document.odatametadata,
                odatatype: document.odatatype,
                odataid: document.odataid,
                odataeditLink: document.odataeditLink,
            },
        )
    }

    fn to_service_document(service: &ServicePropertiesModel) -> ServicePropertiesDocument {
        ServicePropertiesDocument {
            accountName: service.accountName.clone(),
            properties: Self::generated_object_to_json(&service.properties),
        }
    }

    fn from_service_document(document: ServicePropertiesDocument) -> ServicePropertiesModel {
        ServicePropertiesModel {
            accountName: document.accountName,
            properties: Self::generated_object_from_json(document.properties),
        }
    }

    fn to_entity_document(entity: &Entity) -> EntityDocument {
        EntityDocument {
            PartitionKey: entity.PartitionKey.clone(),
            RowKey: entity.RowKey.clone(),
            eTag: entity.eTag.clone(),
            lastModifiedTime: entity.lastModifiedTime.clone(),
            properties: Self::generated_object_to_json(&entity.properties),
            odatametadata: entity.odatametadata.clone(),
            odatatype: entity.odatatype.clone(),
            odataid: entity.odataid.clone(),
            odataeditLink: entity.odataeditLink.clone(),
        }
    }

    fn from_entity_document(document: EntityDocument) -> (EntityKey, Entity) {
        let key = EntityKey::new(document.PartitionKey.clone(), document.RowKey.clone());
        (
            key,
            Entity {
                PartitionKey: document.PartitionKey,
                RowKey: document.RowKey,
                eTag: document.eTag,
                lastModifiedTime: document.lastModifiedTime,
                properties: Self::generated_object_from_json(document.properties),
                odatametadata: document.odatametadata,
                odatatype: document.odatatype,
                odataid: document.odataid,
                odataeditLink: document.odataeditLink,
            },
        )
    }

    fn snapshot_database(&self) -> LokiDatabaseFile {
        let db = self.db.lock().unwrap();
        LokiDatabaseFile {
            tables: db
                .tables
                .iter()
                .map(|(key, table)| Self::to_table_document(key.clone(), table))
                .collect(),
            services: db
                .services
                .values()
                .map(Self::to_service_document)
                .collect(),
            entity_collections: db
                .entity_collections
                .iter()
                .map(|(key, collection)| EntityCollectionDocument {
                    key: key.clone(),
                    entities: collection.values().map(Self::to_entity_document).collect(),
                })
                .collect(),
        }
    }

    async fn loadDatabase(&self) -> Result<(), StorageError> {
        if self.inMemory {
            return Ok(());
        }

        let bytes = match fs::read(&self.lokiDBPath).await {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(Self::internal_error(error.to_string())),
        };
        if bytes.is_empty() {
            return Ok(());
        }

        let persisted: LokiDatabaseFile = serde_json::from_slice(&bytes)
            .map_err(|error| Self::internal_error(error.to_string()))?;
        let mut db = self.db.lock().unwrap();
        db.tables = persisted
            .tables
            .into_iter()
            .map(Self::from_table_document)
            .collect();
        db.services = persisted
            .services
            .into_iter()
            .map(|service| {
                let runtime = Self::from_service_document(service);
                (runtime.accountName.clone(), runtime)
            })
            .collect();
        db.entity_collections = persisted
            .entity_collections
            .into_iter()
            .map(|collection| {
                (
                    collection.key,
                    collection
                        .entities
                        .into_iter()
                        .map(Self::from_entity_document)
                        .collect(),
                )
            })
            .collect();
        db.transactions.clear();
        Ok(())
    }

    async fn saveDatabase(&self) -> Result<(), StorageError> {
        if self.inMemory {
            return Ok(());
        }

        let snapshot = self.snapshot_database();
        if let Some(parent) = Path::new(&self.lokiDBPath).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(|error| Self::internal_error(error.to_string()))?;
            }
        }

        let bytes = serde_json::to_vec_pretty(&snapshot)
            .map_err(|error| Self::internal_error(error.to_string()))?;
        fs::write(&self.lokiDBPath, bytes)
            .await
            .map_err(|error| Self::internal_error(error.to_string()))
    }

    async fn save_if_needed(&self, batchID: Option<&str>) -> Result<(), StorageError> {
        if matches!(batchID, Some(value) if !value.is_empty()) {
            return Ok(());
        }
        self.saveDatabase().await
    }
}

#[async_trait]
#[allow(non_snake_case)]
impl ITableMetadataStore for LokiTableMetadataStore {
    async fn init(&self) -> Result<(), StorageError> {
        self.loadDatabase().await?;
        {
            let mut state = self.db.lock().unwrap();
            state.initialized = true;
            state.closed = false;
            state.transactions.clear();
        }
        self.saveDatabase().await
    }

    async fn close(&self) -> Result<(), StorageError> {
        self.saveDatabase().await?;
        let mut state = self.db.lock().unwrap();
        state.closed = true;
        Ok(())
    }

    async fn createTable(&self, context: &Context, tableModel: Table) -> Result<(), StorageError> {
        let key = Self::table_collection_key(&tableModel.account, &tableModel.table);
        {
            let mut state = self.db.lock().unwrap();
            if state.tables.contains_key(&key) {
                return Err(StorageErrorFactory::getTableAlreadyExists(context));
            }
            state.tables.insert(key.clone(), tableModel);
            state.entity_collections.entry(key).or_default();
        }
        self.saveDatabase().await
    }

    async fn getTable(
        &self,
        account: &str,
        table: &str,
        _context: &Context,
    ) -> Result<Option<Table>, StorageError> {
        let state = self.db.lock().unwrap();
        Ok(self.table_collection(&state, account, table).cloned())
    }

    async fn queryTable(
        &self,
        context: &Context,
        account: &str,
        queryOptions: models::QueryOptions,
        nextTable: Option<&str>,
    ) -> Result<(Vec<Table>, Option<String>), StorageError> {
        let predicate = LokiTableStoreQueryGenerator::generateQueryTableWhereFunction(
            queryOptions.filter.as_deref(),
        )
        .map_err(|_| StorageErrorFactory::getQueryConditionInvalid(context))?;

        let top = queryOptions.top.unwrap_or(QUERY_RESULT_MAX_NUM);
        let state = self.db.lock().unwrap();
        let mut docList = state
            .tables
            .values()
            .filter(|table| table.account == account)
            .filter(|table| nextTable.map_or(true, |next| table.table.as_str() >= next))
            .filter(|table| predicate(table))
            .cloned()
            .collect::<Vec<_>>();
        docList.sort_by(|left, right| left.table.cmp(&right.table));

        let nextTableName = if docList.len() > top {
            docList.pop().map(|tail| tail.table)
        } else {
            None
        };

        Ok((docList, nextTableName))
    }

    async fn deleteTable(
        &self,
        context: &Context,
        table: &str,
        account: &str,
    ) -> Result<(), StorageError> {
        let key = Self::table_collection_key(account, table);
        {
            let mut state = self.db.lock().unwrap();
            if state.tables.remove(&key).is_none() {
                return Err(StorageErrorFactory::ResourceNotFound(context));
            }
            state.entity_collections.remove(&key);
        }
        self.saveDatabase().await
    }

    async fn setTableACL(
        &self,
        account: &str,
        table: &str,
        context: &Context,
        tableACL: Option<TableACL>,
    ) -> Result<(), StorageError> {
        {
            let mut state = self.db.lock().unwrap();
            let persistedTable = self
                .table_collection_mut(&mut state, account, table)
                .ok_or_else(|| StorageErrorFactory::getTableNotFound(context))?;
            persistedTable.tableAcl = tableACL;
        }
        self.saveDatabase().await
    }

    async fn queryTableEntities(
        &self,
        context: &Context,
        account: &str,
        table: &str,
        queryOptions: models::QueryOptions,
        nextPartitionKey: Option<&str>,
        nextRowKey: Option<&str>,
    ) -> Result<(Vec<Entity>, Option<String>, Option<String>), StorageError> {
        let predicate =
            LokiTableStoreQueryGenerator::generateQueryForPersistenceLayer(&queryOptions, context)?;
        let decodedNextPartitionKey = Self::decode_continuation_header(nextPartitionKey);
        let decodedNextRowKey = Self::decode_continuation_header(nextRowKey);
        let maxResults = Self::getMaxResultsOption(&queryOptions);

        let state = self.db.lock().unwrap();
        let mut result = self
            .require_entity_collection(&state, account, table, context)?
            .values()
            .filter(|entity| predicate(entity))
            .filter(|entity| {
                if let Some(next_partition_key) = decodedNextPartitionKey.as_deref() {
                    if entity.PartitionKey.as_str() > next_partition_key {
                        return true;
                    }
                }
                if let Some(next_row_key) = decodedNextRowKey.as_deref() {
                    return entity.RowKey.as_str() >= next_row_key
                        && decodedNextPartitionKey
                            .as_deref()
                            .map_or(true, |next_partition_key| {
                                entity.PartitionKey.as_str() == next_partition_key
                            });
                }
                if let Some(next_partition_key) = decodedNextPartitionKey.as_deref() {
                    if entity.PartitionKey.as_str() < next_partition_key {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect::<Vec<_>>();

        result.sort_by(|left, right| {
            left.PartitionKey
                .cmp(&right.PartitionKey)
                .then_with(|| left.RowKey.cmp(&right.RowKey))
        });
        result.truncate(maxResults + 1);

        let (nextPartitionKeyResponse, nextRowKeyResponse) =
            Self::adjustQueryResultforTop(&mut result, maxResults);
        Ok((result, nextPartitionKeyResponse, nextRowKeyResponse))
    }

    async fn queryTableEntitiesWithPartitionAndRowKey(
        &self,
        context: &Context,
        table: &str,
        account: &str,
        partitionKey: &str,
        rowKey: &str,
        _batchID: Option<&str>,
    ) -> Result<Option<Entity>, StorageError> {
        let state = self.db.lock().unwrap();
        let entityCollection = self.require_entity_collection(&state, account, table, context)?;
        Ok(entityCollection
            .get(&Self::entity_key(partitionKey, rowKey))
            .cloned())
    }

    async fn insertTableEntity(
        &self,
        context: &Context,
        table: &str,
        account: &str,
        entity: Entity,
        batchID: Option<&str>,
    ) -> Result<Entity, StorageError> {
        let collection_key = Self::table_collection_key(account, table);
        let key = Self::entity_key(&entity.PartitionKey, &entity.RowKey);
        let entity = self.normalize_entity_for_storage(context, &entity);

        {
            let mut state = self.db.lock().unwrap();
            if self
                .require_entity_collection(&state, account, table, context)?
                .contains_key(&key)
            {
                return Err(StorageErrorFactory::getEntityAlreadyExist(context));
            }

            if let Some(transaction) =
                Self::ensure_transaction(&mut state, batchID, &collection_key)
            {
                Self::record_inserted_entity(transaction, &key);
            }
            self.require_entity_collection_mut(&mut state, account, table, context)?
                .insert(key, entity.clone());
        }
        self.save_if_needed(batchID).await?;
        Ok(entity)
    }

    async fn insertOrUpdateTableEntity(
        &self,
        context: &Context,
        table: &str,
        account: &str,
        entity: Entity,
        ifMatch: Option<&str>,
        batchID: Option<&str>,
    ) -> Result<Entity, StorageError> {
        if ifMatch.is_none() {
            if self
                .queryTableEntitiesWithPartitionAndRowKey(
                    context,
                    table,
                    account,
                    &entity.PartitionKey,
                    &entity.RowKey,
                    batchID,
                )
                .await?
                .is_some()
            {
                return self
                    .updateTableEntity(context, table, account, entity, ifMatch, batchID)
                    .await;
            }
            return self
                .insertTableEntity(context, table, account, entity, batchID)
                .await;
        }

        self.updateTableEntity(context, table, account, entity, ifMatch, batchID)
            .await
    }

    async fn insertOrMergeTableEntity(
        &self,
        context: &Context,
        table: &str,
        account: &str,
        entity: Entity,
        ifMatch: Option<&str>,
        batchID: Option<&str>,
    ) -> Result<Entity, StorageError> {
        if ifMatch.is_none() {
            if self
                .queryTableEntitiesWithPartitionAndRowKey(
                    context,
                    table,
                    account,
                    &entity.PartitionKey,
                    &entity.RowKey,
                    batchID,
                )
                .await?
                .is_some()
            {
                return self
                    .mergeTableEntity(context, table, account, entity, ifMatch, batchID)
                    .await;
            }
            return self
                .insertTableEntity(context, table, account, entity, batchID)
                .await;
        }

        self.mergeTableEntity(context, table, account, entity, ifMatch, batchID)
            .await
    }

    async fn deleteTableEntity(
        &self,
        context: &Context,
        table: &str,
        account: &str,
        partitionKey: &str,
        rowKey: &str,
        etag: &str,
        batchID: Option<&str>,
    ) -> Result<(), StorageError> {
        let collection_key = Self::table_collection_key(account, table);
        let key = Self::entity_key(partitionKey, rowKey);
        {
            let mut state = self.db.lock().unwrap();
            let existing = self
                .require_entity_collection(&state, account, table, context)?
                .get(&key)
                .cloned()
                .ok_or_else(|| StorageErrorFactory::getEntityNotFound(context))?;
            if !self.matches_if_match(&existing.eTag, Some(etag)) {
                return Err(StorageErrorFactory::getPreconditionFailed(context));
            }
            if let Some(transaction) =
                Self::ensure_transaction(&mut state, batchID, &collection_key)
            {
                Self::record_existing_entity(transaction, &key, &existing);
            }
            self.require_entity_collection_mut(&mut state, account, table, context)?
                .remove(&key);
        }
        self.save_if_needed(batchID).await
    }

    async fn getTableAccessPolicy(
        &self,
        context: &Context,
        table: &str,
        _options: models::TableGetAccessPolicyOptionalParams,
    ) -> Result<models::TableGetAccessPolicyResponse, StorageError> {
        let tableContext = TableStorageContext::new(context);
        let account = tableContext
            .account()
            .ok_or_else(|| StorageErrorFactory::getAccountNameEmpty(context))?;
        let signedIdentifiers = self
            .getTable(&account, table, context)
            .await?
            .and_then(|table| table.tableAcl)
            .unwrap_or_default();

        let mut response = GeneratedResponse::new(200);
        response.body = Some(GeneratedBody::Value(GeneratedValue::Array(
            signedIdentifiers
                .iter()
                .map(Self::signed_identifier_to_value)
                .collect(),
        )));
        Ok(response)
    }

    async fn setTableAccessPolicy(
        &self,
        context: &Context,
        table: &str,
        options: models::TableSetAccessPolicyOptionalParams,
    ) -> Result<models::TableSetAccessPolicyResponse, StorageError> {
        let tableContext = TableStorageContext::new(context);
        let account = tableContext
            .account()
            .ok_or_else(|| StorageErrorFactory::getAccountNameEmpty(context))?;
        let tableACL = Self::parse_signed_identifiers(&options);
        self.setTableACL(&account, table, context, tableACL).await?;
        Ok(GeneratedResponse::new(204))
    }

    async fn getServiceProperties(
        &self,
        _context: &Context,
        account: &str,
    ) -> Result<Option<ServicePropertiesModel>, StorageError> {
        let state = self.db.lock().unwrap();
        Ok(state.services.get(account).cloned())
    }

    async fn setServiceProperties(
        &self,
        context: &Context,
        serviceProperties: ServicePropertiesModel,
    ) -> Result<ServicePropertiesModel, StorageError> {
        let accountName = if serviceProperties.accountName.is_empty() {
            TableStorageContext::new(context)
                .account()
                .ok_or_else(|| StorageErrorFactory::getAccountNameEmpty(context))?
        } else {
            serviceProperties.accountName.clone()
        };
        let update = ServicePropertiesModel {
            accountName: accountName.clone(),
            ..serviceProperties
        };

        let merged = {
            let mut state = self.db.lock().unwrap();
            let merged = match state.services.get(&accountName) {
                Some(current) => Self::merge_service_properties(current, &update),
                None => update,
            };
            state.services.insert(accountName, merged.clone());
            merged
        };
        self.saveDatabase().await?;
        Ok(merged)
    }

    async fn beginBatchTransaction(&self, batchID: &str) -> Result<(), StorageError> {
        if !batchID.is_empty() {
            let mut state = self.db.lock().unwrap();
            state.transactions.entry(batchID.to_string()).or_default();
        }
        Ok(())
    }

    async fn endBatchTransaction(
        &self,
        account: &str,
        table: &str,
        batchID: &str,
        _context: &Context,
        succeeded: bool,
    ) -> Result<(), StorageError> {
        {
            let mut state = self.db.lock().unwrap();
            let Some(transaction) = state.transactions.remove(batchID) else {
                return Ok(());
            };
            if !succeeded {
                let collection_key = transaction
                    .collection_key
                    .unwrap_or_else(|| Self::table_collection_key(account, table));
                let collection = state.entity_collections.entry(collection_key).or_default();
                for inserted in transaction.inserted_entities {
                    collection.remove(&inserted);
                }
                for (key, entity) in transaction.rollback_entities {
                    collection.insert(key, entity);
                }
            }
        }
        self.saveDatabase().await
    }
}

#[async_trait]
impl ICleaner for LokiTableMetadataStore {
    async fn clean(&mut self) -> Result<(), CommonStorageError> {
        if !self.isClosed() {
            return Err(Self::common_error(
                "Cannot clean LokiTableMetadataStore, it's not closed.",
            ));
        }

        {
            let mut db = self.db.lock().unwrap();
            db.tables.clear();
            db.services.clear();
            db.entity_collections.clear();
            db.transactions.clear();
            db.initialized = false;
            db.closed = true;
        }

        if self.inMemory {
            return Ok(());
        }

        match fs::remove_file(&self.lokiDBPath).await {
            Ok(_) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(Self::common_error(error.to_string())),
        }
    }
}

#[allow(non_snake_case)]
impl LokiTableMetadataStore {
    async fn updateTableEntity(
        &self,
        context: &Context,
        table: &str,
        account: &str,
        entity: Entity,
        ifMatch: Option<&str>,
        batchID: Option<&str>,
    ) -> Result<Entity, StorageError> {
        let collection_key = Self::table_collection_key(account, table);
        let key = Self::entity_key(&entity.PartitionKey, &entity.RowKey);
        let replacement = self.normalize_entity_for_storage(context, &entity);

        {
            let mut state = self.db.lock().unwrap();
            let existing = self
                .require_entity_collection(&state, account, table, context)?
                .get(&key)
                .cloned()
                .ok_or_else(|| StorageErrorFactory::getEntityNotFound(context))?;
            if !self.matches_if_match(&existing.eTag, ifMatch) {
                return Err(StorageErrorFactory::getPreconditionFailed(context));
            }
            if let Some(transaction) =
                Self::ensure_transaction(&mut state, batchID, &collection_key)
            {
                Self::record_existing_entity(transaction, &key, &existing);
            }
            self.require_entity_collection_mut(&mut state, account, table, context)?
                .insert(key, replacement.clone());
        }
        self.save_if_needed(batchID).await?;
        Ok(replacement)
    }

    async fn mergeTableEntity(
        &self,
        context: &Context,
        table: &str,
        account: &str,
        entity: Entity,
        ifMatch: Option<&str>,
        batchID: Option<&str>,
    ) -> Result<Entity, StorageError> {
        let collection_key = Self::table_collection_key(account, table);
        let key = Self::entity_key(&entity.PartitionKey, &entity.RowKey);

        let merged = {
            let mut state = self.db.lock().unwrap();
            let existing = self
                .require_entity_collection(&state, account, table, context)?
                .get(&key)
                .cloned()
                .ok_or_else(|| StorageErrorFactory::getEntityNotFound(context))?;
            if !self.matches_if_match(&existing.eTag, ifMatch) {
                return Err(StorageErrorFactory::getPreconditionFailed(context));
            }
            if let Some(transaction) =
                Self::ensure_transaction(&mut state, batchID, &collection_key)
            {
                Self::record_existing_entity(transaction, &key, &existing);
            }

            let mut merged = existing.clone();
            merged.PartitionKey = entity.PartitionKey.clone();
            merged.RowKey = entity.RowKey.clone();
            if !entity.eTag.is_empty() {
                merged.eTag = entity.eTag.clone();
            }
            if !entity.lastModifiedTime.is_empty() {
                merged.lastModifiedTime = entity.lastModifiedTime.clone();
            }

            for (property, value) in &entity.properties {
                if property.ends_with(ODATA_TYPE) {
                    continue;
                }
                merged.properties.insert(property.clone(), value.clone());
                let metadata_key = format!("{property}{ODATA_TYPE}");
                match entity.properties.get(&metadata_key) {
                    Some(metadata) => {
                        merged.properties.insert(metadata_key, metadata.clone());
                    }
                    None => {
                        merged.properties.remove(&metadata_key);
                    }
                }
            }

            let merged = self.normalize_entity_for_storage(context, &merged);
            self.require_entity_collection_mut(&mut state, account, table, context)?
                .insert(key, merged.clone());
            merged
        };
        self.save_if_needed(batchID).await?;
        Ok(merged)
    }
}

#[cfg(test)]
mod tests {
    use crate::generated::{artifacts::models::QueryOptions, context::Context};
    use crate::utils::utils::new_table_entity_etag;

    use super::*;

    fn sample_entity(partition: &str, row: &str, timestamp: &str, name: &str) -> Entity {
        let mut properties = GeneratedObject::new();
        properties.insert("Name".to_string(), GeneratedValue::String(name.to_string()));
        Entity {
            PartitionKey: partition.to_string(),
            RowKey: row.to_string(),
            eTag: new_table_entity_etag(timestamp),
            lastModifiedTime: timestamp.to_string(),
            properties,
            ..Entity::default()
        }
    }

    fn sample_table() -> Table {
        Table {
            account: "devstoreaccount1".to_string(),
            table: "Customers".to_string(),
            ..Table::default()
        }
    }

    #[tokio::test]
    async fn create_get_and_delete_table_are_case_insensitive() {
        let store = LokiTableMetadataStore::default();
        let context = Context::default();
        store.init().await.unwrap();
        store.createTable(&context, sample_table()).await.unwrap();

        let table = store
            .getTable("devstoreaccount1", "customers", &context)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(table.table, "Customers");

        store
            .deleteTable(&context, "customers", "devstoreaccount1")
            .await
            .unwrap();
        assert!(store
            .getTable("devstoreaccount1", "Customers", &context)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn batch_rollback_restores_original_entities() {
        let store = LokiTableMetadataStore::default();
        let context = Context::default();
        store.init().await.unwrap();
        store.createTable(&context, sample_table()).await.unwrap();

        let original = sample_entity("partition", "row", "2025-03-14T10:09:08.1234Z", "before");
        store
            .insertTableEntity(
                &context,
                "Customers",
                "devstoreaccount1",
                original.clone(),
                None,
            )
            .await
            .unwrap();

        store.beginBatchTransaction("batch-1").await.unwrap();
        let updated = sample_entity("partition", "row", "2025-03-14T10:09:09.1234Z", "after");
        store
            .insertOrUpdateTableEntity(
                &context,
                "Customers",
                "devstoreaccount1",
                updated,
                Some("*"),
                Some("batch-1"),
            )
            .await
            .unwrap();
        store
            .insertTableEntity(
                &context,
                "Customers",
                "devstoreaccount1",
                sample_entity("partition", "row-2", "2025-03-14T10:09:10.1234Z", "new"),
                Some("batch-1"),
            )
            .await
            .unwrap();

        store
            .endBatchTransaction("devstoreaccount1", "Customers", "batch-1", &context, false)
            .await
            .unwrap();

        let restored = store
            .queryTableEntitiesWithPartitionAndRowKey(
                &context,
                "Customers",
                "devstoreaccount1",
                "partition",
                "row",
                None,
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            restored
                .properties
                .get("Name")
                .unwrap()
                .as_string()
                .as_deref(),
            Some("before")
        );
        assert!(store
            .queryTableEntitiesWithPartitionAndRowKey(
                &context,
                "Customers",
                "devstoreaccount1",
                "partition",
                "row-2",
                None,
            )
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn query_entities_returns_continuation_tokens() {
        let store = LokiTableMetadataStore::default();
        let context = Context::default();
        store.init().await.unwrap();
        store.createTable(&context, sample_table()).await.unwrap();

        for row in ["001", "002", "003"] {
            store
                .insertTableEntity(
                    &context,
                    "Customers",
                    "devstoreaccount1",
                    sample_entity("partition", row, "2025-03-14T10:09:08.1234Z", row),
                    None,
                )
                .await
                .unwrap();
        }

        let (entities, next_partition, next_row) = store
            .queryTableEntities(
                &context,
                "devstoreaccount1",
                "Customers",
                QueryOptions {
                    top: Some(2),
                    ..QueryOptions::default()
                },
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(entities.len(), 2);
        let expected_partition = BASE64_STANDARD.encode("partition".as_bytes());
        let expected_row = BASE64_STANDARD.encode("003".as_bytes());
        assert_eq!(next_partition.as_deref(), Some(expected_partition.as_str()));
        assert_eq!(next_row.as_deref(), Some(expected_row.as_str()));
    }

    #[tokio::test]
    async fn access_policy_and_service_properties_round_trip() {
        let store = LokiTableMetadataStore::default();
        let context = Context::default();
        TableStorageContext::new(&context).setAccount(Some("devstoreaccount1".to_string()));
        store.init().await.unwrap();
        store.createTable(&context, sample_table()).await.unwrap();

        let mut access_policy = GeneratedObject::new();
        access_policy.insert(
            "start".to_string(),
            GeneratedValue::String("2025-03-14T00:00:00Z".to_string()),
        );
        access_policy.insert(
            "expiry".to_string(),
            GeneratedValue::String("2025-03-15T00:00:00Z".to_string()),
        );
        access_policy.insert(
            "permission".to_string(),
            GeneratedValue::String("raud".to_string()),
        );
        let mut identifier = GeneratedObject::new();
        identifier.insert(
            "id".to_string(),
            GeneratedValue::String("policy".to_string()),
        );
        identifier.insert(
            "accessPolicy".to_string(),
            GeneratedValue::Object(access_policy),
        );
        let mut options = GeneratedObject::new();
        options.insert(
            "signedIdentifiers".to_string(),
            GeneratedValue::Array(vec![GeneratedValue::Object(identifier)]),
        );

        store
            .setTableAccessPolicy(&context, "Customers", options)
            .await
            .unwrap();
        let response = store
            .getTableAccessPolicy(&context, "Customers", GeneratedObject::new())
            .await
            .unwrap();
        match response.body.unwrap() {
            GeneratedBody::Value(GeneratedValue::Array(values)) => assert_eq!(values.len(), 1),
            other => panic!("unexpected response body: {other:?}"),
        }

        let mut properties = GeneratedObject::new();
        properties.insert("logging".to_string(), GeneratedValue::Bool(true));
        let saved = store
            .setServiceProperties(
                &context,
                ServicePropertiesModel {
                    accountName: "devstoreaccount1".to_string(),
                    properties,
                },
            )
            .await
            .unwrap();
        assert_eq!(
            saved.properties.get("logging").unwrap().as_bool(),
            Some(true)
        );
        assert!(store
            .getServiceProperties(&context, "devstoreaccount1")
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn persistence_round_trip_survives_reopen_and_clean() {
        let path =
            std::env::temp_dir().join(format!("azurite-table-{}.json", uuid::Uuid::new_v4()));
        let path_str = path.to_string_lossy().to_string();
        let context = Context::default();

        let store = LokiTableMetadataStore::new(path_str.clone(), false);
        store.init().await.unwrap();
        store.createTable(&context, sample_table()).await.unwrap();
        store
            .insertTableEntity(
                &context,
                "Customers",
                "devstoreaccount1",
                sample_entity("partition", "row", "2025-03-14T10:09:08.1234Z", "persisted"),
                None,
            )
            .await
            .unwrap();
        store.close().await.unwrap();

        let mut reopened = LokiTableMetadataStore::new(path_str.clone(), false);
        reopened.init().await.unwrap();
        let restored = reopened
            .queryTableEntitiesWithPartitionAndRowKey(
                &context,
                "Customers",
                "devstoreaccount1",
                "partition",
                "row",
                None,
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            restored
                .properties
                .get("Name")
                .unwrap()
                .as_string()
                .as_deref(),
            Some("persisted")
        );
        reopened.close().await.unwrap();
        ICleaner::clean(&mut reopened).await.unwrap();
        assert!(!path.exists());
    }
}
