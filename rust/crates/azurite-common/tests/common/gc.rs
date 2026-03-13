use async_trait::async_trait;
use azurite_common::{
    i_data_store::IDataStore, i_gc_extent_provider::IGCExtentProvider, i_gc_manager::IGCManager,
    storage_error::StorageError,
};
use futures::{stream, stream::BoxStream, StreamExt};
use pretty_assertions::assert_eq;

#[derive(Default)]
struct GcManagerFixture {
    starts: usize,
    closes: usize,
}

#[async_trait]
impl IGCManager for GcManagerFixture {
    async fn start(&mut self) -> Result<(), StorageError> {
        self.starts += 1;
        Ok(())
    }

    async fn close(&mut self) -> Result<(), StorageError> {
        self.closes += 1;
        Ok(())
    }
}

fn assert_gc_manager<T: IGCManager>(_manager: &T) {}

#[tokio::test]
async fn gc_manager_requires_explicit_start_and_close() {
    let mut manager = GcManagerFixture::default();

    assert_gc_manager(&manager);

    manager.start().await.expect("start should succeed");
    manager.close().await.expect("close should succeed");

    assert_eq!(manager.starts, 1);
    assert_eq!(manager.closes, 1);
}

#[derive(Default)]
struct GcExtentProviderFixture {
    initialized: bool,
    closed: bool,
    batches: Vec<Vec<String>>,
}

#[allow(non_snake_case)]
#[async_trait]
impl IDataStore for GcExtentProviderFixture {
    async fn init(&mut self) -> Result<(), StorageError> {
        self.initialized = true;
        self.closed = false;
        Ok(())
    }

    fn isInitialized(&self) -> bool {
        self.initialized
    }

    async fn close(&mut self) -> Result<(), StorageError> {
        self.closed = true;
        Ok(())
    }

    fn isClosed(&self) -> bool {
        self.closed
    }
}

impl IGCExtentProvider for GcExtentProviderFixture {
    fn iteratorExtents(&self) -> BoxStream<'_, Vec<String>> {
        Box::pin(stream::iter(self.batches.clone()))
    }
}

fn assert_gc_extent_provider<T: IGCExtentProvider>(_provider: &T) {}

#[tokio::test]
async fn gc_extent_provider_keeps_batched_iteration_and_store_state() {
    let mut provider = GcExtentProviderFixture {
        batches: vec![vec![String::new(), "extent-2".to_owned()], vec![]],
        ..Default::default()
    };

    assert_gc_extent_provider(&provider);
    assert!(!provider.isInitialized());

    provider.init().await.expect("init should succeed");
    assert!(provider.isInitialized());

    let batches = provider.iteratorExtents().collect::<Vec<_>>().await;
    assert_eq!(
        batches,
        vec![vec![String::new(), "extent-2".to_owned()], vec![]]
    );

    provider.close().await.expect("close should succeed");
    assert!(provider.isClosed());
}
