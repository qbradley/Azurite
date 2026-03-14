use crate::errors::{StorageError, StorageErrorFactory};
use crate::generated::artifacts::models::ModifiedAccessConditions;
use crate::generated::context::Context;
use crate::persistence::query_interpreter::query_interpreter::generate_query_blob_with_tags_where_function;
use crate::persistence::{BlobModel, ContainerModel};

use super::condition_resource_adapter::ConditionResourceAdapter;
use super::conditional_headers_adapter::ConditionalHeadersAdapter;
use super::i_condition_resource::IConditionResource;
use super::i_conditional_headers::IConditionalHeaders;
use super::i_conditional_headers_validator::IConditionalHeadersValidator;

/// Convenience function mirroring TypeScript `validateReadConditions()`.
#[allow(non_snake_case)]
pub fn validate_read_conditions(
    context: &Context,
    conditionalHeaders: Option<&ModifiedAccessConditions>,
    model: Option<&BlobModel>,
    is_source_blob: Option<bool>,
) -> Result<(), StorageError> {
    let empty = ModifiedAccessConditions::default();
    let adapted_headers =
        ConditionalHeadersAdapter::new(context, conditionalHeaders.unwrap_or(&empty));
    let adapted_resource = ConditionResourceAdapter::from_blob(model);
    ReadConditionalHeadersValidator.validate(
        context,
        &adapted_headers,
        &adapted_resource,
        is_source_blob,
    )
}

/// Convenience function for container read conditions.
#[allow(non_snake_case)]
pub fn validate_read_conditions_container(
    context: &Context,
    conditionalHeaders: Option<&ModifiedAccessConditions>,
    model: Option<&ContainerModel>,
    is_source_blob: Option<bool>,
) -> Result<(), StorageError> {
    let empty = ModifiedAccessConditions::default();
    let adapted_headers =
        ConditionalHeadersAdapter::new(context, conditionalHeaders.unwrap_or(&empty));
    let adapted_resource = ConditionResourceAdapter::from_container(model);
    ReadConditionalHeadersValidator.validate(
        context,
        &adapted_headers,
        &adapted_resource,
        is_source_blob,
    )
}

/// Mirrors TypeScript `ReadConditionalHeadersValidator` class.
///
/// Validates Conditional Headers for Blob Service Read Operations in Version 2013-08-15 or Later.
/// @link https://docs.microsoft.com/en-us/rest/api/storageservices/specifying-conditional-headers-for-blob-service-operations
pub struct ReadConditionalHeadersValidator;

impl IConditionalHeadersValidator for ReadConditionalHeadersValidator {
    #[allow(non_snake_case)]
    fn validate(
        &self,
        context: &Context,
        conditionalHeaders: &IConditionalHeaders,
        resource: &IConditionResource,
        isSourceBlob: Option<bool>,
    ) -> Result<(), StorageError> {
        let contextId = context.contextId().unwrap_or_default();

        // Read against a non exist resource
        if !resource.exist {
            // If-Match
            if let Some(ref if_match) = conditionalHeaders.ifMatch {
                if !if_match.is_empty() {
                    return Err(StorageErrorFactory::getConditionNotMet(&contextId));
                }
            }

            // If-Unmodified-Since: skip for nonexistent resource

            // If-None-Match
            if let Some(ref if_none_match) = conditionalHeaders.ifNoneMatch {
                if !if_none_match.is_empty() && if_none_match[0] == "*" {
                    return Err(StorageErrorFactory::getUnsatisfiableCondition(&contextId));
                }
            }

            // If-Modified-Since: skip for nonexistent resource
        } else {
            // Read against an existing resource

            // If-Match
            let ifMatchPass = conditionalHeaders.ifMatch.as_ref().map(|if_match| {
                if_match.contains(&resource.etag)
                    || if_match.first().map(|s| s.as_str()) == Some("*")
            });

            // If-Unmodified-Since
            let ifUnModifiedSincePass =
                conditionalHeaders
                    .ifUnmodifiedSince
                    .as_ref()
                    .and_then(|if_unmodified_since| {
                        resource
                            .lastModified
                            .map(|last_modified| last_modified <= *if_unmodified_since)
                    });

            // If-None-Match: check for wildcard *
            if let Some(ref if_none_match) = conditionalHeaders.ifNoneMatch {
                if !if_none_match.is_empty() && if_none_match[0] == "*" {
                    return Err(StorageErrorFactory::getUnsatisfiableCondition(&contextId));
                }
            }

            let ifNoneMatchPass = conditionalHeaders
                .ifNoneMatch
                .as_ref()
                .map(|if_none_match| !if_none_match.contains(&resource.etag));

            // If-Modified-Since
            let isModifiedSincePass =
                conditionalHeaders
                    .ifModifiedSince
                    .as_ref()
                    .and_then(|if_modified_since| {
                        resource
                            .lastModified
                            .map(|last_modified| *if_modified_since < last_modified)
                    });

            if ifMatchPass == Some(false) {
                return Err(StorageErrorFactory::getConditionNotMet(&contextId));
            }

            if ifUnModifiedSincePass == Some(false) {
                return Err(StorageErrorFactory::getConditionNotMet(&contextId));
            }

            if ifNoneMatchPass == Some(false) && isModifiedSincePass != Some(true) {
                return Err(StorageErrorFactory::getNotModified(&contextId));
            }

            if isModifiedSincePass == Some(false) && ifNoneMatchPass != Some(true) {
                return Err(StorageErrorFactory::getNotModified(&contextId));
            }

            // ifTags validation
            if let Some(ref if_tags) = conditionalHeaders.ifTags {
                let against_source_blob = isSourceBlob.unwrap_or(false);
                let header_name = if against_source_blob {
                    "x-ms-source-if-tags"
                } else {
                    "x-ms-if-tags"
                };
                let validate_function = generate_query_blob_with_tags_where_function(
                    context,
                    Some(if_tags),
                    Some(header_name),
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
