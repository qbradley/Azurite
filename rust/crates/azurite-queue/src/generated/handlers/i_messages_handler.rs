use async_trait::async_trait;

use crate::generated::artifacts::models;
use crate::generated::context::Context;

#[allow(non_snake_case)]
#[async_trait]
pub trait IMessagesHandler: Send + Sync {
    async fn dequeue(
        &self,
        options: models::MessagesDequeueOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::MessagesDequeueResponse>;
    async fn clear(
        &self,
        options: models::MessagesClearOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::MessagesClearResponse>;
    async fn enqueue(
        &self,
        queueMessage: models::QueueMessage,
        options: models::MessagesEnqueueOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::MessagesEnqueueResponse>;
    async fn peek(
        &self,
        options: models::MessagesPeekOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::MessagesPeekResponse>;
}
