use super::{IMessageIdHandler, IMessagesHandler, IQueueHandler, IServiceHandler};

#[allow(non_snake_case)]
pub trait IHandlers: Send + Sync {
    fn serviceHandler(&self) -> &(dyn IServiceHandler + Send + Sync);
    fn queueHandler(&self) -> &(dyn IQueueHandler + Send + Sync);
    fn messagesHandler(&self) -> &(dyn IMessagesHandler + Send + Sync);
    fn messageIdHandler(&self) -> &(dyn IMessageIdHandler + Send + Sync);
}
