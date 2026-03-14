use std::collections::BTreeMap;

use crate::generated::i_response::ResponseHeaderValue;

use super::storage_error::StorageError;

pub const DEFAULT_ID: &str = "DefaultBlobRequestID";

pub struct StorageErrorFactory;

#[allow(non_snake_case)]
impl StorageErrorFactory {
    pub fn getContainerNotFound(contextID: Option<&str>) -> StorageError {
        StorageError::new(404, String::from("ContainerNotFound"), String::from("The specified container does not exist."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getRequestEntityTooLarge(contextID: Option<&str>) -> StorageError {
        StorageError::new(413, String::from("RequestEntityTooLarge"), String::from("The uploaded entity blob is too large."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getBlockCountExceedsLimit(contextID: Option<&str>) -> StorageError {
        StorageError::new(409, String::from("BlockCountExceedsLimit"), String::from("The committed block count cannot exceed the maximum limit of 50,000 blocks."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getContainerAlreadyExists(contextID: Option<&str>) -> StorageError {
        StorageError::new(409, String::from("ContainerAlreadyExists"), String::from("The specified container already exists."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getBlobAlreadyExists(contextID: Option<&str>) -> StorageError {
        StorageError::new(409, String::from("BlobAlreadyExists"), String::from("The specified blob already exists."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getBlobNotFound(contextID: Option<&str>) -> StorageError {
        StorageError::new(404, String::from("BlobNotFound"), String::from("The specified blob does not exist."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn ResourceNotFound(contextID: Option<&str>) -> StorageError {
        StorageError::new(404, String::from("ResourceNotFound"), String::from("The specified resource does not exist."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getInvalidQueryParameterValue(contextID: Option<&str>, parameterName: Option<&str>, parameterValue: Option<&str>, reason: Option<&str>) -> StorageError {
        let mut additionalMessages = BTreeMap::new();
        if let Some(parameterName) = parameterName {
            additionalMessages.insert(String::from("QueryParameterName"), String::from(parameterName));
        }
        if let Some(parameterValue) = parameterValue {
            additionalMessages.insert(String::from("QueryParameterValue"), String::from(parameterValue));
        }
        if let Some(reason) = reason {
            additionalMessages.insert(String::from("Reason"), String::from(reason));
        }
        StorageError::new(400, String::from("InvalidQueryParameterValue"), format!("Value for one of the query parameters specified in the request URI is invalid.", ), String::from(contextID.unwrap_or(DEFAULT_ID)), additionalMessages)
    }

    pub fn getOutOfRangeInput(contextID: Option<&str>, parameterName: Option<&str>, parameterValue: Option<&str>, reason: Option<&str>) -> StorageError {
        let mut additionalMessages = BTreeMap::new();
        if let Some(parameterName) = parameterName {
            additionalMessages.insert(String::from("QueryParameterName"), String::from(parameterName));
        }
        if let Some(parameterValue) = parameterValue {
            additionalMessages.insert(String::from("QueryParameterValue"), String::from(parameterValue));
        }
        if let Some(reason) = reason {
            additionalMessages.insert(String::from("Reason"), String::from(reason));
        }
        StorageError::new(400, String::from("OutOfRangeInput"), format!("One of the request inputs is out of range.", ), String::from(contextID.unwrap_or(DEFAULT_ID)), additionalMessages)
    }

    pub fn getInvalidOperation(contextID: Option<&str>, message: Option<&str>) -> StorageError {
        StorageError::new(400, String::from("InvalidOperation"), String::from(message.unwrap_or("")), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getInvalidBlockList(contextID: Option<&str>) -> StorageError {
        StorageError::new(400, String::from("InvalidBlockList"), String::from("The specified block list is invalid."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getInvalidAuthenticationInfo(contextID: Option<&str>) -> StorageError {
        StorageError::new(400, String::from("InvalidAuthenticationInfo"), String::from("Authentication information is not given in the correct format. Check the value of Authorization header."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getMd5Mismatch(contextID: Option<&str>, userSpecifiedMd5: &str, serverCalculatedMd5: &str) -> StorageError {
        StorageError::new(400, String::from("Md5Mismatch"), String::from("The MD5 value specified in the request did not match with the MD5 value calculated by the server."), String::from(contextID.unwrap_or(DEFAULT_ID)), string_map(&[("UserSpecifiedMd5", userSpecifiedMd5), ("ServerCalculatedMd5", serverCalculatedMd5)]))
    }

    pub fn getInvalidPageRange(contextID: &str) -> StorageError {
        StorageError::new(416, String::from("Requested Range Not Satisfiable"), String::from("The page range specified is invalid."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getInvalidPageRange2(contextID: &str, contentRange: Option<&str>) -> StorageError {
        let mut returnValue = StorageError::new(416, String::from("InvalidRange"), String::from("The range specified is invalid for the current size of the resource."), String::from(contextID), StorageError::empty_extra());
        if let Some(contentRange) = contentRange {
            returnValue.headers_mut().insert(String::from("Content-Range"), ResponseHeaderValue::from(contentRange));
        }
        returnValue
    }

    pub fn getInvalidLeaseDuration(contextID: Option<&str>) -> StorageError {
        StorageError::new(400, String::from("InvalidHeaderValue"), String::from("The value for one of the HTTP headers is not in the correct format."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getInvalidLeaseBreakPeriod(contextID: Option<&str>) -> StorageError {
        StorageError::new(400, String::from("InvalidHeaderValue"), String::from("The value for one of the HTTP headers is not in the correct format."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getInvalidId(contextID: &str) -> StorageError {
        StorageError::new(400, String::from("InvalidHeaderValue"), String::from("The value for one of the HTTP headers is not in the correct format."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getInvalidBlobOrBlock(contextID: Option<&str>) -> StorageError {
        StorageError::new(400, String::from("InvalidBlobOrBlock"), String::from("The specified blob or block content is invalid."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getLeaseAlreadyPresent(contextID: Option<&str>) -> StorageError {
        StorageError::new(409, String::from("LeaseAlreadyPresent"), String::from("There is already a lease present."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getLeaseIsBreakingAndCannotBeAcquired(contextID: Option<&str>) -> StorageError {
        StorageError::new(409, String::from("LeaseIsBreakingAndCannotBeAcquired"), String::from("There is already a breaking lease, and can't  be acquired."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getLeaseNotPresentWithLeaseOperation(contextID: Option<&str>) -> StorageError {
        StorageError::new(409, String::from("LeaseNotPresentWithLeaseOperation"), String::from("There is currently no lease on the container or blob."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getLeaseIdMismatchWithLeaseOperation(contextID: Option<&str>) -> StorageError {
        StorageError::new(409, String::from("LeaseIdMismatchWithLeaseOperation"), String::from("The lease ID specified did not match the lease ID for the container or blob."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getLeaseIsBrokenAndCannotBeRenewed(contextID: Option<&str>) -> StorageError {
        StorageError::new(409, String::from("LeaseIsBrokenAndCannotBeRenewed"), String::from("The lease ID matched, but the lease has been broken explicitly and cannot be renewed."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getLeaseIsBreakingAndCannotBeChanged(contextID: Option<&str>) -> StorageError {
        StorageError::new(409, String::from("LeaseIsBreakingAndCannotBeChanged"), String::from("The lease ID matched, but the lease is currently in breaking state and cannot be changed."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getContainerLeaseIdMissing(contextID: Option<&str>) -> StorageError {
        StorageError::new(412, String::from("LeaseIdMissing"), String::from("There is currently a lease on the container and no lease ID was specified in the request."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getContainerLeaseIdMismatchWithContainerOperation(contextID: Option<&str>) -> StorageError {
        StorageError::new(412, String::from("LeaseIdMismatchWithContainerOperation"), String::from("The lease ID specified did not match the lease ID for the container."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getContainerLeaseLost(contextID: Option<&str>) -> StorageError {
        StorageError::new(412, String::from("LeaseNotPresentWithContainerOperation"), String::from("A lease ID was specified, but the lease for the container has expired."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getBlobLeaseIdMismatchWithLeaseOperation(contextID: &str) -> StorageError {
        StorageError::new(409, String::from("LeaseIdMismatchWithLeaseOperation"), String::from("The lease ID specified did not match the lease ID for the blob."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getBlobLeaseNotPresentWithLeaseOperation(contextID: &str) -> StorageError {
        StorageError::new(409, String::from("LeaseNotPresentWithLeaseOperation"), String::from("There is currently no lease on the blob."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getBlobSnapshotsPresent(contextID: Option<&str>) -> StorageError {
        StorageError::new(400, String::from("SnapshotsPresent"), String::from("This operation is not permitted because the blob is snapshot."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getBlobLeaseIdMissing(contextID: Option<&str>) -> StorageError {
        StorageError::new(412, String::from("LeaseIdMissing"), String::from("There is currently a lease on the blob and no lease ID was specified in the request."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getBlobLeaseIdMismatchWithBlobOperation(contextID: Option<&str>) -> StorageError {
        StorageError::new(412, String::from("LeaseIdMismatchWithBlobOperation"), String::from("The lease ID specified did not match the lease ID for the blob."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getBlobLeaseLost(contextID: Option<&str>) -> StorageError {
        StorageError::new(412, String::from("LeaseNotPresentWithBlobOperation"), String::from("A lease ID was specified, but the lease for the blob has expired."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getAuthorizationFailure(contextID: &str) -> StorageError {
        StorageError::new(403, String::from("AuthorizationFailure"), String::from("Server failed to authenticate the request. Make sure the value of the Authorization header is formed correctly including the signature."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getAuthenticationFailed(contextID: Option<&str>, authenticationErrorDetail: &str) -> StorageError {
        StorageError::new(403, String::from("AuthenticationFailed"), String::from("Server failed to authenticate the request. Make sure the value of the Authorization header is formed correctly including the signature."), String::from(contextID.unwrap_or(DEFAULT_ID)), string_map(&[("AuthenticationErrorDetail", authenticationErrorDetail)]))
    }

    pub fn getBlobInvalidBlobType(contextID: Option<&str>) -> StorageError {
        StorageError::new(409, String::from("InvalidBlobType"), String::from("The blob type is invalid for this operation."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getAccessTierNotSupportedForBlobType(contextID: &str) -> StorageError {
        StorageError::new(400, String::from("AccessTierNotSupportedForBlobType"), String::from("The access tier is not supported for this blob type."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getMultipleConditionHeadersNotSupported(contextID: &str) -> StorageError {
        StorageError::new(400, String::from("MultipleConditionHeadersNotSupported"), String::from("Multiple condition headers are not supported."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getBlobSnapshotsPresent_hassnapshot(contextID: &str) -> StorageError {
        StorageError::new(409, String::from("SnapshotsPresent"), String::from("This operation is not permitted because the blob has snapshots."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getBlobCannotChangeToLowerTier(contextID: &str) -> StorageError {
        StorageError::new(409, String::from("CannotChangeToLowerTier"), String::from("A higher blob tier has already been explicitly set."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getBlobBlobTierInadequateForContentLength(contextID: &str) -> StorageError {
        StorageError::new(409, String::from("BlobTierInadequateForContentLength"), String::from("Specified blob tier size limit cannot be less than content length."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getAuthorizationSourceIPMismatch(contextID: &str) -> StorageError {
        StorageError::new(403, String::from("AuthorizationSourceIPMismatch"), String::from("This request is not authorized to perform this operation using this source IP {SourceIP}."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getAuthorizationProtocolMismatch(contextID: &str) -> StorageError {
        StorageError::new(403, String::from("AuthorizationProtocolMismatch"), String::from("This request is not authorized to perform this operation using this protocol."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getAuthorizationPermissionMismatch(contextID: &str) -> StorageError {
        StorageError::new(403, String::from("AuthorizationPermissionMismatch"), String::from("This request is not authorized to perform this operation using this permission."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getAuthorizationServiceMismatch(contextID: &str) -> StorageError {
        StorageError::new(403, String::from("AuthorizationServiceMismatch"), String::from("This request is not authorized to perform this operation using this service."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getAuthorizationResourceTypeMismatch(contextID: &str) -> StorageError {
        StorageError::new(403, String::from("AuthorizationResourceTypeMismatch"), String::from("This request is not authorized to perform this operation using this resource type."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getFeatureVersionMismatch(contextID: &str) -> StorageError {
        StorageError::new(409, String::from("FeatureVersionMismatch"), String::from("Stored access policy contains a permission that is not supported by this version."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getCopyIdMismatch(contextID: &str) -> StorageError {
        StorageError::new(409, String::from("CopyIdMismatch"), String::from("The specified copy ID did not match the copy ID for the pending copy operation."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getBothUserTagsAndSourceTagsCopyPresentException(contextID: &str) -> StorageError {
        StorageError::new(400, String::from("BothUserTagsAndSourceTagsCopyPresentException"), String::from("x-ms-tags header must not be present with x-ms-copy-source-tag-option as COPY."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getNoPendingCopyOperation(contextID: &str) -> StorageError {
        StorageError::new(409, String::from("NoPendingCopyOperation"), String::from("There is currently no pending copy operation."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getSnapshotsPresent(contextID: &str) -> StorageError {
        StorageError::new(409, String::from("SnapshotsPresent"), String::from("This operation is not permitted while the blob has snapshots."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getConditionNotMet(contextID: &str) -> StorageError {
        StorageError::new(412, String::from("ConditionNotMet"), String::from("The condition specified using HTTP conditional header(s) is not met."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getMaxBlobSizeConditionNotMet(contextID: Option<&str>) -> StorageError {
        StorageError::new(412, String::from("MaxBlobSizeConditionNotMet"), String::from("The max blob size condition specified was not met."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getAppendPositionConditionNotMet(contextID: Option<&str>) -> StorageError {
        StorageError::new(412, String::from("AppendPositionConditionNotMet"), String::from("The append position condition specified was not met."), String::from(contextID.unwrap_or(DEFAULT_ID)), StorageError::empty_extra())
    }

    pub fn getSequenceNumberConditionNotMet(contextID: &str) -> StorageError {
        StorageError::new(412, String::from("SequenceNumberConditionNotMet"), String::from("The condition specified using HTTP conditional header(s) is not met."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getNotModified(contextID: &str) -> StorageError {
        StorageError::new(304, String::from("ConditionNotMet"), String::from("The condition specified using HTTP conditional header(s) is not met."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getUnsatisfiableCondition(contextID: &str) -> StorageError {
        StorageError::new(400, String::from("UnsatisfiableCondition"), String::from("The request includes an unsatisfiable condition for this operation."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getInvalidHeaderValue(contextID: Option<&str>, additionalMessages: Option<BTreeMap<String, String>>) -> StorageError {
        let additionalMessages = additionalMessages.unwrap_or_default();
        StorageError::new(400, String::from("InvalidHeaderValue"), String::from("The value for one of the HTTP headers is not in the correct format."), String::from(contextID.unwrap_or("")), additionalMessages)
    }

    pub fn getInvalidAPIVersion(contextID: Option<&str>, apiVersion: Option<&str>) -> StorageError {
        StorageError::new(400, String::from("InvalidHeaderValue"), format!("The API version {} is not supported by Azurite. Please upgrade Azurite to latest version and retry. If you are using Azurite in Visual Studio, please check you have installed latest Visual Studio patch. Azurite command line parameter \\\"--skipApiVersionCheck\\\" or Visual Studio Code configuration \\\"Skip Api Version Check\\\" can skip this error. ", apiVersion.unwrap_or("")), String::from(contextID.unwrap_or("")), StorageError::empty_extra())
    }

    pub fn getBlobArchived(contextID: Option<&str>, additionalMessages: Option<BTreeMap<String, String>>) -> StorageError {
        let additionalMessages = additionalMessages.unwrap_or_default();
        StorageError::new(409, String::from("BlobArchived"), String::from("This operation is not permitted on an archived blob."), String::from(contextID.unwrap_or("")), additionalMessages)
    }

    pub fn getInvalidCorsHeaderValue(contextID: Option<&str>, additionalMessages: Option<BTreeMap<String, String>>) -> StorageError {
        StorageError::new(400, String::from("InvalidHeaderValue"), String::from("A required CORS header is not present."), String::from(contextID.unwrap_or("")), additionalMessages.unwrap_or_default())
    }

    pub fn corsPreflightFailure(contextID: Option<&str>, additionalMessages: Option<BTreeMap<String, String>>) -> StorageError {
        StorageError::new(403, String::from("CorsPreflightFailure"), String::from("CORS not enabled or no matching rule found for this request."), String::from(contextID.unwrap_or("")), additionalMessages.unwrap_or_default())
    }

    pub fn getCannotVerifyCopySource(contextID: &str, statusCode: u16, message: &str, additionalMessages: Option<BTreeMap<String, String>>) -> StorageError {
        StorageError::new(statusCode, String::from("CannotVerifyCopySource"), String::from(message), String::from(contextID), additionalMessages.unwrap_or_default())
    }

    pub fn getInvalidResourceName(contextID: Option<&str>) -> StorageError {
        StorageError::new(400, String::from("InvalidResourceName"), String::from("The specified resource name contains invalid characters."), String::from(contextID.unwrap_or("")), StorageError::empty_extra())
    }

    pub fn getOutOfRangeName(contextID: Option<&str>) -> StorageError {
        StorageError::new(400, String::from("OutOfRangeInput"), String::from("The specified resource name length is not within the permissible limits."), String::from(contextID.unwrap_or("")), StorageError::empty_extra())
    }

    pub fn getUnexpectedSyncCopyStatus(contextID: &str, copyStatus: &str) -> StorageError {
        StorageError::new(409, String::from("UnexpectedSyncCopyStatus"), String::from("Expected copyStatus to be \"success\" but got different status."), String::from(contextID), string_map(&[("ReceivedCopyStatus", copyStatus)]))
    }

    pub fn getInvalidMetadata(contextID: &str) -> StorageError {
        StorageError::new(400, String::from("InvalidMetadata"), String::from("The metadata specified is invalid. It has characters that are not permitted."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getEmptyTagName(contextID: &str) -> StorageError {
        StorageError::new(400, String::from("EmptyTagName"), String::from("The name of one of the tag key-value pairs is empty."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getDuplicateTagNames(contextID: &str) -> StorageError {
        StorageError::new(400, String::from("DuplicateTagNames"), String::from("The tags specified contain duplicate names."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getTagsTooLarge(contextID: &str) -> StorageError {
        StorageError::new(400, String::from("TagsTooLarge"), String::from("The tags specified exceed the maximum permissible limit."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getInvalidTag(contextID: &str) -> StorageError {
        StorageError::new(400, String::from("DuplicateTagNames"), String::from("The tags specified are invalid. It contains characters that are not permitted."), String::from(contextID), StorageError::empty_extra())
    }

    pub fn getInvalidXmlDocument(contextID: Option<&str>) -> StorageError {
        StorageError::new(400, String::from("InvalidXmlDocument"), String::from("XML specified is not syntactically valid."), String::from(contextID.unwrap_or("")), StorageError::empty_extra())
    }

    pub fn getBlobSealed(contextID: Option<&str>) -> StorageError {
        StorageError::new(409, String::from("BlobIsSealed"), String::from("The specified blob is sealed, and its contents can't be modified unless the blob is re-created after a delete."), String::from(contextID.unwrap_or("")), StorageError::empty_extra())
    }

}

fn string_map(pairs: &[(&'static str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| (String::from(*key), String::from(*value)))
        .collect()
}
