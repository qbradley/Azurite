use async_trait::async_trait;

use crate::generated::artifacts::models;
use crate::generated::context::Context;

#[allow(non_snake_case)]
#[async_trait]
pub trait IMessageIdHandler: Send + Sync {
    async fn update(
        &self,
        queueMessage: models::QueueMessage,
        popReceipt: String,
        visibilitytimeout: f64,
        options: models::MessageIdUpdateOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::MessageIdUpdateResponse>;
    async fn delete(
        &self,
        popReceipt: String,
        options: models::MessageIdDeleteMethodOptionalParams,
        context: Context,
    ) -> crate::generated::GeneratedResult<models::MessageIdDeleteResponse>;
}
