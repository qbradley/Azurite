pub mod condition_resource_adapter;
pub mod conditional_headers_adapter;
pub mod i_condition_resource;
pub mod i_conditional_headers;
pub mod i_conditional_headers_validator;
pub mod read_conditional_headers_validator;
pub mod write_conditional_headers_validator;

pub use condition_resource_adapter::ConditionResourceAdapter;
pub use conditional_headers_adapter::ConditionalHeadersAdapter;
pub use i_condition_resource::IConditionResource;
pub use i_conditional_headers::IConditionalHeaders;
pub use i_conditional_headers_validator::IConditionalHeadersValidator;
pub use read_conditional_headers_validator::{
    validate_read_conditions, ReadConditionalHeadersValidator,
};
pub use write_conditional_headers_validator::{
    validate_sequence_number_write_conditions, validate_write_conditions,
    WriteConditionalHeadersValidator,
};

#[derive(Debug, Clone, Default)]
pub struct BlobConditionsModule;
