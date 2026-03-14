use std::collections::BTreeMap;

use crate::generated::context::Context;

use super::storage_error::StorageError;

pub const DEFAULT_ID: &str = "DefaultID";

pub struct StorageErrorFactory;

#[allow(non_snake_case)]
impl StorageErrorFactory {
    pub fn getBatchDuplicateRowKey(context: &Context, rowKey: &str) -> StorageError {
        Self::create(
            context,
            400,
            "InvalidDuplicateRow",
            format!(
                "A command with RowKey '{}' is already present in the batch. An entity can appear only once in a batch. ",
                rowKey
            ),
            None,
            DEFAULT_ID,
        )
    }

    pub fn getInvalidHeaderValue(
        context: &Context,
        additionalMessages: Option<BTreeMap<String, String>>,
    ) -> StorageError {
        Self::create(
            context,
            400,
            "InvalidHeaderValue",
            "The value for one of the HTTP headers is not in the correct format.",
            additionalMessages,
            DEFAULT_ID,
        )
    }

    pub fn getInvalidAPIVersion(context: &Context, apiVersion: Option<&str>) -> StorageError {
        Self::create(
            context,
            400,
            "InvalidHeaderValue",
            format!(
                "The API version {} is not supported by Azurite. Please upgrade Azurite to latest version and retry. If you are using Azurite in Visual Studio, please check you have installed latest Visual Studio patch. Azurite command line parameter \"--skipApiVersionCheck\" or Visual Studio Code configuration \"Skip Api Version Check\" can skip this error. ",
                apiVersion.unwrap_or("undefined")
            ),
            None,
            DEFAULT_ID,
        )
    }

    pub fn getInvalidInput(
        context: &Context,
        additionalMessages: Option<BTreeMap<String, String>>,
    ) -> StorageError {
        Self::create(
            context,
            400,
            "InvalidInput",
            "An error occurred while processing this request.",
            additionalMessages,
            DEFAULT_ID,
        )
    }

    pub fn getTableAlreadyExists(context: &Context) -> StorageError {
        Self::create(
            context,
            409,
            "TableAlreadyExists",
            "The table specified already exists.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getTableNameEmpty(context: &Context) -> StorageError {
        Self::create(
            context,
            400,
            "TableNameEmpty",
            "The specified table name is empty.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getInvalidOperation(context: &Context, message: Option<&str>) -> StorageError {
        Self::create(
            context,
            400,
            "InvalidOperation",
            message.unwrap_or_default(),
            None,
            "",
        )
    }

    pub fn getAuthorizationSourceIPMismatch(context: &Context) -> StorageError {
        Self::create(
            context,
            403,
            "AuthorizationSourceIPMismatch",
            "This request is not authorized to perform this operation using this source IP {SourceIP}.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getAuthorizationProtocolMismatch(context: &Context) -> StorageError {
        Self::create(
            context,
            403,
            "AuthorizationProtocolMismatch",
            "This request is not authorized to perform this operation using this protocol.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getAuthorizationPermissionMismatch(context: &Context) -> StorageError {
        Self::create(
            context,
            403,
            "AuthorizationPermissionMismatch",
            "This request is not authorized to perform this operation using this permission.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getAuthorizationServiceMismatch(context: &Context) -> StorageError {
        Self::create(
            context,
            403,
            "AuthorizationServiceMismatch",
            "This request is not authorized to perform this operation using this service.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getAuthorizationResourceTypeMismatch(context: &Context) -> StorageError {
        Self::create(
            context,
            403,
            "AuthorizationResourceTypeMismatch",
            "This request is not authorized to perform this operation using this resource type.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getAccountNameEmpty(context: &Context) -> StorageError {
        Self::create(
            context,
            400,
            "AccountNameEmpty",
            "The specified account name is empty.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getTableNotExist(context: &Context) -> StorageError {
        Self::create(
            context,
            404,
            "TableNotFound",
            "The table specified does not exist.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getAuthorizationFailure(context: &Context) -> StorageError {
        Self::create(
            context,
            403,
            "AuthorizationFailure",
            "Server failed to authenticate the request. Make sure the value of the Authorization header is formed correctly including the signature.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getEntityAlreadyExist(context: &Context) -> StorageError {
        Self::create(
            context,
            409,
            "EntityAlreadyExists",
            "The specified entity already exists.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getPropertiesNeedValue(context: &Context) -> StorageError {
        Self::create(
            context,
            400,
            "PropertiesNeedValue",
            "The values are not specified for all properties in the entity.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getAtomFormatNotSupported(context: &Context) -> StorageError {
        Self::create(
            context,
            415,
            "AtomFormatNotSupported",
            "Atom format is not supported.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getPreconditionFailed(context: &Context) -> StorageError {
        Self::create(
            context,
            412,
            "UpdateConditionNotSatisfied",
            "The update condition specified in the request was not satisfied.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getTableNotFound(context: &Context) -> StorageError {
        Self::create(
            context,
            404,
            "TableNotFound",
            "The table specified does not exist.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn ResourceNotFound(context: &Context) -> StorageError {
        Self::create(
            context,
            404,
            "ResourceNotFound",
            "The specified resource does not exist.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getEntityNotFound(context: &Context) -> StorageError {
        Self::create(
            context,
            404,
            "ResourceNotFound",
            "The specified resource does not exist.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getQueryConditionInvalid(context: &Context) -> StorageError {
        Self::create(
            context,
            400,
            "InvalidInput",
            "The query condition specified in the request is invalid.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getInvalidResourceName(context: &Context) -> StorageError {
        Self::create(
            context,
            400,
            "",
            "The specified resource name contains invalid characters.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getOutOfRangeName(context: &Context) -> StorageError {
        Self::create(
            context,
            400,
            "",
            "The specified resource name length is not within the permissible limits.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getInvalidXmlDocument(context: &Context) -> StorageError {
        Self::create(
            context,
            400,
            "InvalidXmlDocument",
            "XML specified is not syntactically valid.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getInvalidQueryParameterValue(
        context: &Context,
        additionalMessages: Option<BTreeMap<String, String>>,
    ) -> StorageError {
        Self::create(
            context,
            400,
            "InvalidQueryParameterValue",
            "Value for one of the query parameters specified in the request URI is invalid.",
            additionalMessages,
            DEFAULT_ID,
        )
    }

    pub fn getInvalidCorsHeaderValue(
        context: &Context,
        additionalMessages: Option<BTreeMap<String, String>>,
    ) -> StorageError {
        Self::create(
            context,
            400,
            "InvalidHeaderValue",
            "A required CORS header is not present.",
            additionalMessages,
            DEFAULT_ID,
        )
    }

    pub fn corsPreflightFailure(
        context: &Context,
        additionalMessages: Option<BTreeMap<String, String>>,
    ) -> StorageError {
        Self::create(
            context,
            403,
            "CorsPreflightFailure",
            "CORS not enabled or no matching rule found for this request.",
            additionalMessages,
            DEFAULT_ID,
        )
    }

    pub fn getInvalidAuthenticationInfo(context: &Context) -> StorageError {
        Self::create(
            context,
            400,
            "InvalidAuthenticationInfo",
            "Authentication information is not given in the correct format. Check the value of Authorization header.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getAuthenticationFailed(
        context: &Context,
        authenticationErrorDetail: impl Into<String>,
    ) -> StorageError {
        let mut additionalMessages = BTreeMap::new();
        additionalMessages.insert(
            String::from("AuthenticationErrorDetail"),
            authenticationErrorDetail.into(),
        );

        Self::create(
            context,
            403,
            "AuthenticationFailed",
            "Server failed to authenticate the request. Make sure the value of the Authorization header is formed correctly including the signature.",
            Some(additionalMessages),
            DEFAULT_ID,
        )
    }

    pub fn getNotImplementedError(context: &Context) -> StorageError {
        Self::create(
            context,
            501,
            "NotImplemented",
            "The requested operation is not implemented on the specified resource.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getPropertyValueTooLargeError(context: &Context) -> StorageError {
        Self::create(
            context,
            400,
            "PropertyValueTooLarge",
            "The property value exceeds the maximum allowed size (64KB). If the property value is a string, it is UTF-16 encoded and the maximum number of characters should be 32K or less.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getRequestBodyTooLarge(context: &Context) -> StorageError {
        Self::create(
            context,
            413,
            "RequestBodyTooLarge",
            "The request body is too large and exceeds the maximum permissible limit.",
            None,
            DEFAULT_ID,
        )
    }

    pub fn getEntityTooLarge(context: &Context) -> StorageError {
        Self::create(
            context,
            400,
            "EntityTooLarge",
            "The entity is larger than the maximum allowed size (1MB).",
            None,
            DEFAULT_ID,
        )
    }

    fn create(
        context: &Context,
        statusCode: u16,
        storageErrorCode: impl Into<String>,
        storageErrorMessage: impl Into<String>,
        storageAdditionalErrorMessages: Option<BTreeMap<String, String>>,
        default_request_id: &str,
    ) -> StorageError {
        StorageError::new(
            statusCode,
            storageErrorCode,
            storageErrorMessage,
            context
                .contextId()
                .unwrap_or_else(|| default_request_id.to_string()),
            storageAdditionalErrorMessages.unwrap_or_else(StorageError::empty_extra),
            context,
        )
    }
}
