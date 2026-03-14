use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

use azurite_common::persistence::IGCExtentProvider;
use azurite_common::utils::{convert_date_time_string_ms_to_7_digital, new_etag};

use crate::conditions::conditional_headers_adapter::ConditionalHeadersAdapter;
use crate::conditions::condition_resource_adapter::ConditionResourceAdapter;
use crate::conditions::read_conditional_headers_validator::{
    ReadConditionalHeadersValidator, validate_read_conditions,
};
use crate::conditions::write_conditional_headers_validator::{
    WriteConditionalHeadersValidator, validate_write_conditions,
    validate_sequence_number_write_conditions,
};
use crate::errors::{StorageError, StorageErrorFactory};
use crate::generated::artifacts::models::{
    AccessPolicy, AccessTier, AppendPositionAccessConditions, BlobHTTPHeaders, BlobMetadata,
    BlobPropertiesInternal, BlobTags, BlobType, ContainerProperties, LeaseAccessConditions,
    ModifiedAccessConditions, PublicAccessType, SequenceNumberAccessConditions,
    SequenceNumberActionType, SignedIdentifier, StorageServiceProperties,
    ContainerAcquireLeaseOptionalParams, ContainerReleaseLeaseOptionalParams,
    ContainerRenewLeaseOptionalParams, ContainerBreakLeaseOptionalParams,
    ContainerChangeLeaseOptionalParams, ContainerDeleteMethodOptionalParams,
    BlobDeleteMethodOptionalParams, BlobStartCopyFromURLOptionalParams,
    BlobCopyFromURLOptionalParams, BlobAcquireLeaseOptionalParams,
    BlobReleaseLeaseOptionalParams, BlobRenewLeaseOptionalParams,
    BlobChangeLeaseOptionalParams, BlobBreakLeaseOptionalParams,
    AppendBlobSealOptionalParams, Block, PageRange,
};
use crate::generated::context::Context;
use crate::handlers::page_blob_ranges_manager::PageBlobRangesManager;
use crate::lease::{
    BlobLeaseAdapter, BlobLeaseSyncer, BlobReadLeaseValidator, BlobWriteLeaseSyncer,
    BlobWriteLeaseValidator, ContainerDeleteLeaseValidator, ContainerLeaseAdapter,
    ContainerLeaseSyncer, ContainerReadLeaseValidator, LeaseFactory,
};
use crate::persistence::blob_referred_extents_async_iterator::BlobReferredExtentsAsyncIterator;
use crate::persistence::filter_blob_page::FilterBlobPage;
use crate::persistence::i_blob_metadata_store::{
    AcquireBlobLeaseResponse, AcquireContainerLeaseResponse, BlobId, BlobModel,
    BlobPrefixModel, BlockModel, BreakBlobLeaseResponse, BreakContainerLeaseResponse,
    ChangeBlobLeaseResponse, ChangeContainerLeaseResponse, ContainerModel,
    CreateSnapshotResponse, FilterBlobModel, GetBlobPropertiesRes,
    GetContainerAccessPolicyResponse, GetContainerPropertiesResponse, GetPageRangeResponse,
    IBlobMetadataStore, IContainerMetadata, IExtentChunk, PersistencyBlockModel,
    PersistencyPageRange, ReleaseBlobLeaseResponse, ReleaseContainerLeaseResponse,
    RenewBlobLeaseResponse, RenewContainerLeaseResponse, ServicePropertiesModel,
    SetContainerAccessPolicyOptions, ZERO_EXTENT_ID, BlobLeaseResponse, ContainerLeaseResponse,
    BlockListEntry, BlobTypeResult,
};
use crate::persistence::page_with_delimiter::PageWithDelimiter;
use crate::persistence::query_interpreter::query_interpreter::{
    execute_query, generate_query_blob_with_tags_where_function,
};
use crate::utils::{
    get_blob_tags_count, get_tags_from_string, to_blob_tags,
    DEFAULT_LIST_BLOBS_MAX_RESULTS, DEFAULT_LIST_CONTAINERS_MAX_RESULTS,
    MAX_APPEND_BLOB_BLOCK_COUNT,
};

/// In-memory metadata storage using HashMap-based collections to replace LokiJS.
///
/// This is a metadata source implementation for blob based on in-memory storage.
///
/// Notice that, following design is for emulator purpose only, and doesn't design for best performance.
/// We may want to optimize the persistency layer performance in the future. Such as by distributing metadata
/// into different collections, or make binary payload write as an append-only pattern.
///
/// Collections structure:
///
/// -- SERVICE_PROPERTIES_COLLECTION // Collection contains service properties
///                                  // Default collection name is $SERVICES_COLLECTION$
///                                  // Each document maps to 1 account blob service
///                                  // Unique document properties: accountName
/// -- CONTAINERS_COLLECTION  // Collection contains all containers
///                           // Default collection name is $CONTAINERS_COLLECTION$
///                           // Each document maps to 1 container
///                           // Unique document properties: accountName, (container)name
/// -- BLOBS_COLLECTION       // Collection contains all blobs
///                           // Default collection name is $BLOBS_COLLECTION$
///                           // Each document maps to a blob
///                           // Unique document properties: accountName, containerName, (blob)name, snapshot
/// -- BLOCKS_COLLECTION      // Block blob blocks collection includes all UNCOMMITTED blocks
///                           // Unique document properties: accountName, containerName, blobName, name, isCommitted
pub struct LokiBlobMetadataStore {
    loki_db_path: PathBuf,
    in_memory: bool,
    initialized: bool,
    closed: bool,

    // Collections (using HashMap/BTreeMap for in-memory storage)
    // SERVICE_PROPERTIES_COLLECTION: Key = accountName
    services_collection: Arc<RwLock<HashMap<String, ServicePropertiesModel>>>,

    // CONTAINERS_COLLECTION: Key = (accountName, containerName)
    containers_collection: Arc<RwLock<BTreeMap<(String, String), ContainerModel>>>,

    // BLOBS_COLLECTION: Key = (accountName, containerName, blobName, snapshot)
    blobs_collection: Arc<RwLock<BTreeMap<(String, String, String, String), BlobModel>>>,

    // BLOCKS_COLLECTION: Key = (accountName, containerName, blobName, blockName)
    // Only stores uncommitted blocks (isCommitted = false)
    blocks_collection: Arc<RwLock<BTreeMap<(String, String, String, String), BlockModel>>>,

    page_blob_ranges_manager: PageBlobRangesManager,
}

impl LokiBlobMetadataStore {
    pub fn new(loki_db_path: PathBuf, in_memory: bool) -> Self {
        Self {
            loki_db_path,
            in_memory,
            initialized: false,
            closed: true,
            services_collection: Arc::new(RwLock::new(HashMap::new())),
            containers_collection: Arc::new(RwLock::new(BTreeMap::new())),
            blobs_collection: Arc::new(RwLock::new(BTreeMap::new())),
            blocks_collection: Arc::new(RwLock::new(BTreeMap::new())),
            page_blob_ranges_manager: PageBlobRangesManager::new(),
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    pub async fn init(&mut self) -> Result<(), StorageError> {
        // TODO: If not in_memory, load from file at self.loki_db_path
        // For now, simplified in-memory initialization
        if !self.in_memory {
            // In TS: loads DB from file if exists via fs.stat() + db.loadDatabase()
            // We'll implement file persistence later if needed
            // For now: no-op (always starts empty)
        }

        self.initialized = true;
        self.closed = false;
        Ok(())
    }

    pub async fn close(&mut self) -> Result<(), StorageError> {
        // TODO: If not in_memory, save to file at self.loki_db_path
        // In TS: db.close() with callback
        // For now: no-op
        self.closed = true;
        Ok(())
    }

    pub async fn clean(&self) -> Result<(), StorageError> {
        if !self.is_closed() {
            return Err(StorageError::from("Cannot clean LokiBlobMetadataStore, it's not closed."));
        }
        // TODO: If not in_memory, delete file at self.loki_db_path
        // In TS: rimrafAsync(this.lokiDBPath)
        // For now: no-op
        Ok(())
    }

    /// Private helper: Escape regex special characters.
    fn escape_regex(s: &str) -> String {
        s.replace("\\", "\\\\")
            .replace(".", "\\.")
            .replace("*", "\\*")
            .replace("+", "\\+")
            .replace("?", "\\?")
            .replace("^", "\\^")
            .replace("$", "\\$")
            .replace("[", "\\[")
            .replace("]", "\\]")
            .replace("(", "\\(")
            .replace(")", "\\)")
            .replace("{", "\\{")
            .replace("}", "\\}")
            .replace("|", "\\|")
    }

    /// Private helper: Restore Uint8Array from Loki's persisted object format.
    /// In TS, Loki JSON persistence loses typed arrays, so reads must restore them.
    /// In Rust, we'll assume binary fields are already Vec<u8> or similar, but keep the pattern.
    fn restore_uint8_array(value: Option<&Vec<u8>>) -> Option<Vec<u8>> {
        value.map(|v| v.clone())
    }

    /// Private helper: Get container with lease updated.
    /// TS: lines 3176-3227
    async fn get_container_with_lease_updated(
        &self,
        account: &str,
        container: &str,
        context: &Context,
        throw_if_not_found: bool,
    ) -> Result<Option<ContainerModel>, StorageError> {
        let containers = self.containers_collection.read().unwrap();
        let doc = containers.get(&(account.to_string(), container.to_string())).cloned();
        drop(containers);

        match doc {
            Some(mut doc_val) => {
                // Sync lease state
                let adapter = ContainerLeaseAdapter::new(&doc_val);
                let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
                let syncer = ContainerLeaseSyncer::new(&mut doc_val);
                lease_state.lease().sync_with_syncer(&syncer)?;

                Ok(Some(doc_val))
            }
            None => {
                if throw_if_not_found {
                    Err(StorageErrorFactory::get_container_not_found(&context.context_id))
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Private helper: Get container without lease updates (optional error throwing).
    /// TS: lines 3241-3285
    async fn get_container(
        &self,
        account: &str,
        container: &str,
        _context: &Context,
        throw_if_not_found: bool,
    ) -> Result<Option<ContainerModel>, StorageError> {
        let containers = self.containers_collection.read().unwrap();
        let doc = containers.get(&(account.to_string(), container.to_string())).cloned();
        drop(containers);

        if doc.is_none() && throw_if_not_found {
            Err(StorageErrorFactory::get_container_not_found(&_context.context_id))
        } else {
            Ok(doc)
        }
    }

    /// Private helper: Get blob with lease updated + restore contentMD5.
    /// TS: lines 3302-3403 (supports forceCommitted flag)
    async fn get_blob_with_lease_updated(
        &self,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: &str,
        context: &Context,
        force_committed: bool,
    ) -> Result<Option<BlobModel>, StorageError> {
        let blobs = self.blobs_collection.read().unwrap();
        let key = (account.to_string(), container.to_string(), blob.to_string(), snapshot.to_string());
        let doc = blobs.get(&key).cloned();
        drop(blobs);

        match doc {
            Some(mut doc_val) => {
                // Restore Uint8Array for contentMD5
                if let Some(ref mut props) = doc_val.properties.content_m_d5 {
                    // In Rust we already have Vec<u8>, so this is a no-op
                    // But keeping the pattern from TS lines 3379-3383
                }

                // For snapshots, normalize lease state/status to Available/Unlocked
                // TS lines 3385-3395
                if !snapshot.is_empty() {
                    doc_val.properties.lease_state = Some("Available".to_string());
                    doc_val.properties.lease_status = Some("Unlocked".to_string());
                    // TODO: According to TS TODO, snapshot lease state/status should be undefined,
                    // but current behavior sets them to Available/Unlocked
                }

                // If not a snapshot, sync lease state
                if snapshot.is_empty() {
                    let adapter = BlobLeaseAdapter::new(&doc_val);
                    let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
                    let syncer = BlobLeaseSyncer::new(&mut doc_val);
                    lease_state.lease().sync_with_syncer(&syncer)?;
                }

                // If forceCommitted and blob is uncommitted, return None
                if force_committed && doc_val.is_committed == Some(false) {
                    return Ok(None);
                }

                Ok(Some(doc_val))
            }
            None => Ok(None),
        }
    }

    /// Private helper: Get blob without lease updates.
    async fn get_blob(
        &self,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: &str,
    ) -> Result<Option<BlobModel>, StorageError> {
        let blobs = self.blobs_collection.read().unwrap();
        let key = (account.to_string(), container.to_string(), blob.to_string(), snapshot.to_string());
        let doc = blobs.get(&key).cloned();
        drop(blobs);
        Ok(doc)
    }

    /// Private helper: Parse tier string.
    /// TS: lines 3506-3521
    fn parse_tier(tier: Option<&str>) -> Option<AccessTier> {
        tier.and_then(|t| match t {
            "Hot" => Some(AccessTier::Hot),
            "Cool" => Some(AccessTier::Cool),
            "Archive" => Some(AccessTier::Archive),
            "Cold" => Some(AccessTier::Cold),
            _ => None,
        })
    }
}

#[async_trait]
impl IBlobMetadataStore for LokiBlobMetadataStore {
    // ─── Service Properties ─────────────────────────────────────────────────

    async fn set_service_properties(
        &self,
        _context: &Context,
        service_properties: ServicePropertiesModel,
    ) -> Result<ServicePropertiesModel, StorageError> {
        let mut services = self.services_collection.write().unwrap();
        
        if let Some(doc) = services.get_mut(&service_properties.account_name) {
            // Update existing (undefined properties are ignored)
            if let Some(cors) = &service_properties.properties.cors {
                doc.properties.cors = Some(cors.clone());
            }
            if let Some(hour_metrics) = &service_properties.properties.hour_metrics {
                doc.properties.hour_metrics = Some(hour_metrics.clone());
            }
            if let Some(logging) = &service_properties.properties.logging {
                doc.properties.logging = Some(logging.clone());
            }
            if let Some(minute_metrics) = &service_properties.properties.minute_metrics {
                doc.properties.minute_metrics = Some(minute_metrics.clone());
            }
            if let Some(default_service_version) = &service_properties.properties.default_service_version {
                doc.properties.default_service_version = Some(default_service_version.clone());
            }
            if let Some(delete_retention_policy) = &service_properties.properties.delete_retention_policy {
                doc.properties.delete_retention_policy = Some(delete_retention_policy.clone());
            }
            if let Some(static_website) = &service_properties.properties.static_website {
                doc.properties.static_website = Some(static_website.clone());
            }
            Ok(doc.clone())
        } else {
            // Insert new
            services.insert(service_properties.account_name.clone(), service_properties.clone());
            Ok(service_properties)
        }
    }

    async fn get_service_properties(
        &self,
        _context: &Context,
        account: &str,
    ) -> Result<Option<ServicePropertiesModel>, StorageError> {
        let services = self.services_collection.read().unwrap();
        Ok(services.get(account).cloned())
    }

    // ─── Container Operations ───────────────────────────────────────────────

    async fn list_containers(
        &self,
        context: &Context,
        account: &str,
        prefix: Option<&str>,
        max_results: Option<i64>,
        marker: Option<&str>,
    ) -> Result<(Vec<ContainerModel>, Option<String>), StorageError> {
        let prefix = prefix.unwrap_or("");
        let max_results = max_results.unwrap_or(DEFAULT_LIST_CONTAINERS_MAX_RESULTS as i64) as usize;
        let marker = marker.unwrap_or("");

        let containers = self.containers_collection.read().unwrap();

        // Filter containers matching account and prefix, greater than marker
        let mut matching: Vec<ContainerModel> = containers
            .iter()
            .filter(|((acc, name), _)| {
                acc == account && name.starts_with(prefix) && name.as_str() > marker
            })
            .map(|(_, doc)| doc.clone())
            .collect();

        // Sort by name
        matching.sort_by(|a, b| a.name.cmp(&b.name));

        drop(containers);

        // Take max_results + 1 to determine if there's a next marker
        let mut result_docs: Vec<ContainerModel> = matching.into_iter().take(max_results + 1).collect();
        let next_marker = if result_docs.len() > max_results {
            let last = result_docs.pop();
            result_docs.last().and_then(|doc| doc.name.clone())
        } else {
            None
        };

        // Sync lease state for each container
        let synced_docs: Vec<ContainerModel> = result_docs
            .into_iter()
            .map(|mut doc| {
                let adapter = ContainerLeaseAdapter::new(&doc);
                if let Ok(lease_state) = LeaseFactory::create_lease_state(&adapter, context) {
                    let syncer = ContainerLeaseSyncer::new(&mut doc);
                    let _ = lease_state.lease().sync_with_syncer(&syncer);
                }
                doc
            })
            .collect();

        Ok((synced_docs, next_marker))
    }

    async fn create_container(
        &self,
        context: &Context,
        container: ContainerModel,
    ) -> Result<ContainerModel, StorageError> {
        let mut containers = self.containers_collection.write().unwrap();
        
        let key = (
            container.account_name.clone(),
            container.name.clone().unwrap_or_default(),
        );

        if containers.contains_key(&key) {
            return Err(StorageErrorFactory::get_container_already_exists(&context.context_id));
        }

        containers.insert(key, container.clone());
        Ok(container)
    }

    async fn get_container_properties(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        lease_access_conditions: Option<&LeaseAccessConditions>,
    ) -> Result<GetContainerPropertiesResponse, StorageError> {
        let doc = self.get_container_with_lease_updated(account, container, context, true).await?
            .ok_or_else(|| StorageErrorFactory::get_container_not_found(&context.context_id))?;

        let validator = ContainerReadLeaseValidator::new(lease_access_conditions);
        let adapter = ContainerLeaseAdapter::new(&doc);
        validator.validate(&adapter, context)?;

        let mut res = GetContainerPropertiesResponse::default();
        res.insert("name".to_string(), container.into());
        res.insert("properties".to_string(), serde_json::to_value(&doc.properties).unwrap().into());
        if let Some(metadata) = &doc.metadata {
            res.insert("metadata".to_string(), serde_json::to_value(metadata).unwrap().into());
        }

        Ok(res)
    }

    async fn delete_container(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        options: Option<&ContainerDeleteMethodOptionalParams>,
    ) -> Result<(), StorageError> {
        let options = options.cloned().unwrap_or_default();

        let doc = self.get_container_with_lease_updated(account, container, context, false).await?;

        validate_write_conditions(context, options.modified_access_conditions.as_ref(), doc.as_ref())?;

        let doc = doc.ok_or_else(|| StorageErrorFactory::get_container_not_found(&context.context_id))?;

        let validator = ContainerDeleteLeaseValidator::new(options.lease_access_conditions.as_ref());
        let adapter = ContainerLeaseAdapter::new(&doc);
        validator.validate(&adapter, context)?;

        // Remove container
        let mut containers = self.containers_collection.write().unwrap();
        containers.remove(&(account.to_string(), container.to_string()));
        drop(containers);

        // Remove all blobs in this container
        let mut blobs = self.blobs_collection.write().unwrap();
        blobs.retain(|(acc, cont, _, _), _| !(acc == account && cont == container));
        drop(blobs);

        // Remove all blocks in this container
        let mut blocks = self.blocks_collection.write().unwrap();
        blocks.retain(|(acc, cont, _, _), _| !(acc == account && cont == container));
        drop(blocks);

        Ok(())
    }

    async fn set_container_metadata(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        last_modified: DateTime<Utc>,
        etag: String,
        metadata: Option<IContainerMetadata>,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        modified_access_conditions: Option<&ModifiedAccessConditions>,
    ) -> Result<(), StorageError> {
        let doc = self.get_container_with_lease_updated(account, container, context, false).await?;

        validate_write_conditions(context, modified_access_conditions, doc.as_ref())?;

        let doc = doc.ok_or_else(|| StorageErrorFactory::get_container_not_found(&context.context_id))?;

        let validator = ContainerReadLeaseValidator::new(lease_access_conditions);
        let adapter = ContainerLeaseAdapter::new(&doc);
        validator.validate(&adapter, context)?;

        // Update container
        let mut containers = self.containers_collection.write().unwrap();
        if let Some(mut container_doc) = containers.get_mut(&(account.to_string(), container.to_string())) {
            container_doc.properties.last_modified = Some(last_modified);
            container_doc.properties.etag = Some(etag);
            container_doc.metadata = metadata;
        }

        Ok(())
    }

    async fn get_container_acl(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        lease_access_conditions: Option<&LeaseAccessConditions>,
    ) -> Result<GetContainerAccessPolicyResponse, StorageError> {
        let doc = self.get_container_with_lease_updated(account, container, context, true).await?
            .ok_or_else(|| StorageErrorFactory::get_container_not_found(&context.context_id))?;

        let validator = ContainerReadLeaseValidator::new(lease_access_conditions);
        let adapter = ContainerLeaseAdapter::new(&doc);
        validator.validate(&adapter, context)?;

        Ok(GetContainerAccessPolicyResponse {
            properties: doc.properties,
            container_acl: doc.container_acl,
        })
    }

    async fn set_container_acl(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        set_acl_model: SetContainerAccessPolicyOptions,
    ) -> Result<(), StorageError> {
        let doc = self.get_container_with_lease_updated(account, container, context, false).await?;

        validate_write_conditions(context, set_acl_model.modified_access_conditions.as_ref(), doc.as_ref())?;

        let doc = doc.ok_or_else(|| StorageErrorFactory::get_container_not_found(&context.context_id))?;

        let validator = ContainerReadLeaseValidator::new(set_acl_model.lease_access_conditions.as_ref());
        let adapter = ContainerLeaseAdapter::new(&doc);
        validator.validate(&adapter, context)?;

        // Update container
        let mut containers = self.containers_collection.write().unwrap();
        if let Some(mut container_doc) = containers.get_mut(&(account.to_string(), container.to_string())) {
            if let Some(public_access) = set_acl_model.public_access {
                container_doc.properties.public_access = Some(public_access);
            }
            container_doc.container_acl = set_acl_model.container_acl;
            if let Some(last_modified) = set_acl_model.last_modified {
                container_doc.properties.last_modified = Some(last_modified);
            }
            if let Some(etag) = set_acl_model.etag {
                container_doc.properties.etag = Some(etag);
            }
        }

        Ok(())
    }

    async fn acquire_container_lease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        options: &ContainerAcquireLeaseOptionalParams,
    ) -> Result<AcquireContainerLeaseResponse, StorageError> {
        let doc = self.get_container(account, container, context, false).await?;

        validate_write_conditions(context, options.modified_access_conditions.as_ref(), doc.as_ref())?;

        let mut doc = doc.ok_or_else(|| StorageErrorFactory::get_container_not_found(&context.context_id))?;

        let adapter = ContainerLeaseAdapter::new(&doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let acquired_state = lease_state.acquire(options.duration.unwrap_or(-1), options.proposed_lease_id.clone())?;
        let syncer = ContainerLeaseSyncer::new(&mut doc);
        acquired_state.lease().sync_with_syncer(&syncer)?;

        // Update container
        let mut containers = self.containers_collection.write().unwrap();
        containers.insert((account.to_string(), container.to_string()), doc.clone());
        drop(containers);

        Ok(ContainerLeaseResponse {
            properties: doc.properties,
            lease_id: doc.lease_id,
            lease_time: doc.lease_duration_seconds,
        })
    }

    async fn release_container_lease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        lease_id: &str,
        _options: Option<&ContainerReleaseLeaseOptionalParams>,
    ) -> Result<ReleaseContainerLeaseResponse, StorageError> {
        let mut doc = self.get_container(account, container, context, false).await?
            .ok_or_else(|| StorageErrorFactory::get_container_not_found(&context.context_id))?;

        let adapter = ContainerLeaseAdapter::new(&doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let released_state = lease_state.release(lease_id.to_string())?;
        let syncer = ContainerLeaseSyncer::new(&mut doc);
        released_state.lease().sync_with_syncer(&syncer)?;

        // Update container
        let mut containers = self.containers_collection.write().unwrap();
        containers.insert((account.to_string(), container.to_string()), doc.clone());
        drop(containers);

        Ok(doc.properties)
    }

    async fn renew_container_lease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        lease_id: &str,
        _options: Option<&ContainerRenewLeaseOptionalParams>,
    ) -> Result<RenewContainerLeaseResponse, StorageError> {
        let mut doc = self.get_container(account, container, context, false).await?
            .ok_or_else(|| StorageErrorFactory::get_container_not_found(&context.context_id))?;

        let adapter = ContainerLeaseAdapter::new(&doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let renewed_state = lease_state.renew(lease_id.to_string())?;
        let syncer = ContainerLeaseSyncer::new(&mut doc);
        renewed_state.lease().sync_with_syncer(&syncer)?;

        // Update container
        let mut containers = self.containers_collection.write().unwrap();
        containers.insert((account.to_string(), container.to_string()), doc.clone());
        drop(containers);

        Ok(ContainerLeaseResponse {
            properties: doc.properties,
            lease_id: doc.lease_id,
            lease_time: doc.lease_duration_seconds,
        })
    }

    async fn break_container_lease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        break_period: Option<i64>,
        _options: Option<&ContainerBreakLeaseOptionalParams>,
    ) -> Result<BreakContainerLeaseResponse, StorageError> {
        let mut doc = self.get_container(account, container, context, false).await?
            .ok_or_else(|| StorageErrorFactory::get_container_not_found(&context.context_id))?;

        let adapter = ContainerLeaseAdapter::new(&doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let broken_state = lease_state.break_lease(break_period)?;
        let syncer = ContainerLeaseSyncer::new(&mut doc);
        broken_state.lease().sync_with_syncer(&syncer)?;

        // Update container
        let mut containers = self.containers_collection.write().unwrap();
        containers.insert((account.to_string(), container.to_string()), doc.clone());
        drop(containers);

        Ok(ContainerLeaseResponse {
            properties: doc.properties,
            lease_id: doc.lease_id,
            lease_time: doc.lease_duration_seconds,
        })
    }

    async fn change_container_lease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        lease_id: &str,
        proposed_lease_id: &str,
        _options: Option<&ContainerChangeLeaseOptionalParams>,
    ) -> Result<ChangeContainerLeaseResponse, StorageError> {
        let mut doc = self.get_container(account, container, context, false).await?
            .ok_or_else(|| StorageErrorFactory::get_container_not_found(&context.context_id))?;

        let adapter = ContainerLeaseAdapter::new(&doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let changed_state = lease_state.change(lease_id.to_string(), proposed_lease_id.to_string())?;
        let syncer = ContainerLeaseSyncer::new(&mut doc);
        changed_state.lease().sync_with_syncer(&syncer)?;

        // Update container
        let mut containers = self.containers_collection.write().unwrap();
        containers.insert((account.to_string(), container.to_string()), doc.clone());
        drop(containers);

        Ok(ContainerLeaseResponse {
            properties: doc.properties,
            lease_id: doc.lease_id,
            lease_time: doc.lease_duration_seconds,
        })
    }

    async fn check_container_exist(
        &self,
        context: &Context,
        account: &str,
        container: &str,
    ) -> Result<(), StorageError> {
        let containers = self.containers_collection.read().unwrap();
        if containers.contains_key(&(account.to_string(), container.to_string())) {
            Ok(())
        } else {
            Err(StorageErrorFactory::get_container_not_found(&context.context_id))
        }
    }

    // ─── Blob Operations ────────────────────────────────────────────────────
    
    async fn filter_blobs(
        &self,
        context: &Context,
        account: &str,
        container: Option<&str>,
        where_clause: Option<&str>,
        max_results: Option<i64>,
        marker: Option<&str>,
    ) -> Result<(Vec<FilterBlobModel>, Option<String>), StorageError> {
        let max_results = max_results.unwrap_or(DEFAULT_LIST_BLOBS_MAX_RESULTS as i64) as usize;
        let marker = marker.unwrap_or("");

        // Generate tag filter function
        let filter_fn = if let Some(where_clause) = where_clause {
            generate_query_blob_with_tags_where_function(where_clause, container)?
        } else {
            // No filter, accept all
            Box::new(|_: &HashMap<String, String>| -> bool { true })
        };

        let blobs = self.blobs_collection.read().unwrap();

        // Filter blobs matching account, container (if specified), and marker
        let mut matching: Vec<FilterBlobModel> = blobs
            .iter()
            .filter(|((acc, cont, name, snap), blob)| {
                // Match account
                if acc != account {
                    return false;
                }
                // Match container if specified
                if let Some(c) = container {
                    if cont != c {
                        return false;
                    }
                }
                // Match marker
                if name.as_str() <= marker {
                    return false;
                }
                // Exclude snapshots
                if !snap.is_empty() {
                    return false;
                }
                // Apply tag filter
                let tags_map: HashMap<String, String> = blob
                    .blob_tags
                    .as_ref()
                    .map(|tags| {
                        tags.blob_tag_set
                            .iter()
                            .filter_map(|tag| {
                                tag.key.as_ref().and_then(|k| {
                                    tag.value.as_ref().map(|v| (k.clone(), v.clone()))
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                
                filter_fn(&tags_map)
            })
            .map(|((acc, cont, name, _), blob)| FilterBlobModel {
                name: name.clone(),
                container_name: cont.clone(),
                tags: to_blob_tags(&blob.blob_tags),
            })
            .collect();

        // Sort by name
        matching.sort_by(|a, b| a.name.cmp(&b.name));

        drop(blobs);

        // Apply pagination
        let mut result_docs: Vec<FilterBlobModel> = matching.into_iter().take(max_results + 1).collect();
        let next_marker = if result_docs.len() > max_results {
            result_docs.pop();
            result_docs.last().map(|doc| doc.name.clone())
        } else {
            None
        };

        Ok((result_docs, next_marker))
    }

    async fn list_blobs(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        delimiter: Option<&str>,
        blob: Option<&str>,
        prefix: Option<&str>,
        max_results: Option<i64>,
        marker: Option<&str>,
        include_snapshots: Option<bool>,
        include_uncommitted_blobs: Option<bool>,
    ) -> Result<(Vec<BlobModel>, Vec<BlobPrefixModel>, Option<String>), StorageError> {
        let prefix = prefix.unwrap_or("");
        let max_results = max_results.unwrap_or(DEFAULT_LIST_BLOBS_MAX_RESULTS as i64) as usize;
        let marker = marker.unwrap_or("");
        let include_snapshots = include_snapshots.unwrap_or(false);
        let include_uncommitted_blobs = include_uncommitted_blobs.unwrap_or(false);

        // If delimiter is specified, use PageWithDelimiter pattern
        if let Some(delim) = delimiter {
            let mut page = PageWithDelimiter::<BlobModel, BlobPrefixModel>::new(max_results, delim);

            let blobs = self.blobs_collection.read().unwrap();
            let mut matching: Vec<BlobModel> = blobs
                .iter()
                .filter(|((acc, cont, name, snap), blob)| {
                    acc == account
                        && cont == container
                        && name.starts_with(prefix)
                        && name.as_str() > marker
                        && (include_snapshots || snap.is_empty())
                        && (include_uncommitted_blobs || blob.is_committed != Some(false))
                })
                .map(|(_, blob)| blob.clone())
                .collect();

            // Sort by name
            matching.sort_by(|a, b| a.name.cmp(&b.name));

            for blob in matching {
                let name = blob.name.clone().unwrap_or_default();
                if let Some(stripped) = name.strip_prefix(prefix) {
                    if stripped.contains(delim) {
                        // It's a prefix
                        let prefix_end = stripped.find(delim).unwrap() + delim.len();
                        let prefix_name = format!("{}{}", prefix, &stripped[..prefix_end]);
                        let prefix_model = BlobPrefixModel {
                            name: prefix_name,
                            persistency: None,
                        };
                        if !page.try_add_prefix(prefix_model) {
                            break;
                        }
                    } else {
                        // It's a blob
                        if !page.try_add_item(blob) {
                            break;
                        }
                    }
                }
            }

            let (items, prefixes, next_marker) = page.get_results();
            Ok((items, prefixes, next_marker))
        } else {
            // No delimiter, simple list
            let blobs = self.blobs_collection.read().unwrap();
            let mut matching: Vec<BlobModel> = blobs
                .iter()
                .filter(|((acc, cont, name, snap), blob)| {
                    acc == account
                        && cont == container
                        && name.starts_with(prefix)
                        && name.as_str() > marker
                        && (include_snapshots || snap.is_empty())
                        && (include_uncommitted_blobs || blob.is_committed != Some(false))
                })
                .map(|(_, blob)| blob.clone())
                .collect();

            // Sort by name
            matching.sort_by(|a, b| a.name.cmp(&b.name));

            drop(blobs);

            // Apply pagination
            let mut result_docs: Vec<BlobModel> = matching.into_iter().take(max_results + 1).collect();
            let next_marker = if result_docs.len() > max_results {
                result_docs.pop();
                result_docs.last().and_then(|doc| doc.name.clone())
            } else {
                None
            };

            // Sync lease state for each blob
            let synced_docs: Vec<BlobModel> = result_docs
                .into_iter()
                .map(|mut doc| {
                    // Only sync lease for non-snapshots
                    if doc.snapshot.as_ref().map_or(true, |s| s.is_empty()) {
                        let adapter = BlobLeaseAdapter::new(&doc);
                        if let Ok(lease_state) = LeaseFactory::create_lease_state(&adapter, context) {
                            let syncer = BlobLeaseSyncer::new(&mut doc);
                            let _ = lease_state.lease().sync_with_syncer(&syncer);
                        }
                    }
                    doc
                })
                .collect();

            Ok((synced_docs, vec![], next_marker))
        }
    }

    async fn list_all_blobs(
        &self,
        max_results: Option<i64>,
        marker: Option<&str>,
        include_snapshots: Option<bool>,
        include_uncommitted_blobs: Option<bool>,
    ) -> Result<(Vec<BlobModel>, Option<String>), StorageError> {
        let max_results = max_results.unwrap_or(DEFAULT_LIST_BLOBS_MAX_RESULTS as i64) as usize;
        let marker = marker.unwrap_or("");
        let include_snapshots = include_snapshots.unwrap_or(false);
        let include_uncommitted_blobs = include_uncommitted_blobs.unwrap_or(false);

        let blobs = self.blobs_collection.read().unwrap();

        let mut matching: Vec<BlobModel> = blobs
            .iter()
            .filter(|((acc, cont, name, snap), blob)| {
                // Compare full path (account/container/name) against marker
                let full_path = format!("{}/{}/{}", acc, cont, name);
                full_path > marker
                    && (include_snapshots || snap.is_empty())
                    && (include_uncommitted_blobs || blob.is_committed != Some(false))
            })
            .map(|(_, blob)| blob.clone())
            .collect();

        // Sort by full path
        matching.sort_by(|a, b| {
            let path_a = format!("{}/{}/{}", a.account_name, a.container_name, a.name.as_ref().unwrap_or(&String::new()));
            let path_b = format!("{}/{}/{}", b.account_name, b.container_name, b.name.as_ref().unwrap_or(&String::new()));
            path_a.cmp(&path_b)
        });

        drop(blobs);

        // Apply pagination
        let mut result_docs: Vec<BlobModel> = matching.into_iter().take(max_results + 1).collect();
        let next_marker = if result_docs.len() > max_results {
            let last = result_docs.pop().unwrap();
            Some(format!("{}/{}/{}", last.account_name, last.container_name, last.name.unwrap_or_default()))
        } else {
            None
        };

        Ok((result_docs, next_marker))
    }

    async fn create_blob(
        &self,
        context: &Context,
        blob: BlobModel,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        modified_access_conditions: Option<&ModifiedAccessConditions>,
    ) -> Result<(), StorageError> {
        // Check if container exists
        self.check_container_exist(context, &blob.account_name, &blob.container_name).await?;

        let snapshot = blob.snapshot.clone().unwrap_or_default();
        let blob_name = blob.name.clone().unwrap_or_default();

        // Check if blob exists
        let existing = self.get_blob_with_lease_updated(
            &blob.account_name,
            &blob.container_name,
            &blob_name,
            &snapshot,
            context,
            true, // forceCommitted
        ).await?;

        // Validate write conditions
        validate_write_conditions(context, modified_access_conditions, existing.as_ref())?;

        // Validate lease conditions
        if let Some(existing_blob) = &existing {
            let validator = BlobWriteLeaseValidator::new(lease_access_conditions);
            let adapter = BlobLeaseAdapter::new(existing_blob);
            validator.validate(&adapter, context)?;
        }

        // Insert blob
        let mut blobs = self.blobs_collection.write().unwrap();
        let key = (
            blob.account_name.clone(),
            blob.container_name.clone(),
            blob_name.clone(),
            snapshot.clone(),
        );
        blobs.insert(key, blob.clone());

        Ok(())
    }

    async fn create_snapshot(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        metadata: Option<&BlobMetadata>,
        modified_access_conditions: Option<&ModifiedAccessConditions>,
    ) -> Result<CreateSnapshotResponse, StorageError> {
        let base_blob = self.get_blob_with_lease_updated(account, container, blob, "", context, false).await?
            .ok_or_else(|| StorageErrorFactory::get_blob_not_found(&context.context_id))?;

        validate_write_conditions(context, modified_access_conditions, Some(&base_blob))?;

        let validator = BlobReadLeaseValidator::new(lease_access_conditions);
        let adapter = BlobLeaseAdapter::new(&base_blob);
        validator.validate(&adapter, context)?;

        // Generate snapshot timestamp
        let snapshot_time = convert_date_time_string_ms_to_7_digital(
            &context.start_time.unwrap().to_rfc3339()
        );

        // Create snapshot blob
        let mut snapshot_blob = base_blob.clone();
        snapshot_blob.snapshot = Some(snapshot_time.clone());
        if let Some(meta) = metadata {
            snapshot_blob.metadata = Some(meta.clone());
        }

        // Insert snapshot
        let mut blobs = self.blobs_collection.write().unwrap();
        let key = (
            account.to_string(),
            container.to_string(),
            blob.to_string(),
            snapshot_time.clone(),
        );
        blobs.insert(key, snapshot_blob.clone());

        Ok(CreateSnapshotResponse {
            properties: snapshot_blob.properties,
            snapshot: snapshot_time,
        })
    }

    async fn download_blob(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        modified_access_conditions: Option<&ModifiedAccessConditions>,
    ) -> Result<BlobModel, StorageError> {
        let snapshot = snapshot.unwrap_or("");

        let blob_doc = self.get_blob_with_lease_updated(account, container, blob, snapshot, context, true).await?
            .ok_or_else(|| StorageErrorFactory::get_blob_not_found(&context.context_id))?;

        validate_read_conditions(context, modified_access_conditions, Some(&blob_doc))?;

        let validator = BlobReadLeaseValidator::new(lease_access_conditions);
        let adapter = BlobLeaseAdapter::new(&blob_doc);
        validator.validate(&adapter, context)?;

        Ok(blob_doc)
    }

    async fn get_blob(
        &self,
        _context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
    ) -> Result<Option<BlobModel>, StorageError> {
        let snapshot = snapshot.unwrap_or("");
        self.get_blob(account, container, blob, snapshot).await
    }

    async fn get_blob_properties(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        modified_access_conditions: Option<&ModifiedAccessConditions>,
    ) -> Result<GetBlobPropertiesRes, StorageError> {
        let snapshot = snapshot.unwrap_or("");

        let blob_doc = self.get_blob_with_lease_updated(account, container, blob, snapshot, context, true).await?
            .ok_or_else(|| StorageErrorFactory::get_blob_not_found(&context.context_id))?;

        validate_read_conditions(context, modified_access_conditions, Some(&blob_doc))?;

        let validator = BlobReadLeaseValidator::new(lease_access_conditions);
        let adapter = BlobLeaseAdapter::new(&blob_doc);
        validator.validate(&adapter, context)?;

        // Get committed block count for append blobs
        let blob_committed_block_count = if blob_doc.properties.blob_type == Some(BlobType::AppendBlob) {
            blob_doc.committed_blocks_in_order.as_ref().map(|blocks| blocks.len() as i64)
        } else {
            None
        };

        Ok(GetBlobPropertiesRes {
            properties: blob_doc.properties,
            metadata: blob_doc.metadata,
            blob_committed_block_count,
        })
    }

    async fn delete_blob(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        options: &BlobDeleteMethodOptionalParams,
    ) -> Result<(), StorageError> {
        let snapshot = options.snapshot.as_deref().unwrap_or("");

        let blob_doc = self.get_blob_with_lease_updated(account, container, blob, snapshot, context, false).await?
            .ok_or_else(|| StorageErrorFactory::get_blob_not_found(&context.context_id))?;

        validate_write_conditions(context, options.modified_access_conditions.as_ref(), Some(&blob_doc))?;

        let validator = BlobWriteLeaseValidator::new(options.lease_access_conditions.as_ref());
        let adapter = BlobLeaseAdapter::new(&blob_doc);
        validator.validate(&adapter, context)?;

        // Delete blob
        let mut blobs = self.blobs_collection.write().unwrap();
        blobs.remove(&(account.to_string(), container.to_string(), blob.to_string(), snapshot.to_string()));

        Ok(())
    }

    async fn set_blob_http_headers(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        blob_http_headers: Option<&BlobHTTPHeaders>,
        modified_access_conditions: Option<&ModifiedAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        let mut blob_doc = self.get_blob_with_lease_updated(account, container, blob, "", context, true).await?
            .ok_or_else(|| StorageErrorFactory::get_blob_not_found(&context.context_id))?;

        validate_write_conditions(context, modified_access_conditions, Some(&blob_doc))?;

        let validator = BlobWriteLeaseValidator::new(lease_access_conditions);
        let adapter = BlobLeaseAdapter::new(&blob_doc);
        validator.validate(&adapter, context)?;

        // Update HTTP headers
        if let Some(headers) = blob_http_headers {
            if let Some(cache_control) = &headers.blob_cache_control {
                blob_doc.properties.cache_control = Some(cache_control.clone());
            }
            if let Some(content_type) = &headers.blob_content_type {
                blob_doc.properties.content_type = Some(content_type.clone());
            }
            if let Some(content_md5) = &headers.blob_content_m_d5 {
                blob_doc.properties.content_m_d5 = Some(content_md5.clone());
            }
            if let Some(content_encoding) = &headers.blob_content_encoding {
                blob_doc.properties.content_encoding = Some(content_encoding.clone());
            }
            if let Some(content_language) = &headers.blob_content_language {
                blob_doc.properties.content_language = Some(content_language.clone());
            }
            if let Some(content_disposition) = &headers.blob_content_disposition {
                blob_doc.properties.content_disposition = Some(content_disposition.clone());
            }
        }

        // Sync lease state
        let lease_syncer = BlobWriteLeaseSyncer::new(&mut blob_doc);
        let adapter = BlobLeaseAdapter::new(&blob_doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        lease_state.lease().sync_with_syncer(&lease_syncer)?;

        // Update blob
        let mut blobs = self.blobs_collection.write().unwrap();
        blobs.insert(
            (account.to_string(), container.to_string(), blob.to_string(), "".to_string()),
            blob_doc.clone(),
        );

        Ok(blob_doc.properties)
    }

    async fn set_blob_metadata(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        metadata: Option<&BlobMetadata>,
        modified_access_conditions: Option<&ModifiedAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        let mut blob_doc = self.get_blob_with_lease_updated(account, container, blob, "", context, true).await?
            .ok_or_else(|| StorageErrorFactory::get_blob_not_found(&context.context_id))?;

        validate_write_conditions(context, modified_access_conditions, Some(&blob_doc))?;

        let validator = BlobWriteLeaseValidator::new(lease_access_conditions);
        let adapter = BlobLeaseAdapter::new(&blob_doc);
        validator.validate(&adapter, context)?;

        // Update metadata
        blob_doc.metadata = metadata.cloned();

        // Sync lease state
        let lease_syncer = BlobWriteLeaseSyncer::new(&mut blob_doc);
        let adapter = BlobLeaseAdapter::new(&blob_doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        lease_state.lease().sync_with_syncer(&lease_syncer)?;

        // Update blob
        let mut blobs = self.blobs_collection.write().unwrap();
        blobs.insert(
            (account.to_string(), container.to_string(), blob.to_string(), "".to_string()),
            blob_doc.clone(),
        );

        Ok(blob_doc.properties)
    }

    // ─── Blob Leases ────────────────────────────────────────────────────────

    async fn acquire_blob_lease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        duration: i64,
        proposed_lease_id: Option<&str>,
        options: Option<&BlobAcquireLeaseOptionalParams>,
    ) -> Result<AcquireBlobLeaseResponse, StorageError> {
        let options = options.cloned().unwrap_or_default();

        // Snapshots cannot have leases
        if let Some(snapshot) = &options.snapshot {
            if !snapshot.is_empty() {
                return Err(StorageErrorFactory::get_blob_snapshot_operation_not_supported(&context.context_id));
            }
        }

        let mut blob_doc = self.get_blob(account, container, blob, "").await?
            .ok_or_else(|| StorageErrorFactory::get_blob_not_found(&context.context_id))?;

        validate_write_conditions(context, options.modified_access_conditions.as_ref(), Some(&blob_doc))?;

        let adapter = BlobLeaseAdapter::new(&blob_doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let acquired_state = lease_state.acquire(duration, proposed_lease_id.map(|s| s.to_string()))?;
        let syncer = BlobLeaseSyncer::new(&mut blob_doc);
        acquired_state.lease().sync_with_syncer(&syncer)?;

        // Update blob
        let mut blobs = self.blobs_collection.write().unwrap();
        blobs.insert(
            (account.to_string(), container.to_string(), blob.to_string(), "".to_string()),
            blob_doc.clone(),
        );

        Ok(BlobLeaseResponse {
            properties: blob_doc.properties,
            lease_id: blob_doc.lease_id,
            lease_time: blob_doc.lease_duration_seconds,
        })
    }

    async fn release_blob_lease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        lease_id: &str,
        _options: Option<&BlobReleaseLeaseOptionalParams>,
    ) -> Result<ReleaseBlobLeaseResponse, StorageError> {
        let mut blob_doc = self.get_blob(account, container, blob, "").await?
            .ok_or_else(|| StorageErrorFactory::get_blob_not_found(&context.context_id))?;

        let adapter = BlobLeaseAdapter::new(&blob_doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let released_state = lease_state.release(lease_id.to_string())?;
        let syncer = BlobLeaseSyncer::new(&mut blob_doc);
        released_state.lease().sync_with_syncer(&syncer)?;

        // Update blob
        let mut blobs = self.blobs_collection.write().unwrap();
        blobs.insert(
            (account.to_string(), container.to_string(), blob.to_string(), "".to_string()),
            blob_doc.clone(),
        );

        Ok(blob_doc.properties)
    }

    async fn renew_blob_lease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        lease_id: &str,
        _options: Option<&BlobRenewLeaseOptionalParams>,
    ) -> Result<RenewBlobLeaseResponse, StorageError> {
        let mut blob_doc = self.get_blob(account, container, blob, "").await?
            .ok_or_else(|| StorageErrorFactory::get_blob_not_found(&context.context_id))?;

        let adapter = BlobLeaseAdapter::new(&blob_doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let renewed_state = lease_state.renew(lease_id.to_string())?;
        let syncer = BlobLeaseSyncer::new(&mut blob_doc);
        renewed_state.lease().sync_with_syncer(&syncer)?;

        // Update blob
        let mut blobs = self.blobs_collection.write().unwrap();
        blobs.insert(
            (account.to_string(), container.to_string(), blob.to_string(), "".to_string()),
            blob_doc.clone(),
        );

        Ok(BlobLeaseResponse {
            properties: blob_doc.properties,
            lease_id: blob_doc.lease_id,
            lease_time: blob_doc.lease_duration_seconds,
        })
    }

    async fn change_blob_lease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        lease_id: &str,
        proposed_lease_id: &str,
        _options: Option<&BlobChangeLeaseOptionalParams>,
    ) -> Result<ChangeBlobLeaseResponse, StorageError> {
        let mut blob_doc = self.get_blob(account, container, blob, "").await?
            .ok_or_else(|| StorageErrorFactory::get_blob_not_found(&context.context_id))?;

        let adapter = BlobLeaseAdapter::new(&blob_doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let changed_state = lease_state.change(lease_id.to_string(), proposed_lease_id.to_string())?;
        let syncer = BlobLeaseSyncer::new(&mut blob_doc);
        changed_state.lease().sync_with_syncer(&syncer)?;

        // Update blob
        let mut blobs = self.blobs_collection.write().unwrap();
        blobs.insert(
            (account.to_string(), container.to_string(), blob.to_string(), "".to_string()),
            blob_doc.clone(),
        );

        Ok(BlobLeaseResponse {
            properties: blob_doc.properties,
            lease_id: blob_doc.lease_id,
            lease_time: blob_doc.lease_duration_seconds,
        })
    }

    async fn break_blob_lease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        break_period: Option<i64>,
        _options: Option<&BlobBreakLeaseOptionalParams>,
    ) -> Result<BreakBlobLeaseResponse, StorageError> {
        let mut blob_doc = self.get_blob(account, container, blob, "").await?
            .ok_or_else(|| StorageErrorFactory::get_blob_not_found(&context.context_id))?;

        let adapter = BlobLeaseAdapter::new(&blob_doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let broken_state = lease_state.break_lease(break_period)?;
        let syncer = BlobLeaseSyncer::new(&mut blob_doc);
        broken_state.lease().sync_with_syncer(&syncer)?;

        // Update blob
        let mut blobs = self.blobs_collection.write().unwrap();
        blobs.insert(
            (account.to_string(), container.to_string(), blob.to_string(), "".to_string()),
            blob_doc.clone(),
        );

        Ok(BlobLeaseResponse {
            properties: blob_doc.properties,
            lease_id: blob_doc.lease_id,
            lease_time: blob_doc.lease_duration_seconds,
        })
    }

    async fn check_blob_exist(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
    ) -> Result<(), StorageError> {
        let snapshot = snapshot.unwrap_or("");
        let blobs = self.blobs_collection.read().unwrap();
        if blobs.contains_key(&(account.to_string(), container.to_string(), blob.to_string(), snapshot.to_string())) {
            Ok(())
        } else {
            Err(StorageErrorFactory::get_blob_not_found(&context.context_id))
        }
    }

    async fn get_blob_type(
        &self,
        _account: &str,
        _container: &str,
        _blob: &str,
        _snapshot: Option<&str>,
    ) -> Result<Option<BlobTypeResult>, StorageError> {
        // Simplified implementation - full TS logic is more complex
        // TODO: Implement full getBlobType logic from TS lines 1826-1858
        Ok(None)
    }

    async fn start_copy_from_url(
        &self,
        context: &Context,
        source: BlobId,
        destination: BlobId,
        copy_source: &str,
        metadata: Option<&BlobMetadata>,
        tier: Option<AccessTier>,
        options: Option<&BlobStartCopyFromURLOptionalParams>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        // TS: lines 1860-2047
        let options = options.cloned().unwrap_or_default();

        // Get source blob with lease updated
        let source_blob = self.get_blob_with_lease_updated(
            &source.account,
            &source.container,
            &source.blob,
            &source.snapshot.clone().unwrap_or_default(),
            context,
            true, // forceCommitted
        ).await?
            .ok_or_else(|| StorageErrorFactory::get_blob_not_found(&context.context_id))?;

        // Validate source read conditions
        let source_mac = options.source_modified_access_conditions.clone().unwrap_or_default();
        let read_cond_adapter = ConditionalHeadersAdapter {
            if_modified_since: source_mac.source_if_modified_since,
            if_unmodified_since: source_mac.source_if_unmodified_since,
            if_match: source_mac.source_if_match.clone(),
            if_none_match: source_mac.source_if_none_match.clone(),
            if_tags: source_mac.source_if_tags.clone(),
        };
        let resource_adapter = ConditionResourceAdapter::new(&source_blob.properties, source_blob.metadata.as_ref());
        validate_read_conditions(context, &read_cond_adapter, &resource_adapter, true)?;

        // Get destination blob (may not exist)
        let dest_blob = self.get_blob_with_lease_updated(
            &destination.account,
            &destination.container,
            &destination.blob,
            "",
            context,
            false,
        ).await?;

        // Validate destination write conditions
        validate_write_conditions(
            context,
            options.modified_access_conditions.as_ref(),
            dest_blob.as_ref(),
        )?;

        // Copy if not exists check
        if let Some(ref mac) = options.modified_access_conditions {
            if mac.if_none_match.as_deref() == Some("*") && dest_blob.is_some() {
                return Err(StorageErrorFactory::get_blob_already_exists(&context.context_id));
            }
        }

        // Validate lease on destination if it exists
        if let Some(ref dest) = dest_blob {
            let validator = BlobWriteLeaseValidator::new(options.lease_access_conditions.as_ref());
            let adapter = BlobLeaseAdapter::new(dest);
            validator.validate(&adapter, context)?;
        }

        // Validate source is committed, not deleted
        if source_blob.deleted == Some(true) || source_blob.is_committed == Some(false) {
            return Err(StorageErrorFactory::get_blob_not_found(&context.context_id));
        }

        // Check if source is archived
        if source_blob.properties.access_tier == Some(AccessTier::Archive)
            && (tier.is_none() || source.account != destination.account)
        {
            return Err(StorageErrorFactory::get_blob_archived(&context.context_id));
        }

        // Check container exists
        self.check_container_exist(context, &destination.account, &destination.container).await?;

        // Deep clone source blob
        let mut copied_blob = BlobModel {
            name: Some(destination.blob.clone()),
            deleted: Some(false),
            snapshot: Some("".to_string()),
            properties: source_blob.properties.clone(),
            metadata: if metadata.is_none() || metadata.map(|m| m.is_empty()).unwrap_or(true) {
                source_blob.metadata.clone()
            } else {
                metadata.cloned()
            },
            accountName: destination.account.clone(),
            containerName: destination.container.clone(),
            pageRangesInOrder: source_blob.pageRangesInOrder.clone(),
            isCommitted: source_blob.isCommitted,
            leaseDurationSeconds: dest_blob.as_ref().and_then(|d| d.leaseDurationSeconds),
            leaseId: dest_blob.as_ref().and_then(|d| d.leaseId.clone()),
            leaseExpireTime: dest_blob.as_ref().and_then(|d| d.leaseExpireTime),
            leaseBreakTime: dest_blob.as_ref().and_then(|d| d.leaseBreakTime),
            committedBlocksInOrder: source_blob.committedBlocksInOrder.clone(),
            persistency: source_blob.persistency.clone(),
            blobTags: options.blob_tags_string.as_ref().map(|s| get_tags_from_string(s, &context.context_id)),
        };

        // Update properties for copy
        copied_blob.properties.creation_time = context.start_time;
        copied_blob.properties.last_modified = context.start_time;
        copied_blob.properties.etag = Some(new_etag());
        copied_blob.properties.lease_status = dest_blob.as_ref()
            .and_then(|d| d.properties.lease_status.clone())
            .or_else(|| Some("Unlocked".to_string()));
        copied_blob.properties.lease_state = dest_blob.as_ref()
            .and_then(|d| d.properties.lease_state.clone())
            .or_else(|| Some("Available".to_string()));
        copied_blob.properties.lease_duration = dest_blob.as_ref()
            .and_then(|d| d.properties.lease_duration.clone());
        copied_blob.properties.copy_id = Some(Uuid::new_v4().to_string());
        copied_blob.properties.copy_status = Some("success".to_string());
        copied_blob.properties.copy_source = Some(copy_source.to_string());
        copied_blob.properties.copy_progress = source_blob.properties.content_length
            .map(|len| format!("{}/{}", len, len));
        copied_blob.properties.copy_completion_time = context.start_time;
        copied_blob.properties.copy_status_description = None;
        copied_blob.properties.incremental_copy = Some(false);
        copied_blob.properties.destination_snapshot = None;
        copied_blob.properties.deleted_time = None;
        copied_blob.properties.remaining_retention_days = None;
        copied_blob.properties.archive_status = None;
        copied_blob.properties.access_tier_change_time = None;

        // Handle AppendBlob isSealed
        if source_blob.properties.blob_type == Some(BlobType::AppendBlob) {
            copied_blob.properties.is_sealed = options.seal_blob;
        }

        // Handle tier for BlockBlob
        if copied_blob.properties.blob_type == Some(BlobType::BlockBlob) {
            if let Some(tier_value) = tier {
                copied_blob.properties.access_tier = Self::parse_tier(Some(&tier_value.to_string()));
                if copied_blob.properties.access_tier.is_none() {
                    return Err(StorageErrorFactory::get_invalid_header_value(
                        &context.context_id,
                        "x-ms-access-tier",
                        &tier_value.to_string(),
                    ));
                }
            }
        }

        // PageBlob doesn't support tier in startCopyFromURL
        if copied_blob.properties.blob_type == Some(BlobType::PageBlob) && tier.is_some() {
            return Err(StorageErrorFactory::get_invalid_header_value(
                &context.context_id,
                "x-ms-access-tier",
                &tier.unwrap().to_string(),
            ));
        }

        // Update collection
        let mut blobs = self.blobs_collection.write().unwrap();
        
        // Remove old destination if exists
        if dest_blob.is_some() {
            let dest_key = (
                destination.account.clone(),
                destination.container.clone(),
                destination.blob.clone(),
                "".to_string(),
            );
            blobs.remove(&dest_key);
        }

        // Insert copied blob
        let insert_key = (
            destination.account.clone(),
            destination.container.clone(),
            destination.blob.clone(),
            "".to_string(),
        );
        blobs.insert(insert_key, copied_blob.clone());
        drop(blobs);

        Ok(copied_blob.properties)
    }

    async fn copy_from_url(
        &self,
        context: &Context,
        source: BlobId,
        destination: BlobId,
        copy_source: &str,
        metadata: Option<&BlobMetadata>,
        tier: Option<AccessTier>,
        options: Option<&BlobCopyFromURLOptionalParams>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        // TS: lines 2049-2236
        let options = options.cloned().unwrap_or_default();

        // Get source blob
        let source_blob = self.get_blob_with_lease_updated(
            &source.account,
            &source.container,
            &source.blob,
            &source.snapshot.clone().unwrap_or_default(),
            context,
            true,
        ).await?
            .ok_or_else(|| StorageErrorFactory::get_blob_not_found(&context.context_id))?;

        // Validate source read conditions (note: ignores x-ms-source-if-tags per TS comment)
        let source_mac = options.source_modified_access_conditions.clone().unwrap_or_default();
        let read_cond_adapter = ConditionalHeadersAdapter {
            if_modified_since: source_mac.source_if_modified_since,
            if_unmodified_since: source_mac.source_if_unmodified_since,
            if_match: source_mac.source_if_match.clone(),
            if_none_match: source_mac.source_if_none_match.clone(),
            if_tags: None, // Ignored per TS line 2080
        };
        let resource_adapter = ConditionResourceAdapter::new(&source_blob.properties, source_blob.metadata.as_ref());
        validate_read_conditions(context, &read_cond_adapter, &resource_adapter, false)?;

        // Get destination blob
        let dest_blob = self.get_blob_with_lease_updated(
            &destination.account,
            &destination.container,
            &destination.blob,
            "",
            context,
            false,
        ).await?;

        // Validate destination write conditions
        validate_write_conditions(
            context,
            options.modified_access_conditions.as_ref(),
            dest_blob.as_ref(),
        )?;

        // Copy if not exists check
        if let Some(ref mac) = options.modified_access_conditions {
            if mac.if_none_match.as_deref() == Some("*") && dest_blob.is_some() {
                return Err(StorageErrorFactory::get_blob_already_exists(&context.context_id));
            }
        }

        // Validate lease on destination with syncer
        if let Some(ref dest) = dest_blob {
            let mut dest_clone = dest.clone();
            let lease_adapter = BlobLeaseAdapter::new(&dest_clone);
            let syncer = BlobWriteLeaseSyncer::new(&mut dest_clone);
            syncer.sync(&lease_adapter)?;
            
            let validator = BlobWriteLeaseValidator::new(options.lease_access_conditions.as_ref());
            validator.validate(&lease_adapter, context)?;
        }

        // Validate source is committed, not deleted, not archived
        if source_blob.deleted == Some(true) || source_blob.is_committed == Some(false) {
            return Err(StorageErrorFactory::get_blob_not_found(&context.context_id));
        }

        if source_blob.properties.access_tier == Some(AccessTier::Archive) {
            return Err(StorageErrorFactory::get_blob_archived(&context.context_id));
        }

        // Check container exists
        self.check_container_exist(context, &destination.account, &destination.container).await?;

        // Deep clone source blob
        let mut copied_blob = BlobModel {
            name: Some(destination.blob.clone()),
            deleted: Some(false),
            snapshot: Some("".to_string()),
            properties: source_blob.properties.clone(),
            metadata: if metadata.is_none() || metadata.map(|m| m.is_empty()).unwrap_or(true) {
                source_blob.metadata.clone()
            } else {
                metadata.cloned()
            },
            accountName: destination.account.clone(),
            containerName: destination.container.clone(),
            pageRangesInOrder: source_blob.pageRangesInOrder.clone(),
            isCommitted: source_blob.isCommitted,
            leaseDurationSeconds: dest_blob.as_ref().and_then(|d| d.leaseDurationSeconds),
            leaseId: dest_blob.as_ref().and_then(|d| d.leaseId.clone()),
            leaseExpireTime: dest_blob.as_ref().and_then(|d| d.leaseExpireTime),
            leaseBreakTime: dest_blob.as_ref().and_then(|d| d.leaseBreakTime),
            committedBlocksInOrder: source_blob.committedBlocksInOrder.clone(),
            persistency: source_blob.persistency.clone(),
            blobTags: None, // Set below based on copySourceTags
        };

        // Handle blob tags based on copySourceTags
        if options.copy_source_tags == Some("COPY".to_string()) {
            copied_blob.blobTags = source_blob.blobTags.clone();
        } else if let Some(ref tags_str) = options.blob_tags_string {
            copied_blob.blobTags = Some(get_tags_from_string(tags_str, &context.context_id));
        }

        // Update properties for copy
        copied_blob.properties.creation_time = context.start_time;
        copied_blob.properties.last_modified = context.start_time;
        copied_blob.properties.etag = Some(new_etag());
        copied_blob.properties.lease_status = dest_blob.as_ref()
            .and_then(|d| d.properties.lease_status.clone())
            .or_else(|| Some("Unlocked".to_string()));
        copied_blob.properties.lease_state = dest_blob.as_ref()
            .and_then(|d| d.properties.lease_state.clone())
            .or_else(|| Some("Available".to_string()));
        copied_blob.properties.lease_duration = dest_blob.as_ref()
            .and_then(|d| d.properties.lease_duration.clone());
        copied_blob.properties.copy_id = Some(Uuid::new_v4().to_string());
        copied_blob.properties.copy_status = Some("success".to_string());
        copied_blob.properties.copy_source = Some(copy_source.to_string());
        copied_blob.properties.copy_progress = source_blob.properties.content_length
            .map(|len| format!("{}/{}", len, len));
        copied_blob.properties.copy_completion_time = context.start_time;
        copied_blob.properties.copy_status_description = None;
        copied_blob.properties.incremental_copy = Some(false);
        copied_blob.properties.destination_snapshot = None;
        copied_blob.properties.deleted_time = None;
        copied_blob.properties.remaining_retention_days = None;
        copied_blob.properties.archive_status = None;
        copied_blob.properties.access_tier_change_time = None;

        // Handle tier for BlockBlob
        if copied_blob.properties.blob_type == Some(BlobType::BlockBlob) {
            if let Some(tier_value) = tier {
                copied_blob.properties.access_tier = Self::parse_tier(Some(&tier_value.to_string()));
                if copied_blob.properties.access_tier.is_none() {
                    return Err(StorageErrorFactory::get_invalid_header_value(
                        &context.context_id,
                        "x-ms-access-tier",
                        &tier_value.to_string(),
                    ));
                }
            }
        }

        // PageBlob doesn't support tier
        if copied_blob.properties.blob_type == Some(BlobType::PageBlob) && tier.is_some() {
            return Err(StorageErrorFactory::get_invalid_header_value(
                &context.context_id,
                "x-ms-access-tier",
                &tier.unwrap().to_string(),
            ));
        }

        // Update collection
        let mut blobs = self.blobs_collection.write().unwrap();
        
        // Remove old destination if exists
        if dest_blob.is_some() {
            let dest_key = (
                destination.account.clone(),
                destination.container.clone(),
                destination.blob.clone(),
                "".to_string(),
            );
            blobs.remove(&dest_key);
        }

        // Insert copied blob
        let insert_key = (
            destination.account.clone(),
            destination.container.clone(),
            destination.blob.clone(),
            "".to_string(),
        );
        blobs.insert(insert_key, copied_blob.clone());
        drop(blobs);

        Ok(copied_blob.properties)
    }

    async fn set_tier(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        tier: AccessTier,
        lease_access_conditions: Option<&LeaseAccessConditions>,
    ) -> Result<i32, StorageError> {
        // TS: lines 2238-2311
        let mut doc = self.get_blob_with_lease_updated(account, container, blob, "", context, false).await?
            .ok_or_else(|| StorageErrorFactory::get_blob_not_found(&context.context_id))?;

        // Validate lease
        let validator = BlobWriteLeaseValidator::new(lease_access_conditions);
        let adapter = BlobLeaseAdapter::new(&doc);
        validator.validate(&adapter, context)?;

        // Cannot set tier on snapshot
        if !doc.snapshot.as_ref().unwrap_or(&String::new()).is_empty() {
            return Err(StorageErrorFactory::get_blob_snapshot_operation_not_supported(&context.context_id));
        }

        // Determine response code (202 if from Archive, otherwise 200)
        let response_code = if doc.properties.access_tier == Some(AccessTier::Archive)
            && (tier == AccessTier::Cool || tier == AccessTier::Hot || tier == AccessTier::Cold)
        {
            202
        } else {
            200
        };

        // Validate tier based on blob type
        match doc.properties.blob_type {
            Some(BlobType::BlockBlob) => {
                // BlockBlob supports Archive, Cool, Hot, Cold
                match tier {
                    AccessTier::Archive | AccessTier::Cool | AccessTier::Hot | AccessTier::Cold => {
                        doc.properties.access_tier = Some(tier);
                    }
                    _ => {
                        return Err(StorageErrorFactory::get_invalid_header_value(
                            &context.context_id,
                            "x-ms-access-tier",
                            &tier.to_string(),
                        ));
                    }
                }
            }
            Some(BlobType::PageBlob) => {
                // PageBlob doesn't support tier
                return Err(StorageErrorFactory::get_invalid_header_value(
                    &context.context_id,
                    "x-ms-access-tier",
                    &tier.to_string(),
                ));
            }
            _ => {
                return Err(StorageErrorFactory::get_invalid_header_value(
                    &context.context_id,
                    "x-ms-access-tier",
                    &tier.to_string(),
                ));
            }
        }

        // Update properties
        doc.properties.access_tier_inferred = Some(false);
        doc.properties.access_tier_change_time = context.start_time;

        // Sync lease state
        let adapter = BlobLeaseAdapter::new(&doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let mut syncer = BlobLeaseSyncer::new(&mut doc);
        lease_state.lease().sync_with_syncer(&syncer)?;

        // Update collection
        let mut blobs = self.blobs_collection.write().unwrap();
        let key = (account.to_string(), container.to_string(), blob.to_string(), "".to_string());
        blobs.insert(key, doc);
        drop(blobs);

        Ok(response_code)
    }

    async fn set_blob_tag(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        tags: Option<&BlobTags>,
        _modified_access_conditions: Option<&ModifiedAccessConditions>,
    ) -> Result<(), StorageError> {
        // TS: lines 3419-3448
        // NOTE: TS ignores modifiedAccessConditions parameter (fidelity flag from line 3419)
        let snapshot = snapshot.unwrap_or("");
        
        let mut doc = self.get_blob_with_lease_updated(account, container, blob, snapshot, context, false).await?
            .ok_or_else(|| StorageErrorFactory::get_blob_not_found(&context.context_id))?;

        // Validate lease
        let validator = BlobWriteLeaseValidator::new(lease_access_conditions);
        let adapter = BlobLeaseAdapter::new(&doc);
        validator.validate(&adapter, context)?;

        // Sync lease state
        let adapter = BlobLeaseAdapter::new(&doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let mut syncer = BlobLeaseSyncer::new(&mut doc);
        lease_state.lease().sync_with_syncer(&syncer)?;

        // Update blob tags
        doc.blobTags = tags.cloned();

        // Update collection
        let mut blobs = self.blobs_collection.write().unwrap();
        let key = (account.to_string(), container.to_string(), blob.to_string(), snapshot.to_string());
        blobs.insert(key, doc);
        drop(blobs);

        Ok(())
    }

    async fn get_blob_tag(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        modified_access_conditions: Option<&ModifiedAccessConditions>,
    ) -> Result<Option<BlobTags>, StorageError> {
        // TS: lines 3464-3503
        let snapshot = snapshot.unwrap_or("");
        
        let doc = self.get_blob_with_lease_updated(account, container, blob, snapshot, context, false).await?
            .ok_or_else(|| StorageErrorFactory::get_blob_not_found(&context.context_id))?;

        // Validate read conditions
        validate_read_conditions(
            context,
            &ConditionalHeadersAdapter::from_modified_access_conditions(modified_access_conditions),
            &ConditionResourceAdapter::new(&doc.properties, doc.metadata.as_ref()),
            false,
        )?;

        // Validate lease
        let validator = BlobReadLeaseValidator::new(lease_access_conditions);
        let adapter = BlobLeaseAdapter::new(&doc);
        validator.validate(&adapter, context)?;

        Ok(doc.blobTags)
    }

    async fn seal_blob(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _snapshot: Option<&str>,
        _options: Option<&AppendBlobSealOptionalParams>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        // Placeholder
        // TODO: Implement sealBlob from TS lines 3533-3565
        Err(StorageError::from("sealBlob not yet implemented"))
    }

    // ─── Block Operations ───────────────────────────────────────────────────

    async fn stage_block(
        &self,
        context: &Context,
        block: BlockModel,
        lease_access_conditions: Option<&LeaseAccessConditions>,
    ) -> Result<(), StorageError> {
        // TS: lines 2313-2395
        
        // Check container exists
        self.check_container_exist(context, &block.accountName, &block.containerName).await?;

        // Check if blob exists
        let blob_key = (
            block.accountName.clone(),
            block.containerName.clone(),
            block.blobName.clone(),
            "".to_string(),
        );

        let blob_exists = {
            let blobs = self.blobs_collection.read().unwrap();
            blobs.contains_key(&blob_key)
        };

        if !blob_exists {
            // Create new uncommitted BlockBlob
            let etag = new_etag();
            let new_blob = BlobModel {
                deleted: Some(false),
                accountName: block.accountName.clone(),
                containerName: block.containerName.clone(),
                name: Some(block.blobName.clone()),
                properties: BlobPropertiesInternal {
                    creation_time: context.start_time,
                    last_modified: context.start_time,
                    etag: Some(etag),
                    content_length: Some(0),
                    blob_type: Some(BlobType::BlockBlob),
                    ..Default::default()
                },
                snapshot: Some("".to_string()),
                isCommitted: Some(false),
                ..Default::default()
            };

            let mut blobs = self.blobs_collection.write().unwrap();
            blobs.insert(blob_key, new_blob);
            drop(blobs);
        } else {
            // Validate existing blob
            let blobs = self.blobs_collection.read().unwrap();
            let blob_doc = blobs.get(&blob_key).unwrap();

            // Check blob type is BlockBlob
            if blob_doc.properties.blob_type != Some(BlobType::BlockBlob) {
                return Err(StorageErrorFactory::get_blob_invalid_blob_type(&context.context_id));
            }

            // Validate lease
            let adapter = BlobLeaseAdapter::new(blob_doc);
            let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
            let validator = BlobWriteLeaseValidator::new(lease_access_conditions);
            lease_state.validate(&validator)?;
            
            let mut blob_clone = blob_doc.clone();
            let syncer = BlobWriteLeaseSyncer::new(&mut blob_clone);
            lease_state.lease().sync_with_syncer(&syncer)?;
            drop(blobs);
        }

        // Validate block ID length consistency if blob exists and has blocks
        if blob_exists {
            let blocks = self.blocks_collection.read().unwrap();
            let existing_block = blocks.iter().find(|((acc, cont, blob_name, _), _)| {
                acc == &block.accountName && cont == &block.containerName && blob_name == &block.blobName
            });

            if let Some((_, existing)) = existing_block {
                // Decode base64 block IDs and compare lengths
                if let (Some(ref existing_name), Some(ref new_name)) = (&existing.name, &block.name) {
                    let existing_decoded = base64::decode(existing_name).unwrap_or_default();
                    let new_decoded = base64::decode(new_name).unwrap_or_default();
                    if existing_decoded.len() != new_decoded.len() {
                        return Err(StorageErrorFactory::get_invalid_blob_or_block(&context.context_id));
                    }
                }
            }
            drop(blocks);
        }

        // Find and remove existing block with same name
        let block_key = (
            block.accountName.clone(),
            block.containerName.clone(),
            block.blobName.clone(),
            block.name.clone().unwrap_or_default(),
        );

        let mut blocks = self.blocks_collection.write().unwrap();
        blocks.remove(&block_key);
        blocks.insert(block_key, block);
        drop(blocks);

        Ok(())
    }

    async fn append_block(
        &self,
        context: &Context,
        block: BlockModel,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        modified_access_conditions: Option<&ModifiedAccessConditions>,
        append_position_access_conditions: Option<&AppendPositionAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        // TS: lines 2397-2473
        
        let mut doc = self.get_blob_with_lease_updated(
            &block.accountName,
            &block.containerName,
            &block.blobName,
            "",
            context,
            true, // forceCommitted
        ).await?
            .ok_or_else(|| StorageErrorFactory::get_blob_not_found(&context.context_id))?;

        // Validate write conditions
        validate_write_conditions(context, modified_access_conditions, Some(&doc))?;

        // Validate lease
        let lease_adapter = BlobLeaseAdapter::new(&doc);
        let validator = BlobWriteLeaseValidator::new(lease_access_conditions);
        validator.validate(&lease_adapter, context)?;

        // Check if blob is sealed
        if doc.properties.is_sealed == Some(true) {
            return Err(StorageErrorFactory::get_blob_sealed(&context.context_id));
        }

        // Validate blob type
        if doc.properties.blob_type != Some(BlobType::AppendBlob) {
            return Err(StorageErrorFactory::get_blob_invalid_blob_type(&context.context_id));
        }

        // Check max block count
        let current_block_count = doc.committedBlocksInOrder.as_ref().map(|v| v.len()).unwrap_or(0);
        if current_block_count >= MAX_APPEND_BLOB_BLOCK_COUNT {
            return Err(StorageErrorFactory::get_block_count_exceeds_limit(&context.context_id));
        }

        // Validate append position if specified
        if let Some(ref apc) = append_position_access_conditions {
            if let Some(append_position) = apc.append_position {
                let current_length = doc.properties.content_length.unwrap_or(0);
                if current_length != append_position {
                    return Err(StorageErrorFactory::get_append_position_condition_not_met(&context.context_id));
                }
            }

            // Validate max size if specified
            if let Some(max_size) = apc.max_size {
                let current_length = doc.properties.content_length.unwrap_or(0);
                let block_size = block.size.unwrap_or(0);
                if current_length + block_size > max_size {
                    return Err(StorageErrorFactory::get_max_blob_size_condition_not_met(&context.context_id));
                }
            }
        }

        // Sync lease
        let lease_adapter = BlobLeaseAdapter::new(&doc);
        let mut syncer = BlobWriteLeaseSyncer::new(&mut doc);
        syncer.sync(&lease_adapter)?;

        // Append block
        if doc.committedBlocksInOrder.is_none() {
            doc.committedBlocksInOrder = Some(Vec::new());
        }
        let block_persistency = PersistencyBlockModel {
            name: block.name.clone(),
            size: block.size,
            persistency: block.persistency.clone(),
        };
        doc.committedBlocksInOrder.as_mut().unwrap().push(block_persistency);

        // Update properties
        doc.properties.etag = Some(new_etag());
        doc.properties.last_modified = context.start_time;
        let block_size = block.size.unwrap_or(0);
        doc.properties.content_length = Some(doc.properties.content_length.unwrap_or(0) + block_size);

        // Update collection
        let mut blobs = self.blobs_collection.write().unwrap();
        let key = (
            block.accountName.clone(),
            block.containerName.clone(),
            block.blobName.clone(),
            "".to_string(),
        );
        blobs.insert(key, doc.clone());
        drop(blobs);

        Ok(doc.properties)
    }

    async fn commit_block_list(
        &self,
        _context: &Context,
        _blob: BlobModel,
        _block_list: Vec<BlockListEntry>,
        _lease_access_conditions: Option<&LeaseAccessConditions>,
        _modified_access_conditions: Option<&ModifiedAccessConditions>,
    ) -> Result<(), StorageError> {
        // Placeholder - full implementation would be ~165 lines
        // TODO: Implement commitBlockList from TS lines 2486-2651
        Err(StorageError::from("commitBlockList not yet implemented"))
    }

    async fn get_block_list(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _snapshot: Option<&str>,
        _is_committed: Option<bool>,
        _lease_access_conditions: Option<&LeaseAccessConditions>,
        _modified_access_conditions: Option<&ModifiedAccessConditions>,
    ) -> Result<(BlobPropertiesInternal, Vec<Block>, Vec<Block>), StorageError> {
        // Placeholder - full implementation would be ~80 lines
        // TODO: Implement getBlockList from TS lines 2653-2732
        Err(StorageError::from("getBlockList not yet implemented"))
    }

    async fn list_uncommitted_block_persistency_chunks(
        &self,
        _marker: Option<&str>,
        _max_results: Option<i64>,
    ) -> Result<(Vec<IExtentChunk>, Option<String>), StorageError> {
        // Placeholder - full implementation would be ~30 lines
        // TODO: Implement from TS lines 3079-3108
        Err(StorageError::from("listUncommittedBlockPersistencyChunks not yet implemented"))
    }

    // ─── Page Blob Operations ───────────────────────────────────────────────

    async fn upload_pages(
        &self,
        _context: &Context,
        _blob: BlobModel,
        _start: i64,
        _end: i64,
        _persistency: IExtentChunk,
        _lease_access_conditions: Option<&LeaseAccessConditions>,
        _modified_access_conditions: Option<&ModifiedAccessConditions>,
        _sequence_number_access_conditions: Option<&SequenceNumberAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        // Placeholder - full implementation would be ~70 lines
        // TODO: Implement uploadPages from TS lines 2734-2802
        Err(StorageError::from("uploadPages not yet implemented"))
    }

    async fn clear_range(
        &self,
        _context: &Context,
        _blob: BlobModel,
        _start: i64,
        _end: i64,
        _lease_access_conditions: Option<&LeaseAccessConditions>,
        _modified_access_conditions: Option<&ModifiedAccessConditions>,
        _sequence_number_access_conditions: Option<&SequenceNumberAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        // Placeholder - full implementation would be ~65 lines
        // TODO: Implement clearRange from TS lines 2804-2867
        Err(StorageError::from("clearRange not yet implemented"))
    }

    async fn get_page_ranges(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _snapshot: Option<&str>,
        _lease_access_conditions: Option<&LeaseAccessConditions>,
        _modified_access_conditions: Option<&ModifiedAccessConditions>,
    ) -> Result<GetPageRangeResponse, StorageError> {
        // Placeholder - full implementation would be ~50 lines
        // TODO: Implement getPageRanges from TS lines 2869-2920
        Err(StorageError::from("getPageRanges not yet implemented"))
    }

    async fn resize_page_blob(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _blob_content_length: i64,
        _lease_access_conditions: Option<&LeaseAccessConditions>,
        _modified_access_conditions: Option<&ModifiedAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        // Placeholder - full implementation would be ~70 lines
        // TODO: Implement resizePageBlob from TS lines 2922-2990
        Err(StorageError::from("resizePageBlob not yet implemented"))
    }

    async fn update_sequence_number(
        &self,
        _context: &Context,
        _account: &str,
        _container: &str,
        _blob: &str,
        _sequence_number_action: SequenceNumberActionType,
        _blob_sequence_number: Option<i64>,
        _lease_access_conditions: Option<&LeaseAccessConditions>,
        _modified_access_conditions: Option<&ModifiedAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        // Placeholder - full implementation would be ~85 lines
        // TODO: Implement updateSequenceNumber from TS lines 2992-3077
        Err(StorageError::from("updateSequenceNumber not yet implemented"))
    }
}

#[async_trait]
impl IGCExtentProvider for LokiBlobMetadataStore {
    async fn iterator_extents(&self) -> Box<dyn Iterator<Item = Vec<String>> + Send> {
        Box::new(BlobReferredExtentsAsyncIterator::new(self))
    }
}
