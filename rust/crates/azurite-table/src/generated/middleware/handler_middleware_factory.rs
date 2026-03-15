use std::sync::Arc;

use crate::errors::StorageError;
use crate::generated::artifacts::models::{GeneratedObject, GeneratedValue};
use crate::generated::artifacts::operation::Operation;
use crate::generated::context::Context;
use crate::generated::errors::middleware_error::MiddlewareError;
use crate::generated::handlers::{getHandlerByOperation, IHandlers};
use crate::generated::i_request::GeneratedReadableStream;
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
                "HandlerMiddleware: DeserializedParameters={}",
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
        let response = dispatch_operation(&*self.handlers, operation, &handler_parameters, context)
            .await
            .map_err(|error| Box::new(error) as Box<dyn std::error::Error + Send + Sync>)?;
        context.setHandlerResponses(Some(response));
        Ok(())
    }
}

async fn dispatch_operation<H: IHandlers>(
    handlers: &H,
    operation: Operation,
    handler_parameters: &GeneratedObject,
    context: &Context,
) -> Result<crate::generated::artifacts::models::GeneratedResponse, StorageError> {
    match operation {
        Operation::Service_SetProperties => {
            let tableServiceProperties =
                extract_object(handler_parameters, "tableServiceProperties");
            let options = extract_object(handler_parameters, "options");
            handlers
                .serviceHandler()
                .setProperties(tableServiceProperties, options, context.clone())
                .await
        }
        Operation::Service_GetProperties => {
            let options = extract_object(handler_parameters, "options");
            handlers
                .serviceHandler()
                .getProperties(options, context.clone())
                .await
        }
        Operation::Service_GetStatistics => {
            let options = extract_object(handler_parameters, "options");
            handlers
                .serviceHandler()
                .getStatistics(options, context.clone())
                .await
        }
        Operation::Table_Query => {
            let options = extract_object(handler_parameters, "options");
            handlers
                .tableHandler()
                .query(options, context.clone())
                .await
        }
        Operation::Table_Create => {
            let table = extract_object(handler_parameters, "tableProperties");
            let options = extract_object(handler_parameters, "options");
            handlers
                .tableHandler()
                .create(table, options, context.clone())
                .await
        }
        Operation::Table_Batch => {
            let body = extract_stream(handler_parameters, "body");
            let options = extract_object(handler_parameters, "options");
            handlers
                .tableHandler()
                .batch(body, options, context.clone())
                .await
        }
        Operation::Table_Delete => {
            let options = extract_object(handler_parameters, "options");
            handlers
                .tableHandler()
                .delete(options, context.clone())
                .await
        }
        Operation::Table_QueryEntities => {
            let options = extract_object(handler_parameters, "options");
            handlers
                .tableHandler()
                .queryEntities(options, context.clone())
                .await
        }
        Operation::Table_QueryEntitiesWithPartitionAndRowKey => {
            let options = extract_object(handler_parameters, "options");
            handlers
                .tableHandler()
                .queryEntitiesWithPartitionAndRowKey(options, context.clone())
                .await
        }
        Operation::Table_UpdateEntity => {
            let options = extract_object(handler_parameters, "options");
            let entity = extract_nested_object(&options, "tableEntityProperties");
            handlers
                .tableHandler()
                .updateEntity(entity, options, context.clone())
                .await
        }
        Operation::Table_MergeEntity | Operation::Table_MergeEntityWithMerge => {
            let options = extract_object(handler_parameters, "options");
            let entity = extract_nested_object(&options, "tableEntityProperties");
            handlers
                .tableHandler()
                .mergeEntity(entity, options, context.clone())
                .await
        }
        Operation::Table_DeleteEntity => {
            let options = extract_object(handler_parameters, "options");
            handlers
                .tableHandler()
                .deleteEntity(options, context.clone())
                .await
        }
        Operation::Table_InsertEntity => {
            let options = extract_object(handler_parameters, "options");
            let entity = extract_nested_object(&options, "tableEntityProperties");
            handlers
                .tableHandler()
                .insertEntity(entity, options, context.clone())
                .await
        }
        Operation::Table_GetAccessPolicy => {
            let options = extract_object(handler_parameters, "options");
            handlers
                .tableHandler()
                .getAccessPolicy(options, context.clone())
                .await
        }
        Operation::Table_SetAccessPolicy => {
            let options = extract_object(handler_parameters, "options");
            let signedIdentifiers = extract_signed_identifiers(&options, "tableAcl");
            handlers
                .tableHandler()
                .setAccessPolicy(signedIdentifiers, options, context.clone())
                .await
        }
    }
}

fn extract_object(parameters: &GeneratedObject, key: &str) -> GeneratedObject {
    parameters
        .get(key)
        .and_then(GeneratedValue::as_object)
        .cloned()
        .unwrap_or_default()
}

fn extract_nested_object(parent: &GeneratedObject, key: &str) -> GeneratedObject {
    parent
        .get(key)
        .and_then(GeneratedValue::as_object)
        .cloned()
        .unwrap_or_default()
}

fn extract_stream(parameters: &GeneratedObject, key: &str) -> GeneratedReadableStream {
    parameters
        .get(key)
        .and_then(GeneratedValue::as_stream)
        .unwrap_or_default()
}

#[allow(dead_code)]
fn extract_object_array(parameters: &GeneratedObject, key: &str) -> Vec<GeneratedObject> {
    match parameters.get(key) {
        Some(GeneratedValue::Array(values)) => values
            .iter()
            .filter_map(GeneratedValue::as_object)
            .cloned()
            .collect(),
        _ => Vec::new(),
    }
}

fn extract_nested_object_array(parent: &GeneratedObject, key: &str) -> Vec<GeneratedObject> {
    match parent.get(key) {
        Some(GeneratedValue::Array(values)) => values
            .iter()
            .filter_map(GeneratedValue::as_object)
            .cloned()
            .collect(),
        Some(GeneratedValue::Object(obj)) => vec![obj.clone()],
        _ => Vec::new(),
    }
}

/// Extract SignedIdentifier elements from the XML-parsed tableAcl structure.
/// parseXML produces: `{"SignedIdentifier": {...}}` for one, or
/// `{"SignedIdentifier": [{...}, {...}]}` for multiple.
/// We need to unwrap the `SignedIdentifier` key to get the inner objects.
fn extract_signed_identifiers(parent: &GeneratedObject, key: &str) -> Vec<GeneratedObject> {
    let acl_value = match parent.get(key) {
        Some(v) => v,
        None => return Vec::new(),
    };
    // Try to get the inner SignedIdentifier element(s)
    let inner = acl_value
        .as_object()
        .and_then(|obj| obj.get("SignedIdentifier"));
    match inner {
        Some(GeneratedValue::Array(values)) => values
            .iter()
            .filter_map(GeneratedValue::as_object)
            .cloned()
            .collect(),
        Some(GeneratedValue::Object(obj)) => vec![obj.clone()],
        _ => {
            // Fallback: treat the value itself as the identifiers array
            extract_nested_object_array(parent, key)
        }
    }
}

fn format_handler_parameters(parameters: Option<&GeneratedObject>) -> String {
    parameters
        .map(|value| {
            serde_json::Value::Object(
                value
                    .iter()
                    .map(|(key, value)| (key.clone(), value.to_json_value()))
                    .collect(),
            )
        })
        .map(|value| value.to_string())
        .unwrap_or_else(|| String::from("null"))
}
