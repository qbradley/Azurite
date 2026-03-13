use std::{
    io,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex as StdMutex,
    },
};

use azurite_common::{
    i_cleaner::ICleaner,
    i_data_store::IDataStore,
    i_gc_extent_provider::IGCExtentProvider,
    i_logger::ILogger,
    mutex::Mutex,
    persistence::{
        all_extents_async_iterator::AllExtentsAsyncIterator,
        fs_extent_store::FSExtentStore,
        i_extent_metadata_store::{IExtentMetadataStore, IExtentModel},
        i_extent_store::{
            ExtentDataInput, IExtentChunk, IExtentStore, IStoreDestinationConfigure, ReadableStream,
        },
        i_operation_queue::IOperationQueue,
        loki_extent_metadata_store::LokiExtentMetadata,
        memory_extent_store::{MemoryExtentChunkStore, MemoryExtentStore},
        operation_queue::OperationQueue,
        ZERO_EXTENT_ID,
    },
    storage_error::StorageError,
    zero_bytes_stream::ZeroBytesStream,
};
use bytes::Bytes;
use chrono::{Duration as ChronoDuration, Utc};
use futures::{future::join_all, stream, StreamExt};
use tempfile::tempdir;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::oneshot,
    time::{sleep, Duration},
};
use tokio_util::io::StreamReader;
use uuid::Uuid;

fn noop_logger() -> Arc<dyn ILogger + Send + Sync> {
    Arc::new(NoopLogger)
}

#[derive(Debug)]
struct NoopLogger;

impl ILogger for NoopLogger {
    fn error(&self, _message: &str, _contextID: Option<&str>) {}
    fn warn(&self, _message: &str, _contextID: Option<&str>) {}
    fn info(&self, _message: &str, _contextID: Option<&str>) {}
    fn verbose(&self, _message: &str, _contextID: Option<&str>) {}
    fn debug(&self, _message: &str, _contextID: Option<&str>) {}
}

fn readable_from_chunks(chunks: Vec<Bytes>) -> ReadableStream {
    Box::pin(StreamReader::new(stream::iter(
        chunks
            .into_iter()
            .map(|chunk| Ok::<Bytes, io::Error>(chunk))
            .collect::<Vec<_>>(),
    )))
}

async fn read_all(mut stream: ReadableStream) -> Vec<u8> {
    let mut data = Vec::new();
    stream
        .read_to_end(&mut data)
        .await
        .expect("stream should be readable");
    data
}

fn make_storage_error(
    status_code: u16,
    storage_error_code: String,
    storage_error_message: String,
    storage_request_id: String,
) -> StorageError {
    StorageError::new(format!(
        "{status_code}:{storage_error_code}:{storage_error_message}:{storage_request_id}"
    ))
}

#[tokio::test]
async fn operation_queue_runs_immediately_when_empty() {
    let queue = OperationQueue::withMaxConcurrency(2, noop_logger());

    let result = queue
        .operate(
            || async { Ok::<_, StorageError>("ready") },
            Some("ctx-empty"),
        )
        .await
        .expect("empty queue should execute immediately");

    assert_eq!(result, "ready");
}

#[tokio::test]
async fn operation_queue_preserves_fifo_order_at_max_concurrency_one() {
    let queue = OperationQueue::withMaxConcurrency(1, noop_logger());
    let observed = Arc::new(StdMutex::new(Vec::new()));

    let (release_first_tx, release_first_rx) = oneshot::channel();
    let release_first_rx = Arc::new(StdMutex::new(Some(release_first_rx)));

    let first = tokio::spawn({
        let queue = queue.clone();
        let observed = observed.clone();
        let release_first_rx = release_first_rx.clone();
        async move {
            queue
                .operate(
                    move || async move {
                        observed.lock().unwrap().push("first-start");
                        let release = release_first_rx.lock().unwrap().take().unwrap();
                        release.await.expect("first release should arrive");
                        observed.lock().unwrap().push("first-end");
                        Ok::<_, StorageError>("first")
                    },
                    Some("ctx-fifo"),
                )
                .await
        }
    });

    sleep(Duration::from_millis(20)).await;

    let second = tokio::spawn({
        let queue = queue.clone();
        let observed = observed.clone();
        async move {
            queue
                .operate(
                    move || async move {
                        observed.lock().unwrap().push("second");
                        Ok::<_, StorageError>("second")
                    },
                    Some("ctx-fifo"),
                )
                .await
        }
    });

    let third = tokio::spawn({
        let queue = queue.clone();
        let observed = observed.clone();
        async move {
            queue
                .operate(
                    move || async move {
                        observed.lock().unwrap().push("third");
                        Ok::<_, StorageError>("third")
                    },
                    Some("ctx-fifo"),
                )
                .await
        }
    });

    sleep(Duration::from_millis(20)).await;
    assert_eq!(observed.lock().unwrap().clone(), vec!["first-start"]);

    release_first_tx
        .send(())
        .expect("first operation should be released");

    assert_eq!(first.await.unwrap().unwrap(), "first");
    assert_eq!(second.await.unwrap().unwrap(), "second");
    assert_eq!(third.await.unwrap().unwrap(), "third");
    assert_eq!(
        observed.lock().unwrap().clone(),
        vec!["first-start", "first-end", "second", "third"]
    );
}

#[tokio::test]
async fn operation_queue_respects_parallelism_under_contention() {
    let queue = OperationQueue::withMaxConcurrency(3, noop_logger());
    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));

    let handles = (0..12)
        .map(|index| {
            let queue = queue.clone();
            let active = active.clone();
            let max_active = max_active.clone();
            tokio::spawn(async move {
                queue
                    .operate(
                        move || async move {
                            let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                            max_active.fetch_max(current, Ordering::SeqCst);
                            sleep(Duration::from_millis(15)).await;
                            active.fetch_sub(1, Ordering::SeqCst);
                            Ok::<_, StorageError>(index)
                        },
                        Some("ctx-parallel"),
                    )
                    .await
            })
        })
        .collect::<Vec<_>>();

    let mut results = join_all(handles)
        .await
        .into_iter()
        .map(|join| {
            join.expect("join should succeed")
                .expect("queue op should succeed")
        })
        .collect::<Vec<_>>();
    results.sort_unstable();

    assert_eq!(results, (0..12).collect::<Vec<_>>());
    assert!(max_active.load(Ordering::SeqCst) <= 3);
}

#[tokio::test]
async fn memory_extent_store_appends_reads_and_tracks_metadata() {
    let mut metadata = LokiExtentMetadata::new("memory-metadata".to_string(), true);
    metadata.init().await.expect("metadata init should succeed");

    let chunk_store = MemoryExtentChunkStore::new(Some(4096));
    let category_name = format!("memory-{}", Uuid::new_v4());
    let mut store = MemoryExtentStore::new(
        category_name.clone(),
        chunk_store.clone(),
        metadata.clone(),
        noop_logger(),
        make_storage_error,
    );
    store
        .init()
        .await
        .expect("memory store init should succeed");

    let first = store
        .appendExtent(
            ExtentDataInput::Buffer(Bytes::from_static(b"abc")),
            Some("ctx-memory"),
        )
        .await
        .expect("buffer append should succeed");
    let second = store
        .appendExtent(
            ExtentDataInput::Stream(readable_from_chunks(vec![
                Bytes::from_static(b"de"),
                Bytes::from_static(b"fg"),
            ])),
            Some("ctx-memory"),
        )
        .await
        .expect("stream append should succeed");

    assert!(!first.id.is_empty());
    assert!(!second.id.is_empty());
    assert_ne!(first.id, second.id);
    assert_eq!(chunk_store.totalSize(), 7);

    let (first_metadata, _) = metadata
        .listExtents(Some(&first.id), Some(10), None, None, None)
        .await
        .expect("first metadata should list");
    let (second_metadata, _) = metadata
        .listExtents(Some(&second.id), Some(10), None, None, None)
        .await
        .expect("second metadata should list");

    assert_eq!(first_metadata.len(), 1);
    assert_eq!(first_metadata[0].locationId, first.id);
    assert_eq!(first_metadata[0].path, first.id);
    assert_eq!(first_metadata[0].size, 3);
    assert_eq!(second_metadata.len(), 1);
    assert_eq!(second_metadata[0].locationId, second.id);
    assert_eq!(second_metadata[0].path, second.id);
    assert_eq!(second_metadata[0].size, 4);

    assert_eq!(
        read_all(
            store
                .readExtent(
                    Some(&IExtentChunk {
                        id: second.id.clone(),
                        offset: 1,
                        count: 2
                    }),
                    Some("ctx-memory")
                )
                .await
                .expect("partial extent should read"),
        )
        .await,
        b"ef"
    );
    assert_eq!(
        read_all(
            store
                .readExtents(&[first.clone(), second.clone()], 2, 4, Some("ctx-memory"))
                .await
                .expect("merged extents should read"),
        )
        .await,
        b"cdef"
    );
    assert_eq!(
        read_all(
            store
                .readExtent(
                    Some(&IExtentChunk {
                        id: ZERO_EXTENT_ID.to_string(),
                        offset: 0,
                        count: 3
                    }),
                    Some("ctx-memory"),
                )
                .await
                .expect("zero extent should read"),
        )
        .await,
        vec![0; 3]
    );

    store
        .close()
        .await
        .expect("memory store close should succeed");
    store
        .clean()
        .await
        .expect("memory store clean should succeed");
    assert_eq!(chunk_store.totalSize(), 0);
    assert_eq!(category_name.starts_with("memory-"), true);
}

#[tokio::test]
async fn fs_extent_store_round_trips_files_and_cleans_tempdir() {
    let temp = tempdir().expect("tempdir should exist");
    let extent_dir = temp.path().join("extents");
    let db_path = temp.path().join("extent-metadata.json");

    let mut metadata = LokiExtentMetadata::new(db_path.display().to_string(), true);
    metadata.init().await.expect("metadata init should succeed");

    let mut store = FSExtentStore::new(
        metadata.clone(),
        vec![IStoreDestinationConfigure {
            locationPath: extent_dir.display().to_string(),
            locationId: "primary".to_string(),
            maxConcurrency: 1,
        }],
        noop_logger(),
    );
    store.init().await.expect("fs store init should succeed");

    let first = store
        .appendExtent(
            ExtentDataInput::Buffer(Bytes::from_static(b"hello")),
            Some("ctx-fs"),
        )
        .await
        .expect("first append should succeed");
    let second = store
        .appendExtent(
            ExtentDataInput::Stream(readable_from_chunks(vec![Bytes::from_static(b" world")])),
            Some("ctx-fs"),
        )
        .await
        .expect("second append should succeed");

    assert_eq!(first.id, second.id);
    assert_eq!(first.offset, 0);
    assert_eq!(first.count, 5);
    assert_eq!(second.offset, 5);
    assert_eq!(second.count, 6);
    assert!(extent_dir.join(&first.id).exists());

    let (listed, _) = metadata
        .listExtents(Some(&first.id), Some(10), None, None, None)
        .await
        .expect("extent metadata should list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].locationId, "primary");
    assert_eq!(listed[0].size, 11);

    assert_eq!(
        read_all(
            store
                .readExtent(Some(&first), Some("ctx-fs"))
                .await
                .expect("first chunk should read"),
        )
        .await,
        b"hello"
    );
    assert_eq!(
        read_all(
            store
                .readExtent(Some(&second), Some("ctx-fs"))
                .await
                .expect("second chunk should read"),
        )
        .await,
        b" world"
    );
    assert_eq!(
        read_all(
            store
                .readExtents(&[first.clone(), second.clone()], 0, 11, Some("ctx-fs"))
                .await
                .expect("combined extents should read"),
        )
        .await,
        b"hello world"
    );
    assert_eq!(
        read_all(
            store
                .readExtent(
                    Some(&IExtentChunk {
                        id: ZERO_EXTENT_ID.to_string(),
                        offset: 0,
                        count: 4
                    }),
                    Some("ctx-fs"),
                )
                .await
                .expect("zero extent should read"),
        )
        .await,
        vec![0; 4]
    );

    let inactive_id = format!("inactive-{}", Uuid::new_v4());
    let inactive_path = extent_dir.join(&inactive_id);
    let mut inactive_file = tokio::fs::File::create(&inactive_path)
        .await
        .expect("inactive file should be created");
    inactive_file
        .write_all(b"stale")
        .await
        .expect("inactive file should be writable");
    inactive_file
        .flush()
        .await
        .expect("inactive file should flush");
    metadata
        .updateExtent(IExtentModel {
            id: inactive_id.clone(),
            locationId: "primary".to_string(),
            path: inactive_id.clone(),
            size: 5,
            lastModifiedInMS: (Utc::now() - ChronoDuration::minutes(20)).timestamp_millis(),
        })
        .await
        .expect("inactive metadata should be inserted");

    assert_eq!(
        store
            .deleteExtents(vec![inactive_id.clone()])
            .await
            .expect("inactive extent should delete"),
        1
    );
    assert!(!inactive_path.exists());
    assert!(metadata.getExtentLocationId(&inactive_id).await.is_err());

    store.close().await.expect("fs store close should succeed");
    store.clean().await.expect("fs store clean should succeed");
    assert!(!extent_dir.exists());
}

#[tokio::test]
async fn loki_extent_metadata_store_preserves_ts_update_and_query_semantics() {
    let temp = tempdir().expect("tempdir should exist");
    let db_path = temp.path().join("loki.json");
    let mut metadata = LokiExtentMetadata::new(db_path.display().to_string(), false);
    metadata.init().await.expect("loki init should succeed");

    let old_time = (Utc::now() - ChronoDuration::minutes(30)).timestamp_millis();
    let recent_time = Utc::now().timestamp_millis();

    metadata
        .updateExtent(IExtentModel {
            id: "extent-a".to_string(),
            locationId: "loc-a".to_string(),
            path: "path-a".to_string(),
            size: 5,
            lastModifiedInMS: old_time,
        })
        .await
        .expect("first extent should insert");
    metadata
        .updateExtent(IExtentModel {
            id: "extent-b".to_string(),
            locationId: "loc-b".to_string(),
            path: "path-b".to_string(),
            size: 6,
            lastModifiedInMS: recent_time,
        })
        .await
        .expect("second extent should insert");
    metadata
        .updateExtent(IExtentModel {
            id: ZERO_EXTENT_ID.to_string(),
            locationId: "zero-loc".to_string(),
            path: "zero-path".to_string(),
            size: 0,
            lastModifiedInMS: old_time,
        })
        .await
        .expect("zero extent sentinel should be storable");
    metadata
        .updateExtent(IExtentModel {
            id: "extent-a".to_string(),
            locationId: "loc-a-mutated".to_string(),
            path: "path-a-mutated".to_string(),
            size: 9,
            lastModifiedInMS: old_time + 10,
        })
        .await
        .expect("existing extent should update size and time");

    let (filtered, next_marker) = metadata
        .listExtents(None, Some(10), None, Some(Utc::now()), Some(10 * 60 * 1000))
        .await
        .expect("filtered list should succeed");
    let filtered_ids = filtered
        .iter()
        .map(|extent| extent.id.clone())
        .collect::<Vec<_>>();

    assert_eq!(
        filtered_ids,
        vec!["extent-a".to_string(), ZERO_EXTENT_ID.to_string()]
    );
    assert_eq!(next_marker, None);
    assert_eq!(
        metadata.getExtentLocationId("extent-a").await.unwrap(),
        "loc-a"
    );
    assert_eq!(
        metadata.getExtentLocationId(ZERO_EXTENT_ID).await.unwrap(),
        "zero-loc"
    );
    assert_eq!(
        filtered
            .iter()
            .find(|extent| extent.id == "extent-a")
            .expect("extent-a should still exist")
            .path,
        "path-a"
    );
    assert_eq!(
        filtered
            .iter()
            .find(|extent| extent.id == "extent-a")
            .expect("extent-a should still exist")
            .size,
        9
    );

    metadata
        .close()
        .await
        .expect("close should persist to disk");

    let mut reopened = LokiExtentMetadata::new(db_path.display().to_string(), false);
    reopened.init().await.expect("reopened loki should init");
    let (reloaded, _) = reopened
        .listExtents(None, Some(10), None, None, None)
        .await
        .expect("reloaded extents should list");
    assert_eq!(reloaded.len(), 3);

    let batches = reopened.iteratorExtents().collect::<Vec<_>>().await;
    assert_eq!(
        batches,
        vec![vec!["extent-a".to_string(), ZERO_EXTENT_ID.to_string()]]
    );

    reopened
        .deleteExtent(ZERO_EXTENT_ID)
        .await
        .expect("zero extent sentinel should delete");
    assert!(reopened.getExtentLocationId(ZERO_EXTENT_ID).await.is_err());
    reopened
        .close()
        .await
        .expect("reopened close should succeed");
    reopened
        .clean()
        .await
        .expect("clean should remove loki file");
    assert!(!db_path.exists());
}

#[tokio::test]
async fn all_extents_async_iterator_batches_metadata_like_typescript() {
    let mut metadata = LokiExtentMetadata::new("iterator-metadata".to_string(), true);
    metadata.init().await.expect("metadata init should succeed");

    let old_time = (Utc::now() - ChronoDuration::minutes(30)).timestamp_millis();
    for index in 0..1005 {
        metadata
            .updateExtent(IExtentModel {
                id: format!("extent-{index:04}"),
                locationId: "loc".to_string(),
                path: format!("extent-{index:04}"),
                size: 1,
                lastModifiedInMS: old_time,
            })
            .await
            .expect("extent should insert");
    }

    let mut iterator = AllExtentsAsyncIterator::new(metadata.clone());
    let first = iterator.next().await.expect("first batch should read");
    let second = iterator.next().await.expect("second batch should read");
    let done = iterator.next().await.expect("done batch should read");

    assert!(!first.done);
    assert_eq!(first.value.len(), 1000);
    assert_eq!(first.value.first().unwrap(), "extent-0000");
    assert_eq!(first.value.last().unwrap(), "extent-0999");

    assert!(!second.done);
    assert_eq!(
        second.value,
        (1000..1005)
            .map(|index| format!("extent-{index:04}"))
            .collect::<Vec<_>>()
    );

    assert!(done.done);
    assert!(done.value.is_empty());
}

#[tokio::test]
async fn zero_bytes_stream_emits_zero_filled_chunks_and_eof() {
    let mut stream = ZeroBytesStream::new(1025);
    let mut buffer = vec![0xAB; 2048];

    let first = stream
        .read(&mut buffer)
        .await
        .expect("first read should succeed");
    let second = stream
        .read(&mut buffer)
        .await
        .expect("second read should succeed");
    let third = stream
        .read(&mut buffer)
        .await
        .expect("third read should succeed");
    let fourth = stream
        .read(&mut buffer)
        .await
        .expect("eof read should succeed");

    assert_eq!(first, 512);
    assert_eq!(second, 512);
    assert_eq!(third, 1);
    assert_eq!(fourth, 0);
    assert!(buffer[..first].iter().all(|byte| *byte == 0));
}

#[tokio::test]
async fn mutex_preserves_fifo_order_and_exclusive_access() {
    let key = format!("mutex-fifo-{}", Uuid::new_v4());
    let order = Arc::new(StdMutex::new(Vec::new()));
    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));

    Mutex::lock(&key).await;

    let (second_entered_tx, second_entered_rx) = oneshot::channel();
    let (release_second_tx, release_second_rx) = oneshot::channel();
    let release_second_rx = Arc::new(StdMutex::new(Some(release_second_rx)));

    let second = tokio::spawn({
        let key = key.clone();
        let order = order.clone();
        let active = active.clone();
        let max_active = max_active.clone();
        let release_second_rx = release_second_rx.clone();
        async move {
            Mutex::lock(&key).await;
            let current = active.fetch_add(1, Ordering::SeqCst) + 1;
            max_active.fetch_max(current, Ordering::SeqCst);
            order.lock().unwrap().push("second");
            second_entered_tx
                .send(())
                .expect("second enter signal should send");
            let release = release_second_rx.lock().unwrap().take().unwrap();
            release.await.expect("second release should arrive");
            active.fetch_sub(1, Ordering::SeqCst);
            Mutex::unlock(&key).await;
        }
    });

    sleep(Duration::from_millis(10)).await;

    let third = tokio::spawn({
        let key = key.clone();
        let order = order.clone();
        let active = active.clone();
        let max_active = max_active.clone();
        async move {
            Mutex::lock(&key).await;
            let current = active.fetch_add(1, Ordering::SeqCst) + 1;
            max_active.fetch_max(current, Ordering::SeqCst);
            order.lock().unwrap().push("third");
            active.fetch_sub(1, Ordering::SeqCst);
            Mutex::unlock(&key).await;
        }
    });

    sleep(Duration::from_millis(20)).await;
    assert!(order.lock().unwrap().is_empty());

    Mutex::unlock(&key).await;
    second_entered_rx
        .await
        .expect("second should acquire first");
    sleep(Duration::from_millis(20)).await;
    assert_eq!(order.lock().unwrap().clone(), vec!["second"]);

    release_second_tx
        .send(())
        .expect("second should be released");
    second.await.expect("second join should succeed");
    third.await.expect("third join should succeed");

    assert_eq!(order.lock().unwrap().clone(), vec!["second", "third"]);
    assert_eq!(max_active.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn mutex_serializes_heavy_contention() {
    let key = format!("mutex-contention-{}", Uuid::new_v4());
    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));
    let completed = Arc::new(AtomicUsize::new(0));

    let handles = (0..16)
        .map(|_| {
            let key = key.clone();
            let active = active.clone();
            let max_active = max_active.clone();
            let completed = completed.clone();
            tokio::spawn(async move {
                Mutex::lock(&key).await;
                let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                max_active.fetch_max(current, Ordering::SeqCst);
                sleep(Duration::from_millis(5)).await;
                active.fetch_sub(1, Ordering::SeqCst);
                completed.fetch_add(1, Ordering::SeqCst);
                Mutex::unlock(&key).await;
            })
        })
        .collect::<Vec<_>>();

    join_all(handles)
        .await
        .into_iter()
        .for_each(|join| join.expect("contention task should succeed"));

    assert_eq!(completed.load(Ordering::SeqCst), 16);
    assert_eq!(max_active.load(Ordering::SeqCst), 1);
}
