use crate::errors::{StorageError, StorageErrorFactory};
use crate::generated::artifacts::models::{
    GeneratedValue, ModifiedAccessConditions, SequenceNumberAccessConditions,
};
use crate::generated::context::Context;
use crate::persistence::query_interpreter::query_interpreter::generate_query_blob_with_tags_where_function;
use crate::persistence::{BlobModel, ContainerModel};

use super::condition_resource_adapter::ConditionResourceAdapter;
use super::conditional_headers_adapter::ConditionalHeadersAdapter;
use super::i_condition_resource::IConditionResource;
use super::i_conditional_headers::IConditionalHeaders;
use super::i_conditional_headers_validator::IConditionalHeadersValidator;

/// Mirrors TypeScript `validateSequenceNumberWriteConditions()`.
#[allow(non_snake_case)]
pub fn validate_sequence_number_write_conditions(
    context: &Context,
    conditionalHeaders: Option<&SequenceNumberAccessConditions>,
    model: Option<&BlobModel>,
) -> Result<(), StorageError> {
    let conditionalHeaders = match conditionalHeaders {
        Some(h) => h,
        None => return Ok(()),
    };
    let model = match model {
        Some(m) => m,
        None => return Ok(()),
    };

    let blobSequenceNumber = model
        .properties
        .get("blobSequenceNumber")
        .and_then(|v| v.as_number())
        .ok_or_else(|| {
            // Mirrors TS: throw Error(`validateSequenceNumberWriteConditions() Invalid blob model...`)
            StorageError::new(
                500,
                "InternalError".to_string(),
                "validateSequenceNumberWriteConditions() Invalid blob model, blobSequenceNumber is not specified.".to_string(),
                context.contextId().unwrap_or_default(),
                StorageError::empty_extra(),
            )
        })?;

    let contextId = context.contextId().unwrap_or_default();

    if let Some(GeneratedValue::Number(lte)) =
        conditionalHeaders.get("ifSequenceNumberLessThanOrEqualTo")
    {
        if *lte < blobSequenceNumber {
            return Err(StorageErrorFactory::getSequenceNumberConditionNotMet(
                &contextId,
            ));
        }
    }

    if let Some(GeneratedValue::Number(lt)) = conditionalHeaders.get("ifSequenceNumberLessThan") {
        if *lt <= blobSequenceNumber {
            return Err(StorageErrorFactory::getSequenceNumberConditionNotMet(
                &contextId,
            ));
        }
    }

    if let Some(GeneratedValue::Number(eq)) = conditionalHeaders.get("ifSequenceNumberEqualTo") {
        if (*eq - blobSequenceNumber).abs() > f64::EPSILON {
            return Err(StorageErrorFactory::getSequenceNumberConditionNotMet(
                &contextId,
            ));
        }
    }

    Ok(())
}

/// Convenience function mirroring TypeScript `validateWriteConditions()`.
#[allow(non_snake_case)]
pub fn validate_write_conditions(
    context: &Context,
    conditionalHeaders: Option<&ModifiedAccessConditions>,
    model: Option<&BlobModel>,
) -> Result<(), StorageError> {
    let empty = ModifiedAccessConditions::default();
    let adapted_headers =
        ConditionalHeadersAdapter::new(context, conditionalHeaders.unwrap_or(&empty));
    let adapted_resource = ConditionResourceAdapter::from_blob(model);
    WriteConditionalHeadersValidator.validate(context, &adapted_headers, &adapted_resource, None)
}

/// Convenience function for container write conditions.
#[allow(non_snake_case)]
pub fn validate_write_conditions_container(
    context: &Context,
    conditionalHeaders: Option<&ModifiedAccessConditions>,
    model: Option<&ContainerModel>,
) -> Result<(), StorageError> {
    let empty = ModifiedAccessConditions::default();
    let adapted_headers =
        ConditionalHeadersAdapter::new(context, conditionalHeaders.unwrap_or(&empty));
    let adapted_resource = ConditionResourceAdapter::from_container(model);
    WriteConditionalHeadersValidator.validate(context, &adapted_headers, &adapted_resource, None)
}

/// Mirrors TypeScript `WriteConditionalHeadersValidator` class.
///
/// Validates conditional Headers for Read Operations in Versions Prior to 2013-08-15,
/// and for Write Operations (All Versions).
/// @link https://docs.microsoft.com/en-us/rest/api/storageservices/specifying-conditional-headers-for-blob-service-operations
pub struct WriteConditionalHeadersValidator;

impl IConditionalHeadersValidator for WriteConditionalHeadersValidator {
    #[allow(non_snake_case)]
    fn validate(
        &self,
        context: &Context,
        conditionalHeaders: &IConditionalHeaders,
        resource: &IConditionResource,
        _isSourceBlob: Option<bool>,
    ) -> Result<(), StorageError> {
        let contextId = context.contextId().unwrap_or_default();

        Self::validate_combinations(context, conditionalHeaders)?;

        if !resource.exist {
            if let Some(ref if_none_match) = conditionalHeaders.ifNoneMatch {
                if !if_none_match.is_empty() {
                    // If a request specifies both the If-None-Match and If-Modified-Since headers,
                    // the request is evaluated based on the criteria specified in If-None-Match.
                    // Skip for non exist blob
                    return Ok(());
                }
            }

            if let Some(ref if_match) = conditionalHeaders.ifMatch {
                if !if_match.is_empty() {
                    // If a request specifies both the If-Match and If-Unmodified-Since headers,
                    // the request is evaluated based on the criteria specified in If-Match.
                    // Throw if there is any value in if-match for non exist blob
                    return Err(StorageErrorFactory::getConditionNotMet(&contextId));
                }
            }

            if conditionalHeaders.ifModifiedSince.is_some() {
                // Skip for non exist blob
                return Ok(());
            }

            if conditionalHeaders.ifUnmodifiedSince.is_some() {
                // Skip for non exist blob
                return Ok(());
            }
        } else {
            if let Some(ref if_none_match) = conditionalHeaders.ifNoneMatch {
                if !if_none_match.is_empty() {
                    if if_none_match[0] == "*" {
                        // According to restful doc, specify the wildcard character (*) to perform the operation
                        // only if the resource does not exist, and fail the operation if it does exist.
                        // However, Azure Storage Set Blob Properties Operation for an existing blob doesn't return 412 with *
                        // TODO: Check accurate behavior for different write operations
                        // Put Blob, Commit Block List has special logic for ifNoneMatch equals *, will return 409 conflict
                        // for existing blob, will handled in createBlob metadata store.
                        return Ok(());
                    }
                    if if_none_match[0] == resource.etag {
                        return Err(StorageErrorFactory::getConditionNotMet(&contextId));
                    }

                    // Stop processing
                    // If a request specifies both the If-None-Match and If-Modified-Since headers,
                    // the request is evaluated based on the criteria specified in If-None-Match.
                    return Ok(());
                }
            }

            if let Some(ref if_match) = conditionalHeaders.ifMatch {
                if !if_match.is_empty() {
                    if if_match[0] != "*" && if_match[0] != resource.etag {
                        return Err(StorageErrorFactory::getConditionNotMet(&contextId));
                    }

                    // Stop processing
                    // If a request specifies both the If-Match and If-Unmodified-Since headers,
                    // the request is evaluated based on the criteria specified in If-Match.
                    return Ok(());
                }
            }

            if let Some(ref if_modified_since) = conditionalHeaders.ifModifiedSince {
                if let Some(last_modified) = resource.lastModified {
                    if last_modified <= *if_modified_since {
                        return Err(StorageErrorFactory::getConditionNotMet(&contextId));
                    }
                }
                return Ok(());
            }

            if let Some(ref if_unmodified_since) = conditionalHeaders.ifUnmodifiedSince {
                if let Some(last_modified) = resource.lastModified {
                    if *if_unmodified_since < last_modified {
                        return Err(StorageErrorFactory::getConditionNotMet(&contextId));
                    }
                }
                return Ok(());
            }

            // ifTags validation
            if let Some(ref if_tags) = conditionalHeaders.ifTags {
                let validate_function = generate_query_blob_with_tags_where_function(
                    context,
                    Some(if_tags),
                    Some("x-ms-if-tags"),
                )?;

                if let Some(ref blob_item) = resource.blobItemWithTags {
                    if validate_function(blob_item).is_empty() {
                        return Err(StorageErrorFactory::getConditionNotMet(&contextId));
                    }
                }
            }
        }

        Ok(())
    }
}

impl WriteConditionalHeadersValidator {
    #[allow(non_snake_case)]
    fn validate_combinations(
        context: &Context,
        conditionalHeaders: &IConditionalHeaders,
    ) -> Result<(), StorageError> {
        let contextId = context.contextId().unwrap_or_default();

        let mut ifMatch: i32 = 0;
        if let Some(ref if_match) = conditionalHeaders.ifMatch {
            if !if_match.is_empty() {
                // RFC 2616 allows multiple ETag values in a single header,
                // but requests to the Blob service can only include one ETag value.
                // Specifying more than one ETag value results in status code 400 (Bad Request).
                if if_match.len() > 1 {
                    return Err(
                        StorageErrorFactory::getMultipleConditionHeadersNotSupported(&contextId),
                    );
                }
                ifMatch = 1;
            }
        }

        let mut ifModifiedSince: i32 = 0;
        if conditionalHeaders.ifModifiedSince.is_some() {
            ifModifiedSince = 1;
        }

        let mut ifNoneMatch: i32 = 0;
        if let Some(ref if_none_match) = conditionalHeaders.ifNoneMatch {
            if !if_none_match.is_empty() {
                // RFC 2616 allows multiple ETag values in a single header,
                // but requests to the Blob service can only include one ETag value.
                if if_none_match.len() > 1 {
                    return Err(
                        StorageErrorFactory::getMultipleConditionHeadersNotSupported(&contextId),
                    );
                }
                ifNoneMatch = 1;
            }
        }

        let mut ifUnmodifiedSince: i32 = 0;
        if conditionalHeaders.ifUnmodifiedSince.is_some() {
            ifUnmodifiedSince = 1;
        }

        if ifMatch + ifModifiedSince + ifNoneMatch + ifUnmodifiedSince > 2 {
            return Err(StorageErrorFactory::getMultipleConditionHeadersNotSupported(&contextId));
        }

        if ifMatch + ifModifiedSince + ifNoneMatch + ifUnmodifiedSince == 2
            && ifNoneMatch + ifModifiedSince == 1
        {
            return Err(StorageErrorFactory::getMultipleConditionHeadersNotSupported(&contextId));
        }

        Ok(())
    }
}
