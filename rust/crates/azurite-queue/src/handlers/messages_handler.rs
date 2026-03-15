use std::collections::BTreeMap;

use async_trait::async_trait;
use bytes::Bytes;
use chrono::Duration;
use uuid::Uuid;

use azurite_common::persistence::i_extent_store::ExtentDataInput;

use crate::context::queue_storage_context::QueueStorageContext;
use crate::errors::StorageErrorFactory;
use crate::generated::artifacts::models::{
    GeneratedBody, GeneratedObject, GeneratedResponse, GeneratedValue, MessagesClearOptionalParams,
    MessagesClearResponse, MessagesDequeueOptionalParams, MessagesDequeueResponse,
    MessagesEnqueueOptionalParams, MessagesEnqueueResponse, MessagesPeekOptionalParams,
    MessagesPeekResponse, QueueMessage,
};
use crate::generated::context::Context;
use crate::generated::handlers::i_messages_handler::IMessagesHandler;
use crate::generated::i_request::IRequest;
use crate::persistence::MessageModel;
use crate::utils::constants::{
    DEFAULT_DEQUEUE_VISIBILITYTIMEOUT, DEFAULT_MESSAGETTL, DEQUEUE_NUMOFMESSAGES_MAX,
    DEQUEUE_NUMOFMESSAGES_MIN, DEQUEUE_VISIBILITYTIMEOUT_MAX, DEQUEUE_VISIBILITYTIMEOUT_MIN,
    EMPTY_EXTENT_CHUNK, ENQUEUE_VISIBILITYTIMEOUT_MAX, ENQUEUE_VISIBILITYTIMEOUT_MIN,
    MESSAGETEXT_LENGTH_MAX, MESSAGETTL_MIN, NEVER_EXPIRE_DATE,
};
use crate::utils::utils::{getPopReceipt, getUTF8ByteSize, readStreamToString};

use super::base_handler::{extract_message_text, get_i32, rfc1123_value, string_value, BaseHandler};

#[derive(Clone)]
pub struct MessagesHandler {
    pub base: BaseHandler,
}

impl MessagesHandler {
    pub fn new(base: BaseHandler) -> Self {
        Self { base }
    }
}

#[async_trait]
impl IMessagesHandler for MessagesHandler {
    async fn dequeue(
        &self,
        options: MessagesDequeueOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<MessagesDequeueResponse> {
        let queue_ctx = QueueStorageContext::new(&context);
        let account_name = queue_ctx.account().unwrap_or_default();
        let queue_name = queue_ctx.queue().unwrap_or_default();
        let start_time = BaseHandler::start_time(&context);
        let pop_receipt = getPopReceipt(start_time);

        let mut time_next_visible =
            start_time + Duration::seconds(DEFAULT_DEQUEUE_VISIBILITYTIMEOUT as i64);
        if let Some(visibility_timeout) = get_i32(&options, "visibilitytimeout") {
            if !(DEQUEUE_VISIBILITYTIMEOUT_MIN..=DEQUEUE_VISIBILITYTIMEOUT_MAX)
                .contains(&visibility_timeout)
            {
                let mut details = BTreeMap::new();
                details.insert(
                    String::from("QueryParameterName"),
                    String::from("visibilitytimeout"),
                );
                details.insert(
                    String::from("QueryParameterValue"),
                    visibility_timeout.to_string(),
                );
                details.insert(
                    String::from("MinimumAllowed"),
                    DEQUEUE_VISIBILITYTIMEOUT_MIN.to_string(),
                );
                details.insert(
                    String::from("MaximumAllowed"),
                    DEQUEUE_VISIBILITYTIMEOUT_MAX.to_string(),
                );
                return Err(Box::new(
                    StorageErrorFactory::getOutOfRangeQueryParameterValue(
                        context.contextId().as_deref(),
                        Some(details),
                    ),
                ));
            }
            time_next_visible = start_time + Duration::seconds(visibility_timeout as i64);
        }

        let mut number_of_messages = 1;
        if let Some(value) = get_i32(&options, "numberOfMessages") {
            if !(DEQUEUE_NUMOFMESSAGES_MIN..=DEQUEUE_NUMOFMESSAGES_MAX).contains(&value) {
                let mut details = BTreeMap::new();
                details.insert(
                    String::from("QueryParameterName"),
                    String::from("numofmessages"),
                );
                details.insert(String::from("QueryParameterValue"), value.to_string());
                details.insert(
                    String::from("MinimumAllowed"),
                    DEQUEUE_NUMOFMESSAGES_MIN.to_string(),
                );
                details.insert(
                    String::from("MaximumAllowed"),
                    DEQUEUE_NUMOFMESSAGES_MAX.to_string(),
                );
                return Err(Box::new(
                    StorageErrorFactory::getOutOfRangeQueryParameterValue(
                        context.contextId().as_deref(),
                        Some(details),
                    ),
                ));
            }
            number_of_messages = value;
        }

        let messages = self
            .base
            .metadataStore
            .getMessages(
                &account_name,
                &queue_name,
                time_next_visible,
                &pop_receipt,
                Some(number_of_messages as u32),
                Some(start_time),
                Some(&context),
            )
            .await?;

        let mut dequeued_messages = Vec::with_capacity(messages.len());
        for message in messages {
            let text =
                read_message_text(&self.base, &message, context.contextId().as_deref()).await?;
            dequeued_messages.push(GeneratedValue::Object(dequeued_message_to_object(
                &message, text,
            )));
        }

        let mut response = GeneratedResponse::new(200);
        response.body = Some(GeneratedBody::Value(GeneratedValue::Array(
            dequeued_messages,
        )));
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        Ok(response)
    }

    async fn clear(
        &self,
        options: MessagesClearOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<MessagesClearResponse> {
        let queue_ctx = QueueStorageContext::new(&context);
        let account_name = queue_ctx.account().unwrap_or_default();
        let queue_name = queue_ctx.queue().unwrap_or_default();

        self.base
            .metadataStore
            .clearMessages(&account_name, &queue_name, Some(&context))
            .await?;

        let mut response = GeneratedResponse::new(204);
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        Ok(response)
    }

    async fn enqueue(
        &self,
        queueMessage: QueueMessage,
        options: MessagesEnqueueOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<MessagesEnqueueResponse> {
        let queue_ctx = QueueStorageContext::new(&context);
        let account_name = queue_ctx.account().unwrap_or_default();
        let queue_name = queue_ctx.queue().unwrap_or_default();
        let start_time = BaseHandler::start_time(&context);
        let raw_body = context.request().and_then(|request| request.getBody());

        let Some(message_text) =
            extract_message_text(&queueMessage, raw_body.as_deref()).map_err(|_| {
                Box::new(StorageErrorFactory::getInvalidXmlDocument(
                    context.contextId().as_deref(),
                    None,
                )) as Box<dyn std::error::Error + Send + Sync>
            })?
        else {
            return Err(Box::new(StorageErrorFactory::getInvalidXmlDocument(
                context.contextId().as_deref(),
                None,
            )));
        };

        if getUTF8ByteSize(&message_text) > MESSAGETEXT_LENGTH_MAX {
            let mut details = BTreeMap::new();
            details.insert(String::from("MaxLimit"), MESSAGETEXT_LENGTH_MAX.to_string());
            return Err(Box::new(StorageErrorFactory::getRequestBodyTooLarge(
                context.contextId().as_deref(),
                Some(details),
            )));
        }

        let mut message = MessageModel {
            accountName: account_name,
            queueName: queue_name,
            messageId: Uuid::new_v4().to_string(),
            insertionTime: start_time,
            expirationTime: start_time + Duration::seconds(DEFAULT_MESSAGETTL as i64),
            dequeueCount: 0,
            timeNextVisible: start_time,
            popReceipt: getPopReceipt(start_time),
            persistency: (*EMPTY_EXTENT_CHUNK).clone(),
        };

        if let Some(visibility_timeout) = get_i32(&options, "visibilitytimeout") {
            if !(ENQUEUE_VISIBILITYTIMEOUT_MIN..=ENQUEUE_VISIBILITYTIMEOUT_MAX)
                .contains(&visibility_timeout)
            {
                let mut details = BTreeMap::new();
                details.insert(
                    String::from("QueryParameterName"),
                    String::from("visibilitytimeout"),
                );
                details.insert(
                    String::from("QueryParameterValue"),
                    visibility_timeout.to_string(),
                );
                details.insert(
                    String::from("MinimumAllowed"),
                    ENQUEUE_VISIBILITYTIMEOUT_MIN.to_string(),
                );
                details.insert(
                    String::from("MaximumAllowed"),
                    ENQUEUE_VISIBILITYTIMEOUT_MAX.to_string(),
                );
                return Err(Box::new(
                    StorageErrorFactory::getOutOfRangeQueryParameterValue(
                        context.contextId().as_deref(),
                        Some(details),
                    ),
                ));
            }
            message.timeNextVisible = start_time + Duration::seconds(visibility_timeout as i64);
        }

        if let Some(message_ttl) = get_i32(&options, "messageTimeToLive") {
            if message_ttl == -1 {
                message.expirationTime = *NEVER_EXPIRE_DATE;
            } else if message_ttl < MESSAGETTL_MIN {
                let mut details = BTreeMap::new();
                details.insert(
                    String::from("QueryParameterName"),
                    String::from("messagettl"),
                );
                details.insert(String::from("QueryParameterValue"), message_ttl.to_string());
                details.insert(
                    String::from("Reason"),
                    String::from(
                        "Value must be greater than or equal to 1, or -1 to indicate an infinite TTL.",
                    ),
                );
                return Err(Box::new(
                    StorageErrorFactory::getInvalidQueryParameterValue(
                        context.contextId().as_deref(),
                        Some(details),
                    ),
                ));
            } else if let Some(visibility_timeout) = get_i32(&options, "visibilitytimeout") {
                if visibility_timeout >= message_ttl {
                    let mut details = BTreeMap::new();
                    details.insert(
                        String::from("QueryParameterName"),
                        String::from("visibilitytimeout"),
                    );
                    details.insert(
                        String::from("QueryParameterValue"),
                        visibility_timeout.to_string(),
                    );
                    details.insert(
                        String::from("Reason"),
                        String::from("messagettl must be greater than visibilitytimeout."),
                    );
                    return Err(Box::new(
                        StorageErrorFactory::getInvalidQueryParameterValue(
                            context.contextId().as_deref(),
                            Some(details),
                        ),
                    ));
                }
            }

            if *NEVER_EXPIRE_DATE - Duration::seconds(message_ttl as i64) <= start_time {
                message.expirationTime = *NEVER_EXPIRE_DATE;
            } else {
                message.expirationTime = start_time + Duration::seconds(message_ttl as i64);
            }
        }

        let extent_chunk = {
            let mut store = self.base.extentStore.lock().await;
            store
                .appendExtent(
                    ExtentDataInput::Buffer(Bytes::from(message_text.clone().into_bytes())),
                    context.contextId().as_deref(),
                )
                .await?
        };
        message.persistency = extent_chunk;

        self.base
            .metadataStore
            .insertMessage(message.clone(), Some(&context))
            .await?;

        let mut response = GeneratedResponse::new(201);
        response.body = Some(GeneratedBody::Value(GeneratedValue::Array(vec![
            GeneratedValue::Object(enqueued_message_to_object(&message)),
        ])));
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        Ok(response)
    }

    async fn peek(
        &self,
        options: MessagesPeekOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<MessagesPeekResponse> {
        let queue_ctx = QueueStorageContext::new(&context);
        let account_name = queue_ctx.account().unwrap_or_default();
        let queue_name = queue_ctx.queue().unwrap_or_default();
        let start_time = BaseHandler::start_time(&context);

        let mut number_of_messages = 1;
        if let Some(value) = get_i32(&options, "numberOfMessages") {
            if !(DEQUEUE_NUMOFMESSAGES_MIN..=DEQUEUE_NUMOFMESSAGES_MAX).contains(&value) {
                let mut details = BTreeMap::new();
                details.insert(
                    String::from("QueryParameterName"),
                    String::from("numofmessages"),
                );
                details.insert(String::from("QueryParameterValue"), value.to_string());
                details.insert(
                    String::from("MinimumAllowed"),
                    DEQUEUE_NUMOFMESSAGES_MIN.to_string(),
                );
                details.insert(
                    String::from("MaximumAllowed"),
                    DEQUEUE_NUMOFMESSAGES_MAX.to_string(),
                );
                return Err(Box::new(
                    StorageErrorFactory::getOutOfRangeQueryParameterValue(
                        context.contextId().as_deref(),
                        Some(details),
                    ),
                ));
            }
            number_of_messages = value;
        }

        let messages = self
            .base
            .metadataStore
            .peekMessages(
                &account_name,
                &queue_name,
                Some(number_of_messages as u32),
                Some(start_time),
                Some(&context),
            )
            .await?;

        let mut peeked_messages = Vec::with_capacity(messages.len());
        for message in messages {
            let text =
                read_message_text(&self.base, &message, context.contextId().as_deref()).await?;
            peeked_messages.push(GeneratedValue::Object(peeked_message_to_object(
                &message, text,
            )));
        }

        let mut response = GeneratedResponse::new(200);
        response.body = Some(GeneratedBody::Value(GeneratedValue::Array(peeked_messages)));
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        Ok(response)
    }
}

async fn read_message_text(
    base: &BaseHandler,
    message: &MessageModel,
    context_id: Option<&str>,
) -> crate::generated::GeneratedResult<String> {
    let stream = {
        let store = base.extentStore.lock().await;
        store
            .readExtent(Some(&message.persistency), context_id)
            .await?
    };
    Ok(readStreamToString(stream).await?)
}

fn enqueued_message_to_object(message: &MessageModel) -> GeneratedObject {
    GeneratedObject::from([
        (
            String::from("messageId"),
            string_value(message.messageId.clone()),
        ),
        (
            String::from("insertionTime"),
            rfc1123_value(message.insertionTime),
        ),
        (
            String::from("expirationTime"),
            rfc1123_value(message.expirationTime),
        ),
        (
            String::from("popReceipt"),
            string_value(message.popReceipt.clone()),
        ),
        (
            String::from("timeNextVisible"),
            rfc1123_value(message.timeNextVisible),
        ),
    ])
}

fn dequeued_message_to_object(message: &MessageModel, text: String) -> GeneratedObject {
    let mut value = enqueued_message_to_object(message);
    value.insert(
        String::from("dequeueCount"),
        GeneratedValue::Number(message.dequeueCount as f64),
    );
    value.insert(String::from("messageText"), string_value(text));
    value
}

fn peeked_message_to_object(message: &MessageModel, text: String) -> GeneratedObject {
    GeneratedObject::from([
        (
            String::from("messageId"),
            string_value(message.messageId.clone()),
        ),
        (
            String::from("insertionTime"),
            rfc1123_value(message.insertionTime),
        ),
        (
            String::from("expirationTime"),
            rfc1123_value(message.expirationTime),
        ),
        (
            String::from("dequeueCount"),
            GeneratedValue::Number(message.dequeueCount as f64),
        ),
        (String::from("messageText"), string_value(text)),
    ])
}
