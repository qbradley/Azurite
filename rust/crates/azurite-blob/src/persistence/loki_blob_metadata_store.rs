use async_trait::async_trait;
use base64::Engine;
use chrono::{DateTime, NaiveDateTime, Utc};
use futures::stream::{self, BoxStream};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

use azurite_common::i_data_store::IDataStore;
use azurite_common::i_gc_extent_provider::IGCExtentProvider;
use azurite_common::storage_error::StorageError as CommonStorageError;
use azurite_common::utils::utils::{
    convertDateTimeStringMsTo7Digital as convert_date_time_string_ms_to_7_digital, formatRfc1123,
    newEtag as new_etag,
};

use crate::conditions::read_conditional_headers_validator::validate_read_conditions;
use crate::conditions::write_conditional_headers_validator::{
    validate_sequence_number_write_conditions, validate_write_conditions,
    validate_write_conditions_container,
};
use crate::errors::{StorageError, StorageErrorFactory};
use crate::generated::artifacts::models::{
    AppendBlobSealOptionalParams, AppendPositionAccessConditions, BlobAcquireLeaseOptionalParams,
    BlobBreakLeaseOptionalParams, BlobChangeLeaseOptionalParams, BlobCopyFromURLOptionalParams,
    BlobDeleteMethodOptionalParams, BlobHTTPHeaders, BlobMetadata, BlobPropertiesInternal,
    BlobReleaseLeaseOptionalParams, BlobRenewLeaseOptionalParams,
    BlobStartCopyFromURLOptionalParams, BlobTags, Block, ContainerAcquireLeaseOptionalParams,
    ContainerBreakLeaseOptionalParams, ContainerChangeLeaseOptionalParams,
    ContainerDeleteMethodOptionalParams, ContainerReleaseLeaseOptionalParams,
    ContainerRenewLeaseOptionalParams, GeneratedObject, GeneratedValue, LeaseAccessConditions,
    ModifiedAccessConditions, PageRange, SequenceNumberAccessConditions,
};
use crate::generated::context::Context;
use crate::handlers::page_blob_ranges_manager::PageBlobRangesManager;
use crate::lease::{
    BlobLeaseAdapter, BlobLeaseSyncer, BlobReadLeaseValidator, BlobWriteLeaseSyncer,
    BlobWriteLeaseValidator, ContainerDeleteLeaseValidator, ContainerLeaseAdapter,
    ContainerLeaseSyncer, ContainerReadLeaseValidator, ILeaseSyncer, ILeaseValidator, LeaseFactory,
};
use crate::persistence::blob_referred_extents_async_iterator::BlobReferredExtentsAsyncIterator;
use crate::persistence::i_blob_metadata_store::{
    AcquireBlobLeaseResponse, AcquireContainerLeaseResponse, BlobId, BlobLeaseResponse, BlobModel,
    BlobPrefixModel, BlobTypeResult, BlockListEntry, BlockModel, BreakBlobLeaseResponse,
    BreakContainerLeaseResponse, ChangeBlobLeaseResponse, ChangeContainerLeaseResponse,
    ContainerLeaseResponse, ContainerModel, CreateSnapshotResponse, FilterBlobModel,
    GetBlobPropertiesRes, GetContainerAccessPolicyResponse, GetContainerPropertiesResponse,
    GetPageRangeResponse, IBlobMetadataStore, IContainerMetadata, IExtentChunk,
    PersistencyBlockModel, PersistencyPageRange, ReleaseBlobLeaseResponse,
    ReleaseContainerLeaseResponse, RenewBlobLeaseResponse, RenewContainerLeaseResponse,
    ServicePropertiesModel, SetContainerAccessPolicyOptions,
};
use crate::persistence::page_with_delimiter::PageWithDelimiter;
use crate::persistence::query_interpreter::query_interpreter::generate_query_blob_with_tags_where_function;
use crate::utils::constants::{
    DEFAULT_LIST_BLOBS_MAX_RESULTS, DEFAULT_LIST_CONTAINERS_MAX_RESULTS,
};
use crate::utils::{getTagsFromString as get_tags_from_string, MAX_APPEND_BLOB_BLOCK_COUNT};

const BLOB_TYPE_APPEND_BLOB: &str = "AppendBlob";
const BLOB_TYPE_BLOCK_BLOB: &str = "BlockBlob";
const BLOB_TYPE_PAGE_BLOB: &str = "PageBlob";
const ACCESS_TIER_ARCHIVE: &str = "Archive";
const ACCESS_TIER_COLD: &str = "Cold";
const ACCESS_TIER_COOL: &str = "Cool";
const ACCESS_TIER_HOT: &str = "Hot";

fn get_string(map: &GeneratedObject, key: &str) -> Option<String> {
    map.get(key).and_then(GeneratedValue::as_string)
}

fn get_i64(map: &GeneratedObject, key: &str) -> Option<i64> {
    map.get(key)
        .and_then(GeneratedValue::as_number)
        .map(|value| value as i64)
}

fn get_bool(map: &GeneratedObject, key: &str) -> Option<bool> {
    map.get(key).and_then(GeneratedValue::as_bool)
}

fn get_object(map: &GeneratedObject, key: &str) -> Option<GeneratedObject> {
    map.get(key).and_then(GeneratedValue::as_object).cloned()
}

#[allow(dead_code)]
fn get_object_ref<'a>(map: &'a GeneratedObject, key: &str) -> Option<&'a GeneratedObject> {
    map.get(key).and_then(GeneratedValue::as_object)
}

fn set_value(map: &mut GeneratedObject, key: &str, value: Option<GeneratedValue>) {
    match value {
        Some(value) => {
            map.insert(key.to_string(), value);
        }
        None => {
            map.remove(key);
        }
    }
}

fn set_string(map: &mut GeneratedObject, key: &str, value: Option<String>) {
    set_value(map, key, value.map(GeneratedValue::String));
}

fn set_i64(map: &mut GeneratedObject, key: &str, value: Option<i64>) {
    set_value(
        map,
        key,
        value.map(|value| GeneratedValue::Number(value as f64)),
    );
}

fn set_bool(map: &mut GeneratedObject, key: &str, value: Option<bool>) {
    set_value(map, key, value.map(GeneratedValue::Bool));
}

#[allow(dead_code)]
fn set_object(map: &mut GeneratedObject, key: &str, value: Option<GeneratedObject>) {
    set_value(map, key, value.map(GeneratedValue::Object));
}

fn context_id(context: &Context) -> String {
    context.contextId().unwrap_or_default()
}

/// Remap sourceModifiedAccessConditions fields (sourceIfMatch, sourceIfNoneMatch, etc.)
/// to standard ModifiedAccessConditions fields (ifMatch, ifNoneMatch, etc.)
/// so they can be passed to validate_read_conditions. Matches TS behavior.
fn remap_source_conditions(source_conditions: &GeneratedObject) -> GeneratedObject {
    let mut remapped = GeneratedObject::new();
    let mappings = [
        ("sourceIfModifiedSince", "ifModifiedSince"),
        ("sourceIfUnmodifiedSince", "ifUnmodifiedSince"),
        ("sourceIfMatch", "ifMatch"),
        ("sourceIfNoneMatch", "ifNoneMatch"),
        ("sourceIfTags", "ifTags"),
    ];
    for (src, dst) in &mappings {
        if let Some(v) = source_conditions.get(*src) {
            remapped.insert(dst.to_string(), v.clone());
        }
    }
    remapped
}

fn get_datetime(map: &GeneratedObject, key: &str) -> Option<DateTime<Utc>> {
    get_string(map, key).and_then(|value| {
        // Try RFC 3339/ISO 8601 first, then RFC 1123 / RFC 2822
        DateTime::parse_from_rfc3339(&value)
            .ok()
            .map(|value| value.with_timezone(&Utc))
            .or_else(|| {
                DateTime::parse_from_rfc2822(&value)
                    .ok()
                    .map(|value| value.with_timezone(&Utc))
            })
            .or_else(|| {
                NaiveDateTime::parse_from_str(
                    value.trim_end_matches(" GMT"),
                    "%a, %d %b %Y %H:%M:%S",
                )
                .ok()
                .map(|dt| dt.and_utc())
            })
    })
}

fn set_datetime(map: &mut GeneratedObject, key: &str, value: Option<DateTime<Utc>>) {
    set_value(
        map,
        key,
        value.map(|value| GeneratedValue::String(formatRfc1123(value))),
    );
}

fn apply_copy_properties(
    properties: &mut GeneratedObject,
    context: &Context,
    source_properties: &GeneratedObject,
    destination_blob: Option<&BlobModel>,
    copy_source: &str,
) {
    set_datetime(properties, "creationTime", context.startTime());
    set_datetime(properties, "lastModified", context.startTime());
    set_string(properties, "etag", Some(new_etag()));
    set_string(
        properties,
        "leaseStatus",
        destination_blob
            .and_then(|blob| get_string(&blob.properties, "leaseStatus"))
            .or_else(|| Some("unlocked".to_string())),
    );
    set_string(
        properties,
        "leaseState",
        destination_blob
            .and_then(|blob| get_string(&blob.properties, "leaseState"))
            .or_else(|| Some("available".to_string())),
    );
    set_string(
        properties,
        "leaseDuration",
        destination_blob.and_then(|blob| get_string(&blob.properties, "leaseDuration")),
    );
    set_string(properties, "copyId", Some(Uuid::new_v4().to_string()));
    set_string(properties, "copyStatus", Some("success".to_string()));
    set_string(properties, "copySource", Some(copy_source.to_string()));
    set_string(
        properties,
        "copyProgress",
        get_i64(source_properties, "contentLength").map(|len| format!("{}/{}", len, len)),
    );
    set_datetime(properties, "copyCompletionTime", context.startTime());
    set_string(properties, "copyStatusDescription", None);
    set_bool(properties, "incrementalCopy", Some(false));
    set_string(properties, "destinationSnapshot", None);
    set_datetime(properties, "deletedTime", None);
    set_i64(properties, "remainingRetentionDays", None);
    set_string(properties, "archiveStatus", None);
    set_datetime(properties, "accessTierChangeTime", None);
}

fn invalid_header_value(context_id: Option<&str>, header: &str, value: &str) -> StorageError {
    let mut extra = BTreeMap::new();
    extra.insert("HeaderName".to_string(), header.to_string());
    extra.insert("HeaderValue".to_string(), value.to_string());
    StorageErrorFactory::get_invalid_header_value(context_id, Some(extra))
}

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
#[derive(Clone)]
pub struct LokiBlobMetadataStore {
    #[allow(dead_code)]
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
            return Err(StorageError::new(
                400,
                "InvalidOperation",
                "Cannot clean LokiBlobMetadataStore, it's not closed.",
                "loki-blob-metadata-store",
                StorageError::empty_extra(),
            ));
        }
        // TODO: If not in_memory, delete file at self.loki_db_path
        // In TS: rimrafAsync(this.lokiDBPath)
        // For now: no-op
        Ok(())
    }

    /// Private helper: Escape regex special characters.
    #[allow(dead_code)]
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
        value.cloned()
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
        let doc = containers
            .get(&(account.to_string(), container.to_string()))
            .cloned();
        drop(containers);

        match doc {
            Some(mut doc_val) => {
                // Sync lease state
                let adapter = ContainerLeaseAdapter::new(&doc_val);
                let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
                let mut syncer = ContainerLeaseSyncer::new(&mut doc_val);
                let _ = syncer.sync(lease_state.lease());

                Ok(Some(doc_val))
            }
            None => {
                if throw_if_not_found {
                    Err(StorageErrorFactory::get_container_not_found(
                        context.contextId().as_deref(),
                    ))
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
        let doc = containers
            .get(&(account.to_string(), container.to_string()))
            .cloned();
        drop(containers);

        if doc.is_none() && throw_if_not_found {
            Err(StorageErrorFactory::get_container_not_found(
                _context.contextId().as_deref(),
            ))
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
        // TS line 3346: Check container exists BEFORE checking blob
        self.checkContainerExist(context, account, container)
            .await?;

        let blobs = self.blobs_collection.read().unwrap();
        let key = (
            account.to_string(),
            container.to_string(),
            blob.to_string(),
            snapshot.to_string(),
        );
        let doc = blobs.get(&key).cloned();
        drop(blobs);

        match doc {
            Some(mut doc_val) => {
                // Restore Uint8Array for contentMD5.
                let _ = Self::restore_uint8_array(None);

                // For snapshots, normalize lease state/status to Available/Unlocked
                // TS lines 3385-3395
                if !snapshot.is_empty() {
                    set_string(
                        &mut doc_val.properties,
                        "leaseState",
                        Some("available".to_string()),
                    );
                    set_string(
                        &mut doc_val.properties,
                        "leaseStatus",
                        Some("unlocked".to_string()),
                    );
                    // TODO: According to TS TODO, snapshot lease state/status should be undefined,
                    // but current behavior sets them to Available/Unlocked
                }

                // If not a snapshot, sync lease state
                if snapshot.is_empty() {
                    let adapter = BlobLeaseAdapter::new(&doc_val);
                    let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
                    let mut syncer = BlobLeaseSyncer::new(&mut doc_val);
                    let _ = syncer.sync(lease_state.lease());
                }

                // If forceCommitted and blob is uncommitted, return None
                if force_committed && doc_val.isCommitted == Some(false) {
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
        let key = (
            account.to_string(),
            container.to_string(),
            blob.to_string(),
            snapshot.to_string(),
        );
        let doc = blobs.get(&key).cloned();
        drop(blobs);
        Ok(doc)
    }

    /// Private helper: Parse tier string.
    /// TS: lines 3506-3521
    fn parse_tier(tier: Option<&str>) -> Option<String> {
        tier.and_then(|t| match t {
            ACCESS_TIER_HOT => Some(ACCESS_TIER_HOT.to_string()),
            ACCESS_TIER_COOL => Some(ACCESS_TIER_COOL.to_string()),
            ACCESS_TIER_ARCHIVE => Some(ACCESS_TIER_ARCHIVE.to_string()),
            ACCESS_TIER_COLD => Some(ACCESS_TIER_COLD.to_string()),
            _ => None,
        })
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl IDataStore for LokiBlobMetadataStore {
    async fn init(&mut self) -> Result<(), CommonStorageError> {
        LokiBlobMetadataStore::init(self)
            .await
            .map_err(|error| CommonStorageError::new(error.to_string()))
    }

    fn isInitialized(&self) -> bool {
        self.is_initialized()
    }

    async fn close(&mut self) -> Result<(), CommonStorageError> {
        LokiBlobMetadataStore::close(self)
            .await
            .map_err(|error| CommonStorageError::new(error.to_string()))
    }

    fn isClosed(&self) -> bool {
        self.is_closed()
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl IBlobMetadataStore for LokiBlobMetadataStore {
    // ─── Service Properties ─────────────────────────────────────────────────

    async fn setServiceProperties(
        &self,
        _context: &Context,
        service_properties: ServicePropertiesModel,
    ) -> Result<ServicePropertiesModel, StorageError> {
        let mut services = self.services_collection.write().unwrap();

        if let Some(doc) = services.get_mut(&service_properties.accountName) {
            // Update existing (undefined properties are ignored)
            for key in [
                "cors",
                "hourMetrics",
                "logging",
                "minuteMetrics",
                "defaultServiceVersion",
                "deleteRetentionPolicy",
                "staticWebsite",
            ] {
                if let Some(value) = service_properties.properties.get(key).cloned() {
                    doc.properties.insert(key.to_string(), value);
                }
            }
            Ok(doc.clone())
        } else {
            // Insert new
            services.insert(
                service_properties.accountName.clone(),
                service_properties.clone(),
            );
            Ok(service_properties)
        }
    }

    async fn getServiceProperties(
        &self,
        _context: &Context,
        account: &str,
    ) -> Result<Option<ServicePropertiesModel>, StorageError> {
        let services = self.services_collection.read().unwrap();
        Ok(services.get(account).cloned())
    }

    // ─── Container Operations ───────────────────────────────────────────────

    async fn listContainers(
        &self,
        context: &Context,
        account: &str,
        prefix: Option<&str>,
        max_results: Option<i64>,
        marker: Option<&str>,
    ) -> Result<(Vec<ContainerModel>, Option<String>), StorageError> {
        let prefix = prefix.unwrap_or("");
        let max_results =
            max_results.unwrap_or(DEFAULT_LIST_CONTAINERS_MAX_RESULTS as i64) as usize;
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
        let mut result_docs: Vec<ContainerModel> =
            matching.into_iter().take(max_results + 1).collect();
        let next_marker = if result_docs.len() > max_results {
            let _ = result_docs.pop();
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
                    let mut syncer = ContainerLeaseSyncer::new(&mut doc);
                    let _ = syncer.sync(lease_state.lease());
                }
                doc
            })
            .collect();

        Ok((synced_docs, next_marker))
    }

    async fn createContainer(
        &self,
        context: &Context,
        container: ContainerModel,
    ) -> Result<ContainerModel, StorageError> {
        let mut containers = self.containers_collection.write().unwrap();

        let key = (
            container.accountName.clone(),
            container.name.clone().unwrap_or_default(),
        );

        if containers.contains_key(&key) {
            return Err(StorageErrorFactory::get_container_already_exists(
                context.contextId().as_deref(),
            ));
        }

        containers.insert(key, container.clone());
        Ok(container)
    }

    async fn getContainerProperties(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        lease_access_conditions: Option<&LeaseAccessConditions>,
    ) -> Result<GetContainerPropertiesResponse, StorageError> {
        let doc = self
            .get_container_with_lease_updated(account, container, context, true)
            .await?
            .ok_or_else(|| {
                StorageErrorFactory::get_container_not_found(context.contextId().as_deref())
            })?;

        let validator = ContainerReadLeaseValidator::new(lease_access_conditions.cloned());
        let adapter = ContainerLeaseAdapter::new(&doc);
        validator.validate(&adapter, context)?;

        let mut res = GetContainerPropertiesResponse::default();
        res.insert(
            "name".to_string(),
            GeneratedValue::String(container.to_string()),
        );
        res.insert(
            "properties".to_string(),
            GeneratedValue::Object(doc.properties.clone()),
        );
        if let Some(metadata) = &doc.metadata {
            let metadata_obj: GeneratedObject = metadata
                .iter()
                .map(|(key, value)| (key.clone(), GeneratedValue::String(value.clone())))
                .collect();
            res.insert("metadata".to_string(), GeneratedValue::Object(metadata_obj));
        }

        Ok(res)
    }

    async fn deleteContainer(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        options: Option<&ContainerDeleteMethodOptionalParams>,
    ) -> Result<(), StorageError> {
        let options = options.cloned().unwrap_or_default();

        let doc = self
            .get_container_with_lease_updated(account, container, context, false)
            .await?;

        let modifiedAccessConditions = get_object(&options, "modifiedAccessConditions");
        validate_write_conditions_container(
            context,
            modifiedAccessConditions.as_ref(),
            doc.as_ref(),
        )?;

        let doc = doc.ok_or_else(|| {
            StorageErrorFactory::get_container_not_found(context.contextId().as_deref())
        })?;

        let validator =
            ContainerDeleteLeaseValidator::new(get_object(&options, "leaseAccessConditions"));
        let adapter = ContainerLeaseAdapter::new(&doc);
        validator.validate(&adapter, context)?;

        let mut containers = self.containers_collection.write().unwrap();
        containers.remove(&(account.to_string(), container.to_string()));
        drop(containers);

        let mut blobs = self.blobs_collection.write().unwrap();
        blobs.retain(|(acc, cont, _, _), _| !(acc == account && cont == container));
        drop(blobs);

        let mut blocks = self.blocks_collection.write().unwrap();
        blocks.retain(|(acc, cont, _, _), _| !(acc == account && cont == container));
        drop(blocks);

        Ok(())
    }

    async fn setContainerMetadata(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        lastModified: DateTime<Utc>,
        etag: &str,
        metadata: Option<&IContainerMetadata>,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<(), StorageError> {
        let doc = self
            .get_container_with_lease_updated(account, container, context, false)
            .await?;

        validate_write_conditions_container(context, modifiedAccessConditions, doc.as_ref())?;

        let doc = doc.ok_or_else(|| {
            StorageErrorFactory::get_container_not_found(context.contextId().as_deref())
        })?;

        let validator = ContainerReadLeaseValidator::new(leaseAccessConditions.cloned());
        let adapter = ContainerLeaseAdapter::new(&doc);
        validator.validate(&adapter, context)?;

        // Update container
        let mut containers = self.containers_collection.write().unwrap();
        if let Some(container_doc) =
            containers.get_mut(&(account.to_string(), container.to_string()))
        {
            set_value(
                &mut container_doc.properties,
                "lastModified",
                Some(GeneratedValue::String(formatRfc1123(lastModified))),
            );
            set_string(
                &mut container_doc.properties,
                "etag",
                Some(etag.to_string()),
            );
            container_doc.metadata = metadata.cloned();
        }

        Ok(())
    }

    async fn getContainerACL(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
    ) -> Result<Option<GetContainerAccessPolicyResponse>, StorageError> {
        let doc = self
            .get_container_with_lease_updated(account, container, context, true)
            .await?
            .ok_or_else(|| {
                StorageErrorFactory::get_container_not_found(context.contextId().as_deref())
            })?;

        let validator = ContainerReadLeaseValidator::new(leaseAccessConditions.cloned());
        let adapter = ContainerLeaseAdapter::new(&doc);
        validator.validate(&adapter, context)?;

        Ok(Some(GetContainerAccessPolicyResponse {
            properties: doc.properties,
            containerAcl: doc.containerAcl,
        }))
    }

    async fn setContainerACL(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        set_acl_model: SetContainerAccessPolicyOptions,
    ) -> Result<(), StorageError> {
        let doc = self
            .get_container_with_lease_updated(account, container, context, false)
            .await?;

        validate_write_conditions_container(
            context,
            set_acl_model.modifiedAccessConditions.as_ref(),
            doc.as_ref(),
        )?;

        let doc = doc.ok_or_else(|| {
            StorageErrorFactory::get_container_not_found(context.contextId().as_deref())
        })?;

        let validator =
            ContainerReadLeaseValidator::new(set_acl_model.leaseAccessConditions.clone());
        let adapter = ContainerLeaseAdapter::new(&doc);
        validator.validate(&adapter, context)?;

        // Update container
        let mut containers = self.containers_collection.write().unwrap();
        if let Some(container_doc) =
            containers.get_mut(&(account.to_string(), container.to_string()))
        {
            set_string(
                &mut container_doc.properties,
                "publicAccess",
                set_acl_model.publicAccess.clone(),
            );
            container_doc.containerAcl = set_acl_model.containerAcl.clone();
            if let Some(last_modified) = set_acl_model.lastModified {
                set_value(
                    &mut container_doc.properties,
                    "lastModified",
                    Some(GeneratedValue::String(formatRfc1123(last_modified))),
                );
            }
            if let Some(etag) = set_acl_model.etag.clone() {
                set_string(&mut container_doc.properties, "etag", Some(etag));
            }
        }

        Ok(())
    }

    async fn acquireContainerLease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        options: Option<&ContainerAcquireLeaseOptionalParams>,
    ) -> Result<AcquireContainerLeaseResponse, StorageError> {
        let options = options.cloned().unwrap_or_default();
        let doc = self
            .get_container(account, container, context, false)
            .await?;

        let modifiedAccessConditions = get_object(&options, "modifiedAccessConditions");
        validate_write_conditions_container(
            context,
            modifiedAccessConditions.as_ref(),
            doc.as_ref(),
        )?;

        let mut doc = doc.ok_or_else(|| {
            StorageErrorFactory::get_container_not_found(context.contextId().as_deref())
        })?;

        let adapter = ContainerLeaseAdapter::new(&doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let proposed_lease_id = get_string(&options, "proposedLeaseId");
        let acquired_state = lease_state.acquire(
            get_i64(&options, "duration").unwrap_or(-1),
            proposed_lease_id.as_deref(),
        )?;
        let mut syncer = ContainerLeaseSyncer::new(&mut doc);
        let _ = syncer.sync(acquired_state.lease());

        let mut containers = self.containers_collection.write().unwrap();
        containers.insert((account.to_string(), container.to_string()), doc.clone());
        drop(containers);

        Ok(ContainerLeaseResponse {
            properties: doc.properties,
            leaseId: doc.leaseId,
            leaseTime: doc.leaseDurationSeconds,
        })
    }

    async fn releaseContainerLease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        leaseId: &str,
        options: Option<&ContainerReleaseLeaseOptionalParams>,
    ) -> Result<ReleaseContainerLeaseResponse, StorageError> {
        let doc = self
            .get_container(account, container, context, false)
            .await?;

        let modified_access_conditions =
            options.and_then(|o| get_object(o, "modifiedAccessConditions"));
        validate_write_conditions_container(
            context,
            modified_access_conditions.as_ref(),
            doc.as_ref(),
        )?;

        let mut doc = doc.ok_or_else(|| {
            StorageErrorFactory::get_container_not_found(context.contextId().as_deref())
        })?;

        let adapter = ContainerLeaseAdapter::new(&doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let released_state = lease_state.release(leaseId)?;
        let mut syncer = ContainerLeaseSyncer::new(&mut doc);
        let _ = syncer.sync(released_state.lease());

        // Update container
        let mut containers = self.containers_collection.write().unwrap();
        containers.insert((account.to_string(), container.to_string()), doc.clone());
        drop(containers);

        Ok(doc.properties)
    }

    async fn renewContainerLease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        leaseId: &str,
        options: Option<&ContainerRenewLeaseOptionalParams>,
    ) -> Result<RenewContainerLeaseResponse, StorageError> {
        let doc = self
            .get_container(account, container, context, false)
            .await?;

        let modified_access_conditions =
            options.and_then(|o| get_object(o, "modifiedAccessConditions"));
        validate_write_conditions_container(
            context,
            modified_access_conditions.as_ref(),
            doc.as_ref(),
        )?;

        let mut doc = doc.ok_or_else(|| {
            StorageErrorFactory::get_container_not_found(context.contextId().as_deref())
        })?;

        let adapter = ContainerLeaseAdapter::new(&doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let renewed_state = lease_state.renew(leaseId)?;
        let mut syncer = ContainerLeaseSyncer::new(&mut doc);
        let _ = syncer.sync(renewed_state.lease());

        // Update container
        let mut containers = self.containers_collection.write().unwrap();
        containers.insert((account.to_string(), container.to_string()), doc.clone());
        drop(containers);

        Ok(ContainerLeaseResponse {
            properties: doc.properties,
            leaseId: doc.leaseId,
            leaseTime: doc.leaseDurationSeconds,
        })
    }

    async fn breakContainerLease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        break_period: Option<i64>,
        options: Option<&ContainerBreakLeaseOptionalParams>,
    ) -> Result<BreakContainerLeaseResponse, StorageError> {
        let doc = self
            .get_container(account, container, context, false)
            .await?;

        let modified_access_conditions =
            options.and_then(|o| get_object(o, "modifiedAccessConditions"));
        validate_write_conditions_container(
            context,
            modified_access_conditions.as_ref(),
            doc.as_ref(),
        )?;

        let mut doc = doc.ok_or_else(|| {
            StorageErrorFactory::get_container_not_found(context.contextId().as_deref())
        })?;

        let adapter = ContainerLeaseAdapter::new(&doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let broken_state = lease_state.break_lease(break_period)?;
        let mut syncer = ContainerLeaseSyncer::new(&mut doc);
        let _ = syncer.sync(broken_state.lease());

        // Update container
        let mut containers = self.containers_collection.write().unwrap();
        containers.insert((account.to_string(), container.to_string()), doc.clone());
        drop(containers);

        Ok(ContainerLeaseResponse {
            properties: doc.properties,
            leaseId: doc.leaseId,
            leaseTime: doc.leaseBreakTime.map(|break_time| {
                let diff = break_time - context.startTime().unwrap_or_else(Utc::now);
                // TS uses Math.round(); num_milliseconds()/1000 with rounding matches
                ((diff.num_milliseconds() as f64 / 1000.0).round() as i64).max(0)
            }),
        })
    }

    async fn changeContainerLease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        leaseId: &str,
        proposed_leaseId: &str,
        options: Option<&ContainerChangeLeaseOptionalParams>,
    ) -> Result<ChangeContainerLeaseResponse, StorageError> {
        let doc = self
            .get_container(account, container, context, false)
            .await?;

        let modified_access_conditions =
            options.and_then(|o| get_object(o, "modifiedAccessConditions"));
        validate_write_conditions_container(
            context,
            modified_access_conditions.as_ref(),
            doc.as_ref(),
        )?;

        let mut doc = doc.ok_or_else(|| {
            StorageErrorFactory::get_container_not_found(context.contextId().as_deref())
        })?;

        let adapter = ContainerLeaseAdapter::new(&doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let changed_state = lease_state.change(leaseId, proposed_leaseId)?;
        let mut syncer = ContainerLeaseSyncer::new(&mut doc);
        let _ = syncer.sync(changed_state.lease());

        // Update container
        let mut containers = self.containers_collection.write().unwrap();
        containers.insert((account.to_string(), container.to_string()), doc.clone());
        drop(containers);

        Ok(ContainerLeaseResponse {
            properties: doc.properties,
            leaseId: doc.leaseId,
            leaseTime: doc.leaseDurationSeconds,
        })
    }

    async fn checkContainerExist(
        &self,
        context: &Context,
        account: &str,
        container: &str,
    ) -> Result<(), StorageError> {
        let containers = self.containers_collection.read().unwrap();
        if containers.contains_key(&(account.to_string(), container.to_string())) {
            Ok(())
        } else {
            Err(StorageErrorFactory::get_container_not_found(
                context.contextId().as_deref(),
            ))
        }
    }

    // ─── Blob Operations ────────────────────────────────────────────────────

    async fn filterBlobs(
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
        if where_clause.is_none() {
            return Ok((vec![], None));
        }
        let filter_fn = generate_query_blob_with_tags_where_function(context, where_clause, None)?;

        let blobs = self.blobs_collection.read().unwrap();
        let mut matching: Vec<FilterBlobModel> = blobs
            .iter()
            .filter_map(|((acc, cont, name, snap), blob)| {
                if acc != account || !snap.is_empty() || name.as_str() <= marker {
                    return None;
                }
                if let Some(expected_container) = container {
                    if cont != expected_container {
                        return None;
                    }
                }

                let candidate = FilterBlobModel {
                    name: name.clone(),
                    containerName: cont.clone(),
                    tags: blob.blobTags.clone(),
                };

                if where_clause.is_some() {
                    let matched_tags = filter_fn(&candidate);
                    if matched_tags.is_empty() {
                        return None;
                    }
                    // Return only the matched tags, excluding internal keys like @container
                    let tag_set: Vec<GeneratedValue> = matched_tags
                        .into_iter()
                        .filter(|tc| !tc.key.as_ref().is_some_and(|k| k.starts_with('@')))
                        .map(|tc| {
                            let mut tag = GeneratedObject::new();
                            if let Some(k) = tc.key {
                                tag.insert("key".into(), GeneratedValue::String(k));
                            }
                            if let Some(v) = tc.value {
                                tag.insert("value".into(), GeneratedValue::String(v));
                            }
                            GeneratedValue::Object(tag)
                        })
                        .collect();
                    let mut tags_obj = GeneratedObject::new();
                    tags_obj.insert("blobTagSet".into(), GeneratedValue::Array(tag_set));
                    return Some(FilterBlobModel {
                        name: name.clone(),
                        containerName: cont.clone(),
                        tags: Some(tags_obj),
                    });
                }

                Some(candidate)
            })
            .collect();
        drop(blobs);

        matching.sort_by(|a, b| a.name.cmp(&b.name));

        let mut result_docs: Vec<FilterBlobModel> =
            matching.into_iter().take(max_results + 1).collect();
        let next_marker = if result_docs.len() > max_results {
            result_docs.pop();
            result_docs.last().map(|doc| doc.name.clone())
        } else {
            None
        };

        Ok((result_docs, next_marker))
    }

    async fn listBlobs(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        delimiter: Option<&str>,
        _blob: Option<&str>,
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

        if let Some(delim) = delimiter {
            let matching: Vec<BlobModel> = {
                let blobs = self.blobs_collection.read().unwrap();
                let mut items: Vec<BlobModel> = blobs
                    .iter()
                    .filter(|((acc, cont, name, snap), blob)| {
                        acc == account
                            && cont == container
                            && name.starts_with(prefix)
                            && name.as_str() > marker
                            && (include_snapshots || snap.is_empty())
                            && (include_uncommitted_blobs || blob.isCommitted != Some(false))
                    })
                    .map(|(_, blob)| blob.clone())
                    .collect();
                items.sort_by(|a, b| a.name.cmp(&b.name));
                items
            };

            let mut page = PageWithDelimiter::<BlobModel>::new(
                max_results,
                Some(delim.to_string()),
                Some(prefix.to_string()),
            );
            let matching_for_page = matching.clone();
            let (items, prefixes, next_marker) = page
                .fill(
                    move |offset| {
                        let docs = matching_for_page.clone();
                        async move { docs.into_iter().skip(offset).collect() }
                    },
                    |blob| blob.name.clone().unwrap_or_default(),
                )
                .await;
            let next_marker = if next_marker.is_empty() {
                None
            } else {
                Some(next_marker)
            };
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
                        && (include_uncommitted_blobs || blob.isCommitted != Some(false))
                })
                .map(|(_, blob)| blob.clone())
                .collect();

            // Sort by name
            matching.sort_by(|a, b| a.name.cmp(&b.name));

            drop(blobs);

            // Apply pagination
            let mut result_docs: Vec<BlobModel> =
                matching.into_iter().take(max_results + 1).collect();
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
                        if let Ok(lease_state) = LeaseFactory::create_lease_state(&adapter, context)
                        {
                            let mut syncer = BlobLeaseSyncer::new(&mut doc);
                            let _ = syncer.sync(lease_state.lease());
                        }
                    }
                    doc
                })
                .collect();

            Ok((synced_docs, vec![], next_marker))
        }
    }

    async fn listAllBlobs(
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
                full_path.as_str() > marker
                    && (include_snapshots || snap.is_empty())
                    && (include_uncommitted_blobs || blob.isCommitted != Some(false))
            })
            .map(|(_, blob)| blob.clone())
            .collect();

        // Sort by full path
        matching.sort_by(|a, b| {
            let path_a = format!(
                "{}/{}/{}",
                a.accountName,
                a.containerName,
                a.name.as_ref().unwrap_or(&String::new())
            );
            let path_b = format!(
                "{}/{}/{}",
                b.accountName,
                b.containerName,
                b.name.as_ref().unwrap_or(&String::new())
            );
            path_a.cmp(&path_b)
        });

        drop(blobs);

        // Apply pagination
        let mut result_docs: Vec<BlobModel> = matching.into_iter().take(max_results + 1).collect();
        let next_marker = if result_docs.len() > max_results {
            let last = result_docs.pop().unwrap();
            Some(format!(
                "{}/{}/{}",
                last.accountName,
                last.containerName,
                last.name.unwrap_or_default()
            ))
        } else {
            None
        };

        Ok((result_docs, next_marker))
    }

    async fn createBlob(
        &self,
        context: &Context,
        blob: BlobModel,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<(), StorageError> {
        // Check if container exists
        self.checkContainerExist(context, &blob.accountName, &blob.containerName)
            .await?;

        let snapshot = blob.snapshot.clone().unwrap_or_default();
        let blob_name = blob.name.clone().unwrap_or_default();

        // Check if blob exists
        let existing = self
            .get_blob_with_lease_updated(
                &blob.accountName,
                &blob.containerName,
                &blob_name,
                &snapshot,
                context,
                true, // forceCommitted
            )
            .await?;

        // Validate write conditions
        validate_write_conditions(context, modifiedAccessConditions, existing.as_ref())?;

        // ifNoneMatch=* means "only create if blob does not exist" → 409 if it does
        if existing.is_some() {
            if let Some(mac) = modifiedAccessConditions {
                if get_string(mac, "ifNoneMatch").as_deref() == Some("*") {
                    return Err(StorageErrorFactory::getBlobAlreadyExists(
                        context.contextId().as_deref(),
                    ));
                }
            }
        }

        // Validate lease conditions and sync lease state into new blob
        let mut blob = blob;
        if let Some(existing_blob) = &existing {
            let validator = BlobWriteLeaseValidator::new(lease_access_conditions.cloned());
            let adapter = BlobLeaseAdapter::new(existing_blob);
            validator.validate(&adapter, context)?;

            // Reject overwriting archived blobs (matching TS behavior)
            if get_string(&existing_blob.properties, "accessTier").as_deref() == Some("Archive") {
                return Err(StorageErrorFactory::get_blob_archived(
                    context.contextId().as_deref(),
                ));
            }

            // Sync lease from existing blob into the new blob (BlobWriteLeaseSyncer).
            // If the lease is expired or broken, reset to available; otherwise preserve it.
            let lease_state = adapter.leaseState.as_deref().unwrap_or("available");
            if lease_state == "expired" || lease_state == "broken" {
                blob.properties.insert(
                    "leaseState".to_string(),
                    GeneratedValue::String("available".to_string()),
                );
                blob.properties.insert(
                    "leaseStatus".to_string(),
                    GeneratedValue::String("unlocked".to_string()),
                );
                blob.properties.remove("leaseDuration");
                blob.leaseId = None;
                blob.leaseExpireTime = None;
                blob.leaseDurationSeconds = None;
                blob.leaseBreakTime = None;
            } else {
                blob.leaseId = adapter.leaseId.clone();
                blob.leaseExpireTime = adapter.leaseExpireTime;
                blob.leaseDurationSeconds = adapter.leaseDurationSeconds;
                blob.leaseBreakTime = adapter.leaseBreakTime;
                if let Some(dur) = &adapter.leaseDurationType {
                    blob.properties.insert(
                        "leaseDuration".to_string(),
                        GeneratedValue::String(dur.clone()),
                    );
                }
                if let Some(state) = &adapter.leaseState {
                    blob.properties.insert(
                        "leaseState".to_string(),
                        GeneratedValue::String(state.clone()),
                    );
                }
                if let Some(status) = &adapter.leaseStatus {
                    blob.properties.insert(
                        "leaseStatus".to_string(),
                        GeneratedValue::String(status.clone()),
                    );
                }
            }
        }

        // Insert blob
        let mut blobs = self.blobs_collection.write().unwrap();
        let key = (
            blob.accountName.clone(),
            blob.containerName.clone(),
            blob_name.clone(),
            snapshot.clone(),
        );
        blobs.insert(key, blob.clone());

        Ok(())
    }

    async fn createSnapshot(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        metadata: Option<&BlobMetadata>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<CreateSnapshotResponse, StorageError> {
        let base_blob = self
            .get_blob_with_lease_updated(account, container, blob, "", context, false)
            .await?
            .ok_or_else(|| {
                StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
            })?;

        validate_write_conditions(context, modifiedAccessConditions, Some(&base_blob))?;

        let validator = BlobReadLeaseValidator::new(lease_access_conditions.cloned());
        let adapter = BlobLeaseAdapter::new(&base_blob);
        validator.validate(&adapter, context)?;

        // Generate snapshot timestamp — must use Z suffix (not +00:00) to match Azure format
        let start = context.startTime().unwrap();
        let ms_str = start.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let snapshot_time = convert_date_time_string_ms_to_7_digital(&ms_str);

        // Create snapshot blob
        let mut snapshot_blob = base_blob.clone();
        snapshot_blob.snapshot = Some(snapshot_time.clone());
        if let Some(meta) = metadata {
            snapshot_blob.metadata = Some(meta.clone());
        }

        // Clear lease fields on snapshot (TS: BlobLeaseSyncer with all undefined)
        snapshot_blob.leaseId = None;
        snapshot_blob.leaseExpireTime = None;
        snapshot_blob.leaseDurationSeconds = None;
        snapshot_blob.leaseBreakTime = None;
        snapshot_blob.properties.remove("leaseDuration");
        snapshot_blob.properties.remove("leaseState");
        snapshot_blob.properties.remove("leaseStatus");

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

    async fn downloadBlob(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<BlobModel, StorageError> {
        let snapshot = snapshot.unwrap_or("");

        let blob_doc = self
            .get_blob_with_lease_updated(account, container, blob, snapshot, context, true)
            .await?;

        // TS validates read conditions before checking blob existence,
        // so conditional header errors (412) take precedence over 404.
        validate_read_conditions(
            context,
            modifiedAccessConditions,
            blob_doc.as_ref(),
            Some(false),
        )?;

        let blob_doc = blob_doc.ok_or_else(|| {
            StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
        })?;

        let validator = BlobReadLeaseValidator::new(lease_access_conditions.cloned());
        let adapter = BlobLeaseAdapter::new(&blob_doc);
        validator.validate(&adapter, context)?;

        Ok(blob_doc)
    }

    async fn getBlobProperties(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<GetBlobPropertiesRes, StorageError> {
        let snapshot = snapshot.unwrap_or("");

        let blob_doc = self
            .get_blob_with_lease_updated(account, container, blob, snapshot, context, true)
            .await?;

        // TS validates read conditions before checking blob existence,
        // so conditional header errors (412) take precedence over 404.
        validate_read_conditions(
            context,
            modifiedAccessConditions,
            blob_doc.as_ref(),
            Some(false),
        )?;

        let blob_doc = blob_doc.ok_or_else(|| {
            StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
        })?;

        let validator = BlobReadLeaseValidator::new(lease_access_conditions.cloned());
        let adapter = BlobLeaseAdapter::new(&blob_doc);
        validator.validate(&adapter, context)?;

        let blobCommittedBlockCount = if get_string(&blob_doc.properties, "blobType").as_deref()
            == Some(BLOB_TYPE_APPEND_BLOB)
        {
            blob_doc
                .committedBlocksInOrder
                .as_ref()
                .map(|blocks| blocks.len() as i64)
        } else {
            None
        };

        Ok(GetBlobPropertiesRes {
            properties: blob_doc.properties,
            metadata: blob_doc.metadata,
            blobCommittedBlockCount,
            blobTags: blob_doc.blobTags,
        })
    }

    async fn deleteBlob(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        options: BlobDeleteMethodOptionalParams,
    ) -> Result<(), StorageError> {
        let snapshot_value = get_string(&options, "snapshot");
        let snapshot = snapshot_value.as_deref().unwrap_or("");

        self.checkContainerExist(context, account, container)
            .await?;

        let blob_doc = self
            .get_blob_with_lease_updated(account, container, blob, snapshot, context, false)
            .await?;

        let modified_access_conditions = get_object(&options, "modifiedAccessConditions");
        validate_write_conditions(
            context,
            modified_access_conditions.as_ref(),
            blob_doc.as_ref(),
        )?;

        let blob_doc = blob_doc.ok_or_else(|| {
            StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
        })?;

        let against_base_blob = snapshot.is_empty();
        let delete_snapshots = get_string(&options, "deleteSnapshots");

        // Check bad requests: deleteSnapshots header on a snapshot target
        if !against_base_blob && delete_snapshots.is_some() {
            return Err(StorageErrorFactory::getInvalidOperation(
                context.contextId().as_deref(),
                Some("Invalid operation against a blob snapshot."),
            ));
        }

        let validator = BlobWriteLeaseValidator::new(get_object(&options, "leaseAccessConditions"));
        let adapter = BlobLeaseAdapter::new(&blob_doc);
        validator.validate(&adapter, context)?;

        let mut blobs = self.blobs_collection.write().unwrap();

        // Scenario: Delete base blob only (no deleteSnapshots header)
        if against_base_blob && delete_snapshots.is_none() {
            let count = blobs
                .keys()
                .filter(|(a, c, n, _)| a == account && c == container && n == blob)
                .count();
            if count > 1 {
                return Err(StorageErrorFactory::getSnapshotsPresent(
                    context.contextId().as_deref().unwrap_or(""),
                ));
            }
            blobs.remove(&(
                account.to_string(),
                container.to_string(),
                blob.to_string(),
                "".to_string(),
            ));
        }

        // Scenario: Delete one snapshot only
        if !against_base_blob {
            blobs.remove(&(
                account.to_string(),
                container.to_string(),
                blob.to_string(),
                snapshot.to_string(),
            ));
        }

        // Scenario: Delete base blob and all snapshots
        if against_base_blob && delete_snapshots.as_deref() == Some("include") {
            blobs.retain(|k, _| !(k.0 == account && k.1 == container && k.2 == blob));
        }

        // Scenario: Delete all snapshots only (keep base blob)
        if against_base_blob && delete_snapshots.as_deref() == Some("only") {
            blobs.retain(|k, _| {
                !(k.0 == account && k.1 == container && k.2 == blob && !k.3.is_empty())
            });
        }

        Ok(())
    }

    async fn setBlobHTTPHeaders(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        blob_http_headers: Option<&BlobHTTPHeaders>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        let mut blob_doc = self
            .get_blob_with_lease_updated(account, container, blob, "", context, true)
            .await?
            .ok_or_else(|| {
                StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
            })?;

        validate_write_conditions(context, modifiedAccessConditions, Some(&blob_doc))?;

        let validator = BlobWriteLeaseValidator::new(lease_access_conditions.cloned());
        let adapter = BlobLeaseAdapter::new(&blob_doc);
        validator.validate(&adapter, context)?;

        if let Some(headers) = blob_http_headers {
            set_string(
                &mut blob_doc.properties,
                "cacheControl",
                get_string(headers, "blobCacheControl"),
            );
            set_string(
                &mut blob_doc.properties,
                "contentType",
                get_string(headers, "blobContentType"),
            );
            set_string(
                &mut blob_doc.properties,
                "contentMD5",
                get_string(headers, "blobContentMD5"),
            );
            set_string(
                &mut blob_doc.properties,
                "contentEncoding",
                get_string(headers, "blobContentEncoding"),
            );
            set_string(
                &mut blob_doc.properties,
                "contentLanguage",
                get_string(headers, "blobContentLanguage"),
            );
            set_string(
                &mut blob_doc.properties,
                "contentDisposition",
                get_string(headers, "blobContentDisposition"),
            );
        }

        // Update etag and lastModified (matching TS behavior)
        set_string(&mut blob_doc.properties, "etag", Some(new_etag()));
        let last_modified = context.startTime().unwrap_or_else(Utc::now);
        set_string(
            &mut blob_doc.properties,
            "lastModified",
            Some(formatRfc1123(last_modified)),
        );

        let adapter = BlobLeaseAdapter::new(&blob_doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let mut lease_syncer = BlobWriteLeaseSyncer::new(&mut blob_doc);
        let _ = lease_syncer.sync(lease_state.lease());

        // Update blob
        let mut blobs = self.blobs_collection.write().unwrap();
        blobs.insert(
            (
                account.to_string(),
                container.to_string(),
                blob.to_string(),
                "".to_string(),
            ),
            blob_doc.clone(),
        );

        Ok(blob_doc.properties)
    }

    async fn setBlobMetadata(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        metadata: Option<&BlobMetadata>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        let mut blob_doc = self
            .get_blob_with_lease_updated(account, container, blob, "", context, true)
            .await?
            .ok_or_else(|| {
                StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
            })?;

        validate_write_conditions(context, modifiedAccessConditions, Some(&blob_doc))?;

        let validator = BlobWriteLeaseValidator::new(lease_access_conditions.cloned());
        let adapter = BlobLeaseAdapter::new(&blob_doc);
        validator.validate(&adapter, context)?;

        // Update metadata
        blob_doc.metadata = metadata.cloned();

        // Update etag and lastModified (matching TS behavior)
        set_string(&mut blob_doc.properties, "etag", Some(new_etag()));
        let last_modified = context.startTime().unwrap_or_else(Utc::now);
        set_string(
            &mut blob_doc.properties,
            "lastModified",
            Some(formatRfc1123(last_modified)),
        );

        let adapter = BlobLeaseAdapter::new(&blob_doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let mut lease_syncer = BlobWriteLeaseSyncer::new(&mut blob_doc);
        let _ = lease_syncer.sync(lease_state.lease());

        // Update blob
        let mut blobs = self.blobs_collection.write().unwrap();
        blobs.insert(
            (
                account.to_string(),
                container.to_string(),
                blob.to_string(),
                "".to_string(),
            ),
            blob_doc.clone(),
        );

        Ok(blob_doc.properties)
    }

    // ─── Blob Leases ────────────────────────────────────────────────────────

    async fn acquireBlobLease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        duration: i64,
        proposed_leaseId: Option<&str>,
        options: Option<&BlobAcquireLeaseOptionalParams>,
    ) -> Result<AcquireBlobLeaseResponse, StorageError> {
        let options = options.cloned().unwrap_or_default();

        let snapshot = get_string(&options, "snapshot");
        if snapshot.as_deref().is_some_and(|value| !value.is_empty()) {
            return Err(
                StorageErrorFactory::get_blob_snapshot_operation_not_supported(
                    context.contextId().as_deref(),
                ),
            );
        }

        self.checkContainerExist(context, account, container)
            .await?;

        let blob_doc = self
            .get_blob_with_lease_updated(account, container, blob, "", context, false)
            .await?;

        let modified_access_conditions = get_object(&options, "modifiedAccessConditions");
        validate_write_conditions(
            context,
            modified_access_conditions.as_ref(),
            blob_doc.as_ref(),
        )?;

        let mut blob_doc = blob_doc.ok_or_else(|| {
            StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
        })?;

        let adapter = BlobLeaseAdapter::new(&blob_doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let acquired_state = lease_state.acquire(duration, proposed_leaseId)?;
        let mut syncer = BlobLeaseSyncer::new(&mut blob_doc);
        let _ = syncer.sync(acquired_state.lease());

        // Update blob
        let mut blobs = self.blobs_collection.write().unwrap();
        blobs.insert(
            (
                account.to_string(),
                container.to_string(),
                blob.to_string(),
                "".to_string(),
            ),
            blob_doc.clone(),
        );

        Ok(BlobLeaseResponse {
            properties: blob_doc.properties,
            leaseId: blob_doc.leaseId,
            leaseTime: blob_doc.leaseDurationSeconds,
        })
    }

    async fn releaseBlobLease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        leaseId: &str,
        options: Option<&BlobReleaseLeaseOptionalParams>,
    ) -> Result<ReleaseBlobLeaseResponse, StorageError> {
        self.checkContainerExist(context, account, container)
            .await?;

        let blob_doc = self
            .get_blob_with_lease_updated(account, container, blob, "", context, false)
            .await?;

        let modified_access_conditions =
            options.and_then(|o| get_object(o, "modifiedAccessConditions"));
        validate_write_conditions(
            context,
            modified_access_conditions.as_ref(),
            blob_doc.as_ref(),
        )?;

        let mut blob_doc = blob_doc.ok_or_else(|| {
            StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
        })?;

        let adapter = BlobLeaseAdapter::new(&blob_doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let released_state = lease_state.release(leaseId)?;
        let mut syncer = BlobLeaseSyncer::new(&mut blob_doc);
        let _ = syncer.sync(released_state.lease());

        // Update blob
        let mut blobs = self.blobs_collection.write().unwrap();
        blobs.insert(
            (
                account.to_string(),
                container.to_string(),
                blob.to_string(),
                "".to_string(),
            ),
            blob_doc.clone(),
        );

        Ok(blob_doc.properties)
    }

    async fn renewBlobLease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        leaseId: &str,
        options: Option<&BlobRenewLeaseOptionalParams>,
    ) -> Result<RenewBlobLeaseResponse, StorageError> {
        self.checkContainerExist(context, account, container)
            .await?;

        let blob_doc = self
            .get_blob_with_lease_updated(account, container, blob, "", context, false)
            .await?;

        let modified_access_conditions =
            options.and_then(|o| get_object(o, "modifiedAccessConditions"));
        validate_write_conditions(
            context,
            modified_access_conditions.as_ref(),
            blob_doc.as_ref(),
        )?;

        let mut blob_doc = blob_doc.ok_or_else(|| {
            StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
        })?;

        let adapter = BlobLeaseAdapter::new(&blob_doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let renewed_state = lease_state.renew(leaseId)?;
        let mut syncer = BlobLeaseSyncer::new(&mut blob_doc);
        let _ = syncer.sync(renewed_state.lease());

        // Update blob
        let mut blobs = self.blobs_collection.write().unwrap();
        blobs.insert(
            (
                account.to_string(),
                container.to_string(),
                blob.to_string(),
                "".to_string(),
            ),
            blob_doc.clone(),
        );

        Ok(BlobLeaseResponse {
            properties: blob_doc.properties,
            leaseId: blob_doc.leaseId,
            leaseTime: blob_doc.leaseDurationSeconds,
        })
    }

    async fn changeBlobLease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        leaseId: &str,
        proposed_leaseId: &str,
        options: Option<&BlobChangeLeaseOptionalParams>,
    ) -> Result<ChangeBlobLeaseResponse, StorageError> {
        self.checkContainerExist(context, account, container)
            .await?;

        let blob_doc = self
            .get_blob_with_lease_updated(account, container, blob, "", context, false)
            .await?;

        let modified_access_conditions =
            options.and_then(|o| get_object(o, "modifiedAccessConditions"));
        validate_write_conditions(
            context,
            modified_access_conditions.as_ref(),
            blob_doc.as_ref(),
        )?;

        let mut blob_doc = blob_doc.ok_or_else(|| {
            StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
        })?;

        let adapter = BlobLeaseAdapter::new(&blob_doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let changed_state = lease_state.change(leaseId, proposed_leaseId)?;
        let mut syncer = BlobLeaseSyncer::new(&mut blob_doc);
        let _ = syncer.sync(changed_state.lease());

        // Update blob
        let mut blobs = self.blobs_collection.write().unwrap();
        blobs.insert(
            (
                account.to_string(),
                container.to_string(),
                blob.to_string(),
                "".to_string(),
            ),
            blob_doc.clone(),
        );

        Ok(BlobLeaseResponse {
            properties: blob_doc.properties,
            leaseId: blob_doc.leaseId,
            leaseTime: blob_doc.leaseDurationSeconds,
        })
    }

    async fn breakBlobLease(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        break_period: Option<i64>,
        options: Option<&BlobBreakLeaseOptionalParams>,
    ) -> Result<BreakBlobLeaseResponse, StorageError> {
        self.checkContainerExist(context, account, container)
            .await?;

        let blob_doc = self
            .get_blob_with_lease_updated(account, container, blob, "", context, false)
            .await?;

        let modified_access_conditions =
            options.and_then(|o| get_object(o, "modifiedAccessConditions"));
        validate_write_conditions(
            context,
            modified_access_conditions.as_ref(),
            blob_doc.as_ref(),
        )?;

        let mut blob_doc = blob_doc.ok_or_else(|| {
            StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
        })?;

        let adapter = BlobLeaseAdapter::new(&blob_doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let broken_state = lease_state.break_lease(break_period)?;
        let mut syncer = BlobLeaseSyncer::new(&mut blob_doc);
        let _ = syncer.sync(broken_state.lease());

        // Update blob
        let mut blobs = self.blobs_collection.write().unwrap();
        blobs.insert(
            (
                account.to_string(),
                container.to_string(),
                blob.to_string(),
                "".to_string(),
            ),
            blob_doc.clone(),
        );

        Ok(BlobLeaseResponse {
            properties: blob_doc.properties,
            leaseId: blob_doc.leaseId,
            leaseTime: Some(
                blob_doc
                    .leaseBreakTime
                    .map(|break_time| {
                        let diff = break_time - context.startTime().unwrap_or_else(Utc::now);
                        // TS uses Math.round(); num_milliseconds()/1000 with rounding matches
                        ((diff.num_milliseconds() as f64 / 1000.0).round() as i64).max(0)
                    })
                    .unwrap_or(0),
            ),
        })
    }

    async fn checkBlobExist(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
    ) -> Result<(), StorageError> {
        let snapshot = snapshot.unwrap_or("");
        let blobs = self.blobs_collection.read().unwrap();
        if blobs.contains_key(&(
            account.to_string(),
            container.to_string(),
            blob.to_string(),
            snapshot.to_string(),
        )) {
            Ok(())
        } else {
            Err(StorageErrorFactory::get_blob_not_found(
                context.contextId().as_deref(),
            ))
        }
    }

    async fn getBlobType(
        &self,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
    ) -> Result<Option<BlobTypeResult>, StorageError> {
        let snapshot = snapshot.unwrap_or("");
        let key = (
            account.to_string(),
            container.to_string(),
            blob.to_string(),
            snapshot.to_string(),
        );
        let blobs = self.blobs_collection.read().unwrap();
        match blobs.get(&key) {
            Some(doc) => Ok(Some(BlobTypeResult {
                blobType: get_string(&doc.properties, "blobType"),
                isCommitted: doc.isCommitted.unwrap_or(true),
            })),
            None => Ok(None),
        }
    }

    async fn startCopyFromURL(
        &self,
        context: &Context,
        source: BlobId,
        destination: BlobId,
        copySource: &str,
        metadata: Option<&BlobMetadata>,
        tier: Option<&str>,
        leaseAccessConditions: Option<&BlobStartCopyFromURLOptionalParams>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        // TS: lines 1860-2047
        let options = leaseAccessConditions.cloned().unwrap_or_default();

        // Get source blob with lease updated
        let source_blob = self
            .get_blob_with_lease_updated(
                &source.account,
                &source.container,
                &source.blob,
                &source.snapshot.clone().unwrap_or_default(),
                context,
                true, // forceCommitted
            )
            .await?;

        let source_modified_access_conditions =
            get_object(&options, "sourceModifiedAccessConditions");
        let remapped_source_conditions = source_modified_access_conditions
            .as_ref()
            .map(remap_source_conditions);
        validate_read_conditions(
            context,
            remapped_source_conditions.as_ref(),
            source_blob.as_ref(),
            Some(true),
        )?;

        let source_blob = source_blob.ok_or_else(|| {
            StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
        })?;

        // Get destination blob (may not exist)
        let dest_blob = self
            .get_blob_with_lease_updated(
                &destination.account,
                &destination.container,
                &destination.blob,
                "",
                context,
                false,
            )
            .await?;

        let modified_access_conditions = get_object(&options, "modifiedAccessConditions");
        validate_write_conditions(
            context,
            modified_access_conditions.as_ref(),
            dest_blob.as_ref(),
        )?;

        if let Some(mac) = modified_access_conditions.as_ref() {
            if get_string(mac, "ifNoneMatch").as_deref() == Some("*") && dest_blob.is_some() {
                return Err(StorageErrorFactory::get_blob_already_exists(
                    context.contextId().as_deref(),
                ));
            }
        }

        if let Some(dest) = dest_blob.as_ref() {
            let validator =
                BlobWriteLeaseValidator::new(get_object(&options, "leaseAccessConditions"));
            let adapter = BlobLeaseAdapter::new(dest);
            validator.validate(&adapter, context)?;
        }

        // Validate source is committed, not deleted
        if source_blob.deleted == Some(true) || source_blob.isCommitted == Some(false) {
            return Err(StorageErrorFactory::get_blob_not_found(
                context.contextId().as_deref(),
            ));
        }

        if get_string(&source_blob.properties, "accessTier").as_deref() == Some(ACCESS_TIER_ARCHIVE)
            && (tier.is_none() || source.account != destination.account)
        {
            return Err(StorageErrorFactory::get_blob_archived(
                context.contextId().as_deref(),
            ));
        }

        // Check container exists
        self.checkContainerExist(context, &destination.account, &destination.container)
            .await?;

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
            blobTags: match get_string(&options, "blobTagsString") {
                Some(tags) => get_tags_from_string(&tags, &context_id(context))?,
                None => None,
            },
        };

        apply_copy_properties(
            &mut copied_blob.properties,
            context,
            &source_blob.properties,
            dest_blob.as_ref(),
            copySource,
        );

        if get_string(&source_blob.properties, "blobType").as_deref() == Some(BLOB_TYPE_APPEND_BLOB)
        {
            set_bool(
                &mut copied_blob.properties,
                "isSealed",
                get_bool(&options, "sealBlob"),
            );
        }

        if get_string(&copied_blob.properties, "blobType").as_deref() == Some(BLOB_TYPE_BLOCK_BLOB)
        {
            if let Some(tier_value) = tier {
                let parsed_tier = Self::parse_tier(Some(tier_value));
                if parsed_tier.is_none() {
                    return Err(invalid_header_value(
                        context.contextId().as_deref(),
                        "x-ms-access-tier",
                        tier_value,
                    ));
                }
                set_string(&mut copied_blob.properties, "accessTier", parsed_tier);
            }
        }

        if get_string(&copied_blob.properties, "blobType").as_deref() == Some(BLOB_TYPE_PAGE_BLOB)
            && tier.is_some()
        {
            return Err(invalid_header_value(
                context.contextId().as_deref(),
                "x-ms-access-tier",
                tier.unwrap_or_default(),
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

    async fn copyFromURL(
        &self,
        context: &Context,
        source: BlobId,
        destination: BlobId,
        copySource: &str,
        metadata: Option<&BlobMetadata>,
        tier: Option<&str>,
        leaseAccessConditions: Option<&BlobCopyFromURLOptionalParams>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        // TS: lines 2049-2236
        let options = leaseAccessConditions.cloned().unwrap_or_default();

        // Get source blob
        let source_blob = self
            .get_blob_with_lease_updated(
                &source.account,
                &source.container,
                &source.blob,
                &source.snapshot.clone().unwrap_or_default(),
                context,
                true,
            )
            .await?;

        let source_modified_access_conditions =
            get_object(&options, "sourceModifiedAccessConditions");
        // Remap sourceIf* → if* but skip ifTags for copyFromURL (TS behavior)
        let remapped_source_conditions = source_modified_access_conditions.as_ref().map(|sc| {
            let mut remapped = remap_source_conditions(sc);
            remapped.remove("ifTags"); // Storage service ignores x-ms-source-if-tags for copyFromUrl
            remapped
        });
        validate_read_conditions(
            context,
            remapped_source_conditions.as_ref(),
            source_blob.as_ref(),
            Some(false),
        )?;

        let source_blob = source_blob.ok_or_else(|| {
            StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
        })?;

        // Get destination blob
        let dest_blob = self
            .get_blob_with_lease_updated(
                &destination.account,
                &destination.container,
                &destination.blob,
                "",
                context,
                false,
            )
            .await?;

        let modified_access_conditions = get_object(&options, "modifiedAccessConditions");
        validate_write_conditions(
            context,
            modified_access_conditions.as_ref(),
            dest_blob.as_ref(),
        )?;

        if let Some(mac) = modified_access_conditions.as_ref() {
            if get_string(mac, "ifNoneMatch").as_deref() == Some("*") && dest_blob.is_some() {
                return Err(StorageErrorFactory::get_blob_already_exists(
                    context.contextId().as_deref(),
                ));
            }
        }

        if let Some(dest) = dest_blob.as_ref() {
            let validator =
                BlobWriteLeaseValidator::new(get_object(&options, "leaseAccessConditions"));
            let lease_adapter = BlobLeaseAdapter::new(dest);
            validator.validate(&lease_adapter, context)?;
        }

        // Validate source is committed, not deleted, not archived
        if source_blob.deleted == Some(true) || source_blob.isCommitted == Some(false) {
            return Err(StorageErrorFactory::get_blob_not_found(
                context.contextId().as_deref(),
            ));
        }

        if get_string(&source_blob.properties, "accessTier").as_deref() == Some(ACCESS_TIER_ARCHIVE)
        {
            return Err(StorageErrorFactory::get_blob_archived(
                context.contextId().as_deref(),
            ));
        }

        // Check container exists
        self.checkContainerExist(context, &destination.account, &destination.container)
            .await?;

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

        if get_string(&options, "copySourceTags").as_deref() == Some("COPY") {
            copied_blob.blobTags = source_blob.blobTags.clone();
        } else if let Some(tags_str) = get_string(&options, "blobTagsString") {
            copied_blob.blobTags = get_tags_from_string(&tags_str, &context_id(context))?;
        }

        apply_copy_properties(
            &mut copied_blob.properties,
            context,
            &source_blob.properties,
            dest_blob.as_ref(),
            copySource,
        );

        if get_string(&copied_blob.properties, "blobType").as_deref() == Some(BLOB_TYPE_BLOCK_BLOB)
        {
            if let Some(tier_value) = tier {
                let parsed_tier = Self::parse_tier(Some(tier_value));
                if parsed_tier.is_none() {
                    return Err(invalid_header_value(
                        context.contextId().as_deref(),
                        "x-ms-access-tier",
                        tier_value,
                    ));
                }
                set_string(&mut copied_blob.properties, "accessTier", parsed_tier);
            }
        }

        if get_string(&copied_blob.properties, "blobType").as_deref() == Some(BLOB_TYPE_PAGE_BLOB)
            && tier.is_some()
        {
            return Err(invalid_header_value(
                context.contextId().as_deref(),
                "x-ms-access-tier",
                tier.unwrap_or_default(),
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

    async fn setTier(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        tier: &str,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
    ) -> Result<u16, StorageError> {
        self.checkContainerExist(context, account, container)
            .await?;

        let mut doc = self
            .get_blob_with_lease_updated(account, container, blob, "", context, true)
            .await?
            .ok_or_else(|| {
                StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
            })?;

        let validator = BlobWriteLeaseValidator::new(leaseAccessConditions.cloned());
        let adapter = BlobLeaseAdapter::new(&doc);
        validator.validate(&adapter, context)?;

        if !doc.snapshot.as_deref().unwrap_or_default().is_empty() {
            return Err(
                StorageErrorFactory::get_blob_snapshot_operation_not_supported(
                    context.contextId().as_deref(),
                ),
            );
        }

        let response_code = if get_string(&doc.properties, "accessTier").as_deref()
            == Some(ACCESS_TIER_ARCHIVE)
            && matches!(tier, ACCESS_TIER_COOL | ACCESS_TIER_HOT | ACCESS_TIER_COLD)
        {
            202
        } else {
            200
        };

        if get_string(&doc.properties, "blobType").as_deref() != Some(BLOB_TYPE_BLOCK_BLOB) {
            return Err(StorageErrorFactory::getAccessTierNotSupportedForBlobType(
                context.contextId().as_deref().unwrap_or(""),
            ));
        }

        if !matches!(
            tier,
            ACCESS_TIER_ARCHIVE | ACCESS_TIER_COOL | ACCESS_TIER_HOT | ACCESS_TIER_COLD
        ) {
            return Err(invalid_header_value(
                context.contextId().as_deref(),
                "x-ms-access-tier",
                tier,
            ));
        }

        set_string(&mut doc.properties, "accessTier", Some(tier.to_string()));
        set_bool(&mut doc.properties, "accessTierInferred", Some(false));
        set_value(
            &mut doc.properties,
            "accessTierChangeTime",
            context
                .startTime()
                .map(|value| GeneratedValue::String(formatRfc1123(value))),
        );

        let adapter = BlobLeaseAdapter::new(&doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let mut syncer = BlobWriteLeaseSyncer::new(&mut doc);
        let _ = syncer.sync(lease_state.lease());

        let mut blobs = self.blobs_collection.write().unwrap();
        let key = (
            account.to_string(),
            container.to_string(),
            blob.to_string(),
            "".to_string(),
        );
        blobs.insert(key, doc);

        Ok(response_code)
    }

    async fn setBlobTag(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        tags: Option<&BlobTags>,
        _modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<(), StorageError> {
        // TS: lines 3419-3448
        // NOTE: TS ignores modifiedAccessConditions parameter (fidelity flag from line 3419)
        let snapshot = snapshot.unwrap_or("");

        self.checkContainerExist(context, account, container)
            .await?;

        let mut doc = self
            .get_blob_with_lease_updated(account, container, blob, snapshot, context, true)
            .await?
            .ok_or_else(|| {
                StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
            })?;

        // Validate lease
        let validator = BlobWriteLeaseValidator::new(lease_access_conditions.cloned());
        let adapter = BlobLeaseAdapter::new(&doc);
        validator.validate(&adapter, context)?;

        // Sync lease state
        let adapter = BlobLeaseAdapter::new(&doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let mut syncer = BlobWriteLeaseSyncer::new(&mut doc);
        let _ = syncer.sync(lease_state.lease());

        // Update blob tags
        doc.blobTags = tags.cloned();

        // Update collection
        let mut blobs = self.blobs_collection.write().unwrap();
        let key = (
            account.to_string(),
            container.to_string(),
            blob.to_string(),
            snapshot.to_string(),
        );
        blobs.insert(key, doc);
        drop(blobs);

        Ok(())
    }

    async fn getBlobTag(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<Option<BlobTags>, StorageError> {
        // TS: lines 3464-3503
        let snapshot = snapshot.unwrap_or("");

        self.checkContainerExist(context, account, container)
            .await?;

        let doc = self
            .get_blob_with_lease_updated(account, container, blob, snapshot, context, true)
            .await?;

        validate_read_conditions(context, modifiedAccessConditions, doc.as_ref(), Some(false))?;

        let doc = doc.ok_or_else(|| {
            StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
        })?;

        // Validate lease
        let validator = BlobReadLeaseValidator::new(lease_access_conditions.cloned());
        let adapter = BlobLeaseAdapter::new(&doc);
        validator.validate(&adapter, context)?;

        Ok(doc.blobTags)
    }

    async fn sealBlob(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        _snapshot: Option<&str>,
        options: AppendBlobSealOptionalParams,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        let mut doc = self
            .get_blob(account, container, blob, "")
            .await?
            .ok_or_else(|| {
                StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
            })?;

        validate_write_conditions(
            context,
            get_object(&options, "modifiedAccessConditions").as_ref(),
            Some(&doc),
        )?;

        if get_string(&doc.properties, "blobType").as_deref() != Some(BLOB_TYPE_APPEND_BLOB) {
            return Err(StorageErrorFactory::get_blob_invalid_blob_type(
                context.contextId().as_deref(),
            ));
        }

        let validator = BlobWriteLeaseValidator::new(get_object(&options, "leaseAccessConditions"));
        let adapter = BlobLeaseAdapter::new(&doc);
        validator.validate(&adapter, context)?;

        let mut syncer = BlobWriteLeaseSyncer::new(&mut doc);
        let _ = syncer.sync(&adapter);

        set_bool(&mut doc.properties, "isSealed", Some(true));
        set_value(
            &mut doc.properties,
            "lastModified",
            context
                .startTime()
                .map(|value| GeneratedValue::String(formatRfc1123(value))),
        );
        set_string(&mut doc.properties, "etag", Some(new_etag()));

        let mut blobs = self.blobs_collection.write().unwrap();
        blobs.insert(
            (
                account.to_string(),
                container.to_string(),
                blob.to_string(),
                "".to_string(),
            ),
            doc.clone(),
        );

        Ok(doc.properties)
    }

    // ─── Block Operations ───────────────────────────────────────────────────

    async fn stageBlock(
        &self,
        context: &Context,
        block: BlockModel,
        lease_access_conditions: Option<&LeaseAccessConditions>,
    ) -> Result<(), StorageError> {
        // TS: lines 2313-2395

        // Check container exists
        self.checkContainerExist(context, &block.accountName, &block.containerName)
            .await?;

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
            let etag = new_etag();
            let mut properties = BlobPropertiesInternal::default();
            set_datetime(&mut properties, "creationTime", context.startTime());
            set_datetime(&mut properties, "lastModified", context.startTime());
            set_string(&mut properties, "etag", Some(etag));
            set_i64(&mut properties, "contentLength", Some(0));
            set_string(
                &mut properties,
                "blobType",
                Some(BLOB_TYPE_BLOCK_BLOB.to_string()),
            );
            let new_blob = BlobModel {
                deleted: Some(false),
                accountName: block.accountName.clone(),
                containerName: block.containerName.clone(),
                name: Some(block.blobName.clone()),
                properties,
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

            if get_string(&blob_doc.properties, "blobType").as_deref() != Some(BLOB_TYPE_BLOCK_BLOB)
            {
                return Err(StorageErrorFactory::get_blob_invalid_blob_type(
                    context.contextId().as_deref(),
                ));
            }

            // Validate lease
            let adapter = BlobLeaseAdapter::new(blob_doc);
            let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
            let validator = BlobWriteLeaseValidator::new(lease_access_conditions.cloned());
            lease_state.validate(&validator)?;

            let mut blob_clone = blob_doc.clone();
            let mut syncer = BlobWriteLeaseSyncer::new(&mut blob_clone);
            let _ = syncer.sync(lease_state.lease());
            drop(blobs);
        }

        // Validate block ID length consistency if blob exists and has blocks
        if blob_exists {
            let blocks = self.blocks_collection.read().unwrap();
            let existing_block = blocks.iter().find(|((acc, cont, blob_name, _), _)| {
                acc == &block.accountName
                    && cont == &block.containerName
                    && blob_name == &block.blobName
            });

            if let Some((_, existing)) = existing_block {
                // Decode base64 block IDs and compare lengths
                if let (Some(ref existing_name), Some(ref new_name)) = (&existing.name, &block.name)
                {
                    let existing_decoded = base64::engine::general_purpose::STANDARD
                        .decode(existing_name)
                        .unwrap_or_default();
                    let new_decoded = base64::engine::general_purpose::STANDARD
                        .decode(new_name)
                        .unwrap_or_default();
                    if existing_decoded.len() != new_decoded.len() {
                        return Err(StorageErrorFactory::get_invalid_blob_or_block(
                            context.contextId().as_deref(),
                        ));
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

    async fn appendBlock(
        &self,
        context: &Context,
        block: BlockModel,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
        append_position_access_conditions: Option<&AppendPositionAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        // TS: lines 2397-2473

        let mut doc = self
            .get_blob_with_lease_updated(
                &block.accountName,
                &block.containerName,
                &block.blobName,
                "",
                context,
                true, // forceCommitted
            )
            .await?
            .ok_or_else(|| {
                StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
            })?;

        // Validate write conditions
        validate_write_conditions(context, modifiedAccessConditions, Some(&doc))?;

        // Validate lease
        let lease_adapter = BlobLeaseAdapter::new(&doc);
        let validator = BlobWriteLeaseValidator::new(lease_access_conditions.cloned());
        validator.validate(&lease_adapter, context)?;

        if get_bool(&doc.properties, "isSealed") == Some(true) {
            return Err(StorageErrorFactory::get_blob_sealed(
                context.contextId().as_deref(),
            ));
        }

        if get_string(&doc.properties, "blobType").as_deref() != Some(BLOB_TYPE_APPEND_BLOB) {
            return Err(StorageErrorFactory::get_blob_invalid_blob_type(
                context.contextId().as_deref(),
            ));
        }

        // Check max block count
        let current_block_count = doc
            .committedBlocksInOrder
            .as_ref()
            .map(|v| v.len())
            .unwrap_or(0);
        if current_block_count >= MAX_APPEND_BLOB_BLOCK_COUNT as usize {
            return Err(StorageErrorFactory::get_block_count_exceeds_limit(
                context.contextId().as_deref(),
            ));
        }

        if let Some(apc) = append_position_access_conditions {
            if let Some(append_position) = get_i64(apc, "appendPosition") {
                let current_length = get_i64(&doc.properties, "contentLength").unwrap_or(0);
                if current_length != append_position {
                    return Err(StorageErrorFactory::get_append_position_condition_not_met(
                        context.contextId().as_deref(),
                    ));
                }
            }

            if let Some(max_size) = get_i64(apc, "maxSize") {
                let current_length = get_i64(&doc.properties, "contentLength").unwrap_or(0);
                let block_size = block.size.unwrap_or(0);
                if current_length + block_size > max_size {
                    return Err(StorageErrorFactory::get_max_blob_size_condition_not_met(
                        context.contextId().as_deref(),
                    ));
                }
            }
        }

        let lease_adapter = BlobLeaseAdapter::new(&doc);
        let lease_state = LeaseFactory::create_lease_state(&lease_adapter, context)?;
        let mut syncer = BlobWriteLeaseSyncer::new(&mut doc);
        let _ = syncer.sync(lease_state.lease());

        // Prepare block for insertion
        let block_persistency = PersistencyBlockModel {
            name: block.name.clone(),
            size: block.size,
            persistency: block.persistency.clone(),
        };
        let block_size = block.size.unwrap_or(0);

        // Atomically append block under write lock to prevent concurrent
        // appendBlock calls from overwriting each other's blocks.
        let mut blobs = self.blobs_collection.write().unwrap();
        let key = (
            block.accountName.clone(),
            block.containerName.clone(),
            block.blobName.clone(),
            "".to_string(),
        );
        let current_doc = blobs.get_mut(&key).ok_or_else(|| {
            StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
        })?;

        if current_doc.committedBlocksInOrder.is_none() {
            current_doc.committedBlocksInOrder = Some(Vec::new());
        }
        current_doc
            .committedBlocksInOrder
            .as_mut()
            .unwrap()
            .push(block_persistency);

        set_string(&mut current_doc.properties, "etag", Some(new_etag()));
        set_datetime(
            &mut current_doc.properties,
            "lastModified",
            context.startTime(),
        );
        let next_content_length =
            get_i64(&current_doc.properties, "contentLength").unwrap_or(0) + block_size;
        set_i64(
            &mut current_doc.properties,
            "contentLength",
            Some(next_content_length),
        );

        // Lease sync on current doc
        let adapter = BlobLeaseAdapter::new(current_doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let mut syncer = BlobWriteLeaseSyncer::new(current_doc);
        let _ = syncer.sync(lease_state.lease());

        let result = current_doc.properties.clone();
        drop(blobs);

        Ok(result)
    }

    async fn commitBlockList(
        &self,
        context: &Context,
        blob: BlobModel,
        block_list: Vec<BlockListEntry>,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<(), StorageError> {
        let blob_name = blob.name.clone().unwrap_or_default();
        let snapshot = blob.snapshot.clone().unwrap_or_default();

        let doc = self
            .get_blob_with_lease_updated(
                &blob.accountName,
                &blob.containerName,
                &blob_name,
                &snapshot,
                context,
                false,
            )
            .await?;

        validate_write_conditions(context, modifiedAccessConditions, doc.as_ref())?;

        if let Some(mac) = modifiedAccessConditions {
            if get_string(mac, "ifNoneMatch").as_deref() == Some("*")
                && doc
                    .as_ref()
                    .map(|existing| existing.isCommitted == Some(true))
                    .unwrap_or(false)
            {
                return Err(StorageErrorFactory::get_blob_already_exists(
                    context.contextId().as_deref(),
                ));
            }
        }

        let mut doc = doc;
        if let Some(existing) = doc.as_ref() {
            if get_string(&existing.properties, "blobType").as_deref() != Some(BLOB_TYPE_BLOCK_BLOB)
            {
                return Err(StorageErrorFactory::get_blob_invalid_blob_type(
                    context.contextId().as_deref(),
                ));
            }

            let validator = BlobWriteLeaseValidator::new(lease_access_conditions.cloned());
            let adapter = BlobLeaseAdapter::new(existing);
            validator.validate(&adapter, context)?;
        }

        // Atomically read and clear blocks for this blob under write lock to prevent
        // concurrent stageBlock calls from adding blocks that get deleted without being seen.
        let persisted_blocks: Vec<BlockModel> = {
            let mut blocks = self.blocks_collection.write().unwrap();
            let items: Vec<BlockModel> = blocks
                .iter()
                .filter(|((acc, cont, block_blob, _), _)| {
                    acc == &blob.accountName
                        && cont == &blob.containerName
                        && block_blob == &blob_name
                })
                .map(|(_, block_doc)| block_doc.clone())
                .collect();
            // Clear blocks for this blob within same lock scope
            blocks.retain(|(acc, cont, block_blob, _), _| {
                !(acc == &blob.accountName
                    && cont == &blob.containerName
                    && block_blob == &blob_name)
            });
            drop(blocks);
            items
        };

        let mut committed_blocks_map: HashMap<String, PersistencyBlockModel> = HashMap::new();
        if let Some(existing) = doc.as_ref() {
            if let Some(committed_blocks) = existing.committedBlocksInOrder.as_ref() {
                for committed_block in committed_blocks {
                    if let Some(name) = committed_block.name.as_ref() {
                        committed_blocks_map.insert(name.clone(), committed_block.clone());
                    }
                }
            }
        }

        let mut uncommitted_blocks_map: HashMap<String, PersistencyBlockModel> = HashMap::new();
        for persisted_block in persisted_blocks {
            if !persisted_block.isCommitted {
                if let Some(name) = persisted_block.name.as_ref() {
                    uncommitted_blocks_map.insert(
                        name.clone(),
                        PersistencyBlockModel {
                            name: persisted_block.name.clone(),
                            size: persisted_block.size,
                            persistency: persisted_block.persistency.clone(),
                        },
                    );
                }
            }
        }

        let mut selected_block_list = Vec::new();
        for block_item in block_list {
            let selected = match block_item.blockCommitType.to_lowercase().as_str() {
                "uncommitted" => uncommitted_blocks_map.get(&block_item.blockName),
                "committed" => committed_blocks_map.get(&block_item.blockName),
                "latest" => uncommitted_blocks_map
                    .get(&block_item.blockName)
                    .or_else(|| committed_blocks_map.get(&block_item.blockName)),
                _ => {
                    return Err(StorageErrorFactory::get_invalid_block_list(
                        context.contextId().as_deref(),
                    ))
                }
            };

            let selected = selected.ok_or_else(|| {
                StorageErrorFactory::get_invalid_block_list(context.contextId().as_deref())
            })?;
            selected_block_list.push(selected.clone());
        }

        let content_length: i64 = selected_block_list
            .iter()
            .map(|block_doc| block_doc.size.unwrap_or(0))
            .sum();

        let blob_key = (
            blob.accountName.clone(),
            blob.containerName.clone(),
            blob_name.clone(),
            snapshot.clone(),
        );

        let mut blobs = self.blobs_collection.write().unwrap();
        if let Some(existing) = doc.as_mut() {
            set_string(
                &mut existing.properties,
                "blobType",
                get_string(&blob.properties, "blobType"),
            );
            set_datetime(
                &mut existing.properties,
                "lastModified",
                get_datetime(&blob.properties, "lastModified"),
            );
            existing.committedBlocksInOrder = Some(selected_block_list.clone());
            existing.isCommitted = Some(true);
            existing.metadata = blob.metadata.clone();
            set_string(
                &mut existing.properties,
                "accessTier",
                get_string(&blob.properties, "accessTier"),
            );
            set_bool(
                &mut existing.properties,
                "accessTierInferred",
                get_bool(&blob.properties, "accessTierInferred"),
            );
            set_string(
                &mut existing.properties,
                "etag",
                get_string(&blob.properties, "etag"),
            );
            set_string(
                &mut existing.properties,
                "cacheControl",
                get_string(&blob.properties, "cacheControl"),
            );
            set_string(
                &mut existing.properties,
                "contentType",
                get_string(&blob.properties, "contentType"),
            );
            set_string(
                &mut existing.properties,
                "contentMD5",
                get_string(&blob.properties, "contentMD5"),
            );
            set_string(
                &mut existing.properties,
                "contentEncoding",
                get_string(&blob.properties, "contentEncoding"),
            );
            set_string(
                &mut existing.properties,
                "contentLanguage",
                get_string(&blob.properties, "contentLanguage"),
            );
            set_string(
                &mut existing.properties,
                "contentDisposition",
                get_string(&blob.properties, "contentDisposition"),
            );
            existing.blobTags = blob.blobTags.clone();
            set_i64(
                &mut existing.properties,
                "contentLength",
                Some(content_length),
            );

            let lease_adapter = BlobLeaseAdapter::new(existing);
            let lease_state = LeaseFactory::create_lease_state(&lease_adapter, context)?;
            let mut syncer = BlobWriteLeaseSyncer::new(existing);
            let _ = syncer.sync(lease_state.lease());

            blobs.insert(blob_key, existing.clone());
        } else {
            let mut new_blob = blob.clone();
            new_blob.committedBlocksInOrder = Some(selected_block_list);
            set_i64(
                &mut new_blob.properties,
                "contentLength",
                Some(content_length),
            );
            blobs.insert(blob_key, new_blob);
        }
        drop(blobs);

        Ok(())
    }

    async fn getBlockList(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
        is_committed: Option<bool>,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<crate::persistence::i_blob_metadata_store::GetBlockListResult, StorageError> {
        let snapshot = snapshot.unwrap_or("");

        let doc = self
            .get_blob_with_lease_updated(account, container, blob, snapshot, context, false)
            .await?;

        validate_read_conditions(context, modifiedAccessConditions, doc.as_ref(), Some(false))?;

        let doc = doc.ok_or_else(|| {
            StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
        })?;

        let validator = BlobReadLeaseValidator::new(lease_access_conditions.cloned());
        let adapter = BlobLeaseAdapter::new(&doc);
        validator.validate(&adapter, context)?;

        if get_string(&doc.properties, "blobType").as_deref() != Some(BLOB_TYPE_BLOCK_BLOB) {
            return Err(StorageErrorFactory::get_blob_invalid_blob_type(
                context.contextId().as_deref(),
            ));
        }

        let to_block = |name: Option<String>, size: Option<i64>| {
            let mut block_doc = Block::default();
            if let Some(name) = name {
                block_doc.insert("name".to_string(), GeneratedValue::String(name));
            }
            if let Some(size) = size {
                block_doc.insert("size".to_string(), GeneratedValue::Number(size as f64));
            }
            block_doc
        };

        let mut committed_blocks = Vec::new();
        if is_committed != Some(false) {
            if let Some(items) = doc.committedBlocksInOrder.as_ref() {
                for item in items {
                    committed_blocks.push(to_block(item.name.clone(), item.size));
                }
            }
        }

        let mut uncommitted_blocks = Vec::new();
        if is_committed != Some(true) {
            let blocks = self.blocks_collection.read().unwrap();
            for ((acc, cont, block_blob, _), item) in blocks.iter() {
                if acc == account && cont == container && block_blob == blob {
                    uncommitted_blocks.push(to_block(item.name.clone(), item.size));
                }
            }
            drop(blocks);
            // Sort by block name for deterministic ordering (HashMap iteration is non-deterministic)
            uncommitted_blocks.sort_by(|a, b| {
                let name_a = a
                    .get("name")
                    .and_then(|v| match v {
                        GeneratedValue::String(s) => Some(s.as_str()),
                        _ => None,
                    })
                    .unwrap_or("");
                let name_b = b
                    .get("name")
                    .and_then(|v| match v {
                        GeneratedValue::String(s) => Some(s.as_str()),
                        _ => None,
                    })
                    .unwrap_or("");
                name_a.cmp(name_b)
            });
        }

        Ok(
            crate::persistence::i_blob_metadata_store::GetBlockListResult {
                properties: doc.properties,
                uncommittedBlocks: uncommitted_blocks,
                committedBlocks: committed_blocks,
            },
        )
    }

    async fn listUncommittedBlockPersistencyChunks(
        &self,
        _marker: Option<&str>,
        _max_results: Option<i64>,
    ) -> Result<(Vec<IExtentChunk>, Option<String>), StorageError> {
        // Placeholder - full implementation would be ~30 lines
        // TODO: Implement from TS lines 3079-3108
        Err(StorageError::new(
            500,
            "NotImplemented".to_string(),
            "listUncommittedBlockPersistencyChunks not yet implemented".to_string(),
            String::new(),
            BTreeMap::new(),
        ))
    }

    // ─── Page Blob Operations ───────────────────────────────────────────────

    async fn uploadPages(
        &self,
        context: &Context,
        blob: BlobModel,
        start: i64,
        end: i64,
        persistency: IExtentChunk,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
        sequence_number_access_conditions: Option<&SequenceNumberAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        let blob_name = blob.name.clone().unwrap_or_default();
        let snapshot = blob.snapshot.clone().unwrap_or_default();

        // Read snapshot for validation (not under write lock)
        let doc = self
            .get_blob_with_lease_updated(
                &blob.accountName,
                &blob.containerName,
                &blob_name,
                &snapshot,
                context,
                false,
            )
            .await?;

        validate_write_conditions(context, modifiedAccessConditions, doc.as_ref())?;
        validate_sequence_number_write_conditions(
            context,
            sequence_number_access_conditions,
            doc.as_ref(),
        )?;

        let doc = doc.ok_or_else(|| {
            StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
        })?;

        if get_string(&doc.properties, "blobType").as_deref() != Some(BLOB_TYPE_PAGE_BLOB) {
            return Err(StorageErrorFactory::get_blob_invalid_blob_type(
                context.contextId().as_deref(),
            ));
        }

        let validator = BlobWriteLeaseValidator::new(lease_access_conditions.cloned());
        let lease_adapter = BlobLeaseAdapter::new(&doc);
        validator.validate(&lease_adapter, context)?;

        // Prepare the range to merge
        let mut range = PageRange::default();
        range.insert("start".to_string(), GeneratedValue::Number(start as f64));
        range.insert("end".to_string(), GeneratedValue::Number(end as f64));

        // Atomically merge page range under write lock to prevent concurrent
        // uploadPages calls from overwriting each other's ranges.
        let mut blobs = self.blobs_collection.write().unwrap();
        let key = (
            blob.accountName.clone(),
            blob.containerName.clone(),
            blob_name,
            snapshot,
        );
        let current_doc = blobs.get_mut(&key).ok_or_else(|| {
            StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
        })?;

        if current_doc.pageRangesInOrder.is_none() {
            current_doc.pageRangesInOrder = Some(Vec::new());
        }
        self.page_blob_ranges_manager.merge_range(
            current_doc.pageRangesInOrder.as_mut().unwrap(),
            PersistencyPageRange { range, persistency },
        );

        set_string(&mut current_doc.properties, "etag", Some(new_etag()));
        set_datetime(
            &mut current_doc.properties,
            "lastModified",
            context.startTime(),
        );

        // Lease sync
        let adapter = BlobLeaseAdapter::new(current_doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let mut syncer = BlobWriteLeaseSyncer::new(current_doc);
        let _ = syncer.sync(lease_state.lease());

        let result = current_doc.properties.clone();
        drop(blobs);

        Ok(result)
    }

    async fn clearRange(
        &self,
        context: &Context,
        blob: BlobModel,
        start: i64,
        end: i64,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
        sequence_number_access_conditions: Option<&SequenceNumberAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        let blob_name = blob.name.clone().unwrap_or_default();
        let snapshot = blob.snapshot.clone().unwrap_or_default();

        // Read snapshot for validation (not under write lock)
        let doc = self
            .get_blob_with_lease_updated(
                &blob.accountName,
                &blob.containerName,
                &blob_name,
                &snapshot,
                context,
                false,
            )
            .await?;

        validate_write_conditions(context, modifiedAccessConditions, doc.as_ref())?;
        validate_sequence_number_write_conditions(
            context,
            sequence_number_access_conditions,
            doc.as_ref(),
        )?;

        let doc = doc.ok_or_else(|| {
            StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
        })?;

        let validator = BlobWriteLeaseValidator::new(lease_access_conditions.cloned());
        let lease_adapter = BlobLeaseAdapter::new(&doc);
        validator.validate(&lease_adapter, context)?;

        // Prepare the range to clear
        let mut range = PageRange::default();
        range.insert("start".to_string(), GeneratedValue::Number(start as f64));
        range.insert("end".to_string(), GeneratedValue::Number(end as f64));

        // Atomically clear page range under write lock
        let mut blobs = self.blobs_collection.write().unwrap();
        let key = (
            blob.accountName.clone(),
            blob.containerName.clone(),
            blob_name,
            snapshot,
        );
        let current_doc = blobs.get_mut(&key).ok_or_else(|| {
            StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
        })?;

        if current_doc.pageRangesInOrder.is_none() {
            current_doc.pageRangesInOrder = Some(Vec::new());
        }
        self.page_blob_ranges_manager
            .clear_range(current_doc.pageRangesInOrder.as_mut().unwrap(), range);

        set_string(&mut current_doc.properties, "etag", Some(new_etag()));
        set_datetime(
            &mut current_doc.properties,
            "lastModified",
            context.startTime(),
        );

        // Lease sync
        let adapter = BlobLeaseAdapter::new(current_doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let mut syncer = BlobWriteLeaseSyncer::new(current_doc);
        let _ = syncer.sync(lease_state.lease());

        let result = current_doc.properties.clone();
        drop(blobs);

        Ok(result)
    }

    async fn getPageRanges(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        snapshot: Option<&str>,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<GetPageRangeResponse, StorageError> {
        let snapshot = snapshot.unwrap_or("");

        let doc = self
            .get_blob_with_lease_updated(account, container, blob, snapshot, context, false)
            .await?;

        validate_read_conditions(context, modifiedAccessConditions, doc.as_ref(), Some(false))?;

        let doc = doc.ok_or_else(|| {
            StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
        })?;

        if get_string(&doc.properties, "blobType").as_deref() != Some(BLOB_TYPE_PAGE_BLOB) {
            return Err(StorageErrorFactory::get_blob_invalid_blob_type(
                context.contextId().as_deref(),
            ));
        }

        let validator = BlobReadLeaseValidator::new(lease_access_conditions.cloned());
        let adapter = BlobLeaseAdapter::new(&doc);
        validator.validate(&adapter, context)?;

        Ok(GetPageRangeResponse {
            properties: doc.properties,
            pageRangesInOrder: doc.pageRangesInOrder,
        })
    }

    async fn resizePageBlob(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        blob_content_length: i64,
        lease_access_conditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        let doc = self
            .get_blob_with_lease_updated(account, container, blob, "", context, false)
            .await?;

        validate_write_conditions(context, modifiedAccessConditions, doc.as_ref())?;

        let doc = doc.ok_or_else(|| {
            StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
        })?;

        if get_string(&doc.properties, "blobType").as_deref() != Some(BLOB_TYPE_PAGE_BLOB) {
            return Err(StorageErrorFactory::get_invalid_operation(
                context.contextId().as_deref(),
                Some("Resize could only be against a page blob."),
            ));
        }

        let validator = BlobWriteLeaseValidator::new(lease_access_conditions.cloned());
        let lease_adapter = BlobLeaseAdapter::new(&doc);
        validator.validate(&lease_adapter, context)?;

        // Atomically resize under write lock to prevent concurrent operations
        // from losing page range updates.
        let mut blobs = self.blobs_collection.write().unwrap();
        let key = (
            account.to_string(),
            container.to_string(),
            blob.to_string(),
            "".to_string(),
        );
        let current_doc = blobs.get_mut(&key).ok_or_else(|| {
            StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
        })?;

        if current_doc.pageRangesInOrder.is_none() {
            current_doc.pageRangesInOrder = Some(Vec::new());
        }

        if get_i64(&current_doc.properties, "contentLength").unwrap_or(0) > blob_content_length {
            let mut range = PageRange::default();
            range.insert(
                "start".to_string(),
                GeneratedValue::Number(blob_content_length as f64),
            );
            range.insert(
                "end".to_string(),
                GeneratedValue::Number(
                    (get_i64(&current_doc.properties, "contentLength").unwrap_or(0) - 1) as f64,
                ),
            );
            self.page_blob_ranges_manager
                .clear_range(current_doc.pageRangesInOrder.as_mut().unwrap(), range);
        }

        set_i64(
            &mut current_doc.properties,
            "contentLength",
            Some(blob_content_length),
        );
        set_datetime(
            &mut current_doc.properties,
            "lastModified",
            context.startTime(),
        );
        set_string(&mut current_doc.properties, "etag", Some(new_etag()));

        let adapter = BlobLeaseAdapter::new(current_doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let mut syncer = BlobWriteLeaseSyncer::new(current_doc);
        let _ = syncer.sync(lease_state.lease());

        let result = current_doc.properties.clone();
        drop(blobs);

        Ok(result)
    }

    async fn updateSequenceNumber(
        &self,
        context: &Context,
        account: &str,
        container: &str,
        blob: &str,
        sequenceNumberAction: &str,
        blobSequenceNumber: Option<i64>,
        leaseAccessConditions: Option<&LeaseAccessConditions>,
        modifiedAccessConditions: Option<&ModifiedAccessConditions>,
    ) -> Result<BlobPropertiesInternal, StorageError> {
        let doc = self
            .get_blob_with_lease_updated(account, container, blob, "", context, false)
            .await?;

        validate_write_conditions(context, modifiedAccessConditions, doc.as_ref())?;

        let doc = doc.ok_or_else(|| {
            StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
        })?;

        if get_string(&doc.properties, "blobType").as_deref() != Some(BLOB_TYPE_PAGE_BLOB) {
            return Err(StorageErrorFactory::get_invalid_operation(
                context.contextId().as_deref(),
                Some("Get Page Ranges could only be against a page blob."),
            ));
        }

        let validator = BlobWriteLeaseValidator::new(leaseAccessConditions.cloned());
        let lease_adapter = BlobLeaseAdapter::new(&doc);
        validator.validate(&lease_adapter, context)?;

        // Validate action + blobSequenceNumber parameter combination early
        // (before taking the write lock)
        match sequenceNumberAction.to_lowercase().as_str() {
            "max" | "update" => {
                if blobSequenceNumber.is_none() {
                    let msg = format!(
                        "x-ms-blob-sequence-number is required when x-ms-sequence-number-action is set to {}.",
                        sequenceNumberAction
                    );
                    return Err(StorageErrorFactory::get_invalid_operation(
                        context.contextId().as_deref(),
                        Some(&msg),
                    ));
                }
            }
            "increment" => {
                if blobSequenceNumber.is_some() {
                    return Err(StorageErrorFactory::get_invalid_operation(
                        context.contextId().as_deref(),
                        Some("x-ms-blob-sequence-number cannot be provided when x-ms-sequence-number-action is set to increment."),
                    ));
                }
            }
            _ => {
                return Err(StorageErrorFactory::get_invalid_operation(
                    context.contextId().as_deref(),
                    Some("Unsupported x-ms-sequence-number-action value."),
                ))
            }
        }

        // Atomically update sequence number under write lock to prevent
        // concurrent increment/max from reading stale values.
        let mut blobs = self.blobs_collection.write().unwrap();
        let key = (
            account.to_string(),
            container.to_string(),
            blob.to_string(),
            "".to_string(),
        );
        let current_doc = blobs.get_mut(&key).ok_or_else(|| {
            StorageErrorFactory::get_blob_not_found(context.contextId().as_deref())
        })?;

        let current_sequence_number =
            get_i64(&current_doc.properties, "blobSequenceNumber").unwrap_or(0);
        let next_sequence_number = match sequenceNumberAction.to_lowercase().as_str() {
            "max" => current_sequence_number.max(blobSequenceNumber.unwrap()),
            "increment" => current_sequence_number + 1,
            "update" => blobSequenceNumber.unwrap(),
            _ => unreachable!(), // validated above
        };

        set_i64(
            &mut current_doc.properties,
            "blobSequenceNumber",
            Some(next_sequence_number),
        );
        set_string(&mut current_doc.properties, "etag", Some(new_etag()));
        set_value(
            &mut current_doc.properties,
            "lastModified",
            context
                .startTime()
                .map(|value| GeneratedValue::String(formatRfc1123(value))),
        );

        let adapter = BlobLeaseAdapter::new(current_doc);
        let lease_state = LeaseFactory::create_lease_state(&adapter, context)?;
        let mut syncer = BlobWriteLeaseSyncer::new(current_doc);
        let _ = syncer.sync(lease_state.lease());

        let result = current_doc.properties.clone();
        drop(blobs);

        Ok(result)
    }
}

impl IGCExtentProvider for LokiBlobMetadataStore {
    fn iteratorExtents(&self) -> BoxStream<'_, Vec<String>> {
        let iterator = BlobReferredExtentsAsyncIterator::new(Arc::new(self.clone()));
        Box::pin(stream::unfold(Some(iterator), |state| async move {
            let mut iterator = state?;
            match iterator.next().await {
                Ok((extents, true)) if extents.is_empty() => None,
                Ok((extents, true)) => Some((extents, None)),
                Ok((extents, false)) => Some((extents, Some(iterator))),
                Err(_) => None,
            }
        }))
    }
}
