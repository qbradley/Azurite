use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use std::{
    collections::{HashMap, VecDeque},
    io::Cursor,
    pin::Pin,
    sync::{Arc, LazyLock, Mutex},
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};
use uuid::Uuid;

use crate::{
    i_cleaner::ICleaner, i_data_store::IDataStore, i_logger::ILogger, storage_error::StorageError,
    zero_bytes_stream::ZeroBytesStream,
};

use super::{
    i_extent_metadata_store::{IExtentMetadataStore, IExtentModel},
    i_extent_store::{ExtentDataInput, IExtentChunk, IExtentStore, ReadableStream},
    ZERO_EXTENT_ID,
};

fn getTotalMemoryInBytes() -> u64 {
    match std::fs::read_to_string("/proc/meminfo") {
        Ok(content) => content
            .lines()
            .find_map(|line| {
                line.strip_prefix("MemTotal:")
                    .and_then(|value| value.split_whitespace().next())
                    .and_then(|value| value.parse::<u64>().ok())
                    .map(|value| value * 1024)
            })
            .unwrap_or(0),
        Err(_) => 0,
    }
}

pub static DEFAULT_EXTENT_MEMORY_LIMIT: LazyLock<u64> =
    LazyLock::new(|| getTotalMemoryInBytes().saturating_div(2));
pub static SharedChunkStore: LazyLock<MemoryExtentChunkStore> =
    LazyLock::new(|| MemoryExtentChunkStore::new(Some(*DEFAULT_EXTENT_MEMORY_LIMIT)));

pub type MakeErrorCallback =
    Arc<dyn Fn(u16, String, String, String) -> StorageError + Send + Sync + 'static>;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryExtentChunk {
    pub id: String,
    pub offset: u64,
    pub count: u64,
    pub chunks: Vec<Bytes>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
struct ExtentCategoryChunks {
    chunks: HashMap<String, MemoryExtentChunk>,
    totalSize: u64,
}

#[allow(non_snake_case)]
#[derive(Debug, Default)]
struct MemoryExtentChunkStoreState {
    _sizeLimit: Option<u64>,
    _chunks: HashMap<String, ExtentCategoryChunks>,
    _totalSize: u64,
}

#[derive(Clone, Debug)]
pub struct MemoryExtentChunkStore {
    state: Arc<Mutex<MemoryExtentChunkStoreState>>,
}

impl MemoryExtentChunkStore {
    pub fn new(sizeLimit: Option<u64>) -> Self {
        Self {
            state: Arc::new(Mutex::new(MemoryExtentChunkStoreState {
                _sizeLimit: sizeLimit,
                _chunks: HashMap::new(),
                _totalSize: 0,
            })),
        }
    }

    pub fn clear(&self, categoryName: &str) {
        let mut state = self.state.lock().unwrap();
        let Some(category) = state._chunks.get(categoryName).cloned() else {
            return;
        };

        state._totalSize = state._totalSize.saturating_sub(category.totalSize);
        state._chunks.remove(categoryName);
    }

    pub fn set(&self, categoryName: &str, chunk: MemoryExtentChunk) {
        if !self.trySet(categoryName, chunk) {
            let sizeLimit = self.sizeLimit();
            panic!(
                "Cannot add an extent chunk to the in-memory store. Size limit of {:?} bytes will be exceeded.",
                sizeLimit
            );
        }
    }

    pub fn trySet(&self, categoryName: &str, chunk: MemoryExtentChunk) -> bool {
        let mut state = self.state.lock().unwrap();
        let sizeLimit = state._sizeLimit;
        let totalSize = state._totalSize;
        let category = state
            ._chunks
            .entry(categoryName.to_string())
            .or_insert_with(ExtentCategoryChunks::default);

        let mut delta = chunk.count as i64;
        if let Some(existing) = category.chunks.get(&chunk.id) {
            delta -= existing.count as i64;
        }

        if let Some(sizeLimit) = sizeLimit {
            let nextSize = totalSize as i64 + delta;
            if nextSize > sizeLimit as i64 {
                return false;
            }
        }

        category.chunks.insert(chunk.id.clone(), chunk);
        category.totalSize = (category.totalSize as i64 + delta) as u64;
        state._totalSize = (totalSize as i64 + delta) as u64;
        true
    }

    pub fn get(&self, categoryName: &str, id: &str) -> Option<MemoryExtentChunk> {
        let state = self.state.lock().unwrap();
        state
            ._chunks
            .get(categoryName)
            .and_then(|category| category.chunks.get(id))
            .cloned()
    }

    pub fn delete(&self, categoryName: &str, id: &str) -> bool {
        let mut state = self.state.lock().unwrap();
        let (existingCount, shouldRemoveCategory) = {
            let Some(category) = state._chunks.get_mut(categoryName) else {
                return false;
            };

            let Some(existing) = category.chunks.remove(id) else {
                return false;
            };

            category.totalSize = category.totalSize.saturating_sub(existing.count);
            (existing.count, category.chunks.is_empty())
        };

        state._totalSize = state._totalSize.saturating_sub(existingCount);
        if shouldRemoveCategory {
            state._chunks.remove(categoryName);
        }

        true
    }

    pub fn totalSize(&self) -> u64 {
        self.state.lock().unwrap()._totalSize
    }

    pub fn setSizeLimit(&self, sizeLimit: Option<u64>) -> bool {
        let mut state = self.state.lock().unwrap();
        if let Some(sizeLimit) = sizeLimit {
            if sizeLimit < state._totalSize {
                return false;
            }
        }

        state._sizeLimit = sizeLimit;
        true
    }

    pub fn sizeLimit(&self) -> Option<u64> {
        self.state.lock().unwrap()._sizeLimit
    }
}

#[derive(Default)]
struct ConcatenatedReadableStream {
    streams: VecDeque<ReadableStream>,
}

impl ConcatenatedReadableStream {
    fn fromBytes(chunks: Vec<Bytes>) -> Self {
        Self {
            streams: chunks
                .into_iter()
                .map(|chunk| Box::pin(Cursor::new(chunk)) as ReadableStream)
                .collect(),
        }
    }

    fn fromStreams(streams: Vec<ReadableStream>) -> Self {
        Self {
            streams: streams.into(),
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
pub struct MemoryExtentStore<M>
where
    M: IExtentMetadataStore + Clone + Send + Sync + 'static,
{
    categoryName: String,
    chunks: MemoryExtentChunkStore,
    metadataStore: M,
    logger: Arc<dyn ILogger + Send + Sync>,
    makeError: MakeErrorCallback,
    initialized: bool,
    closed: bool,
}

impl<M> MemoryExtentStore<M>
where
    M: IExtentMetadataStore + Clone + Send + Sync + 'static,
{
    pub fn new<F>(
        categoryName: String,
        chunks: MemoryExtentChunkStore,
        metadata: M,
        logger: Arc<dyn ILogger + Send + Sync>,
        makeError: F,
    ) -> Self
    where
        F: Fn(u16, String, String, String) -> StorageError + Send + Sync + 'static,
    {
        Self {
            categoryName,
            chunks,
            metadataStore: metadata,
            logger,
            makeError: Arc::new(makeError),
            initialized: false,
            closed: true,
        }
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl<M> IDataStore for MemoryExtentStore<M>
where
    M: IExtentMetadataStore + Clone + Send + Sync + 'static,
{
    async fn init(&mut self) -> Result<(), StorageError> {
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
impl<M> ICleaner for MemoryExtentStore<M>
where
    M: IExtentMetadataStore + Clone + Send + Sync + 'static,
{
    async fn clean(&mut self) -> Result<(), StorageError> {
        if self.isClosed() {
            self.chunks.clear(&self.categoryName);
            return Ok(());
        }

        Err(StorageError::new(
            "Cannot clean MemoryExtentStore, it's not closed.",
        ))
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl<M> IExtentStore for MemoryExtentStore<M>
where
    M: IExtentMetadataStore + Clone + Send + Sync + 'static,
{
    async fn appendExtent(
        &mut self,
        data: ExtentDataInput,
        contextId: Option<&str>,
    ) -> Result<IExtentChunk, StorageError> {
        let mut chunks = Vec::new();
        let mut count = 0u64;

        match data {
            ExtentDataInput::Buffer(buffer) => {
                if !buffer.is_empty() {
                    count = buffer.len() as u64;
                    chunks.push(buffer);
                }
            }
            ExtentDataInput::Stream(mut stream) => {
                let mut buffer = vec![0; 64 * 1024];
                loop {
                    let read = stream.read(&mut buffer).await.map_err(|error| {
                        StorageError::new(format!("MemoryExtentStore append read error: {error}"))
                    })?;
                    if read == 0 {
                        break;
                    }

                    let chunk = Bytes::copy_from_slice(&buffer[..read]);
                    count += chunk.len() as u64;
                    chunks.push(chunk);
                }
            }
        }

        let extentChunk = MemoryExtentChunk {
            count,
            offset: 0,
            id: Uuid::new_v4().to_string(),
            chunks,
        };

        self.logger.info(
            &format!(
                "MemoryExtentStore:appendExtent() Add chunks to in-memory map. id:{} count:{} chunks.length:{}",
                extentChunk.id,
                count,
                extentChunk.chunks.len()
            ),
            contextId,
        );

        if !self.chunks.trySet(&self.categoryName, extentChunk.clone()) {
            return Err((self.makeError)(
                409,
                "MemoryExtentStoreAtSizeLimit".to_string(),
                format!(
                    "Cannot add an extent chunk to the in-memory store. Size limit of {:?} bytes will be exceeded",
                    self.chunks.sizeLimit()
                ),
                contextId.unwrap_or_default().to_string(),
            ));
        }

        self.logger.debug(
            &format!(
                "MemoryExtentStore:appendExtent() Added chunks to in-memory map. id:{} ",
                extentChunk.id
            ),
            contextId,
        );

        let extent = IExtentModel {
            id: extentChunk.id.clone(),
            locationId: extentChunk.id.clone(),
            path: extentChunk.id.clone(),
            size: count,
            lastModifiedInMS: Utc::now().timestamp_millis(),
        };

        self.metadataStore.updateExtent(extent).await?;

        self.logger.debug(
            &format!(
                "MemoryExtentStore:appendExtent() Added new extent to metadata store. id:{}",
                extentChunk.id
            ),
            contextId,
        );

        Ok(IExtentChunk {
            id: extentChunk.id,
            offset: extentChunk.offset,
            count: extentChunk.count,
        })
    }

    async fn readExtent(
        &self,
        extentChunk: Option<&IExtentChunk>,
        contextId: Option<&str>,
    ) -> Result<ReadableStream, StorageError> {
        let Some(extentChunk) = extentChunk else {
            return Ok(Box::pin(ZeroBytesStream::new(0)));
        };

        if extentChunk.count == 0 {
            return Ok(Box::pin(ZeroBytesStream::new(0)));
        }

        if extentChunk.id == ZERO_EXTENT_ID {
            return Ok(Box::pin(ZeroBytesStream::new(extentChunk.count)));
        }

        self.logger.info(
            &format!(
                "MemoryExtentStore:readExtent() Fetch chunks from in-memory map. id:{}",
                extentChunk.id
            ),
            contextId,
        );

        let Some(matchedChunk) = self.chunks.get(&self.categoryName, &extentChunk.id) else {
            return Err(StorageError::new(format!(
                "Extend {} does not exist.",
                extentChunk.id
            )));
        };

        self.logger.debug(
            &format!(
                "MemoryExtentStore:readExtent() Fetched chunks from in-memory map. id:{} count:{} chunks.length:{} totalSize:{}",
                matchedChunk.id,
                matchedChunk.count,
                matchedChunk.chunks.len(),
                self.chunks.totalSize()
            ),
            contextId,
        );

        let mut buffers = Vec::new();
        let mut skip = extentChunk.offset;
        let mut take = extentChunk.count;
        let mut skippedChunks = 0u64;
        let mut partialChunks = 0u64;
        let mut readChunks = 0u64;

        for chunk in matchedChunk.chunks {
            if take == 0 {
                break;
            }

            let chunkLength = chunk.len() as u64;
            if skip > 0 {
                if chunkLength <= skip {
                    skip -= chunkLength;
                    skippedChunks += 1;
                } else {
                    let chunkStart = skip as usize;
                    let chunkEnd = (skip + take.min(chunkLength - skip)) as usize;
                    let slice = chunk.slice(chunkStart..chunkEnd);
                    take -= slice.len() as u64;
                    skip = 0;
                    partialChunks += 1;
                    buffers.push(slice);
                }
            } else if chunkLength > take {
                let slice = chunk.slice(0..take as usize);
                take -= slice.len() as u64;
                partialChunks += 1;
                buffers.push(slice);
            } else {
                take -= chunkLength;
                readChunks += 1;
                buffers.push(chunk);
            }
        }

        self.logger.debug(
            &format!(
                "MemoryExtentStore:readExtent() Pushed in-memory chunks to Readable stream. id:{} chunks:{} skipped:{} partial:{}",
                matchedChunk.id,
                readChunks,
                skippedChunks,
                partialChunks
            ),
            contextId,
        );

        Ok(Box::pin(ConcatenatedReadableStream::fromBytes(buffers)))
    }

    async fn readExtents(
        &self,
        extentChunkArray: &[IExtentChunk],
        offset: u64,
        count: u64,
        contextId: Option<&str>,
    ) -> Result<ReadableStream, StorageError> {
        self.logger.info(
            "MemoryExtentStore:readExtents() Start read from multi extents...",
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
            self.logger.info(
                &format!("MemoryExtentStore:deleteExtents() Delete extent:{id}"),
                None,
            );
            if self.chunks.get(&self.categoryName, &id).is_some() {
                self.chunks.delete(&self.categoryName, &id);
            }
            self.metadataStore.deleteExtent(&id).await?;
            self.logger.debug(
                &format!(
                    "MemoryExtentStore:deleteExtents() Deleted extent:{id} totalSize:{}",
                    self.chunks.totalSize()
                ),
                None,
            );
            count += 1;
        }
        Ok(count)
    }

    fn getMetadataStore(&self) -> Arc<dyn IExtentMetadataStore + Send + Sync> {
        Arc::new(self.metadataStore.clone())
    }
}
