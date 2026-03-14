use std::collections::BTreeMap;

use super::storage_error::StorageError;

const DEFAULT_ID: &str = "DefaultID";

#[derive(Debug, Clone, Default)]
pub struct StorageErrorFactory;

#[allow(non_snake_case)]
impl StorageErrorFactory {
    pub fn notImplement(contextID: Option<&str>) -> StorageError {
        StorageError::new(
            500,
            "functionNotImplement",
            "No function.",
            contextID.unwrap_or(DEFAULT_ID),
            StorageError::empty_extra(),
        )
    }

    pub fn InternalError(contextID: Option<&str>) -> StorageError {
        StorageError::new(
            500,
            "InternalError",
            "The server encountered an internal error. Please retry the request.",
            contextID.unwrap_or(DEFAULT_ID),
            StorageError::empty_extra(),
        )
    }

    pub fn getInvalidHeaderValue(
        contextID: Option<&str>,
        additionalMessages: Option<BTreeMap<String, String>>,
    ) -> StorageError {
        StorageError::new(
            400,
            "InvalidHeaderValue",
            "The value for one of the HTTP headers is not in the correct format.",
            contextID.unwrap_or(""),
            additionalMessages.unwrap_or_default(),
        )
    }

    pub fn getInvalidAPIVersion(contextID: Option<&str>, apiVersion: Option<&str>) -> StorageError {
        StorageError::new(
            400,
            "InvalidHeaderValue",
            format!(
                "The API version {} is not supported by Azurite. Please upgrade Azurite to latest version and retry. If you are using Azurite in Visual Studio, please check you have installed latest Visual Studio patch. Azurite command line parameter \"--skipApiVersionCheck\" or Visual Studio Code configuration \"Skip Api Version Check\" can skip this error. ",
                apiVersion.unwrap_or("undefined")
            ),
            contextID.unwrap_or(""),
            StorageError::empty_extra(),
        )
    }

    pub fn corsPreflightFailure(
        contextID: Option<&str>,
        additionalMessages: Option<BTreeMap<String, String>>,
    ) -> StorageError {
        StorageError::new(
            403,
            "CorsPreflightFailure",
            "CORS not enabled or no matching rule found for this request.",
            contextID.unwrap_or(DEFAULT_ID),
            additionalMessages.unwrap_or_default(),
        )
    }

    pub fn getInvalidUri(
        contextID: Option<&str>,
        _additionalMessages: Option<BTreeMap<String, String>>,
    ) -> StorageError {
        StorageError::new(
            400,
            "InvalidUri",
            "The specified resource name contains invalid characters.",
            contextID.unwrap_or(DEFAULT_ID),
            StorageError::empty_extra(),
        )
    }

    pub fn getInvalidAuthenticationInfo(contextID: Option<&str>) -> StorageError {
        StorageError::new(
            400,
            "InvalidAuthenticationInfo",
            "Authentication information is not given in the correct format. Check the value of Authorization header.",
            contextID.unwrap_or(DEFAULT_ID),
            StorageError::empty_extra(),
        )
    }

    pub fn getAuthenticationFailed(
        contextID: Option<&str>,
        authenticationErrorDetail: impl Into<String>,
    ) -> StorageError {
        let mut additionalMessages = BTreeMap::new();
        additionalMessages.insert(
            String::from("AuthenticationErrorDetail"),
            authenticationErrorDetail.into(),
        );

        StorageError::new(
            403,
            "AuthenticationFailed",
            "Server failed to authenticate the request. Make sure the value of the Authorization header is formed correctly including the signature.",
            contextID.unwrap_or(DEFAULT_ID),
            additionalMessages,
        )
    }

    pub fn getInvalidCorsHeaderValue(
        contextID: Option<&str>,
        additionalMessages: Option<BTreeMap<String, String>>,
    ) -> StorageError {
        StorageError::new(
            400,
            "InvalidHeaderValue",
            "A required CORS header is not present.",
            contextID.unwrap_or(DEFAULT_ID),
            additionalMessages.unwrap_or_default(),
        )
    }

    pub fn getAuthorizationFailure(contextID: Option<&str>) -> StorageError {
        StorageError::new(
            403,
            "AuthenticationFailed",
            "Server failed to authenticate the request.Make sure the value of the Authorization header is formed correctly including the signature.",
            contextID.unwrap_or(DEFAULT_ID),
            StorageError::empty_extra(),
        )
    }

    pub fn getInvalidOperation(contextID: &str, message: Option<&str>) -> StorageError {
        StorageError::new(
            400,
            "InvalidOperation",
            message.unwrap_or(""),
            contextID,
            StorageError::empty_extra(),
        )
    }

    pub fn ResourceNotFound(contextID: &str) -> StorageError {
        StorageError::new(
            404,
            "ResourceNotFound",
            "The specified resource does not exist.",
            contextID,
            StorageError::empty_extra(),
        )
    }

    pub fn getAuthorizationSourceIPMismatch(contextID: &str) -> StorageError {
        StorageError::new(
            403,
            "AuthorizationSourceIPMismatch",
            "This request is not authorized to perform this operation using this source IP {SourceIP}.",
            contextID,
            StorageError::empty_extra(),
        )
    }

    pub fn getAuthorizationProtocolMismatch(contextID: &str) -> StorageError {
        StorageError::new(
            403,
            "AuthorizationProtocolMismatch",
            "This request is not authorized to perform this operation using this protocol.",
            contextID,
            StorageError::empty_extra(),
        )
    }

    pub fn getAuthorizationPermissionMismatch(contextID: &str) -> StorageError {
        StorageError::new(
            403,
            "AuthorizationPermissionMismatch",
            "This request is not authorized to perform this operation using this permission.",
            contextID,
            StorageError::empty_extra(),
        )
    }

    pub fn getAuthorizationServiceMismatch(contextID: &str) -> StorageError {
        StorageError::new(
            403,
            "AuthorizationServiceMismatch",
            "This request is not authorized to perform this operation using this service.",
            contextID,
            StorageError::empty_extra(),
        )
    }

    pub fn getAuthorizationResourceTypeMismatch(contextID: &str) -> StorageError {
        StorageError::new(
            403,
            "AuthorizationResourceTypeMismatch",
            "This request is not authorized to perform this operation using this resource type.",
            contextID,
            StorageError::empty_extra(),
        )
    }

    pub fn getInvalidXmlDocument(
        contextID: Option<&str>,
        additionalMessages: Option<BTreeMap<String, String>>,
    ) -> StorageError {
        StorageError::new(
            400,
            "InvalidXmlDocument",
            "XML specified is not syntactically valid.",
            contextID.unwrap_or(DEFAULT_ID),
            additionalMessages.unwrap_or_default(),
        )
    }

    pub fn getInvalidQueryParameterValue(
        contextID: Option<&str>,
        additionalMessages: Option<BTreeMap<String, String>>,
    ) -> StorageError {
        StorageError::new(
            400,
            "InvalidQueryParameterValue",
            "Value for one of the query parameters specified in the request URI is invalid.",
            contextID.unwrap_or(DEFAULT_ID),
            additionalMessages.unwrap_or_default(),
        )
    }

    pub fn getOutOfRangeQueryParameterValue(
        contextID: Option<&str>,
        additionalMessages: Option<BTreeMap<String, String>>,
    ) -> StorageError {
        StorageError::new(
            400,
            "OutOfRangeQueryParameterValue",
            "One of the query parameters specified in the request URI is outside the permissible range.",
            contextID.unwrap_or(DEFAULT_ID),
            additionalMessages.unwrap_or_default(),
        )
    }

    pub fn getRequestBodyTooLarge(
        contextID: Option<&str>,
        additionalMessages: Option<BTreeMap<String, String>>,
    ) -> StorageError {
        StorageError::new(
            413,
            "RequestBodyTooLarge",
            "The request body is too large and exceeds the maximum permissible limit.",
            contextID.unwrap_or(DEFAULT_ID),
            additionalMessages.unwrap_or_default(),
        )
    }

    pub fn getMessageTooLarge(contextID: Option<&str>) -> StorageError {
        StorageError::new(
            400,
            "MessageTooLarge",
            "The message exceeds the maximum allowed size.",
            contextID.unwrap_or(DEFAULT_ID),
            StorageError::empty_extra(),
        )
    }

    pub fn getPopReceiptMismatch(contextID: Option<&str>) -> StorageError {
        StorageError::new(
            400,
            "PopReceiptMismatch",
            "The specified pop receipt did not match the pop receipt for a dequeued message.",
            contextID.unwrap_or(DEFAULT_ID),
            StorageError::empty_extra(),
        )
    }

    pub fn getMessageNotFound(contextID: Option<&str>) -> StorageError {
        StorageError::new(
            404,
            "MessageNotFound",
            "The specified message does not exist.",
            contextID.unwrap_or(DEFAULT_ID),
            StorageError::empty_extra(),
        )
    }

    pub fn getInvalidResourceName(contextID: Option<&str>) -> StorageError {
        StorageError::new(
            400,
            "InvalidResourceName",
            "The specified resource name contains invalid characters.",
            contextID.unwrap_or(DEFAULT_ID),
            StorageError::empty_extra(),
        )
    }

    pub fn getOutOfRangeName(contextID: Option<&str>) -> StorageError {
        StorageError::new(
            400,
            "OutOfRangeInput",
            "The specified resource name length is not within the permissible limits.",
            contextID.unwrap_or(DEFAULT_ID),
            StorageError::empty_extra(),
        )
    }

    pub fn getQueueAlreadyExists(contextID: Option<&str>) -> StorageError {
        StorageError::new(
            409,
            "QueueAlreadyExists",
            "The specified queue already exists.",
            contextID.unwrap_or(DEFAULT_ID),
            StorageError::empty_extra(),
        )
    }

    pub fn getQueueNotFound(contextID: Option<&str>) -> StorageError {
        StorageError::new(
            404,
            "QueueNotFound",
            "The specified queue does not exist.",
            contextID.unwrap_or(DEFAULT_ID),
            StorageError::empty_extra(),
        )
    }
}
