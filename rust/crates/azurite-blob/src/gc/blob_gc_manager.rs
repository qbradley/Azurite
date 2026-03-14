use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use azurite_common::{
    i_gc_extent_provider::IGCExtentProvider, i_gc_manager::IGCManager, i_logger::ILogger,
    persistence::i_extent_store::IExtentStore, storage_error::StorageError,
};
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

pub type BlobGCManagerErrorHandler = Arc<dyn Fn(StorageError) + Send + Sync + 'static>;

pub struct BlobGCManager {
    referredExtentsProvider: Arc<AsyncMutex<Box<dyn IGCExtentProvider + Send + Sync>>>,
    allExtentsProvider: Arc<AsyncMutex<Box<dyn IGCExtentProvider + Send + Sync>>>,
    extentStore: Arc<AsyncMutex<Box<dyn IExtentStore + Send + Sync>>>,
    errorHandler: BlobGCManagerErrorHandler,
    logger: Arc<dyn ILogger + Send + Sync>,
    pub gcIntervalInMS: u64,
    status: Arc<Mutex<Status>>,
    abort: Arc<Notify>,
    loopTask: Option<JoinHandle<()>>,
}

impl BlobGCManager {
    pub fn new(
        referredExtentsProvider: Box<dyn IGCExtentProvider + Send + Sync>,
        allExtentsProvider: Box<dyn IGCExtentProvider + Send + Sync>,
        extentStore: Box<dyn IExtentStore + Send + Sync>,
        errorHandler: BlobGCManagerErrorHandler,
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

    async fn markSweepLoop(
        logger: Arc<dyn ILogger + Send + Sync>,
        status: Arc<Mutex<Status>>,
        abort: Arc<Notify>,
        extentStore: Arc<AsyncMutex<Box<dyn IExtentStore + Send + Sync>>>,
        gcIntervalInMS: u64,
    ) -> Result<(), StorageError> {
        while *status.lock().unwrap() == Status::Running {
            logger.info(
                "BlobGCManager:markSweepLoop() Start next mark and sweep.",
                None,
            );
            let start = std::time::Instant::now();
            Self::markSweep(logger.clone(), status.clone(), extentStore.clone()).await?;
            logger.info(
                &format!(
                    "BlobGCManager:markSweepLoop() Mark and sweep finished, taken {}ms.",
                    start.elapsed().as_millis()
                ),
                None,
            );

            if *status.lock().unwrap() == Status::Running {
                logger.info(
                    &format!(
                        "BlobGCManager:markSweepLoop() Sleep for {}ms.",
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
        extentStore: Arc<AsyncMutex<Box<dyn IExtentStore + Send + Sync>>>,
    ) -> Result<(), StorageError> {
        logger.info("BlobGCManager:markSweep() Get all extents.", None);
        let allExtents = Self::getAllExtents(status.clone()).await?;
        logger.info(
            &format!(
                "BlobGCManager:markSweep() Got {} extents.",
                allExtents.len()
            ),
            None,
        );

        if *status.lock().unwrap() != Status::Running {
            return Ok(());
        }

        logger.info("BlobGCManager:markSweep() Get referred extents.", None);
        logger.info(
            &format!(
                "BlobGCManager:markSweep() Got referred extents, unreferenced extents count is {}.",
                allExtents.len()
            ),
            None,
        );

        if !allExtents.is_empty() {
            let deletedCount = extentStore
                .lock()
                .await
                .deleteExtents(allExtents.into_iter().collect())
                .await?;
            logger.info(
                &format!(
                    "BlobGCManager:markSweep() Deleted unreferenced {} extents, after excluding active write extents.",
                    deletedCount
                ),
                None,
            );
        }

        Ok(())
    }

    async fn getAllExtents(status: Arc<Mutex<Status>>) -> Result<BTreeSet<String>, StorageError> {
        if *status.lock().unwrap() != Status::Running {
            return Ok(BTreeSet::new());
        }

        Ok(BTreeSet::new())
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
impl IGCManager for BlobGCManager {
    async fn start(&mut self) -> Result<(), StorageError> {
        if self.status() == Status::Running {
            self.logger.info(
                "BlobGCManager:start() BlobGCManager successfully started. BlobGCManager is already in Running status.",
                None,
            );
            return Ok(());
        }

        if self.status() != Status::Closed {
            return Err(StorageError::new(format!(
                "BlobGCManager:start() BlobGCManager cannot start, current manager is under {:?}",
                self.status()
            )));
        }

        self.logger.info(
            "BlobGCManager:start() Starting BlobGCManager. Set status to Initializing.",
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
            "BlobGCManager:start() Trigger mark and sweep loop. Set status to Running.",
            None,
        );
        Self::set_status(&self.status, Status::Running);

        let logger = self.logger.clone();
        let status = self.status.clone();
        let abort = self.abort.clone();
        let extentStore = self.extentStore.clone();
        let errorHandler = self.errorHandler.clone();
        let gcIntervalInMS = self.gcIntervalInMS;
        self.loopTask = Some(tokio::spawn(async move {
            if let Err(error) = BlobGCManager::markSweepLoop(
                logger.clone(),
                status.clone(),
                abort,
                extentStore,
                gcIntervalInMS,
            )
            .await
            {
                logger.info(
                    &format!(
                        "BlobGCManager:start() Mark and sweep loop emits error: {}",
                        error.message
                    ),
                    None,
                );
                BlobGCManager::set_status(&status, Status::Closed);
                (errorHandler)(error);
            } else {
                logger.info("BlobGCManager:start() Mark and sweep loop is closed.", None);
                BlobGCManager::set_status(&status, Status::Closed);
            }
        }));

        self.logger.info(
            "BlobGCManager:start() BlobGCManager successfully started.",
            None,
        );
        Ok(())
    }

    async fn close(&mut self) -> Result<(), StorageError> {
        if self.status() == Status::Closed {
            self.logger.info(
                "BlobGCManager:close() BlobGCManager successfully closed. BlobGCManager is already in Closed status.",
                None,
            );
            return Ok(());
        }

        if self.status() != Status::Running {
            return Err(StorageError::new(format!(
                "BlobGCManager:close() BlobGCManager cannot close, current manager is under {:?}",
                self.status()
            )));
        }

        self.logger.info(
            "BlobGCManager:close() Start closing BlobGCManager. Set status to Closing.",
            None,
        );
        Self::set_status(&self.status, Status::Closing);
        self.abort.notify_waiters();

        if let Some(loopTask) = self.loopTask.take() {
            let _ = loopTask.await;
        }

        Self::set_status(&self.status, Status::Closed);
        self.logger.info(
            "BlobGCManager:close() BlobGCManager successfully closed. Set status to Closed.",
            None,
        );
        Ok(())
    }
}
