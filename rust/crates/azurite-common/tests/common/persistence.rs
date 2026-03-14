use std::{collections::HashMap, io, sync::Arc, sync::Mutex as StdMutex};

use async_trait::async_trait;
use azurite_common::{
    i_cleaner::ICleaner,
    i_data_store::IDataStore,
    i_gc_extent_provider::IGCExtentProvider,
    i_logger::ILogger,
    persistence::{
        i_extent_metadata::{IExtentMetadata, IExtentModel as LegacyExtentModel},
        i_extent_metadata_store::{IExtentMetadataStore, IExtentModel as MetadataStoreModel},
        i_extent_store::{
            ExtentDataInput, IExtentChunk, IExtentStore, IStoreDestinationConfigure,
            ReadableStream, StoreDestinationArray,
        },
        i_operation_queue::IOperationQueue,
        operation_queue::OperationQueue,
    },
    storage_error::StorageError,
};
use bytes::Bytes;
use chrono::{TimeZone, Utc};
use futures::{stream, stream::BoxStream, StreamExt};
use pretty_assertions::assert_eq;
use tokio::{
    io::AsyncReadExt,
    sync::oneshot,
    time::{sleep, Duration},
};
use tokio_util::io::StreamReader;

fn readable_from_bytes(bytes: Bytes) -> ReadableStream {
    Box::pin(StreamReader::new(stream::once(async move {
        Ok::<Bytes, io::Error>(bytes)
    })))
}

async fn read_all(mut stream: ReadableStream) -> Vec<u8> {
    let mut data = Vec::new();
    stream
        .read_to_end(&mut data)
        .await
        .expect("stream should be readable");
    data
}

fn slice_bytes(bytes: &Bytes, offset: u64, count: u64) -> Bytes {
    let start = usize::try_from(offset).unwrap_or(usize::MAX);
    if start >= bytes.len() {
        return Bytes::new();
    }

    let end = start.saturating_add(usize::try_from(count).unwrap_or(usize::MAX));
    bytes.slice(start..end.min(bytes.len()))
}

#[derive(Default)]
struct Lifecycle {
    initialized: bool,
    closed: bool,
    cleaned: bool,
}

struct LegacyMetadataFixture {
    lifecycle: Lifecycle,
    extents: Vec<LegacyExtentModel>,
    iterator_batches: Vec<Vec<String>>,
}

#[allow(non_snake_case)]
#[async_trait]
impl IDataStore for LegacyMetadataFixture {
    async fn init(&mut self) -> Result<(), StorageError> {
        self.lifecycle.initialized = true;
        self.lifecycle.closed = false;
        Ok(())
    }

    fn isInitialized(&self) -> bool {
        self.lifecycle.initialized
    }

    async fn close(&mut self) -> Result<(), StorageError> {
        self.lifecycle.closed = true;
        Ok(())
    }

    fn isClosed(&self) -> bool {
        self.lifecycle.closed
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl IExtentMetadata for LegacyMetadataFixture {
    async fn updateExtent(&mut self, extent: LegacyExtentModel) -> Result<(), StorageError> {
        if let Some(existing) = self
            .extents
            .iter_mut()
            .find(|current| current.id == extent.id)
        {
            *existing = extent;
        } else {
            self.extents.push(extent);
        }
        Ok(())
    }

    async fn listExtents(
        &self,
        id: Option<&str>,
        _maxResults: Option<u64>,
        marker: Option<u64>,
        _queryTime: Option<chrono::DateTime<Utc>>,
        _UnmodifiedTime: Option<u64>,
    ) -> Result<(Vec<LegacyExtentModel>, Option<u64>), StorageError> {
        let extents = self
            .extents
            .iter()
            .filter(|extent| id.map(|expected| extent.id == expected).unwrap_or(true))
            .cloned()
            .collect();

        Ok((extents, marker.map(|value| value.saturating_add(1))))
    }

    fn getExtentIterator(&self) -> BoxStream<'_, Vec<String>> {
        Box::pin(stream::iter(self.iterator_batches.clone()))
    }

    async fn deleteExtent(&mut self, extentId: &str) -> Result<(), StorageError> {
        self.extents.retain(|extent| extent.id != extentId);
        Ok(())
    }

    async fn getExtentPersistencyId(&self, extentId: &str) -> Result<String, StorageError> {
        self.extents
            .iter()
            .find(|extent| extent.id == extentId)
            .map(|extent| extent.persistencyId.clone())
            .ok_or_else(|| StorageError::new(format!("missing legacy extent: {extentId}")))
    }
}

struct MetadataStoreFixture {
    lifecycle: Lifecycle,
    extents: Vec<MetadataStoreModel>,
    iterator_batches: Vec<Vec<String>>,
}

#[allow(non_snake_case)]
#[async_trait]
impl IDataStore for MetadataStoreFixture {
    async fn init(&mut self) -> Result<(), StorageError> {
        self.lifecycle.initialized = true;
        self.lifecycle.closed = false;
        Ok(())
    }

    fn isInitialized(&self) -> bool {
        self.lifecycle.initialized
    }

    async fn close(&mut self) -> Result<(), StorageError> {
        self.lifecycle.closed = true;
        Ok(())
    }

    fn isClosed(&self) -> bool {
        self.lifecycle.closed
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl ICleaner for MetadataStoreFixture {
    async fn clean(&mut self) -> Result<(), StorageError> {
        self.lifecycle.cleaned = true;
        Ok(())
    }
}

impl IGCExtentProvider for MetadataStoreFixture {
    fn iteratorExtents(&self) -> BoxStream<'_, Vec<String>> {
        Box::pin(stream::iter(self.iterator_batches.clone()))
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl IExtentMetadataStore for MetadataStoreFixture {
    async fn updateExtent(&mut self, extent: MetadataStoreModel) -> Result<(), StorageError> {
        if let Some(existing) = self
            .extents
            .iter_mut()
            .find(|current| current.id == extent.id)
        {
            *existing = extent;
        } else {
            self.extents.push(extent);
        }
        Ok(())
    }

    async fn deleteExtent(&mut self, extentId: &str) -> Result<(), StorageError> {
        self.extents.retain(|extent| extent.id != extentId);
        Ok(())
    }

    async fn listExtents(
        &self,
        id: Option<&str>,
        _maxResults: Option<u64>,
        marker: Option<u64>,
        _queryTime: Option<chrono::DateTime<Utc>>,
        _protectTimeInMs: Option<u64>,
    ) -> Result<(Vec<MetadataStoreModel>, Option<u64>), StorageError> {
        let extents = self
            .extents
            .iter()
            .filter(|extent| id.map(|expected| extent.id == expected).unwrap_or(true))
            .cloned()
            .collect();

        Ok((extents, marker.map(|value| value.saturating_add(1))))
    }

    async fn getExtentLocationId(&self, extentId: &str) -> Result<String, StorageError> {
        self.extents
            .iter()
            .find(|extent| extent.id == extentId)
            .map(|extent| extent.locationId.clone())
            .ok_or_else(|| StorageError::new(format!("missing extent metadata: {extentId}")))
    }
}

struct ExtentStoreFixture {
    lifecycle: Lifecycle,
    extents: HashMap<String, Bytes>,
    metadata_store: Arc<MetadataStoreFixture>,
}

impl ExtentStoreFixture {
    fn new(metadata_store: Arc<MetadataStoreFixture>) -> Self {
        Self {
            lifecycle: Lifecycle::default(),
            extents: HashMap::new(),
            metadata_store,
        }
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl IDataStore for ExtentStoreFixture {
    async fn init(&mut self) -> Result<(), StorageError> {
        self.lifecycle.initialized = true;
        self.lifecycle.closed = false;
        Ok(())
    }

    fn isInitialized(&self) -> bool {
        self.lifecycle.initialized
    }

    async fn close(&mut self) -> Result<(), StorageError> {
        self.lifecycle.closed = true;
        Ok(())
    }

    fn isClosed(&self) -> bool {
        self.lifecycle.closed
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl ICleaner for ExtentStoreFixture {
    async fn clean(&mut self) -> Result<(), StorageError> {
        self.lifecycle.cleaned = true;
        Ok(())
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl IExtentStore for ExtentStoreFixture {
    async fn appendExtent(
        &mut self,
        data: ExtentDataInput,
        _contextId: Option<&str>,
    ) -> Result<IExtentChunk, StorageError> {
        let data = match data {
            ExtentDataInput::Buffer(bytes) => bytes,
            ExtentDataInput::Stream(stream) => Bytes::from(read_all(stream).await),
        };

        let id = format!("extent-{}", self.extents.len() + 1);
        let chunk = IExtentChunk {
            id: id.clone(),
            offset: 0,
            count: data.len() as u64,
        };
        self.extents.insert(id, data);

        Ok(chunk)
    }

    async fn readExtent(
        &self,
        extentChunk: Option<&IExtentChunk>,
        _contextId: Option<&str>,
    ) -> Result<ReadableStream, StorageError> {
        let bytes = extentChunk
            .and_then(|chunk| {
                self.extents
                    .get(&chunk.id)
                    .map(|bytes| slice_bytes(bytes, chunk.offset, chunk.count))
            })
            .unwrap_or_default();

        Ok(readable_from_bytes(bytes))
    }

    async fn readExtents(
        &self,
        extentChunkArray: &[IExtentChunk],
        offset: u64,
        count: u64,
        _contextId: Option<&str>,
    ) -> Result<ReadableStream, StorageError> {
        let mut merged = Vec::new();
        for chunk in extentChunkArray {
            if let Some(bytes) = self.extents.get(&chunk.id) {
                merged.extend_from_slice(&slice_bytes(bytes, chunk.offset, chunk.count));
            }
        }

        let merged = Bytes::from(merged);
        Ok(readable_from_bytes(slice_bytes(&merged, offset, count)))
    }

    async fn deleteExtents(&mut self, persistency: Vec<String>) -> Result<u64, StorageError> {
        let mut deleted = 0;
        for id in persistency {
            if self.extents.remove(&id).is_some() {
                deleted += 1;
            }
        }
        Ok(deleted)
    }

    fn getMetadataStore(&self) -> Arc<dyn IExtentMetadataStore + Send + Sync> {
        self.metadata_store.clone()
    }
}

#[derive(Default)]
struct ImmediateOperationQueue {
    contexts: StdMutex<Vec<Option<String>>>,
}

#[allow(non_snake_case)]
#[async_trait]
impl IOperationQueue for ImmediateOperationQueue {
    async fn operate<T, F, Fut>(&self, op: F, contextId: Option<&str>) -> Result<T, StorageError>
    where
        T: Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, StorageError>> + Send + 'static,
    {
        self.contexts
            .lock()
            .expect("queue context mutex poisoned")
            .push(contextId.map(str::to_owned));
        op().await
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DebugEntry {
    message: String,
    context: Option<String>,
}

#[derive(Default)]
struct RecordingLogger {
    debug_entries: StdMutex<Vec<DebugEntry>>,
}

impl RecordingLogger {
    fn entries(&self) -> Vec<DebugEntry> {
        self.debug_entries
            .lock()
            .expect("logger mutex poisoned")
            .clone()
    }
}

#[allow(non_snake_case)]
impl ILogger for RecordingLogger {
    fn error(&self, _message: &str, _contextID: Option<&str>) {}

    fn warn(&self, _message: &str, _contextID: Option<&str>) {}

    fn info(&self, _message: &str, _contextID: Option<&str>) {}

    fn verbose(&self, _message: &str, _contextID: Option<&str>) {}

    fn debug(&self, message: &str, contextID: Option<&str>) {
        self.debug_entries
            .lock()
            .expect("logger mutex poisoned")
            .push(DebugEntry {
                message: message.to_owned(),
                context: contextID.map(str::to_owned),
            });
    }
}

#[test]
fn extent_support_types_preserve_zero_and_empty_values() {
    let chunk = IExtentChunk {
        id: String::new(),
        offset: 0,
        count: 0,
    };
    let destinations: StoreDestinationArray = vec![IStoreDestinationConfigure {
        locationPath: String::new(),
        locationId: String::new(),
        maxConcurrency: 0,
    }];

    assert_eq!(chunk, chunk.clone());
    assert_eq!(destinations.len(), 1);
    assert_eq!(destinations[0].locationPath, "");
    assert_eq!(destinations[0].locationId, "");
    assert_eq!(destinations[0].maxConcurrency, 0);
}

#[tokio::test]
async fn legacy_extent_metadata_preserves_legacy_field_names_and_batches() {
    let mut metadata = LegacyMetadataFixture {
        lifecycle: Lifecycle::default(),
        extents: Vec::new(),
        iterator_batches: vec![vec![String::new()]],
    };
    let model = LegacyExtentModel {
        id: String::new(),
        persistencyId: "persist-1".to_owned(),
        path: String::new(),
        size: 0,
        LastModifyInMS: 0,
    };

    metadata.init().await.expect("init should succeed");
    metadata
        .updateExtent(model.clone())
        .await
        .expect("update should succeed");

    let (listed, next_marker) = metadata
        .listExtents(
            Some(""),
            Some(0),
            Some(0),
            Some(Utc.timestamp_millis_opt(0).single().unwrap()),
            Some(0),
        )
        .await
        .expect("list should succeed");

    assert_eq!(listed, vec![model.clone()]);
    assert_eq!(next_marker, Some(1));
    assert_eq!(
        metadata.getExtentIterator().collect::<Vec<_>>().await,
        vec![vec![String::new()]]
    );
    assert_eq!(
        metadata
            .getExtentPersistencyId("")
            .await
            .expect("persistency id should exist"),
        "persist-1"
    );

    metadata
        .deleteExtent("")
        .await
        .expect("delete should succeed");
    assert!(metadata
        .listExtents(None, None, None, None, None)
        .await
        .expect("list after delete should succeed")
        .0
        .is_empty());
}

#[tokio::test]
async fn extent_metadata_store_preserves_location_id_field_and_gc_batches() {
    let mut metadata_store = MetadataStoreFixture {
        lifecycle: Lifecycle::default(),
        extents: Vec::new(),
        iterator_batches: vec![vec![String::new(), "extent-2".to_owned()], vec![]],
    };
    let model = MetadataStoreModel {
        id: String::new(),
        locationId: String::new(),
        path: "path".to_owned(),
        size: 0,
        lastModifiedInMS: 0,
    };

    metadata_store
        .init()
        .await
        .expect("metadata init should succeed");
    metadata_store
        .updateExtent(model.clone())
        .await
        .expect("update should succeed");

    let (listed, next_marker) = metadata_store
        .listExtents(
            Some(""),
            Some(0),
            Some(5),
            Some(Utc.timestamp_millis_opt(0).single().unwrap()),
            Some(0),
        )
        .await
        .expect("list should succeed");

    assert_eq!(listed, vec![model]);
    assert_eq!(next_marker, Some(6));
    assert_eq!(
        metadata_store.iteratorExtents().collect::<Vec<_>>().await,
        vec![vec![String::new(), "extent-2".to_owned()], vec![]]
    );
    assert_eq!(
        metadata_store
            .getExtentLocationId("")
            .await
            .expect("location id should exist"),
        ""
    );

    metadata_store.clean().await.expect("clean should succeed");
    metadata_store.close().await.expect("close should succeed");
    assert!(metadata_store.lifecycle.cleaned);
    assert!(metadata_store.isClosed());
}

#[tokio::test]
async fn extent_store_contract_handles_buffer_stream_and_optional_chunks() {
    let metadata_store = Arc::new(MetadataStoreFixture {
        lifecycle: Lifecycle::default(),
        extents: vec![MetadataStoreModel {
            id: "extent-1".to_owned(),
            locationId: "primary".to_owned(),
            path: "path/extent-1".to_owned(),
            size: 3,
            lastModifiedInMS: 0,
        }],
        iterator_batches: vec![vec!["extent-1".to_owned()]],
    });
    let mut extent_store = ExtentStoreFixture::new(metadata_store.clone());

    extent_store.init().await.expect("init should succeed");

    let chunk = extent_store
        .appendExtent(
            ExtentDataInput::Buffer(Bytes::from_static(b"abc")),
            Some(""),
        )
        .await
        .expect("buffer append should succeed");
    let chunk_stream = extent_store
        .appendExtent(
            ExtentDataInput::Stream(readable_from_bytes(Bytes::from_static(b"def"))),
            None,
        )
        .await
        .expect("stream append should succeed");

    assert_eq!(
        chunk,
        IExtentChunk {
            id: "extent-1".to_owned(),
            offset: 0,
            count: 3,
        }
    );
    assert_eq!(chunk_stream.count, 3);
    assert_eq!(
        read_all(
            extent_store
                .readExtent(None, None)
                .await
                .expect("empty extent should read"),
        )
        .await,
        b""
    );
    assert_eq!(
        read_all(
            extent_store
                .readExtent(Some(&chunk), Some("ctx"))
                .await
                .expect("single extent should read"),
        )
        .await,
        b"abc"
    );
    assert_eq!(
        read_all(
            extent_store
                .readExtents(&[chunk.clone(), chunk_stream.clone()], 2, 3, Some(""))
                .await
                .expect("merged extents should read"),
        )
        .await,
        b"cde"
    );
    assert_eq!(
        extent_store
            .deleteExtents(vec![chunk.id.clone(), "missing".to_owned()])
            .await
            .expect("delete should succeed"),
        1
    );
    assert_eq!(
        extent_store
            .getMetadataStore()
            .getExtentLocationId("extent-1")
            .await
            .expect("metadata store should expose location id"),
        "primary"
    );

    extent_store.clean().await.expect("clean should succeed");
    extent_store.close().await.expect("close should succeed");
    assert!(extent_store.lifecycle.cleaned);
    assert!(extent_store.isClosed());
}

#[tokio::test]
async fn operation_queue_operate_is_lazy_and_generic() {
    let queue = ImmediateOperationQueue::default();
    let executions = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let text_result: Vec<String> = queue
        .operate(
            {
                let executions = executions.clone();
                move || async move {
                    executions.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok::<_, StorageError>(vec![String::new(), "queued".to_owned()])
                }
            },
            Some(""),
        )
        .await
        .expect("string operation should succeed");
    let number_result: u64 = queue
        .operate(|| async { Ok::<_, StorageError>(0) }, None)
        .await
        .expect("numeric operation should succeed");

    assert_eq!(text_result, vec![String::new(), "queued".to_owned()]);
    assert_eq!(number_result, 0);
    assert_eq!(executions.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(
        queue
            .contexts
            .lock()
            .expect("queue context mutex poisoned")
            .clone(),
        vec![Some(String::new()), None]
    );
}

#[tokio::test]
async fn concrete_operation_queue_serializes_work_at_max_concurrency_one() {
    let logger = Arc::new(RecordingLogger::default());
    let queue = OperationQueue::withMaxConcurrency(1, logger.clone());
    let started = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let max_active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (first_started_tx, first_started_rx) = oneshot::channel();
    let (release_first_tx, release_first_rx) = oneshot::channel();
    let release_first = Arc::new(StdMutex::new(Some(release_first_rx)));

    let first = tokio::spawn({
        let queue = queue.clone();
        let started = started.clone();
        let active = active.clone();
        let max_active = max_active.clone();
        let release_first = release_first.clone();
        async move {
            queue
                .operate(
                    move || async move {
                        started.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        let current = active.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                        max_active.fetch_max(current, std::sync::atomic::Ordering::SeqCst);
                        first_started_tx
                            .send(())
                            .expect("first start signal should send");
                        let release = release_first
                            .lock()
                            .expect("release mutex poisoned")
                            .take()
                            .expect("release receiver should exist");
                        release.await.expect("release signal should arrive");
                        active.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                        Ok::<_, StorageError>("first")
                    },
                    Some("ctx"),
                )
                .await
        }
    });

    first_started_rx
        .await
        .expect("first operation should report it started");

    let second = tokio::spawn({
        let queue = queue.clone();
        let started = started.clone();
        let active = active.clone();
        let max_active = max_active.clone();
        async move {
            queue
                .operate(
                    move || async move {
                        started.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        let current = active.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                        max_active.fetch_max(current, std::sync::atomic::Ordering::SeqCst);
                        active.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                        Ok::<_, StorageError>("second")
                    },
                    Some("ctx"),
                )
                .await
        }
    });

    sleep(Duration::from_millis(25)).await;
    assert_eq!(started.load(std::sync::atomic::Ordering::SeqCst), 1);

    release_first_tx
        .send(())
        .expect("first operation should be released");

    assert_eq!(
        first.await.expect("first join should succeed").unwrap(),
        "first"
    );
    assert_eq!(
        second.await.expect("second join should succeed").unwrap(),
        "second"
    );
    assert_eq!(started.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert_eq!(max_active.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(
        logger
            .entries()
            .into_iter()
            .filter(|entry| entry.context.as_deref() == Some("ctx"))
            .count(),
        6
    );
}
