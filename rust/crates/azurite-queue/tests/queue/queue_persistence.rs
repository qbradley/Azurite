#![allow(non_snake_case)]
//! Parity tests for LokiQueueMetadataStore CRUD operations.
//! Validates queue creation, deletion, message lifecycle, and service properties.

use azurite_common::i_data_store::IDataStore;
use azurite_queue::persistence::i_queue_metadata_store::{
    IQueueMetadataStore, QueueModel, ServicePropertiesModel,
};
use azurite_queue::persistence::loki_queue_metadata_store::LokiQueueMetadataStore;
use std::path::PathBuf;

async fn make_store() -> LokiQueueMetadataStore {
    let mut store = LokiQueueMetadataStore::new(PathBuf::from(""), true);
    store.init().await.expect("init should succeed");
    store
}

fn queue(name: &str) -> QueueModel {
    QueueModel {
        accountName: "devstoreaccount1".to_string(),
        name: name.to_string(),
        ..Default::default()
    }
}

#[tokio::test]
async fn create_and_get_queue() {
    let store = make_store().await;
    store
        .createQueue(queue("testqueue"), None)
        .await
        .expect("createQueue should succeed");

    let q = store
        .getQueue("devstoreaccount1", "testqueue", None)
        .await
        .expect("getQueue should succeed");

    assert_eq!(q.name, "testqueue");
    assert_eq!(q.accountName, "devstoreaccount1");
}

#[tokio::test]
async fn list_queues_returns_created_queues() {
    let store = make_store().await;
    for name in &["alpha", "bravo", "charlie"] {
        store
            .createQueue(queue(name), None)
            .await
            .expect("createQueue should succeed");
    }

    let (queues, _) = store
        .listQueues("devstoreaccount1", None, None, None)
        .await
        .expect("listQueues should succeed");

    assert_eq!(queues.len(), 3);
}

#[tokio::test]
async fn delete_queue_removes_it() {
    let store = make_store().await;
    store
        .createQueue(queue("todelete"), None)
        .await
        .unwrap();

    store
        .deleteQueue("devstoreaccount1", "todelete", None)
        .await
        .expect("deleteQueue should succeed");

    let result = store
        .getQueue("devstoreaccount1", "todelete", None)
        .await;

    assert!(result.is_err(), "queue should not exist after deletion");
}

#[tokio::test]
async fn service_properties_round_trip() {
    let store = make_store().await;
    let props = ServicePropertiesModel {
        accountName: "devstoreaccount1".to_string(),
        ..Default::default()
    };

    store
        .updateServiceProperties(props)
        .await
        .expect("updateServiceProperties should succeed");

    let result = store
        .getServiceProperties("devstoreaccount1")
        .await
        .expect("getServiceProperties should succeed");

    assert!(result.is_some(), "service properties should exist");
}

#[tokio::test]
async fn message_count_starts_at_zero() {
    let store = make_store().await;
    store
        .createQueue(queue("emptyqueue"), None)
        .await
        .unwrap();

    let count = store
        .getMessagesCount("devstoreaccount1", "emptyqueue", None)
        .await
        .expect("getMessagesCount should succeed");

    assert_eq!(count, 0, "new queue should have 0 messages");
}

#[tokio::test]
async fn list_queues_with_prefix_filter() {
    let store = make_store().await;
    for name in &["test-alpha", "test-bravo", "other-charlie"] {
        store.createQueue(queue(name), None).await.unwrap();
    }

    let (queues, _) = store
        .listQueues("devstoreaccount1", Some("test-"), None, None)
        .await
        .expect("listQueues should succeed");

    assert_eq!(queues.len(), 2, "only test- prefixed queues should be returned");
}

