use crate::errors::StorageError;
use crate::generated::context::Context;

use super::i_condition_resource::IConditionResource;
use super::i_conditional_headers::IConditionalHeaders;

/// Mirrors TypeScript `IConditionalHeadersValidator` interface.
pub trait IConditionalHeadersValidator {
    fn validate(
        &self,
        context: &Context,
        conditional_headers: &IConditionalHeaders,
        resource: &IConditionResource,
        is_source_blob: Option<bool>,
    ) -> Result<(), StorageError>;
}
