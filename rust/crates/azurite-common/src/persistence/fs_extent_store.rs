use async_trait::async_trait;
use bytes::Bytes;
use std::{
    collections::{HashMap, VecDeque},
    io::{Cursor, ErrorKind},
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncRead, AsyncReadExt, AsyncSeekExt, ReadBuf, SeekFrom},
    sync::Mutex,
};
use uuid::Uuid;

use crate::{
    i_cleaner::ICleaner,
    i_data_store::IDataStore,
    i_logger::ILogger,
    storage_error::StorageError,
    utils::constants::{DEFAULT_MAX_EXTENT_SIZE, DEFAULT_READ_CONCURRENCY},
    zero_bytes_stream::ZeroBytesStream,
};

use super::{
    i_extent_metadata_store::{IExtentMetadataStore, IExtentModel},
    i_extent_store::{
        ExtentDataInput, IExtentChunk, IExtentStore, ReadableStream, StoreDestinationArray,
    },
    i_operation_queue::IOperationQueue,
    operation_queue::OperationQueue,
    ZERO_EXTENT_ID,
};

const MAX_EXTENT_SIZE: u64 = DEFAULT_MAX_EXTENT_SIZE;

#[allow(non_snake_case)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppendStatusCode {
    Idle,
    Appending,
}

#[allow(non_snake_case)]
#[derive(Debug)]
struct AppendExtent {
    id: String,
    offset: u64,
    appendStatus: AppendStatusCode,
    locationId: String,
    file: Option<File>,
}

#[derive(Default)]
struct ConcatenatedReadableStream {
    streams: VecDeque<ReadableStream>,
}

impl ConcatenatedReadableStream {
    fn fromStreams(streams: Vec<ReadableStream>) -> Self {
        Self {
            streams: streams.into(),
        }
    }

    fn fromBytes(chunks: Vec<Bytes>) -> Self {
        Self {
            streams: chunks
                .into_iter()
                .map(|chunk| Box::pin(Cursor::new(chunk)) as ReadableStream)
                .collect(),
        }
    }
}

impl AsyncRead for ConcatenatedReadableStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        loop {
            let Some(stream) = self.streams.front_mut() else {
                return Poll::Ready(Ok(()));
            };
            let filled = buf.filled().len();
            match stream.as_mut().poll_read(cx, buf) {
                Poll::Ready(Ok(())) if buf.filled().len() == filled => {
                    self.streams.pop_front();
                }
                poll => return poll,
            }
        }
    }
}

#[allow(non_snake_case)]
pub struct FSExtentStore<M>
where
    M: IExtentMetadataStore + Clone + Send + Sync + 'static,
{
    metadataStore: M,
    appendQueue: OperationQueue,
    readQueue: OperationQueue,
    initialized: bool,
    closed: bool,
    activeWriteExtents: Vec<Arc<Mutex<AppendExtent>>>,
    activeWriteExtentsNumber: usize,
    persistencyConfiguration: StoreDestinationArray,
    persistencyPath: HashMap<String, PathBuf>,
    logger: Arc<dyn ILogger + Send + Sync>,
}

impl<M> FSExtentStore<M>
where
    M: IExtentMetadataStore + Clone + Send + Sync + 'static,
{
    pub fn new(
        metadata: M,
        persistencyConfiguration: StoreDestinationArray,
        logger: Arc<dyn ILogger + Send + Sync>,
    ) -> Self {
        let mut activeWriteExtents = Vec::new();
        let mut persistencyPath = HashMap::new();

        for storeDestination in &persistencyConfiguration {
            persistencyPath.insert(
                storeDestination.locationId.clone(),
                PathBuf::from(&storeDestination.locationPath),
            );
            for _ in 0..storeDestination.maxConcurrency {
                activeWriteExtents.push(Arc::new(Mutex::new(Self::createAppendExtent(
                    &storeDestination.locationId,
                ))));
            }
        }
        let activeWriteExtentsNumber = activeWriteExtents.len();

        Self {
            metadataStore: metadata,
            appendQueue: OperationQueue::withMaxConcurrency(
                activeWriteExtentsNumber,
                logger.clone(),
            ),
            readQueue: OperationQueue::withMaxConcurrency(
                DEFAULT_READ_CONCURRENCY as usize,
                logger.clone(),
            ),
            initialized: false,
            closed: true,
            activeWriteExtents,
            activeWriteExtentsNumber,
            persistencyConfiguration,
            persistencyPath,
            logger,
        }
    }

    async fn streamPipe<R>(
        logger: Arc<dyn ILogger + Send + Sync>,
        mut rs: R,
        ws: &mut File,
        contextId: Option<String>,
    ) -> Result<u64, StorageError>
    where
        R: AsyncRead + Unpin + Send,
    {
        logger.debug(
            "FSExtentStore:streamPipe() Start piping data to write stream",
            contextId.as_deref(),
        );

        let count = tokio::io::copy(&mut rs, ws)
            .await
            .map_err(|error| StorageError::new(error.to_string()))?;
        ws.sync_data()
            .await
            .map_err(|error| StorageError::new(error.to_string()))?;

        logger.debug(
            &format!(
                "FSExtentStore:streamPipe() Flush data successfully. Resolve streamPipe() with {count} bytes."
            ),
            contextId.as_deref(),
        );
        Ok(count)
    }

    async fn isActiveExtent(&self, id: &str) -> bool {
        for extent in &self.activeWriteExtents {
            if extent.lock().await.id == id {
                return true;
            }
        }
        false
    }

    fn createAppendExtent(persistencyId: &str) -> AppendExtent {
        AppendExtent {
            id: Uuid::new_v4().to_string(),
            offset: 0,
            appendStatus: AppendStatusCode::Idle,
            locationId: persistencyId.to_string(),
            file: None,
        }
    }

    fn getNewExtent(appendExtent: &mut AppendExtent) {
        appendExtent.id = Uuid::new_v4().to_string();
        appendExtent.offset = 0;
        appendExtent.file = None;
    }

    fn generateExtentPath(&self, persistencyId: &str, extentId: &str) -> PathBuf {
        let directoryPath = self
            .persistencyPath
            .get(persistencyId)
            .unwrap_or_else(|| panic!("Missing persistency path for {persistencyId}"));
        directoryPath.join(extentId)
    }

    async fn cleanupActiveFiles(&self) {
        for extent in &self.activeWriteExtents {
            extent.lock().await.file = None;
        }
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl<M> IDataStore for FSExtentStore<M>
where
    M: IExtentMetadataStore + Clone + Send + Sync + 'static,
{
    async fn init(&mut self) -> Result<(), StorageError> {
        for storeDestination in &self.persistencyConfiguration {
            match fs::metadata(&storeDestination.locationPath).await {
                Ok(_) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {
                    fs::create_dir_all(&storeDestination.locationPath)
                        .await
                        .map_err(|error| StorageError::new(error.to_string()))?;
                }
                Err(error) => return Err(StorageError::new(error.to_string())),
            }
        }

        if !self.metadataStore.isInitialized() {
            self.metadataStore.init().await?;
        }

        self.initialized = true;
        self.closed = false;
        Ok(())
    }

    fn isInitialized(&self) -> bool {
        self.initialized
    }

    async fn close(&mut self) -> Result<(), StorageError> {
        self.cleanupActiveFiles().await;
        if !self.metadataStore.isClosed() {
            self.metadataStore.close().await?;
        }

        self.closed = true;
        Ok(())
    }

    fn isClosed(&self) -> bool {
        self.closed
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl<M> ICleaner for FSExtentStore<M>
where
    M: IExtentMetadataStore + Clone + Send + Sync + 'static,
{
    async fn clean(&mut self) -> Result<(), StorageError> {
        if !self.isClosed() {
            return Err(StorageError::new(
                "Cannot clean FSExtentStore, it's not closed.",
            ));
        }

        for path in &self.persistencyConfiguration {
            let result = fs::remove_dir_all(&path.locationPath).await;
            if let Err(error) = result {
                if error.kind() != ErrorKind::NotFound {
                    continue;
                }
            }
        }
        Ok(())
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl<M> IExtentStore for FSExtentStore<M>
where
    M: IExtentMetadataStore + Clone + Send + Sync + 'static,
{
    async fn appendExtent(
        &mut self,
        data: ExtentDataInput,
        contextId: Option<&str>,
    ) -> Result<IExtentChunk, StorageError> {
        let activeWriteExtents = self.activeWriteExtents.clone();
        let activeWriteExtentsNumber = self.activeWriteExtentsNumber;
        let logger = self.logger.clone();
        let persistencyPath = self.persistencyPath.clone();
        let mut metadataStore = self.metadataStore.clone();
        let contextIdOwned = contextId.map(|value| value.to_string());

        self.appendQueue
            .operate(
                move || async move {
                    let appendExtentIdx = loop {
                        let mut selectedIndex = None;
                        for i in 0..activeWriteExtentsNumber {
                            if let Ok(mut appendExtent) = activeWriteExtents[i].try_lock() {
                                if appendExtent.appendStatus == AppendStatusCode::Idle {
                                    appendExtent.appendStatus = AppendStatusCode::Appending;
                                    selectedIndex = Some(i);
                                    break;
                                }
                            }
                        }

                        if let Some(index) = selectedIndex {
                            break index;
                        }

                        tokio::task::yield_now().await;
                    };

                    let appendExtentMutex = activeWriteExtents[appendExtentIdx].clone();
                    let mut appendExtent = appendExtentMutex.lock().await;
                    logger.info(
                        &format!(
                            "FSExtentStore:appendExtent() Select extent from idle location for extent append operation. LocationId:{appendExtentIdx} extentId:{} offset:{} MAX_EXTENT_SIZE:{MAX_EXTENT_SIZE} ",
                            appendExtent.id,
                            appendExtent.offset
                        ),
                        contextIdOwned.as_deref(),
                    );

                    if appendExtent.offset >= MAX_EXTENT_SIZE {
                        logger.info(
                            &format!(
                                "FSExtentStore:appendExtent() Size of selected extent offset is larger than maximum extent size {MAX_EXTENT_SIZE} bytes, try appending to new extent."
                            ),
                            contextIdOwned.as_deref(),
                        );
                        appendExtent.file = None;
                        FSExtentStore::<M>::getNewExtent(&mut appendExtent);
                        logger.info(
                            &format!(
                                "FSExtentStore:appendExtent() Allocated new extent LocationID:{appendExtentIdx} extentId:{} offset:{} MAX_EXTENT_SIZE:{MAX_EXTENT_SIZE} ",
                                appendExtent.id,
                                appendExtent.offset
                            ),
                            contextIdOwned.as_deref(),
                        );
                    }

                    let id = appendExtent.id.clone();
                    let path = persistencyPath
                        .get(&appendExtent.locationId)
                        .map(|directoryPath| directoryPath.join(&id))
                        .unwrap_or_else(|| Path::new("").join(&id));
                    if appendExtent.file.is_none() {
                        appendExtent.file = Some(
                            OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(&path)
                                .await
                                .map_err(|error| StorageError::new(error.to_string()))?,
                        );
                        logger.debug(
                            &format!(
                                "FSExtentStore:appendExtent() Open file:{} for extent:{id}",
                                path.display()
                            ),
                            contextIdOwned.as_deref(),
                        );
                    }

                    let offset = appendExtent.offset;
                    let countResult = match data {
                        ExtentDataInput::Buffer(buffer) => {
                            let reader = ConcatenatedReadableStream::fromBytes(vec![buffer]);
                            FSExtentStore::<M>::streamPipe(
                                logger.clone(),
                                reader,
                                appendExtent.file.as_mut().unwrap(),
                                contextIdOwned.clone(),
                            )
                            .await
                        }
                        ExtentDataInput::Stream(stream) => {
                            FSExtentStore::<M>::streamPipe(
                                logger.clone(),
                                stream,
                                appendExtent.file.as_mut().unwrap(),
                                contextIdOwned.clone(),
                            )
                            .await
                        }
                    };

                    match countResult {
                        Ok(count) => {
                            appendExtent.offset += count;
                            let extent = IExtentModel {
                                id: id.clone(),
                                locationId: appendExtent.locationId.clone(),
                                path: id.clone(),
                                size: count + offset,
                                lastModifiedInMS: chrono::Utc::now().timestamp_millis(),
                            };
                            logger.debug(
                                &format!(
                                    "FSExtentStore:appendExtent() Write finish, start updating extent metadata. extent:{:?}",
                                    extent
                                ),
                                contextIdOwned.as_deref(),
                            );
                            metadataStore.updateExtent(extent).await?;
                            appendExtent.appendStatus = AppendStatusCode::Idle;
                            Ok(IExtentChunk { id, offset, count })
                        }
                        Err(error) => {
                            appendExtent.file = None;
                            match OpenOptions::new().write(true).create(true).open(&path).await {
                                Ok(file) => {
                                    if let Err(truncateError) = file.set_len(offset).await {
                                        logger.error(
                                            &format!(
                                                "FSExtentStore:appendExtent() Truncate path:{} len: {offset} error:{}.",
                                                path.display(),
                                                truncateError
                                            ),
                                            contextIdOwned.as_deref(),
                                        );
                                    }
                                }
                                Err(truncateOpenError) => logger.error(
                                    &format!(
                                        "FSExtentStore:appendExtent() Open for truncate path:{} error:{}.",
                                        path.display(),
                                        truncateOpenError
                                    ),
                                    contextIdOwned.as_deref(),
                                ),
                            }
                            appendExtent.appendStatus = AppendStatusCode::Idle;
                            Err(error)
                        }
                    }
                },
                contextId,
            )
            .await
    }

    async fn readExtent(
        &self,
        extentChunk: Option<&IExtentChunk>,
        contextId: Option<&str>,
    ) -> Result<ReadableStream, StorageError> {
        let Some(extentChunk) = extentChunk.cloned() else {
            return Ok(Box::pin(ZeroBytesStream::new(0)));
        };

        if extentChunk.count == 0 {
            return Ok(Box::pin(ZeroBytesStream::new(0)));
        }

        if extentChunk.id == ZERO_EXTENT_ID {
            return Ok(Box::pin(ZeroBytesStream::new(extentChunk.count)));
        }

        let persistencyId = self
            .metadataStore
            .getExtentLocationId(&extentChunk.id)
            .await?;
        let path = self.generateExtentPath(&persistencyId, &extentChunk.id);
        let logger = self.logger.clone();
        let contextIdOwned = contextId.map(|value| value.to_string());

        self.readQueue
            .operate(
                move || async move {
                    logger.verbose(
                        &format!(
                            "FSExtentStore:readExtent() Creating read stream. LocationId:{persistencyId} extentId:{} path:{} offset:{} count:{} end:{}",
                            extentChunk.id,
                            path.display(),
                            extentChunk.offset,
                            extentChunk.count,
                            extentChunk.offset + extentChunk.count - 1
                        ),
                        contextIdOwned.as_deref(),
                    );
                    let mut file = File::open(&path)
                        .await
                        .map_err(|error| StorageError::new(error.to_string()))?;
                    file.seek(SeekFrom::Start(extentChunk.offset))
                        .await
                        .map_err(|error| StorageError::new(error.to_string()))?;
                    Ok(Box::pin(file.take(extentChunk.count)) as ReadableStream)
                },
                contextId,
            )
            .await
    }

    async fn readExtents(
        &self,
        extentChunkArray: &[IExtentChunk],
        offset: u64,
        count: u64,
        contextId: Option<&str>,
    ) -> Result<ReadableStream, StorageError> {
        self.logger.verbose(
            "FSExtentStore:readExtents() Start read from multi extents...",
            contextId,
        );

        if count == 0 {
            return Ok(Box::pin(ZeroBytesStream::new(0)));
        }

        let start = offset;
        let end = if count == u64::MAX {
            u64::MAX
        } else {
            offset.saturating_add(count)
        };

        let mut streams = Vec::new();
        let mut accumulatedOffset = 0u64;

        for chunk in extentChunkArray {
            let nextOffset = accumulatedOffset.saturating_add(chunk.count);
            if nextOffset <= start {
                accumulatedOffset = nextOffset;
                continue;
            }
            if end <= accumulatedOffset {
                break;
            }

            let mut chunkStart = chunk.offset;
            let mut chunkEnd = chunk.offset.saturating_add(chunk.count);
            if start > accumulatedOffset {
                chunkStart = chunkStart.saturating_add(start - accumulatedOffset);
            }
            if end <= nextOffset {
                chunkEnd = chunkEnd.saturating_sub(nextOffset - end);
            }

            streams.push(
                self.readExtent(
                    Some(&IExtentChunk {
                        id: chunk.id.clone(),
                        offset: chunkStart,
                        count: chunkEnd.saturating_sub(chunkStart),
                    }),
                    contextId,
                )
                .await?,
            );
            accumulatedOffset = nextOffset;
        }

        if end != u64::MAX && accumulatedOffset < end {
            return Err(StorageError::new(format!(
                "Not enough payload data error. Total length of payloads is {accumulatedOffset}, while required data offset is {offset}, count is {count}."
            )));
        }

        Ok(Box::pin(ConcatenatedReadableStream::fromStreams(streams)))
    }

    async fn deleteExtents(&mut self, extents: Vec<String>) -> Result<u64, StorageError> {
        let mut count = 0u64;
        for id in extents {
            if self.isActiveExtent(&id).await {
                self.logger.debug(
                    &format!("FSExtentStore:deleteExtents() Skip deleting active extent:{id}"),
                    None,
                );
                continue;
            }

            let locationId = self.metadataStore.getExtentLocationId(&id).await?;
            let path = self.generateExtentPath(&locationId, &id);
            self.logger.debug(
                &format!(
                    "FSExtentStore:deleteExtents() Delete extent:{id} location:{locationId} path:{}",
                    path.display()
                ),
                None,
            );

            match fs::remove_file(&path).await {
                Ok(_) => self.metadataStore.deleteExtent(&id).await?,
                Err(error) if error.kind() == ErrorKind::NotFound => {
                    self.metadataStore.deleteExtent(&id).await?
                }
                Err(error) => return Err(StorageError::new(error.to_string())),
            }

            count += 1;
        }

        Ok(count)
    }

    fn getMetadataStore(&self) -> Arc<dyn IExtentMetadataStore + Send + Sync> {
        Arc::new(self.metadataStore.clone())
    }
}
