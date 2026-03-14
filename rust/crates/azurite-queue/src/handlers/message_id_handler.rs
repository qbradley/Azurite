use std::collections::BTreeMap;

use async_trait::async_trait;
use bytes::Bytes;
use chrono::Duration;

use azurite_common::persistence::i_extent_store::ExtentDataInput;

use crate::context::queue_storage_context::QueueStorageContext;
use crate::errors::StorageErrorFactory;
use crate::generated::artifacts::models::{
    GeneratedResponse, MessageIdDeleteMethodOptionalParams, MessageIdDeleteResponse,
    MessageIdUpdateOptionalParams, MessageIdUpdateResponse, QueueMessage,
};
use crate::generated::context::Context;
use crate::generated::handlers::i_message_id_handler::IMessageIdHandler;
use crate::generated::i_request::IRequest;
use crate::persistence::MessageUpdateProperties;
use crate::utils::constants::{
    DEFAULT_UPDATE_VISIBILITYTIMEOUT, MESSAGETEXT_LENGTH_MAX, UPDATE_VISIBILITYTIMEOUT_MAX,
    UPDATE_VISIBILITYTIMEOUT_MIN,
};
use crate::utils::utils::{getPopReceipt, getUTF8ByteSize};

use super::base_handler::{extract_message_text, json_value, string_value, BaseHandler};

#[derive(Clone)]
pub struct MessageIdHandler {
    pub base: BaseHandler,
}

impl MessageIdHandler {
    pub fn new(base: BaseHandler) -> Self {
        Self { base }
    }
}

#[async_trait]
impl IMessageIdHandler for MessageIdHandler {
    async fn update(
        &self,
        queueMessage: QueueMessage,
        popReceipt: String,
        visibilitytimeout: f64,
        options: MessageIdUpdateOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<MessageIdUpdateResponse> {
        let queue_ctx = QueueStorageContext::new(&context);
        let account_name = queue_ctx.account().unwrap_or_default();
        let queue_name = queue_ctx.queue().unwrap_or_default();
        let message_id = queue_ctx.messageId().unwrap_or_default();
        let start_time = BaseHandler::start_time(&context);
        let raw_body = context.request().and_then(|request| request.getBody());

        let message_text =
            extract_message_text(&queueMessage, raw_body.as_deref()).map_err(|_| {
                Box::new(StorageErrorFactory::getInvalidXmlDocument(
                    context.contextId().as_deref(),
                    None,
                )) as Box<dyn std::error::Error + Send + Sync>
            })?;

        if let Some(message_text) = &message_text {
            if getUTF8ByteSize(message_text) > MESSAGETEXT_LENGTH_MAX {
                let mut details = BTreeMap::new();
                details.insert(String::from("MaxLimit"), MESSAGETEXT_LENGTH_MAX.to_string());
                return Err(Box::new(StorageErrorFactory::getRequestBodyTooLarge(
                    context.contextId().as_deref(),
                    Some(details),
                )));
            }
        }

        let visibility_timeout = visibilitytimeout as i32;
        if !(UPDATE_VISIBILITYTIMEOUT_MIN..=UPDATE_VISIBILITYTIMEOUT_MAX)
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
                UPDATE_VISIBILITYTIMEOUT_MIN.to_string(),
            );
            details.insert(
                String::from("MaximumAllowed"),
                UPDATE_VISIBILITYTIMEOUT_MAX.to_string(),
            );
            return Err(Box::new(
                StorageErrorFactory::getOutOfRangeQueryParameterValue(
                    context.contextId().as_deref(),
                    Some(details),
                ),
            ));
        }

        let new_pop_receipt = getPopReceipt(start_time);
        let time_next_visible = if visibility_timeout == DEFAULT_UPDATE_VISIBILITYTIMEOUT {
            start_time + Duration::seconds(DEFAULT_UPDATE_VISIBILITYTIMEOUT as i64)
        } else {
            start_time + Duration::seconds(visibility_timeout as i64)
        };

        let persistency = if let Some(message_text) = message_text {
            let extent_chunk = {
                let mut store = self.base.extentStore.lock().await;
                store
                    .appendExtent(
                        ExtentDataInput::Buffer(Bytes::from(message_text.into_bytes())),
                        context.contextId().as_deref(),
                    )
                    .await?
            };
            Some(extent_chunk)
        } else {
            None
        };

        self.base
            .metadataStore
            .updateMessage(
                MessageUpdateProperties {
                    accountName: account_name,
                    queueName: queue_name,
                    messageId: message_id,
                    popReceipt: new_pop_receipt.clone(),
                    timeNextVisible: time_next_visible,
                    persistency,
                },
                &popReceipt,
                Some(&context),
            )
            .await?;

        let mut response = GeneratedResponse::new(204);
        response.insert_field("popReceipt", string_value(new_pop_receipt));
        response.insert_field("timeNextVisible", json_value(time_next_visible));
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        Ok(response)
    }

    async fn delete(
        &self,
        popReceipt: String,
        options: MessageIdDeleteMethodOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<MessageIdDeleteResponse> {
        let queue_ctx = QueueStorageContext::new(&context);
        let account_name = queue_ctx.account().unwrap_or_default();
        let queue_name = queue_ctx.queue().unwrap_or_default();
        let message_id = queue_ctx.messageId().unwrap_or_default();

        self.base
            .metadataStore
            .deleteMessage(
                &account_name,
                &queue_name,
                &message_id,
                &popReceipt,
                Some(&context),
            )
            .await?;

        let mut response = GeneratedResponse::new(204);
        self.base
            .add_response_metadata(&mut response, &options, &context, true);
        Ok(response)
    }
}
