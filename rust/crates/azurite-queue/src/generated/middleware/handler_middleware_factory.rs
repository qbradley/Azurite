use std::sync::Arc;

use crate::generated::artifacts::models::{GeneratedObject, GeneratedValue};
use crate::generated::artifacts::operation::Operation;
use crate::generated::context::Context;
use crate::generated::errors::middleware_error::MiddlewareError;
use crate::generated::handlers::{getHandlerByOperation, IHandlers};
use crate::generated::utils::i_logger::ILogger;

pub struct HandlerMiddlewareFactory<H: IHandlers, L: ILogger + ?Sized> {
    handlers: Arc<H>,
    logger: Arc<L>,
}

impl<H: IHandlers, L: ILogger + ?Sized> HandlerMiddlewareFactory<H, L> {
    pub fn new(handlers: Arc<H>, logger: Arc<L>) -> Self {
        Self { handlers, logger }
    }

    pub async fn call(&self, context: &Context) -> crate::generated::GeneratedResult<()> {
        self.logger.info(
            &format!(
                "HandlerMiddleware: DeserializedParameters= {}",
                format_handler_parameters(context.handlerParameters().as_ref())
            ),
            context.contextId().as_deref(),
        );

        let operation = if let Some(operation) = context.operation() {
            operation
        } else {
            let handlerError = MiddlewareError::new(500, "Operation is undefined.");
            self.logger.error(
                &format!("HandlerMiddleware: {}", handlerError.message),
                context.contextId().as_deref(),
            );
            return Err(Box::new(handlerError));
        };

        let _handlerPath = getHandlerByOperation(operation);
        let handler_parameters = context.handlerParameters().unwrap_or_default();
        let response = match operation {
            Operation::Service_SetProperties => {
                let storageServiceProperties =
                    extract_object(&handler_parameters, "storageServiceProperties")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .serviceHandler()
                    .setProperties(storageServiceProperties, options, context.clone())
                    .await
            }

            Operation::Service_GetProperties => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .serviceHandler()
                    .getProperties(options, context.clone())
                    .await
            }

            Operation::Service_GetStatistics => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .serviceHandler()
                    .getStatistics(options, context.clone())
                    .await
            }

            Operation::Service_ListQueuesSegment => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .serviceHandler()
                    .listQueuesSegment(options, context.clone())
                    .await
            }

            Operation::Queue_Create => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .queueHandler()
                    .create(options, context.clone())
                    .await
            }

            Operation::Queue_Delete => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .queueHandler()
                    .delete(options, context.clone())
                    .await
            }

            Operation::Queue_GetProperties => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .queueHandler()
                    .getProperties(options, context.clone())
                    .await
            }

            Operation::Queue_GetPropertiesWithHead => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .queueHandler()
                    .getPropertiesWithHead(options, context.clone())
                    .await
            }

            Operation::Queue_SetMetadata => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .queueHandler()
                    .setMetadata(options, context.clone())
                    .await
            }

            Operation::Queue_GetAccessPolicy => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .queueHandler()
                    .getAccessPolicy(options, context.clone())
                    .await
            }

            Operation::Queue_GetAccessPolicyWithHead => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .queueHandler()
                    .getAccessPolicyWithHead(options, context.clone())
                    .await
            }

            Operation::Queue_SetAccessPolicy => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .queueHandler()
                    .setAccessPolicy(options, context.clone())
                    .await
            }

            Operation::Messages_Dequeue => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .messagesHandler()
                    .dequeue(options, context.clone())
                    .await
            }

            Operation::Messages_Clear => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .messagesHandler()
                    .clear(options, context.clone())
                    .await
            }

            Operation::Messages_Enqueue => {
                let queueMessage = extract_object(&handler_parameters, "queueMessage")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .messagesHandler()
                    .enqueue(queueMessage, options, context.clone())
                    .await
            }

            Operation::Messages_Peek => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .messagesHandler()
                    .peek(options, context.clone())
                    .await
            }

            Operation::MessageId_Update => {
                let queueMessage = extract_object(&handler_parameters, "queueMessage")?;
                let popReceipt = extract_string(&handler_parameters, "popReceipt")?;
                let visibilitytimeout = extract_number(&handler_parameters, "visibilitytimeout")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .messageIdHandler()
                    .update(
                        queueMessage,
                        popReceipt,
                        visibilitytimeout,
                        options,
                        context.clone(),
                    )
                    .await
            }

            Operation::MessageId_Delete => {
                let popReceipt = extract_string(&handler_parameters, "popReceipt")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .messageIdHandler()
                    .delete(popReceipt, options, context.clone())
                    .await
            }
        }?;
        context.setHandlerResponses(Some(response));
        Ok(())
    }
}

fn format_handler_parameters(value: Option<&GeneratedObject>) -> String {
    value
        .map(|parameters| {
            generated_value_to_log_string(&GeneratedValue::Object(parameters.clone()))
        })
        .unwrap_or_else(|| String::from("undefined"))
}

fn generated_value_to_log_string(value: &GeneratedValue) -> String {
    serde_json::to_string(&value.to_json_value()).unwrap_or_else(|_| String::from("null"))
}

fn extract_value(
    parameters: &GeneratedObject,
    key: &str,
) -> crate::generated::GeneratedResult<GeneratedValue> {
    parameters.get(key).cloned().ok_or_else(|| {
        Box::new(MiddlewareError::new(
            500,
            format!("Missing handler parameter {}", key),
        )) as _
    })
}

fn extract_string(
    parameters: &GeneratedObject,
    key: &str,
) -> crate::generated::GeneratedResult<String> {
    extract_value(parameters, key)?.as_string().ok_or_else(|| {
        Box::new(MiddlewareError::new(
            500,
            format!("Handler parameter {} is not a string", key),
        )) as _
    })
}

fn extract_number(
    parameters: &GeneratedObject,
    key: &str,
) -> crate::generated::GeneratedResult<f64> {
    extract_value(parameters, key)?.as_number().ok_or_else(|| {
        Box::new(MiddlewareError::new(
            500,
            format!("Handler parameter {} is not a number", key),
        )) as _
    })
}

fn extract_object(
    parameters: &GeneratedObject,
    key: &str,
) -> crate::generated::GeneratedResult<GeneratedObject> {
    extract_value(parameters, key)?
        .as_object()
        .cloned()
        .ok_or_else(|| {
            Box::new(MiddlewareError::new(
                500,
                format!("Handler parameter {} is not an object", key),
            )) as _
        })
}
