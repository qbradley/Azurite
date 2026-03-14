use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use azurite_common::i_cleaner::ICleaner;
use azurite_common::i_gc_extent_provider::IGCExtentProvider;
use azurite_common::persistence::i_extent_store::IExtentChunk;

use crate::errors::StorageError;
use crate::generated::artifacts::models::{QueueItem, SignedIdentifier, StorageServiceProperties};
use crate::generated::context::Context;
use crate::utils::constants::QUEUE_STATUSCODE;

pub type QueueACL = Vec<SignedIdentifier>;
pub type IQueueMetadata = BTreeMap<String, String>;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct ServicePropertiesModel {
    pub accountName: String,
    pub properties: StorageServiceProperties,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
pub struct QueueModel {
    pub accountName: String,
    pub name: String,
    pub metadata: Option<IQueueMetadata>,
    pub queueAcl: Option<QueueACL>,
    pub properties: QueueItem,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MessageUpdateProperties {
    pub accountName: String,
    pub queueName: String,
    pub messageId: String,
    pub popReceipt: String,
    pub timeNextVisible: DateTime<Utc>,
    pub persistency: Option<IExtentChunk>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MessageModel {
    pub accountName: String,
    pub queueName: String,
    pub messageId: String,
    pub popReceipt: String,
    pub timeNextVisible: DateTime<Utc>,
    pub insertionTime: DateTime<Utc>,
    pub expirationTime: DateTime<Utc>,
    pub dequeueCount: u32,
    pub persistency: IExtentChunk,
}

#[allow(non_snake_case)]
#[async_trait]
pub trait IQueueMetadataStore: IGCExtentProvider + ICleaner + Send + Sync {
    async fn updateServiceProperties(
        &self,
        updateProperties: ServicePropertiesModel,
    ) -> Result<(), StorageError>;

    async fn getServiceProperties(
        &self,
        account: &str,
    ) -> Result<Option<ServicePropertiesModel>, StorageError>;

    async fn listQueues(
        &self,
        account: &str,
        prefix: Option<&str>,
        maxResults: Option<u64>,
        marker: Option<u64>,
    ) -> Result<(Vec<QueueModel>, Option<u64>), StorageError>;

    async fn getQueue(
        &self,
        account: &str,
        queue: &str,
        context: Option<&Context>,
    ) -> Result<QueueModel, StorageError>;

    async fn createQueue(
        &self,
        queue: QueueModel,
        context: Option<&Context>,
    ) -> Result<QUEUE_STATUSCODE, StorageError>;

    async fn deleteQueue(
        &self,
        account: &str,
        queue: &str,
        context: Option<&Context>,
    ) -> Result<(), StorageError>;

    async fn setQueueACL(
        &self,
        account: &str,
        queue: &str,
        queueACL: Option<QueueACL>,
        context: Option<&Context>,
    ) -> Result<(), StorageError>;

    async fn setQueueMetadata(
        &self,
        account: &str,
        queue: &str,
        metadata: Option<IQueueMetadata>,
        context: Option<&Context>,
    ) -> Result<(), StorageError>;

    async fn getMessagesCount(
        &self,
        account: &str,
        queue: &str,
        context: Option<&Context>,
    ) -> Result<u64, StorageError>;

    async fn insertMessage(
        &self,
        message: MessageModel,
        context: Option<&Context>,
    ) -> Result<(), StorageError>;

    async fn peekMessages(
        &self,
        account: &str,
        queue: &str,
        numOfMessages: Option<u32>,
        queryDate: Option<DateTime<Utc>>,
        context: Option<&Context>,
    ) -> Result<Vec<MessageModel>, StorageError>;

    async fn getMessages(
        &self,
        account: &str,
        queue: &str,
        timeNextVisible: DateTime<Utc>,
        popReceipt: &str,
        numOfMessages: Option<u32>,
        queryDate: Option<DateTime<Utc>>,
        context: Option<&Context>,
    ) -> Result<Vec<MessageModel>, StorageError>;

    async fn deleteMessage(
        &self,
        account: &str,
        queue: &str,
        messageId: &str,
        validatingPopReceipt: &str,
        context: Option<&Context>,
    ) -> Result<(), StorageError>;

    async fn updateMessage(
        &self,
        message: MessageUpdateProperties,
        validatingPopReceipt: &str,
        context: Option<&Context>,
    ) -> Result<(), StorageError>;

    async fn clearMessages(
        &self,
        account: &str,
        queue: &str,
        context: Option<&Context>,
    ) -> Result<(), StorageError>;

    async fn listMessages(
        &self,
        maxResults: Option<u64>,
        marker: Option<u64>,
    ) -> Result<(Vec<MessageModel>, Option<u64>), StorageError>;
}
