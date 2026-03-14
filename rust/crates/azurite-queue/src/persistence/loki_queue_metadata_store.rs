use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::{self, BoxStream};

use azurite_common::i_cleaner::ICleaner;
use azurite_common::i_data_store::IDataStore;
use azurite_common::i_gc_extent_provider::IGCExtentProvider;
use azurite_common::storage_error::StorageError as CommonStorageError;

use crate::errors::{StorageError, StorageErrorFactory};
use crate::generated::context::Context;
use crate::persistence::i_queue_metadata_store::{
    IQueueMetadata, IQueueMetadataStore, MessageModel, MessageUpdateProperties, QueueACL,
    QueueModel, ServicePropertiesModel,
};
use crate::persistence::queue_referred_extents_async_iterator::QueueReferredExtentsAsyncIterator;
use crate::utils::constants::QUEUE_STATUSCODE;

#[derive(Clone, Debug)]
struct StoredQueueModel {
    record_id: u64,
    model: QueueModel,
}

#[derive(Clone, Debug)]
struct StoredMessageModel {
    record_id: u64,
    model: MessageModel,
}

#[derive(Clone, Copy, Debug, Default)]
struct LifecycleState {
    initialized: bool,
    closed: bool,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug)]
pub struct LokiQueueMetadataStore {
    pub lokiDBPath: PathBuf,
    inMemory: bool,
    lifecycle: Arc<RwLock<LifecycleState>>,
    services_collection: Arc<RwLock<HashMap<String, ServicePropertiesModel>>>,
    queues_collection: Arc<RwLock<BTreeMap<(String, String), StoredQueueModel>>>,
    messages_collection: Arc<RwLock<BTreeMap<(String, String, String), StoredMessageModel>>>,
    next_queue_id: Arc<AtomicU64>,
    next_message_id: Arc<AtomicU64>,
}

#[allow(non_snake_case)]
impl LokiQueueMetadataStore {
    pub fn new(lokiDBPath: PathBuf, inMemory: bool) -> Self {
        Self {
            lokiDBPath,
            inMemory,
            lifecycle: Arc::new(RwLock::new(LifecycleState::default())),
            services_collection: Arc::new(RwLock::new(HashMap::new())),
            queues_collection: Arc::new(RwLock::new(BTreeMap::new())),
            messages_collection: Arc::new(RwLock::new(BTreeMap::new())),
            next_queue_id: Arc::new(AtomicU64::new(1)),
            next_message_id: Arc::new(AtomicU64::new(1)),
        }
    }

    fn request_id(context: Option<&Context>) -> Option<String> {
        context.and_then(|value| value.contextId())
    }

    fn request_time(queryDate: Option<DateTime<Utc>>, context: Option<&Context>) -> DateTime<Utc> {
        queryDate
            .or_else(|| context.and_then(|value| value.startTime()))
            .unwrap_or_else(Utc::now)
    }

    fn metadata_matches(existing: Option<&IQueueMetadata>, candidate: Option<&IQueueMetadata>) -> bool {
        match (existing, candidate) {
            (None, None) => true,
            (Some(_), None) | (None, Some(_)) => false,
            (Some(existing), Some(candidate)) => {
                if existing.len() != candidate.len() {
                    return false;
                }

                let candidate_lookup: HashMap<String, &String> = candidate
                    .iter()
                    .map(|(key, value)| (key.to_lowercase(), value))
                    .collect();

                existing.iter().all(|(key, value)| {
                    candidate_lookup
                        .get(&key.to_lowercase())
                        .is_some_and(|candidate_value| **candidate_value == *value)
                })
            }
        }
    }

    fn queue_key(account: &str, queue: &str) -> (String, String) {
        (account.to_string(), queue.to_string())
    }

    fn message_key(account: &str, queue: &str, messageId: &str) -> (String, String, String) {
        (
            account.to_string(),
            queue.to_string(),
            messageId.to_string(),
        )
    }

    fn checkQueueExist(&self, account: &str, queue: &str, context: Option<&Context>) -> Result<(), StorageError> {
        let queues = self.queues_collection.read().unwrap();
        if queues.contains_key(&Self::queue_key(account, queue)) {
            return Ok(());
        }

        let request_id = Self::request_id(context);
        Err(StorageErrorFactory::getQueueNotFound(request_id.as_deref()))
    }

    fn clearExpiredMessages(&self, account: &str, queue: &str, context: Option<&Context>) -> Result<(), StorageError> {
        self.checkQueueExist(account, queue, context)?;

        let query_time = Self::request_time(None, context);
        let mut messages = self.messages_collection.write().unwrap();
        let expired_keys: Vec<_> = messages
            .iter()
            .filter(|((message_account, message_queue, _), stored)| {
                message_account == account
                    && message_queue == queue
                    && stored.model.expirationTime <= query_time
            })
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_keys {
            messages.remove(&key);
        }

        Ok(())
    }
}

#[async_trait]
impl IDataStore for LokiQueueMetadataStore {
    async fn init(&mut self) -> Result<(), CommonStorageError> {
        let mut lifecycle = self.lifecycle.write().unwrap();
        lifecycle.initialized = true;
        lifecycle.closed = false;
        Ok(())
    }

    fn isInitialized(&self) -> bool {
        self.lifecycle.read().unwrap().initialized
    }

    async fn close(&mut self) -> Result<(), CommonStorageError> {
        self.lifecycle.write().unwrap().closed = true;
        Ok(())
    }

    fn isClosed(&self) -> bool {
        self.lifecycle.read().unwrap().closed
    }
}

#[async_trait]
impl ICleaner for LokiQueueMetadataStore {
    async fn clean(&mut self) -> Result<(), CommonStorageError> {
        if !self.isClosed() {
            return Err(CommonStorageError::new(
                "Cannot clean LokiQueueMetadataStore, it's not closed.",
            ));
        }

        if !self.inMemory {
            match fs::remove_file(&self.lokiDBPath) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(CommonStorageError::new(error.to_string())),
            }
        }

        Ok(())
    }
}

impl IGCExtentProvider for LokiQueueMetadataStore {
    fn iteratorExtents(&self) -> BoxStream<'_, Vec<String>> {
        let iterator = QueueReferredExtentsAsyncIterator::new(self.clone());
        Box::pin(stream::unfold(Some(iterator), |state| async move {
            let mut iterator = state?;
            match iterator.next().await {
                Ok((_, true)) => None,
                Ok((extents, false)) => Some((extents, Some(iterator))),
                Err(_) => None,
            }
        }))
    }
}

#[async_trait]
impl IQueueMetadataStore for LokiQueueMetadataStore {
    async fn updateServiceProperties(
        &self,
        updateProperties: ServicePropertiesModel,
    ) -> Result<(), StorageError> {
        let mut services = self.services_collection.write().unwrap();
        let entry = services
            .entry(updateProperties.accountName.clone())
            .or_insert_with(|| ServicePropertiesModel {
                accountName: updateProperties.accountName.clone(),
                properties: BTreeMap::new(),
            });

        entry.properties.extend(updateProperties.properties);
        Ok(())
    }

    async fn getServiceProperties(
        &self,
        account: &str,
    ) -> Result<Option<ServicePropertiesModel>, StorageError> {
        Ok(self.services_collection.read().unwrap().get(account).cloned())
    }

    async fn listQueues(
        &self,
        account: &str,
        prefix: Option<&str>,
        maxResults: Option<u64>,
        marker: Option<u64>,
    ) -> Result<(Vec<QueueModel>, Option<u64>), StorageError> {
        let prefix = prefix.unwrap_or_default();
        let marker = marker.unwrap_or(0);
        let max_results = maxResults.unwrap_or(5000) as usize;
        let queues = self.queues_collection.read().unwrap();

        let mut matching: Vec<_> = queues
            .values()
            .filter(|stored| {
                stored.record_id > marker
                    && stored.model.accountName == account
                    && (prefix.is_empty() || stored.model.name.starts_with(prefix))
            })
            .cloned()
            .collect();

        matching.sort_by(|left, right| {
            left.model
                .name
                .cmp(&right.model.name)
                .then(left.record_id.cmp(&right.record_id))
        });

        let mut limited = matching.into_iter().take(max_results + 1).collect::<Vec<_>>();
        if limited.len() <= max_results {
            return Ok((limited.into_iter().map(|stored| stored.model).collect(), None));
        }

        let next_marker = limited
            .last()
            .map(|stored| stored.record_id.saturating_sub(1));
        limited.truncate(max_results);
        Ok((
            limited.into_iter().map(|stored| stored.model).collect(),
            next_marker,
        ))
    }

    async fn getQueue(
        &self,
        account: &str,
        queue: &str,
        context: Option<&Context>,
    ) -> Result<QueueModel, StorageError> {
        let queues = self.queues_collection.read().unwrap();
        let key = Self::queue_key(account, queue);
        if let Some(stored) = queues.get(&key) {
            return Ok(stored.model.clone());
        }

        let request_id = Self::request_id(context);
        Err(StorageErrorFactory::getQueueNotFound(request_id.as_deref()))
    }

    async fn createQueue(
        &self,
        queue: QueueModel,
        context: Option<&Context>,
    ) -> Result<QUEUE_STATUSCODE, StorageError> {
        let key = Self::queue_key(&queue.accountName, &queue.name);
        let mut queues = self.queues_collection.write().unwrap();

        if let Some(existing) = queues.get(&key) {
            if Self::metadata_matches(existing.model.metadata.as_ref(), queue.metadata.as_ref()) {
                return Ok(QUEUE_STATUSCODE::NOCONTENT);
            }

            let request_id = Self::request_id(context);
            return Err(StorageErrorFactory::getQueueAlreadyExists(
                request_id.as_deref(),
            ));
        }

        let record_id = self.next_queue_id.fetch_add(1, Ordering::SeqCst);
        queues.insert(key, StoredQueueModel { record_id, model: queue });
        Ok(QUEUE_STATUSCODE::CREATED)
    }

    async fn deleteQueue(
        &self,
        account: &str,
        queue: &str,
        context: Option<&Context>,
    ) -> Result<(), StorageError> {
        let key = Self::queue_key(account, queue);
        let removed = self.queues_collection.write().unwrap().remove(&key);
        if removed.is_none() {
            let request_id = Self::request_id(context);
            return Err(StorageErrorFactory::getQueueNotFound(request_id.as_deref()));
        }

        self.messages_collection
            .write()
            .unwrap()
            .retain(|(message_account, message_queue, _), _| {
                !(message_account == account && message_queue == queue)
            });
        Ok(())
    }

    async fn setQueueACL(
        &self,
        account: &str,
        queue: &str,
        queueACL: Option<QueueACL>,
        context: Option<&Context>,
    ) -> Result<(), StorageError> {
        let key = Self::queue_key(account, queue);
        let mut queues = self.queues_collection.write().unwrap();
        let Some(stored) = queues.get_mut(&key) else {
            let request_id = Self::request_id(context);
            return Err(StorageErrorFactory::getQueueNotFound(request_id.as_deref()));
        };

        stored.model.queueAcl = queueACL;
        Ok(())
    }

    async fn setQueueMetadata(
        &self,
        account: &str,
        queue: &str,
        metadata: Option<IQueueMetadata>,
        context: Option<&Context>,
    ) -> Result<(), StorageError> {
        let key = Self::queue_key(account, queue);
        let mut queues = self.queues_collection.write().unwrap();
        let Some(stored) = queues.get_mut(&key) else {
            let request_id = Self::request_id(context);
            return Err(StorageErrorFactory::getQueueNotFound(request_id.as_deref()));
        };

        stored.model.metadata = metadata;
        Ok(())
    }

    async fn getMessagesCount(
        &self,
        account: &str,
        queue: &str,
        context: Option<&Context>,
    ) -> Result<u64, StorageError> {
        self.checkQueueExist(account, queue, context)?;

        Ok(self
            .messages_collection
            .read()
            .unwrap()
            .values()
            .filter(|stored| stored.model.accountName == account && stored.model.queueName == queue)
            .count() as u64)
    }

    async fn insertMessage(
        &self,
        message: MessageModel,
        context: Option<&Context>,
    ) -> Result<(), StorageError> {
        self.checkQueueExist(&message.accountName, &message.queueName, context)?;

        let key = Self::message_key(&message.accountName, &message.queueName, &message.messageId);
        let record_id = self.next_message_id.fetch_add(1, Ordering::SeqCst);
        self.messages_collection
            .write()
            .unwrap()
            .insert(key, StoredMessageModel { record_id, model: message });
        Ok(())
    }

    async fn peekMessages(
        &self,
        account: &str,
        queue: &str,
        numOfMessages: Option<u32>,
        queryDate: Option<DateTime<Utc>>,
        context: Option<&Context>,
    ) -> Result<Vec<MessageModel>, StorageError> {
        self.checkQueueExist(account, queue, context)?;
        self.clearExpiredMessages(account, queue, context)?;

        let query_time = Self::request_time(queryDate, context);
        let limit = numOfMessages.unwrap_or(1).max(1) as usize;
        let messages = self.messages_collection.read().unwrap();
        let mut visible: Vec<_> = messages
            .values()
            .filter(|stored| {
                stored.model.accountName == account
                    && stored.model.queueName == queue
                    && stored.model.timeNextVisible <= query_time
            })
            .cloned()
            .collect();

        visible.sort_by(|left, right| {
            left.model
                .timeNextVisible
                .cmp(&right.model.timeNextVisible)
                .then(left.record_id.cmp(&right.record_id))
        });

        Ok(visible
            .into_iter()
            .take(limit)
            .map(|stored| stored.model)
            .collect())
    }

    async fn getMessages(
        &self,
        account: &str,
        queue: &str,
        timeNextVisible: DateTime<Utc>,
        popReceipt: &str,
        numOfMessages: Option<u32>,
        queryDate: Option<DateTime<Utc>>,
        context: Option<&Context>,
    ) -> Result<Vec<MessageModel>, StorageError> {
        self.checkQueueExist(account, queue, context)?;
        self.clearExpiredMessages(account, queue, context)?;

        let query_time = Self::request_time(queryDate, context);
        let limit = numOfMessages.unwrap_or(1).max(1) as usize;
        let mut messages = self.messages_collection.write().unwrap();

        let mut visible_keys: Vec<_> = messages
            .iter()
            .filter(|((message_account, message_queue, _), stored)| {
                message_account == account
                    && message_queue == queue
                    && stored.model.timeNextVisible <= query_time
            })
            .map(|(key, stored)| {
                (
                    key.clone(),
                    stored.record_id,
                    stored.model.timeNextVisible,
                )
            })
            .collect();

        visible_keys.sort_by(|left, right| left.2.cmp(&right.2).then(left.1.cmp(&right.1)));

        let mut dequeued = Vec::new();
        for (key, _, _) in visible_keys.into_iter().take(limit) {
            if let Some(stored) = messages.get_mut(&key) {
                stored.model.timeNextVisible = timeNextVisible;
                stored.model.popReceipt = popReceipt.to_string();
                stored.model.dequeueCount += 1;
                dequeued.push(stored.model.clone());
            }
        }

        Ok(dequeued)
    }

    async fn deleteMessage(
        &self,
        account: &str,
        queue: &str,
        messageId: &str,
        validatingPopReceipt: &str,
        context: Option<&Context>,
    ) -> Result<(), StorageError> {
        self.checkQueueExist(account, queue, context)?;
        self.clearExpiredMessages(account, queue, context)?;

        let key = Self::message_key(account, queue, messageId);
        let mut messages = self.messages_collection.write().unwrap();
        let Some(stored) = messages.get(&key) else {
            let request_id = Self::request_id(context);
            return Err(StorageErrorFactory::getMessageNotFound(request_id.as_deref()));
        };

        if stored.model.popReceipt != validatingPopReceipt {
            let request_id = Self::request_id(context);
            return Err(StorageErrorFactory::getPopReceiptMismatch(request_id.as_deref()));
        }

        messages.remove(&key);
        Ok(())
    }

    async fn updateMessage(
        &self,
        message: MessageUpdateProperties,
        validatingPopReceipt: &str,
        context: Option<&Context>,
    ) -> Result<(), StorageError> {
        self.checkQueueExist(&message.accountName, &message.queueName, context)?;
        self.clearExpiredMessages(&message.accountName, &message.queueName, context)?;

        let key = Self::message_key(&message.accountName, &message.queueName, &message.messageId);
        let mut messages = self.messages_collection.write().unwrap();
        let Some(stored) = messages.get_mut(&key) else {
            let request_id = Self::request_id(context);
            return Err(StorageErrorFactory::getMessageNotFound(request_id.as_deref()));
        };

        if stored.model.popReceipt != validatingPopReceipt {
            let request_id = Self::request_id(context);
            return Err(StorageErrorFactory::getPopReceiptMismatch(request_id.as_deref()));
        }

        stored.model.popReceipt = message.popReceipt;
        stored.model.timeNextVisible = message.timeNextVisible;
        if let Some(persistency) = message.persistency {
            stored.model.persistency = persistency;
        }

        Ok(())
    }

    async fn clearMessages(
        &self,
        account: &str,
        queue: &str,
        context: Option<&Context>,
    ) -> Result<(), StorageError> {
        self.checkQueueExist(account, queue, context)?;

        self.messages_collection
            .write()
            .unwrap()
            .retain(|(message_account, message_queue, _), _| {
                !(message_account == account && message_queue == queue)
            });
        Ok(())
    }

    async fn listMessages(
        &self,
        maxResults: Option<u64>,
        marker: Option<u64>,
    ) -> Result<(Vec<MessageModel>, Option<u64>), StorageError> {
        let marker = marker.unwrap_or(0);
        let max_results = maxResults.unwrap_or(5000) as usize;
        let messages = self.messages_collection.read().unwrap();

        let mut listed: Vec<_> = messages
            .values()
            .filter(|stored| stored.record_id > marker)
            .cloned()
            .collect();
        listed.sort_by(|left, right| left.record_id.cmp(&right.record_id));
        listed.truncate(max_results);

        let next_marker = if listed.len() < max_results {
            None
        } else {
            listed.last().map(|stored| stored.record_id)
        };

        Ok((
            listed.into_iter().map(|stored| stored.model).collect(),
            next_marker,
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use chrono::{Duration, TimeZone, Utc};
    use futures::StreamExt;

    use azurite_common::i_data_store::IDataStore;
    use azurite_common::i_gc_extent_provider::IGCExtentProvider;
    use azurite_common::persistence::i_extent_store::IExtentChunk;

    use crate::generated::artifacts::models::{GeneratedValue, QueueItem, StorageServiceProperties};
    use crate::persistence::i_queue_metadata_store::{
        IQueueMetadataStore, MessageModel, MessageUpdateProperties, QueueModel,
        ServicePropertiesModel,
    };
    use crate::utils::constants::QUEUE_STATUSCODE;

    use super::LokiQueueMetadataStore;

    fn queue_model(account: &str, name: &str) -> QueueModel {
        QueueModel {
            accountName: account.to_string(),
            name: name.to_string(),
            metadata: None,
            queueAcl: None,
            properties: QueueItem::new(),
        }
    }

    fn extent(id: &str) -> IExtentChunk {
        IExtentChunk {
            id: id.to_string(),
            offset: 0,
            count: 1,
        }
    }

    fn message_model(account: &str, queue: &str, message_id: &str, visible_at: DateTime<Utc>) -> MessageModel {
        MessageModel {
            accountName: account.to_string(),
            queueName: queue.to_string(),
            messageId: message_id.to_string(),
            popReceipt: format!("pop-{message_id}"),
            timeNextVisible: visible_at,
            insertionTime: visible_at,
            expirationTime: visible_at + Duration::days(1),
            dequeueCount: 0,
            persistency: extent(message_id),
        }
    }

    #[tokio::test]
    async fn create_queue_reuses_matching_metadata_case_insensitively() {
        let mut store = LokiQueueMetadataStore::new(PathBuf::from("queue.db"), true);
        store.init().await.unwrap();

        let mut first = queue_model("devstoreaccount1", "orders");
        first.metadata = Some(BTreeMap::from([(String::from("Owner"), String::from("team-a"))]));
        assert_eq!(
            store.createQueue(first, None).await.unwrap(),
            QUEUE_STATUSCODE::CREATED
        );

        let mut duplicate = queue_model("devstoreaccount1", "orders");
        duplicate.metadata = Some(BTreeMap::from([(String::from("owner"), String::from("team-a"))]));
        assert_eq!(
            store.createQueue(duplicate, None).await.unwrap(),
            QUEUE_STATUSCODE::NOCONTENT
        );

        let mut conflict = queue_model("devstoreaccount1", "orders");
        conflict.metadata = Some(BTreeMap::from([(String::from("owner"), String::from("team-b"))]));
        let error = store.createQueue(conflict, None).await.unwrap_err();
        assert_eq!(error.storageErrorCode, "QueueAlreadyExists");
    }

    #[tokio::test]
    async fn peek_get_update_and_delete_messages_follow_visibility_and_receipts() {
        let mut store = LokiQueueMetadataStore::new(PathBuf::from("queue.db"), true);
        store.init().await.unwrap();
        store
            .createQueue(queue_model("devstoreaccount1", "orders"), None)
            .await
            .unwrap();

        let now = Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0).unwrap();
        let first = message_model("devstoreaccount1", "orders", "m1", now);
        let second = message_model("devstoreaccount1", "orders", "m2", now);
        store.insertMessage(first, None).await.unwrap();
        store.insertMessage(second, None).await.unwrap();

        let dequeued = store
            .getMessages(
                "devstoreaccount1",
                "orders",
                now + Duration::seconds(30),
                "receipt-1",
                Some(1),
                Some(now),
                None,
            )
            .await
            .unwrap();
        assert_eq!(dequeued.len(), 1);
        assert_eq!(dequeued[0].messageId, "m1");
        assert_eq!(dequeued[0].popReceipt, "receipt-1");
        assert_eq!(dequeued[0].dequeueCount, 1);

        let peeked = store
            .peekMessages("devstoreaccount1", "orders", Some(2), Some(now), None)
            .await
            .unwrap();
        assert_eq!(peeked.len(), 1);
        assert_eq!(peeked[0].messageId, "m2");

        let update = MessageUpdateProperties {
            accountName: String::from("devstoreaccount1"),
            queueName: String::from("orders"),
            messageId: String::from("m1"),
            popReceipt: String::from("receipt-2"),
            timeNextVisible: now + Duration::seconds(90),
            persistency: Some(extent("updated")),
        };
        store.updateMessage(update, "receipt-1", None).await.unwrap();

        let listed = store.listMessages(Some(10), None).await.unwrap().0;
        let updated = listed.into_iter().find(|message| message.messageId == "m1").unwrap();
        assert_eq!(updated.popReceipt, "receipt-2");
        assert_eq!(updated.persistency.id, "updated");

        let mismatch = store
            .deleteMessage("devstoreaccount1", "orders", "m1", "bad-receipt", None)
            .await
            .unwrap_err();
        assert_eq!(mismatch.storageErrorCode, "PopReceiptMismatch");

        store
            .deleteMessage("devstoreaccount1", "orders", "m1", "receipt-2", None)
            .await
            .unwrap();
        assert_eq!(
            store.getMessagesCount("devstoreaccount1", "orders", None)
                .await
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn service_properties_are_merged_per_account() {
        let mut store = LokiQueueMetadataStore::new(PathBuf::from("queue.db"), true);
        store.init().await.unwrap();

        let first = ServicePropertiesModel {
            accountName: String::from("devstoreaccount1"),
            properties: StorageServiceProperties::from([(
                String::from("logging"),
                GeneratedValue::String(String::from("enabled")),
            )]),
        };
        store.updateServiceProperties(first).await.unwrap();

        let second = ServicePropertiesModel {
            accountName: String::from("devstoreaccount1"),
            properties: StorageServiceProperties::from([(
                String::from("cors"),
                GeneratedValue::String(String::from("*")),
            )]),
        };
        store.updateServiceProperties(second).await.unwrap();

        let props = store
            .getServiceProperties("devstoreaccount1")
            .await
            .unwrap()
            .unwrap();
        assert!(props.properties.contains_key("logging"));
        assert!(props.properties.contains_key("cors"));
    }

    #[tokio::test]
    async fn iterator_extents_pages_over_message_chunks() {
        let mut store = LokiQueueMetadataStore::new(PathBuf::from("queue.db"), true);
        store.init().await.unwrap();
        store
            .createQueue(queue_model("devstoreaccount1", "orders"), None)
            .await
            .unwrap();

        let now = Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0).unwrap();
        for index in 0..1002 {
            let message = message_model(
                "devstoreaccount1",
                "orders",
                &format!("m-{index}"),
                now,
            );
            store.insertMessage(message, None).await.unwrap();
        }

        let batches = store.iteratorExtents().collect::<Vec<_>>().await;
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].len(), 1000);
        assert_eq!(batches[1].len(), 2);
        assert_eq!(batches[0][0], "m-0");
        assert_eq!(batches[1][1], "m-1001");
    }
}
