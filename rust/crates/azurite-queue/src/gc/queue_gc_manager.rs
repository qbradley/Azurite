use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use azurite_common::{
    i_gc_extent_provider::IGCExtentProvider, i_gc_manager::IGCManager, i_logger::ILogger,
    persistence::i_extent_store::IExtentStore, storage_error::StorageError,
};
use futures::StreamExt;
use tokio::{
    sync::{Mutex as AsyncMutex, Notify},
    task::JoinHandle,
    time::{sleep, Duration},
};

use crate::utils::constants::DEFAULT_GC_INTERVAL_MS;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Initializing,
    Running,
    Closing,
    Closed,
}

pub type QueueGCManagerErrorHandler = Arc<dyn Fn(StorageError) + Send + Sync + 'static>;

pub struct QueueGCManager {
    referredExtentsProvider: Arc<AsyncMutex<Box<dyn IGCExtentProvider + Send + Sync>>>,
    allExtentsProvider: Arc<AsyncMutex<Box<dyn IGCExtentProvider + Send + Sync>>>,
    extentStore: Arc<AsyncMutex<Box<dyn IExtentStore + Send + Sync>>>,
    errorHandler: QueueGCManagerErrorHandler,
    logger: Arc<dyn ILogger + Send + Sync>,
    pub gcIntervalInMS: u64,
    status: Arc<Mutex<Status>>,
    abort: Arc<Notify>,
    loopTask: Option<JoinHandle<()>>,
}

impl QueueGCManager {
    pub fn new(
        referredExtentsProvider: Box<dyn IGCExtentProvider + Send + Sync>,
        allExtentsProvider: Box<dyn IGCExtentProvider + Send + Sync>,
        extentStore: Box<dyn IExtentStore + Send + Sync>,
        errorHandler: QueueGCManagerErrorHandler,
        logger: Arc<dyn ILogger + Send + Sync>,
        gcIntervalInMS: Option<u64>,
    ) -> Self {
        let gcIntervalInMS = gcIntervalInMS.unwrap_or(DEFAULT_GC_INTERVAL_MS).max(1);
        Self {
            referredExtentsProvider: Arc::new(AsyncMutex::new(referredExtentsProvider)),
            allExtentsProvider: Arc::new(AsyncMutex::new(allExtentsProvider)),
            extentStore: Arc::new(AsyncMutex::new(extentStore)),
            errorHandler,
            logger,
            gcIntervalInMS,
            status: Arc::new(Mutex::new(Status::Closed)),
            abort: Arc::new(Notify::new()),
            loopTask: None,
        }
    }

    pub fn status(&self) -> Status {
        *self.status.lock().unwrap()
    }

    fn set_status(status: &Arc<Mutex<Status>>, value: Status) {
        *status.lock().unwrap() = value;
    }

    fn is_running(status: &Arc<Mutex<Status>>) -> bool {
        *status.lock().unwrap() == Status::Running
    }

    async fn markSweepLoop(
        logger: Arc<dyn ILogger + Send + Sync>,
        status: Arc<Mutex<Status>>,
        abort: Arc<Notify>,
        referredExtentsProvider: Arc<AsyncMutex<Box<dyn IGCExtentProvider + Send + Sync>>>,
        allExtentsProvider: Arc<AsyncMutex<Box<dyn IGCExtentProvider + Send + Sync>>>,
        extentStore: Arc<AsyncMutex<Box<dyn IExtentStore + Send + Sync>>>,
        gcIntervalInMS: u64,
    ) -> Result<(), StorageError> {
        while Self::is_running(&status) {
            logger.info(
                "QueueGCManager:markSweepLoop() Start new mark and sweep.",
                None,
            );
            let start = std::time::Instant::now();
            Self::markSweep(
                logger.clone(),
                status.clone(),
                referredExtentsProvider.clone(),
                allExtentsProvider.clone(),
                extentStore.clone(),
            )
            .await?;
            logger.info(
                &format!(
                    "QueueGCManager:markSweepLoop() Mark and sweep finished, take {}ms.",
                    start.elapsed().as_millis()
                ),
                None,
            );

            if Self::is_running(&status) {
                logger.info(
                    &format!(
                        "QueueGCManager:markSweepLoop() Sleep for {}ms.",
                        gcIntervalInMS
                    ),
                    None,
                );
                Self::sleep(abort.clone(), gcIntervalInMS).await;
            }
        }

        Ok(())
    }

    async fn markSweep(
        logger: Arc<dyn ILogger + Send + Sync>,
        status: Arc<Mutex<Status>>,
        referredExtentsProvider: Arc<AsyncMutex<Box<dyn IGCExtentProvider + Send + Sync>>>,
        allExtentsProvider: Arc<AsyncMutex<Box<dyn IGCExtentProvider + Send + Sync>>>,
        extentStore: Arc<AsyncMutex<Box<dyn IExtentStore + Send + Sync>>>,
    ) -> Result<(), StorageError> {
        logger.info("QueueGCManager:markSweep() Get all extents.", None);
        let mut allExtents = Self::getAllExtents(status.clone(), allExtentsProvider).await?;
        logger.info(
            &format!(
                "QueueGCManager:markSweep() Get {} extents.",
                allExtents.len()
            ),
            None,
        );

        if !Self::is_running(&status) {
            return Ok(());
        }

        logger.info(
            "QueueGCManager:markSweep() Get referred extents, then remove from allExtents.",
            None,
        );
        {
            let referredExtentsProvider = referredExtentsProvider.lock().await;
            let mut iterator = referredExtentsProvider.iteratorExtents();
            while let Some(bucket) = iterator.next().await {
                if !Self::is_running(&status) {
                    break;
                }
                for item in bucket {
                    allExtents.remove(&item);
                }
            }
        }

        logger.info(
            &format!(
                "QueueGCManager:markSweep() Got referred extents, unreferenced extents count is {}.",
                allExtents.len()
            ),
            None,
        );

        if !allExtents.is_empty() {
            logger.info(
                &format!(
                    "QueueGCManager:markSweep() Start to delete {} unreferenced extents.",
                    allExtents.len()
                ),
                None,
            );
            let deletedCount = {
                let mut extentStore = extentStore.lock().await;
                extentStore
                    .deleteExtents(allExtents.into_iter().collect())
                    .await?
            };
            logger.info(
                &format!(
                    "QueueGCManager:markSweep() Deleted {} unreferenced extents, after excluding active write extents.",
                    deletedCount
                ),
                None,
            );
        }

        Ok(())
    }

    async fn getAllExtents(
        status: Arc<Mutex<Status>>,
        allExtentsProvider: Arc<AsyncMutex<Box<dyn IGCExtentProvider + Send + Sync>>>,
    ) -> Result<BTreeSet<String>, StorageError> {
        let mut allExtents = BTreeSet::new();

        if !Self::is_running(&status) {
            return Ok(allExtents);
        }

        let allExtentsProvider = allExtentsProvider.lock().await;
        let mut iterator = allExtentsProvider.iteratorExtents();
        while let Some(bucket) = iterator.next().await {
            if !Self::is_running(&status) {
                break;
            }
            allExtents.extend(bucket);
        }

        Ok(allExtents)
    }

    async fn sleep(abort: Arc<Notify>, timeInMS: u64) {
        if timeInMS == 0 {
            return;
        }

        tokio::select! {
            _ = sleep(Duration::from_millis(timeInMS)) => {}
            _ = abort.notified() => {}
        }
    }
}

#[async_trait]
impl IGCManager for QueueGCManager {
    async fn start(&mut self) -> Result<(), StorageError> {
        if self.status() == Status::Running {
            self.logger.info(
                "QueueGCManager:start() QueueGCManager successfully started. QueueGCManager is already in Running status.",
                None,
            );
            return Ok(());
        }

        if self.status() != Status::Closed {
            return Err(StorageError::new(format!(
                "QueueGCManager:start() QueueGCManager cannot start, current manager is under {:?}",
                self.status()
            )));
        }

        self.logger.info(
            "QueueGCManager:start() Starting QueueGCManager. Set status to Initializing.",
            None,
        );
        Self::set_status(&self.status, Status::Initializing);

        {
            let mut referredExtentsProvider = self.referredExtentsProvider.lock().await;
            if !referredExtentsProvider.isInitialized() {
                referredExtentsProvider.init().await?;
            }
        }
        {
            let mut allExtentsProvider = self.allExtentsProvider.lock().await;
            if !allExtentsProvider.isInitialized() {
                allExtentsProvider.init().await?;
            }
        }
        {
            let mut extentStore = self.extentStore.lock().await;
            if !extentStore.isInitialized() {
                extentStore.init().await?;
            }
        }

        self.logger.info(
            "QueueGCManager:start() Trigger mark and sweep loop. Set status to Running.",
            None,
        );
        Self::set_status(&self.status, Status::Running);

        let logger = self.logger.clone();
        let status = self.status.clone();
        let abort = self.abort.clone();
        let referredExtentsProvider = self.referredExtentsProvider.clone();
        let allExtentsProvider = self.allExtentsProvider.clone();
        let extentStore = self.extentStore.clone();
        let errorHandler = self.errorHandler.clone();
        let gcIntervalInMS = self.gcIntervalInMS;
        self.loopTask = Some(tokio::spawn(async move {
            if let Err(error) = QueueGCManager::markSweepLoop(
                logger.clone(),
                status.clone(),
                abort,
                referredExtentsProvider,
                allExtentsProvider,
                extentStore,
                gcIntervalInMS,
            )
            .await
            {
                logger.info(
                    &format!(
                        "QueueGCManager:start() Mark and sweep loop emits error: {}",
                        error.message
                    ),
                    None,
                );
                QueueGCManager::set_status(&status, Status::Closed);
                (errorHandler)(error);
            } else {
                logger.info(
                    "QueueGCManager:start() Mark and sweep loop is closed.",
                    None,
                );
                QueueGCManager::set_status(&status, Status::Closed);
            }
        }));

        self.logger.info(
            "QueueGCManager:start() QueueGCManager successfully started.",
            None,
        );
        Ok(())
    }

    async fn close(&mut self) -> Result<(), StorageError> {
        if self.status() == Status::Closed {
            self.logger.info(
                "QueueGCManager:close() QueueGCManager successfully closed. QueueGCManager is already in Closed status.",
                None,
            );
            return Ok(());
        }

        if self.status() != Status::Running {
            return Err(StorageError::new(format!(
                "QueueGCManager:close() QueueGCManager cannot close, current manager is under {:?}",
                self.status()
            )));
        }

        self.logger.info(
            "QueueGCManager:close() Start closing QueueGCManager. Set status to Closing.",
            None,
        );
        Self::set_status(&self.status, Status::Closing);
        self.abort.notify_waiters();

        if let Some(loopTask) = self.loopTask.take() {
            let _ = loopTask.await;
        }

        Self::set_status(&self.status, Status::Closed);
        self.logger.info(
            "QueueGCManager:close() QueueGCManager successfully closed. Set status to Closed.",
            None,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    use azurite_common::{
        i_cleaner::ICleaner,
        i_data_store::IDataStore,
        persistence::{
            i_extent_metadata_store::IExtentMetadataStore,
            i_extent_store::{ExtentDataInput, IExtentChunk, ReadableStream},
            loki_extent_metadata_store::LokiExtentMetadata,
        },
    };
    use futures::{stream, stream::BoxStream};
    use tokio::io::empty;

    #[derive(Default)]
    struct TestLogger;

    impl ILogger for TestLogger {
        fn error(&self, _message: &str, _contextID: Option<&str>) {}
        fn warn(&self, _message: &str, _contextID: Option<&str>) {}
        fn info(&self, _message: &str, _contextID: Option<&str>) {}
        fn verbose(&self, _message: &str, _contextID: Option<&str>) {}
        fn debug(&self, _message: &str, _contextID: Option<&str>) {}
    }

    #[derive(Clone)]
    struct MockExtentProvider {
        batches: Vec<Vec<String>>,
        initialized: Arc<AtomicBool>,
        closed: Arc<AtomicBool>,
    }

    impl MockExtentProvider {
        fn new(batches: Vec<Vec<String>>) -> Self {
            Self {
                batches,
                initialized: Arc::new(AtomicBool::new(false)),
                closed: Arc::new(AtomicBool::new(true)),
            }
        }
    }

    #[async_trait]
    impl IDataStore for MockExtentProvider {
        async fn init(&mut self) -> Result<(), StorageError> {
            self.initialized.store(true, Ordering::SeqCst);
            self.closed.store(false, Ordering::SeqCst);
            Ok(())
        }

        fn isInitialized(&self) -> bool {
            self.initialized.load(Ordering::SeqCst)
        }

        async fn close(&mut self) -> Result<(), StorageError> {
            self.closed.store(true, Ordering::SeqCst);
            Ok(())
        }

        fn isClosed(&self) -> bool {
            self.closed.load(Ordering::SeqCst)
        }
    }

    impl IGCExtentProvider for MockExtentProvider {
        fn iteratorExtents(&self) -> BoxStream<'_, Vec<String>> {
            Box::pin(stream::iter(self.batches.clone()))
        }
    }

    struct MockExtentStore {
        deleted: Arc<Mutex<Vec<String>>>,
        initialized: Arc<AtomicBool>,
        closed: Arc<AtomicBool>,
        metadataStore: Arc<dyn IExtentMetadataStore + Send + Sync>,
    }

    impl MockExtentStore {
        fn new(deleted: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                deleted,
                initialized: Arc::new(AtomicBool::new(false)),
                closed: Arc::new(AtomicBool::new(true)),
                metadataStore: Arc::new(LokiExtentMetadata::new(String::new(), true)),
            }
        }
    }

    #[async_trait]
    impl IDataStore for MockExtentStore {
        async fn init(&mut self) -> Result<(), StorageError> {
            self.initialized.store(true, Ordering::SeqCst);
            self.closed.store(false, Ordering::SeqCst);
            Ok(())
        }

        fn isInitialized(&self) -> bool {
            self.initialized.load(Ordering::SeqCst)
        }

        async fn close(&mut self) -> Result<(), StorageError> {
            self.closed.store(true, Ordering::SeqCst);
            Ok(())
        }

        fn isClosed(&self) -> bool {
            self.closed.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl ICleaner for MockExtentStore {
        async fn clean(&mut self) -> Result<(), StorageError> {
            Ok(())
        }
    }

    #[async_trait]
    impl IExtentStore for MockExtentStore {
        async fn appendExtent(
            &mut self,
            _data: ExtentDataInput,
            _contextId: Option<&str>,
        ) -> Result<IExtentChunk, StorageError> {
            Err(StorageError::new(
                "appendExtent is not used in queue GC tests",
            ))
        }

        async fn readExtent(
            &self,
            _extentChunk: Option<&IExtentChunk>,
            _contextId: Option<&str>,
        ) -> Result<ReadableStream, StorageError> {
            Ok(Box::pin(empty()))
        }

        async fn readExtents(
            &self,
            _extentChunkArray: &[IExtentChunk],
            _offset: u64,
            _count: u64,
            _contextId: Option<&str>,
        ) -> Result<ReadableStream, StorageError> {
            Ok(Box::pin(empty()))
        }

        async fn deleteExtents(&mut self, persistency: Vec<String>) -> Result<u64, StorageError> {
            *self.deleted.lock().unwrap() = persistency.clone();
            Ok(persistency.len() as u64)
        }

        fn getMetadataStore(&self) -> Arc<dyn IExtentMetadataStore + Send + Sync> {
            self.metadataStore.clone()
        }
    }

    #[tokio::test]
    async fn mark_sweep_deletes_only_unreferenced_extents() {
        let logger: Arc<dyn ILogger + Send + Sync> = Arc::new(TestLogger);
        let status = Arc::new(Mutex::new(Status::Running));
        let deleted = Arc::new(Mutex::new(Vec::new()));
        let referredExtentsProvider: Arc<AsyncMutex<Box<dyn IGCExtentProvider + Send + Sync>>> =
            Arc::new(AsyncMutex::new(Box::new(MockExtentProvider::new(vec![
                vec!["extent-2".to_string()],
            ]))));
        let allExtentsProvider: Arc<AsyncMutex<Box<dyn IGCExtentProvider + Send + Sync>>> =
            Arc::new(AsyncMutex::new(Box::new(MockExtentProvider::new(vec![
                vec!["extent-1".to_string(), "extent-2".to_string()],
                vec!["extent-3".to_string()],
            ]))));
        let extentStore: Arc<AsyncMutex<Box<dyn IExtentStore + Send + Sync>>> = Arc::new(
            AsyncMutex::new(Box::new(MockExtentStore::new(deleted.clone()))),
        );

        QueueGCManager::markSweep(
            logger,
            status,
            referredExtentsProvider,
            allExtentsProvider,
            extentStore,
        )
        .await
        .unwrap();

        assert_eq!(
            *deleted.lock().unwrap(),
            vec!["extent-1".to_string(), "extent-3".to_string()]
        );
    }

    #[tokio::test]
    async fn start_and_close_transition_status_and_initialize_dependencies() {
        let logger: Arc<dyn ILogger + Send + Sync> = Arc::new(TestLogger);
        let referredExtentsProvider = MockExtentProvider::new(vec![Vec::new()]);
        let referredInitialized = referredExtentsProvider.initialized.clone();
        let allExtentsProvider = MockExtentProvider::new(vec![Vec::new()]);
        let allInitialized = allExtentsProvider.initialized.clone();
        let extentStore = MockExtentStore::new(Arc::new(Mutex::new(Vec::new())));
        let extentStoreInitialized = extentStore.initialized.clone();
        let errorHandled = Arc::new(AtomicBool::new(false));
        let errorHandledForClosure = errorHandled.clone();
        let mut manager = QueueGCManager::new(
            Box::new(referredExtentsProvider),
            Box::new(allExtentsProvider),
            Box::new(extentStore),
            Arc::new(move |_| {
                errorHandledForClosure.store(true, Ordering::SeqCst);
            }),
            logger,
            Some(60_000),
        );

        manager.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;

        assert_eq!(manager.status(), Status::Running);
        assert!(referredInitialized.load(Ordering::SeqCst));
        assert!(allInitialized.load(Ordering::SeqCst));
        assert!(extentStoreInitialized.load(Ordering::SeqCst));

        manager.close().await.unwrap();

        assert_eq!(manager.status(), Status::Closed);
        assert!(!errorHandled.load(Ordering::SeqCst));
    }
}
