use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use std::{
    io::ErrorKind,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};
use tokio::{fs, sync::RwLock};

use crate::{
    i_cleaner::ICleaner, i_data_store::IDataStore, i_gc_extent_provider::IGCExtentProvider,
    storage_error::StorageError,
};

use super::{
    all_extents_async_iterator::AllExtentsAsyncIterator,
    i_extent_metadata_store::{IExtentMetadataStore, IExtentModel},
};

fn default_nextMarker() -> u64 {
    1
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct LokiExtentDocument {
    id: String,
    locationId: String,
    path: String,
    size: u64,
    LastModifyInMS: i64,
    marker: u64,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct LokiExtentDatabaseFile {
    extents: Vec<LokiExtentDocument>,
    #[serde(default = "default_nextMarker")]
    nextMarker: u64,
}

impl Default for LokiExtentDatabaseFile {
    fn default() -> Self {
        Self {
            extents: Vec::new(),
            nextMarker: default_nextMarker(),
        }
    }
}

#[allow(non_snake_case)]
#[derive(Clone)]
pub struct LokiExtentMetadata {
    pub lokiDBPath: String,
    db: Arc<RwLock<Vec<LokiExtentDocument>>>,
    initialized: Arc<AtomicBool>,
    closed: Arc<AtomicBool>,
    inMemory: bool,
    nextMarker: Arc<AtomicU64>,
    EXTENTS_COLLECTION: &'static str,
}

impl LokiExtentMetadata {
    pub fn new(lokiDBPath: String, inMemory: bool) -> Self {
        Self {
            lokiDBPath,
            db: Arc::new(RwLock::new(Vec::new())),
            initialized: Arc::new(AtomicBool::new(false)),
            closed: Arc::new(AtomicBool::new(true)),
            inMemory,
            nextMarker: Arc::new(AtomicU64::new(default_nextMarker())),
            EXTENTS_COLLECTION: "$EXTENTS_COLLECTION$",
        }
    }

    async fn loadDatabase(&self) -> Result<(), StorageError> {
        if self.inMemory {
            return Ok(());
        }

        let bytes = match fs::read(&self.lokiDBPath).await {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(StorageError::new(error.to_string())),
        };

        if bytes.is_empty() {
            return Ok(());
        }

        let database: LokiExtentDatabaseFile =
            serde_json::from_slice(&bytes).map_err(|error| StorageError::new(error.to_string()))?;
        let nextMarker = database
            .extents
            .iter()
            .map(|document| document.marker)
            .max()
            .map(|marker| marker + 1)
            .unwrap_or(database.nextMarker.max(default_nextMarker()));

        let mut db = self.db.write().await;
        *db = database.extents;
        self.nextMarker.store(nextMarker, Ordering::SeqCst);
        Ok(())
    }

    async fn saveDatabase(&self) -> Result<(), StorageError> {
        if self.inMemory {
            return Ok(());
        }

        let extents = self.db.read().await.clone();
        let database = LokiExtentDatabaseFile {
            extents,
            nextMarker: self.nextMarker.load(Ordering::SeqCst),
        };

        let bytes = serde_json::to_vec_pretty(&database)
            .map_err(|error| StorageError::new(error.to_string()))?;
        fs::write(&self.lokiDBPath, bytes)
            .await
            .map_err(|error| StorageError::new(error.to_string()))
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl IDataStore for LokiExtentMetadata {
    async fn init(&mut self) -> Result<(), StorageError> {
        let _ = self.EXTENTS_COLLECTION;

        match fs::metadata(&self.lokiDBPath).await {
            Ok(_) => self.loadDatabase().await?,
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(StorageError::new(error.to_string())),
        }

        self.saveDatabase().await?;
        self.initialized.store(true, Ordering::SeqCst);
        self.closed.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn isInitialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    async fn close(&mut self) -> Result<(), StorageError> {
        self.saveDatabase().await?;
        self.closed.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn isClosed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl ICleaner for LokiExtentMetadata {
    async fn clean(&mut self) -> Result<(), StorageError> {
        if !self.isClosed() {
            return Err(StorageError::new(
                "Cannot clean LokiExtentMetadata, it's not closed.",
            ));
        }

        {
            let mut db = self.db.write().await;
            db.clear();
        }
        self.nextMarker
            .store(default_nextMarker(), Ordering::SeqCst);

        match fs::remove_file(&self.lokiDBPath).await {
            Ok(_) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(StorageError::new(error.to_string())),
        }
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl IExtentMetadataStore for LokiExtentMetadata {
    async fn updateExtent(&mut self, extent: IExtentModel) -> Result<(), StorageError> {
        {
            let mut db = self.db.write().await;
            if let Some(document) = db.iter_mut().find(|document| document.id == extent.id) {
                document.size = extent.size;
                document.LastModifyInMS = extent.lastModifiedInMS;
            } else {
                db.push(LokiExtentDocument {
                    id: extent.id,
                    locationId: extent.locationId,
                    path: extent.path,
                    size: extent.size,
                    LastModifyInMS: extent.lastModifiedInMS,
                    marker: self.nextMarker.fetch_add(1, Ordering::SeqCst),
                });
            }
        }

        self.saveDatabase().await
    }

    async fn deleteExtent(&mut self, extentId: &str) -> Result<(), StorageError> {
        {
            let mut db = self.db.write().await;
            db.retain(|document| document.id != extentId);
        }

        self.saveDatabase().await
    }

    async fn listExtents(
        &self,
        id: Option<&str>,
        maxResults: Option<u64>,
        marker: Option<u64>,
        queryTime: Option<DateTime<Utc>>,
        protectTimeInMs: Option<u64>,
    ) -> Result<(Vec<IExtentModel>, Option<u64>), StorageError> {
        let mut documents = self.db.read().await.clone();
        documents.sort_by_key(|document| document.marker);

        let maxResults = maxResults.unwrap_or(5000) as usize;
        let protectTimeInMs = protectTimeInMs.unwrap_or(0) as i64;
        let queryTimeThreshold = queryTime.map(|time| time.timestamp_millis() - protectTimeInMs);

        let filtered: Vec<LokiExtentDocument> = documents
            .into_iter()
            .filter(|document| match id {
                Some(id) => document.id == id,
                None => true,
            })
            .filter(|document| match queryTimeThreshold {
                Some(threshold) => document.LastModifyInMS < threshold,
                None => true,
            })
            .filter(|document| match marker {
                Some(marker) => document.marker > marker,
                None => true,
            })
            .take(maxResults)
            .collect();

        let nextMarker = if filtered.len() < maxResults {
            None
        } else {
            filtered.last().map(|document| document.marker)
        };

        Ok((
            filtered
                .into_iter()
                .map(|document| IExtentModel {
                    id: document.id,
                    locationId: document.locationId,
                    path: document.path,
                    size: document.size,
                    lastModifiedInMS: document.LastModifyInMS,
                })
                .collect(),
            nextMarker,
        ))
    }

    async fn getExtentLocationId(&self, extentId: &str) -> Result<String, StorageError> {
        let db = self.db.read().await;
        db.iter()
            .find(|document| document.id == extentId)
            .map(|document| document.locationId.clone())
            .ok_or_else(|| StorageError::new(format!("Extent {extentId} does not exist.")))
    }
}

#[allow(non_snake_case)]
impl IGCExtentProvider for LokiExtentMetadata {
    fn iteratorExtents(&self) -> BoxStream<'_, Vec<String>> {
        AllExtentsAsyncIterator::new(self.clone()).into_stream()
    }
}
