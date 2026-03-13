use async_trait::async_trait;
use azurite_common::{
    i_account_data_store::{IAccountDataStore, IAccountProperties},
    i_cleaner::ICleaner,
    i_data_store::IDataStore,
    storage_error::StorageError,
};
use pretty_assertions::assert_eq;

#[derive(Default)]
struct FakeAccountDataStore {
    initialized: bool,
    closed: bool,
    cleaned: bool,
    account: Option<IAccountProperties>,
}

#[async_trait]
impl IDataStore for FakeAccountDataStore {
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

#[async_trait]
impl ICleaner for FakeAccountDataStore {
    async fn clean(&mut self) -> Result<(), StorageError> {
        self.cleaned = true;
        Ok(())
    }
}

impl IAccountDataStore for FakeAccountDataStore {
    fn getAccount(&self, name: &str) -> Option<IAccountProperties> {
        self.account.clone().filter(|account| account.name == name)
    }
}

#[tokio::test]
async fn data_store_lifecycle_uses_explicit_init_and_close_state() {
    let mut store = FakeAccountDataStore::default();

    assert!(!store.isInitialized());
    assert!(!store.isClosed());

    store.init().await.expect("init should succeed");
    assert!(store.isInitialized());
    assert!(!store.isClosed());

    store.close().await.expect("close should succeed");
    assert!(store.isClosed());
}

#[tokio::test]
async fn cleaner_remains_distinct_from_close() {
    let mut store = FakeAccountDataStore::default();

    store.init().await.expect("init should succeed");
    store.close().await.expect("close should succeed");
    assert!(!store.cleaned);

    store.clean().await.expect("clean should succeed");
    assert!(store.cleaned);
}

#[tokio::test]
async fn account_lookup_returns_none_for_unknown_accounts() {
    let store = FakeAccountDataStore {
        account: Some(IAccountProperties {
            name: "devstoreaccount1".to_owned(),
            key1: vec![1, 2, 3],
            key2: None,
        }),
        ..Default::default()
    };

    assert_eq!(store.getAccount("missing"), None);
}

#[tokio::test]
async fn account_lookup_preserves_optional_secondary_key() {
    let key2 = vec![4, 5, 6];
    let store = FakeAccountDataStore {
        account: Some(IAccountProperties {
            name: "devstoreaccount1".to_owned(),
            key1: vec![1, 2, 3],
            key2: Some(key2.clone()),
        }),
        ..Default::default()
    };

    let account = store
        .getAccount("devstoreaccount1")
        .expect("account should exist");

    assert_eq!(account.name, "devstoreaccount1");
    assert_eq!(account.key1, vec![1, 2, 3]);
    assert_eq!(account.key2, Some(key2));
}
