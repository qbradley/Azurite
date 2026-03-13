use std::sync::Arc;

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

            Operation::Service_ListContainersSegment => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .serviceHandler()
                    .listContainersSegment(options, context.clone())
                    .await
            }

            Operation::Service_GetUserDelegationKey => {
                let keyInfo = extract_object(&handler_parameters, "keyInfo")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .serviceHandler()
                    .getUserDelegationKey(keyInfo, options, context.clone())
                    .await
            }

            Operation::Service_GetAccountInfo => {
                self.handlers
                    .serviceHandler()
                    .getAccountInfo(context.clone())
                    .await
            }

            Operation::Service_GetAccountInfoWithHead => {
                self.handlers
                    .serviceHandler()
                    .getAccountInfo(context.clone())
                    .await
            }

            Operation::Service_SubmitBatch => {
                let body = extract_stream(&handler_parameters, "body")?;
                let contentLength = extract_number(&handler_parameters, "contentLength")?;
                let multipartContentType =
                    extract_string(&handler_parameters, "multipartContentType")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .serviceHandler()
                    .submitBatch(
                        body,
                        contentLength,
                        multipartContentType,
                        options,
                        context.clone(),
                    )
                    .await
            }

            Operation::Service_FilterBlobs => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .serviceHandler()
                    .filterBlobs(options, context.clone())
                    .await
            }

            Operation::Container_Create => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .containerHandler()
                    .create(options, context.clone())
                    .await
            }

            Operation::Container_GetProperties => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .containerHandler()
                    .getProperties(options, context.clone())
                    .await
            }

            Operation::Container_GetPropertiesWithHead => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .containerHandler()
                    .getProperties(options, context.clone())
                    .await
            }

            Operation::Container_Delete => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .containerHandler()
                    .delete(options, context.clone())
                    .await
            }

            Operation::Container_SetMetadata => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .containerHandler()
                    .setMetadata(options, context.clone())
                    .await
            }

            Operation::Container_GetAccessPolicy => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .containerHandler()
                    .getAccessPolicy(options, context.clone())
                    .await
            }

            Operation::Container_SetAccessPolicy => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .containerHandler()
                    .setAccessPolicy(options, context.clone())
                    .await
            }

            Operation::Container_Restore => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .containerHandler()
                    .restore(options, context.clone())
                    .await
            }

            Operation::Container_SubmitBatch => {
                let body = extract_stream(&handler_parameters, "body")?;
                let contentLength = extract_number(&handler_parameters, "contentLength")?;
                let multipartContentType =
                    extract_string(&handler_parameters, "multipartContentType")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .containerHandler()
                    .submitBatch(
                        body,
                        contentLength,
                        multipartContentType,
                        options,
                        context.clone(),
                    )
                    .await
            }

            Operation::Container_FilterBlobs => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .containerHandler()
                    .filterBlobs(options, context.clone())
                    .await
            }

            Operation::Container_AcquireLease => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .containerHandler()
                    .acquireLease(options, context.clone())
                    .await
            }

            Operation::Container_ReleaseLease => {
                let leaseId = extract_string(&handler_parameters, "leaseId")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .containerHandler()
                    .releaseLease(leaseId, options, context.clone())
                    .await
            }

            Operation::Container_RenewLease => {
                let leaseId = extract_string(&handler_parameters, "leaseId")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .containerHandler()
                    .renewLease(leaseId, options, context.clone())
                    .await
            }

            Operation::Container_BreakLease => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .containerHandler()
                    .breakLease(options, context.clone())
                    .await
            }

            Operation::Container_ChangeLease => {
                let leaseId = extract_string(&handler_parameters, "leaseId")?;
                let proposedLeaseId = extract_string(&handler_parameters, "proposedLeaseId")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .containerHandler()
                    .changeLease(leaseId, proposedLeaseId, options, context.clone())
                    .await
            }

            Operation::Container_ListBlobFlatSegment => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .containerHandler()
                    .listBlobFlatSegment(options, context.clone())
                    .await
            }

            Operation::Container_ListBlobHierarchySegment => {
                let delimiter = extract_string(&handler_parameters, "delimiter")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .containerHandler()
                    .listBlobHierarchySegment(delimiter, options, context.clone())
                    .await
            }

            Operation::Container_GetAccountInfo => {
                self.handlers
                    .containerHandler()
                    .getAccountInfo(context.clone())
                    .await
            }

            Operation::Container_GetAccountInfoWithHead => {
                self.handlers
                    .containerHandler()
                    .getAccountInfo(context.clone())
                    .await
            }

            Operation::Blob_Download => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .download(options, context.clone())
                    .await
            }

            Operation::Blob_GetProperties => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .getProperties(options, context.clone())
                    .await
            }

            Operation::Blob_Delete => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .delete(options, context.clone())
                    .await
            }

            Operation::Blob_Undelete => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .undelete(options, context.clone())
                    .await
            }

            Operation::Blob_SetExpiry => {
                let expiryOptions = extract_string(&handler_parameters, "expiryOptions")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .setExpiry(expiryOptions, options, context.clone())
                    .await
            }

            Operation::Blob_SetHTTPHeaders => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .setHTTPHeaders(options, context.clone())
                    .await
            }

            Operation::Blob_SetImmutabilityPolicy => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .setImmutabilityPolicy(options, context.clone())
                    .await
            }

            Operation::Blob_DeleteImmutabilityPolicy => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .deleteImmutabilityPolicy(options, context.clone())
                    .await
            }

            Operation::Blob_SetLegalHold => {
                let legalHold = extract_bool(&handler_parameters, "legalHold")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .setLegalHold(legalHold, options, context.clone())
                    .await
            }

            Operation::Blob_SetMetadata => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .setMetadata(options, context.clone())
                    .await
            }

            Operation::Blob_AcquireLease => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .acquireLease(options, context.clone())
                    .await
            }

            Operation::Blob_ReleaseLease => {
                let leaseId = extract_string(&handler_parameters, "leaseId")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .releaseLease(leaseId, options, context.clone())
                    .await
            }

            Operation::Blob_RenewLease => {
                let leaseId = extract_string(&handler_parameters, "leaseId")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .renewLease(leaseId, options, context.clone())
                    .await
            }

            Operation::Blob_ChangeLease => {
                let leaseId = extract_string(&handler_parameters, "leaseId")?;
                let proposedLeaseId = extract_string(&handler_parameters, "proposedLeaseId")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .changeLease(leaseId, proposedLeaseId, options, context.clone())
                    .await
            }

            Operation::Blob_BreakLease => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .breakLease(options, context.clone())
                    .await
            }

            Operation::Blob_CreateSnapshot => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .createSnapshot(options, context.clone())
                    .await
            }

            Operation::Blob_StartCopyFromURL => {
                let copySource = extract_string(&handler_parameters, "copySource")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .startCopyFromURL(copySource, options, context.clone())
                    .await
            }

            Operation::Blob_CopyFromURL => {
                let copySource = extract_string(&handler_parameters, "copySource")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .copyFromURL(copySource, options, context.clone())
                    .await
            }

            Operation::Blob_AbortCopyFromURL => {
                let copyId = extract_string(&handler_parameters, "copyId")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .abortCopyFromURL(copyId, options, context.clone())
                    .await
            }

            Operation::Blob_SetTier => {
                let tier = extract_string(&handler_parameters, "tier")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .setTier(tier, options, context.clone())
                    .await
            }

            Operation::Blob_GetAccountInfo => {
                self.handlers
                    .blobHandler()
                    .getAccountInfo(context.clone())
                    .await
            }

            Operation::Blob_GetAccountInfoWithHead => {
                self.handlers
                    .blobHandler()
                    .getAccountInfo(context.clone())
                    .await
            }

            Operation::Blob_Query => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .query(options, context.clone())
                    .await
            }

            Operation::Blob_GetTags => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .getTags(options, context.clone())
                    .await
            }

            Operation::Blob_SetTags => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blobHandler()
                    .setTags(options, context.clone())
                    .await
            }

            Operation::PageBlob_Create => {
                let contentLength = extract_number(&handler_parameters, "contentLength")?;
                let blobContentLength = extract_number(&handler_parameters, "blobContentLength")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .pageBlobHandler()
                    .create(contentLength, blobContentLength, options, context.clone())
                    .await
            }

            Operation::PageBlob_UploadPages => {
                let body = extract_stream(&handler_parameters, "body")?;
                let contentLength = extract_number(&handler_parameters, "contentLength")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .pageBlobHandler()
                    .uploadPages(body, contentLength, options, context.clone())
                    .await
            }

            Operation::PageBlob_ClearPages => {
                let contentLength = extract_number(&handler_parameters, "contentLength")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .pageBlobHandler()
                    .clearPages(contentLength, options, context.clone())
                    .await
            }

            Operation::PageBlob_UploadPagesFromURL => {
                let sourceUrl = extract_string(&handler_parameters, "sourceUrl")?;
                let sourceRange = extract_string(&handler_parameters, "sourceRange")?;
                let contentLength = extract_number(&handler_parameters, "contentLength")?;
                let range = extract_string(&handler_parameters, "range")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .pageBlobHandler()
                    .uploadPagesFromURL(
                        sourceUrl,
                        sourceRange,
                        contentLength,
                        range,
                        options,
                        context.clone(),
                    )
                    .await
            }

            Operation::PageBlob_GetPageRanges => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .pageBlobHandler()
                    .getPageRanges(options, context.clone())
                    .await
            }

            Operation::PageBlob_GetPageRangesDiff => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .pageBlobHandler()
                    .getPageRangesDiff(options, context.clone())
                    .await
            }

            Operation::PageBlob_Resize => {
                let blobContentLength = extract_number(&handler_parameters, "blobContentLength")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .pageBlobHandler()
                    .resize(blobContentLength, options, context.clone())
                    .await
            }

            Operation::PageBlob_UpdateSequenceNumber => {
                let sequenceNumberAction =
                    extract_string(&handler_parameters, "sequenceNumberAction")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .pageBlobHandler()
                    .updateSequenceNumber(sequenceNumberAction, options, context.clone())
                    .await
            }

            Operation::PageBlob_CopyIncremental => {
                let copySource = extract_string(&handler_parameters, "copySource")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .pageBlobHandler()
                    .copyIncremental(copySource, options, context.clone())
                    .await
            }

            Operation::AppendBlob_Create => {
                let contentLength = extract_number(&handler_parameters, "contentLength")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .appendBlobHandler()
                    .create(contentLength, options, context.clone())
                    .await
            }

            Operation::AppendBlob_AppendBlock => {
                let body = extract_stream(&handler_parameters, "body")?;
                let contentLength = extract_number(&handler_parameters, "contentLength")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .appendBlobHandler()
                    .appendBlock(body, contentLength, options, context.clone())
                    .await
            }

            Operation::AppendBlob_AppendBlockFromUrl => {
                let sourceUrl = extract_string(&handler_parameters, "sourceUrl")?;
                let contentLength = extract_number(&handler_parameters, "contentLength")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .appendBlobHandler()
                    .appendBlockFromUrl(sourceUrl, contentLength, options, context.clone())
                    .await
            }

            Operation::AppendBlob_Seal => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .appendBlobHandler()
                    .seal(options, context.clone())
                    .await
            }

            Operation::BlockBlob_Upload => {
                let body = extract_stream(&handler_parameters, "body")?;
                let contentLength = extract_number(&handler_parameters, "contentLength")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blockBlobHandler()
                    .upload(body, contentLength, options, context.clone())
                    .await
            }

            Operation::BlockBlob_PutBlobFromUrl => {
                let contentLength = extract_number(&handler_parameters, "contentLength")?;
                let copySource = extract_string(&handler_parameters, "copySource")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blockBlobHandler()
                    .putBlobFromUrl(contentLength, copySource, options, context.clone())
                    .await
            }

            Operation::BlockBlob_StageBlock => {
                let blockId = extract_string(&handler_parameters, "blockId")?;
                let contentLength = extract_number(&handler_parameters, "contentLength")?;
                let body = extract_stream(&handler_parameters, "body")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blockBlobHandler()
                    .stageBlock(blockId, contentLength, body, options, context.clone())
                    .await
            }

            Operation::BlockBlob_StageBlockFromURL => {
                let blockId = extract_string(&handler_parameters, "blockId")?;
                let contentLength = extract_number(&handler_parameters, "contentLength")?;
                let sourceUrl = extract_string(&handler_parameters, "sourceUrl")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blockBlobHandler()
                    .stageBlockFromURL(blockId, contentLength, sourceUrl, options, context.clone())
                    .await
            }

            Operation::BlockBlob_CommitBlockList => {
                let blocks = extract_object(&handler_parameters, "blocks")?;
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blockBlobHandler()
                    .commitBlockList(blocks, options, context.clone())
                    .await
            }

            Operation::BlockBlob_GetBlockList => {
                let options = extract_object(&handler_parameters, "options")?;
                self.handlers
                    .blockBlobHandler()
                    .getBlockList(options, context.clone())
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
    match value {
        GeneratedValue::Stream(_) => String::from("\"ReadableStream\""),
        other => {
            serde_json::to_string(&other.to_json_value()).unwrap_or_else(|_| String::from("null"))
        }
    }
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

fn extract_bool(
    parameters: &GeneratedObject,
    key: &str,
) -> crate::generated::GeneratedResult<bool> {
    extract_value(parameters, key)?.as_bool().ok_or_else(|| {
        Box::new(MiddlewareError::new(
            500,
            format!("Handler parameter {} is not a boolean", key),
        )) as _
    })
}

fn extract_stream(
    parameters: &GeneratedObject,
    key: &str,
) -> crate::generated::GeneratedResult<GeneratedReadableStream> {
    extract_value(parameters, key)?.as_stream().ok_or_else(|| {
        Box::new(MiddlewareError::new(
            500,
            format!("Handler parameter {} is not a stream", key),
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
