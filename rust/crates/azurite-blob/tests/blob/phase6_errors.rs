use std::collections::BTreeMap;

use azurite_blob::context::BlobStorageContext;
use azurite_blob::errors::{
    NotImplementedError, NotImplementedinSQLError, StorageError, StorageErrorFactory,
    StrictModelNotSupportedError,
};
use azurite_blob::generated::artifacts::models::GeneratedValue;
use azurite_blob::generated::context::Context;
use azurite_blob::generated::i_response::ResponseHeaderValue;
use pretty_assertions::assert_eq;
use quick_xml::escape::escape;
use regex::Regex;

const REQUEST_ID: &str = "req-123";
const DEFAULT_BLOB_REQUEST_ID: &str = "DefaultBlobRequestID";
const DYNAMIC_MESSAGE: &str = "dynamic message";
const AUTH_DETAIL: &str = "signature mismatch";
const API_VERSION: &str = "2099-01-01";
const TIMESTAMP_PATTERN: &str = r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z";

struct FactoryCase {
    name: &'static str,
    error: StorageError,
    status: u16,
    code: &'static str,
    message: &'static str,
    request_id: &'static str,
    extra_fragments: &'static [(&'static str, &'static str)],
    header_fragments: &'static [(&'static str, &'static str)],
}

fn case(
    name: &'static str,
    error: StorageError,
    status: u16,
    code: &'static str,
    message: &'static str,
) -> FactoryCase {
    FactoryCase {
        name,
        error,
        status,
        code,
        message,
        request_id: REQUEST_ID,
        extra_fragments: &[],
        header_fragments: &[],
    }
}

fn extra_messages() -> BTreeMap<String, String> {
    BTreeMap::from([
        (String::from("Alpha"), String::from("alpha")),
        (String::from("Beta"), String::from("beta")),
    ])
}

fn body_xml(error: &StorageError) -> String {
    match error
        .body
        .as_ref()
        .expect("storage error body should exist")
    {
        GeneratedValue::String(value) => value.clone(),
        other => panic!("expected XML string body, got {other:?}"),
    }
}

fn header_value(error: &StorageError, name: &str) -> Option<String> {
    error
        .headers
        .as_ref()
        .and_then(|headers| headers.get(name))
        .and_then(ResponseHeaderValue::as_single)
}

fn assert_storage_error_case(case: FactoryCase) {
    assert_eq!(case.error.statusCode, case.status, "{} status", case.name);
    assert_eq!(case.error.message, case.message, "{} message", case.name);
    assert_eq!(
        case.error.statusMessage.as_deref(),
        Some(case.message),
        "{} status message",
        case.name
    );
    assert_eq!(
        case.error.storageErrorCode, case.code,
        "{} storage code",
        case.name
    );
    assert_eq!(
        case.error.storageErrorMessage, case.message,
        "{} storage message",
        case.name
    );
    assert_eq!(
        case.error.storageRequestID, case.request_id,
        "{} request id",
        case.name
    );
    assert_eq!(
        case.error.contentType.as_deref(),
        Some("application/xml"),
        "{} content type",
        case.name
    );
    assert_eq!(
        header_value(&case.error, "x-ms-error-code").as_deref(),
        Some(case.code),
        "{} x-ms-error-code header",
        case.name
    );
    assert_eq!(
        header_value(&case.error, "x-ms-request-id").as_deref(),
        Some(case.request_id),
        "{} x-ms-request-id header",
        case.name
    );

    for (name, value) in case.header_fragments {
        assert_eq!(
            header_value(&case.error, name).as_deref(),
            Some(*value),
            "{} header {name}",
            case.name
        );
    }

    let escaped_message = escape(case.message).to_string();
    let escaped_request_id = escape(case.request_id).to_string();
    let body = body_xml(&case.error);
    let message_regex = Regex::new(&format!(
        r"(?s)<Message>{}\nRequestId:{}\nTime:{}</Message>",
        regex::escape(&escaped_message),
        regex::escape(&escaped_request_id),
        TIMESTAMP_PATTERN,
    ))
    .expect("message regex should compile");

    assert!(
        body.starts_with(&format!("<Error><Code>{}</Code>", escape(case.code))),
        "{} body should start with code: {body}",
        case.name
    );
    assert!(
        message_regex.is_match(&body),
        "{} message body mismatch: {body}",
        case.name
    );
    assert!(
        body.ends_with("</Error>"),
        "{} body should end with </Error>: {body}",
        case.name
    );

    for (name, value) in case.extra_fragments {
        let fragment = format!("<{name}>{}</{name}>", escape(*value));
        assert!(
            body.contains(&fragment),
            "{} missing body fragment {fragment}: {body}",
            case.name
        );
    }
}

#[test]
fn storage_error_constructor_matches_ts_xml_headers_and_timestamp_shape() {
    let error = StorageError::new(
        409,
        "Code<&>",
        "Quoted \"message\" & <xml>",
        REQUEST_ID,
        BTreeMap::from([(String::from("Extra"), String::from("1 < 2 & 3"))]),
    );

    assert_eq!(error.statusCode, 409);
    assert_eq!(error.message, "Quoted \"message\" & <xml>");
    assert_eq!(
        error.statusMessage.as_deref(),
        Some("Quoted \"message\" & <xml>")
    );
    assert_eq!(error.storageErrorCode, "Code<&>");
    assert_eq!(error.storageRequestID, REQUEST_ID);
    assert_eq!(
        header_value(&error, "x-ms-error-code").as_deref(),
        Some("Code<&>")
    );
    assert_eq!(
        header_value(&error, "x-ms-request-id").as_deref(),
        Some(REQUEST_ID)
    );

    let body = body_xml(&error);
    assert!(body.contains("<Code>Code&lt;&amp;&gt;</Code>"));
    assert!(body.contains("<Extra>1 &lt; 2 &amp; 3</Extra>"));

    let message_regex = Regex::new(&format!(
        r#"(?s)<Message>Quoted &quot;message&quot; &amp; &lt;xml&gt;\nRequestId:{}\nTime:{}</Message>"#,
        REQUEST_ID, TIMESTAMP_PATTERN
    ))
    .expect("storage error message regex should compile");
    assert!(message_regex.is_match(&body), "unexpected XML body: {body}");
}

#[test]
fn storage_error_factory_methods_match_ts_status_codes_messages_headers_and_request_ids() {
    const QUERY_FIELDS: &[(&str, &str)] = &[
        ("QueryParameterName", "prefix"),
        ("QueryParameterValue", "value"),
        ("Reason", "because"),
    ];
    const MD5_FIELDS: &[(&str, &str)] = &[
        ("UserSpecifiedMd5", "user-md5"),
        ("ServerCalculatedMd5", "server-md5"),
    ];
    const AUTH_FIELDS: &[(&str, &str)] = &[("AuthenticationErrorDetail", AUTH_DETAIL)];
    const CONTENT_RANGE_HEADER: &[(&str, &str)] = &[("Content-Range", "bytes */5")];
    const EXTRA_FIELDS: &[(&str, &str)] = &[("Alpha", "alpha"), ("Beta", "beta")];
    const COPY_STATUS_FIELDS: &[(&str, &str)] = &[("ReceivedCopyStatus", "pending")];

    let cases = vec![
        case("getContainerNotFound", StorageErrorFactory::getContainerNotFound(Some(REQUEST_ID)), 404, "ContainerNotFound", "The specified container does not exist."),
        case("getRequestEntityTooLarge", StorageErrorFactory::getRequestEntityTooLarge(Some(REQUEST_ID)), 413, "RequestEntityTooLarge", "The uploaded entity blob is too large."),
        case("getBlockCountExceedsLimit", StorageErrorFactory::getBlockCountExceedsLimit(Some(REQUEST_ID)), 409, "BlockCountExceedsLimit", "The committed block count cannot exceed the maximum limit of 50,000 blocks."),
        case("getContainerAlreadyExists", StorageErrorFactory::getContainerAlreadyExists(Some(REQUEST_ID)), 409, "ContainerAlreadyExists", "The specified container already exists."),
        case("getBlobAlreadyExists", StorageErrorFactory::getBlobAlreadyExists(Some(REQUEST_ID)), 409, "BlobAlreadyExists", "The specified blob already exists."),
        case("getBlobNotFound", StorageErrorFactory::getBlobNotFound(Some(REQUEST_ID)), 404, "BlobNotFound", "The specified blob does not exist."),
        case("ResourceNotFound", StorageErrorFactory::ResourceNotFound(Some(REQUEST_ID)), 404, "ResourceNotFound", "The specified resource does not exist."),
        FactoryCase { extra_fragments: QUERY_FIELDS, ..case("getInvalidQueryParameterValue", StorageErrorFactory::getInvalidQueryParameterValue(Some(REQUEST_ID), Some("prefix"), Some("value"), Some("because")), 400, "InvalidQueryParameterValue", "Value for one of the query parameters specified in the request URI is invalid.") },
        FactoryCase { extra_fragments: QUERY_FIELDS, ..case("getOutOfRangeInput", StorageErrorFactory::getOutOfRangeInput(Some(REQUEST_ID), Some("prefix"), Some("value"), Some("because")), 400, "OutOfRangeInput", "One of the request inputs is out of range.") },
        case("getInvalidOperation", StorageErrorFactory::getInvalidOperation(Some(REQUEST_ID), Some(DYNAMIC_MESSAGE)), 400, "InvalidOperation", DYNAMIC_MESSAGE),
        case("getInvalidBlockList", StorageErrorFactory::getInvalidBlockList(Some(REQUEST_ID)), 400, "InvalidBlockList", "The specified block list is invalid."),
        case("getInvalidAuthenticationInfo", StorageErrorFactory::getInvalidAuthenticationInfo(Some(REQUEST_ID)), 400, "InvalidAuthenticationInfo", "Authentication information is not given in the correct format. Check the value of Authorization header."),
        FactoryCase { extra_fragments: MD5_FIELDS, ..case("getMd5Mismatch", StorageErrorFactory::getMd5Mismatch(Some(REQUEST_ID), "user-md5", "server-md5"), 400, "Md5Mismatch", "The MD5 value specified in the request did not match with the MD5 value calculated by the server.") },
        case("getInvalidPageRange", StorageErrorFactory::getInvalidPageRange(REQUEST_ID), 416, "Requested Range Not Satisfiable", "The page range specified is invalid."),
        FactoryCase { header_fragments: CONTENT_RANGE_HEADER, ..case("getInvalidPageRange2", StorageErrorFactory::getInvalidPageRange2(REQUEST_ID, Some("bytes */5")), 416, "InvalidRange", "The range specified is invalid for the current size of the resource.") },
        case("getInvalidLeaseDuration", StorageErrorFactory::getInvalidLeaseDuration(Some(REQUEST_ID)), 400, "InvalidHeaderValue", "The value for one of the HTTP headers is not in the correct format."),
        case("getInvalidLeaseBreakPeriod", StorageErrorFactory::getInvalidLeaseBreakPeriod(Some(REQUEST_ID)), 400, "InvalidHeaderValue", "The value for one of the HTTP headers is not in the correct format."),
        case("getInvalidId", StorageErrorFactory::getInvalidId(REQUEST_ID), 400, "InvalidHeaderValue", "The value for one of the HTTP headers is not in the correct format."),
        case("getInvalidBlobOrBlock", StorageErrorFactory::getInvalidBlobOrBlock(Some(REQUEST_ID)), 400, "InvalidBlobOrBlock", "The specified blob or block content is invalid."),
        case("getLeaseAlreadyPresent", StorageErrorFactory::getLeaseAlreadyPresent(Some(REQUEST_ID)), 409, "LeaseAlreadyPresent", "There is already a lease present."),
        case("getLeaseIsBreakingAndCannotBeAcquired", StorageErrorFactory::getLeaseIsBreakingAndCannotBeAcquired(Some(REQUEST_ID)), 409, "LeaseIsBreakingAndCannotBeAcquired", "There is already a breaking lease, and can't  be acquired."),
        case("getLeaseNotPresentWithLeaseOperation", StorageErrorFactory::getLeaseNotPresentWithLeaseOperation(Some(REQUEST_ID)), 409, "LeaseNotPresentWithLeaseOperation", "There is currently no lease on the container or blob."),
        case("getLeaseIdMismatchWithLeaseOperation", StorageErrorFactory::getLeaseIdMismatchWithLeaseOperation(Some(REQUEST_ID)), 409, "LeaseIdMismatchWithLeaseOperation", "The lease ID specified did not match the lease ID for the container or blob."),
        case("getLeaseIsBrokenAndCannotBeRenewed", StorageErrorFactory::getLeaseIsBrokenAndCannotBeRenewed(Some(REQUEST_ID)), 409, "LeaseIsBrokenAndCannotBeRenewed", "The lease ID matched, but the lease has been broken explicitly and cannot be renewed."),
        case("getLeaseIsBreakingAndCannotBeChanged", StorageErrorFactory::getLeaseIsBreakingAndCannotBeChanged(Some(REQUEST_ID)), 409, "LeaseIsBreakingAndCannotBeChanged", "The lease ID matched, but the lease is currently in breaking state and cannot be changed."),
        case("getContainerLeaseIdMissing", StorageErrorFactory::getContainerLeaseIdMissing(Some(REQUEST_ID)), 412, "LeaseIdMissing", "There is currently a lease on the container and no lease ID was specified in the request."),
        case("getContainerLeaseIdMismatchWithContainerOperation", StorageErrorFactory::getContainerLeaseIdMismatchWithContainerOperation(Some(REQUEST_ID)), 412, "LeaseIdMismatchWithContainerOperation", "The lease ID specified did not match the lease ID for the container."),
        case("getContainerLeaseLost", StorageErrorFactory::getContainerLeaseLost(Some(REQUEST_ID)), 412, "LeaseNotPresentWithContainerOperation", "A lease ID was specified, but the lease for the container has expired."),
        case("getBlobLeaseIdMismatchWithLeaseOperation", StorageErrorFactory::getBlobLeaseIdMismatchWithLeaseOperation(REQUEST_ID), 409, "LeaseIdMismatchWithLeaseOperation", "The lease ID specified did not match the lease ID for the blob."),
        case("getBlobLeaseNotPresentWithLeaseOperation", StorageErrorFactory::getBlobLeaseNotPresentWithLeaseOperation(REQUEST_ID), 409, "LeaseNotPresentWithLeaseOperation", "There is currently no lease on the blob."),
        case("getBlobSnapshotsPresent", StorageErrorFactory::getBlobSnapshotsPresent(Some(REQUEST_ID)), 400, "SnapshotsPresent", "This operation is not permitted because the blob is snapshot."),
        case("getBlobLeaseIdMissing", StorageErrorFactory::getBlobLeaseIdMissing(Some(REQUEST_ID)), 412, "LeaseIdMissing", "There is currently a lease on the blob and no lease ID was specified in the request."),
        case("getBlobLeaseIdMismatchWithBlobOperation", StorageErrorFactory::getBlobLeaseIdMismatchWithBlobOperation(Some(REQUEST_ID)), 412, "LeaseIdMismatchWithBlobOperation", "The lease ID specified did not match the lease ID for the blob."),
        case("getBlobLeaseLost", StorageErrorFactory::getBlobLeaseLost(Some(REQUEST_ID)), 412, "LeaseNotPresentWithBlobOperation", "A lease ID was specified, but the lease for the blob has expired."),
        case("getAuthorizationFailure", StorageErrorFactory::getAuthorizationFailure(REQUEST_ID), 403, "AuthorizationFailure", "Server failed to authenticate the request. Make sure the value of the Authorization header is formed correctly including the signature."),
        FactoryCase { extra_fragments: AUTH_FIELDS, ..case("getAuthenticationFailed", StorageErrorFactory::getAuthenticationFailed(Some(REQUEST_ID), AUTH_DETAIL), 403, "AuthenticationFailed", "Server failed to authenticate the request. Make sure the value of the Authorization header is formed correctly including the signature.") },
        case("getBlobInvalidBlobType", StorageErrorFactory::getBlobInvalidBlobType(Some(REQUEST_ID)), 409, "InvalidBlobType", "The blob type is invalid for this operation."),
        case("getAccessTierNotSupportedForBlobType", StorageErrorFactory::getAccessTierNotSupportedForBlobType(REQUEST_ID), 400, "AccessTierNotSupportedForBlobType", "The access tier is not supported for this blob type."),
        case("getMultipleConditionHeadersNotSupported", StorageErrorFactory::getMultipleConditionHeadersNotSupported(REQUEST_ID), 400, "MultipleConditionHeadersNotSupported", "Multiple condition headers are not supported."),
        case("getBlobSnapshotsPresent_hassnapshot", StorageErrorFactory::getBlobSnapshotsPresent_hassnapshot(REQUEST_ID), 409, "SnapshotsPresent", "This operation is not permitted because the blob has snapshots."),
        case("getBlobCannotChangeToLowerTier", StorageErrorFactory::getBlobCannotChangeToLowerTier(REQUEST_ID), 409, "CannotChangeToLowerTier", "A higher blob tier has already been explicitly set."),
        case("getBlobBlobTierInadequateForContentLength", StorageErrorFactory::getBlobBlobTierInadequateForContentLength(REQUEST_ID), 409, "BlobTierInadequateForContentLength", "Specified blob tier size limit cannot be less than content length."),
        case("getAuthorizationSourceIPMismatch", StorageErrorFactory::getAuthorizationSourceIPMismatch(REQUEST_ID), 403, "AuthorizationSourceIPMismatch", "This request is not authorized to perform this operation using this source IP {SourceIP}."),
        case("getAuthorizationProtocolMismatch", StorageErrorFactory::getAuthorizationProtocolMismatch(REQUEST_ID), 403, "AuthorizationProtocolMismatch", "This request is not authorized to perform this operation using this protocol."),
        case("getAuthorizationPermissionMismatch", StorageErrorFactory::getAuthorizationPermissionMismatch(REQUEST_ID), 403, "AuthorizationPermissionMismatch", "This request is not authorized to perform this operation using this permission."),
        case("getAuthorizationServiceMismatch", StorageErrorFactory::getAuthorizationServiceMismatch(REQUEST_ID), 403, "AuthorizationServiceMismatch", "This request is not authorized to perform this operation using this service."),
        case("getAuthorizationResourceTypeMismatch", StorageErrorFactory::getAuthorizationResourceTypeMismatch(REQUEST_ID), 403, "AuthorizationResourceTypeMismatch", "This request is not authorized to perform this operation using this resource type."),
        case("getFeatureVersionMismatch", StorageErrorFactory::getFeatureVersionMismatch(REQUEST_ID), 409, "FeatureVersionMismatch", "Stored access policy contains a permission that is not supported by this version."),
        case("getCopyIdMismatch", StorageErrorFactory::getCopyIdMismatch(REQUEST_ID), 409, "CopyIdMismatch", "The specified copy ID did not match the copy ID for the pending copy operation."),
        case("getBothUserTagsAndSourceTagsCopyPresentException", StorageErrorFactory::getBothUserTagsAndSourceTagsCopyPresentException(REQUEST_ID), 400, "BothUserTagsAndSourceTagsCopyPresentException", "x-ms-tags header must not be present with x-ms-copy-source-tag-option as COPY."),
        case("getNoPendingCopyOperation", StorageErrorFactory::getNoPendingCopyOperation(REQUEST_ID), 409, "NoPendingCopyOperation", "There is currently no pending copy operation."),
        case("getSnapshotsPresent", StorageErrorFactory::getSnapshotsPresent(REQUEST_ID), 409, "SnapshotsPresent", "This operation is not permitted while the blob has snapshots."),
        case("getConditionNotMet", StorageErrorFactory::getConditionNotMet(REQUEST_ID), 412, "ConditionNotMet", "The condition specified using HTTP conditional header(s) is not met."),
        case("getMaxBlobSizeConditionNotMet", StorageErrorFactory::getMaxBlobSizeConditionNotMet(Some(REQUEST_ID)), 412, "MaxBlobSizeConditionNotMet", "The max blob size condition specified was not met."),
        case("getAppendPositionConditionNotMet", StorageErrorFactory::getAppendPositionConditionNotMet(Some(REQUEST_ID)), 412, "AppendPositionConditionNotMet", "The append position condition specified was not met."),
        case("getSequenceNumberConditionNotMet", StorageErrorFactory::getSequenceNumberConditionNotMet(REQUEST_ID), 412, "SequenceNumberConditionNotMet", "The condition specified using HTTP conditional header(s) is not met."),
        case("getNotModified", StorageErrorFactory::getNotModified(REQUEST_ID), 304, "ConditionNotMet", "The condition specified using HTTP conditional header(s) is not met."),
        case("getUnsatisfiableCondition", StorageErrorFactory::getUnsatisfiableCondition(REQUEST_ID), 400, "UnsatisfiableCondition", "The request includes an unsatisfiable condition for this operation."),
        FactoryCase { extra_fragments: EXTRA_FIELDS, ..case("getInvalidHeaderValue", StorageErrorFactory::getInvalidHeaderValue(Some(REQUEST_ID), Some(extra_messages())), 400, "InvalidHeaderValue", "The value for one of the HTTP headers is not in the correct format.") },
        case("getInvalidAPIVersion", StorageErrorFactory::getInvalidAPIVersion(Some(REQUEST_ID), Some(API_VERSION)), 400, "InvalidHeaderValue", "The API version 2099-01-01 is not supported by Azurite. Please upgrade Azurite to latest version and retry. If you are using Azurite in Visual Studio, please check you have installed latest Visual Studio patch. Azurite command line parameter \"--skipApiVersionCheck\" or Visual Studio Code configuration \"Skip Api Version Check\" can skip this error. "),
        FactoryCase { extra_fragments: EXTRA_FIELDS, ..case("getBlobArchived", StorageErrorFactory::getBlobArchived(Some(REQUEST_ID), Some(extra_messages())), 409, "BlobArchived", "This operation is not permitted on an archived blob.") },
        FactoryCase { extra_fragments: EXTRA_FIELDS, ..case("getInvalidCorsHeaderValue", StorageErrorFactory::getInvalidCorsHeaderValue(Some(REQUEST_ID), Some(extra_messages())), 400, "InvalidHeaderValue", "A required CORS header is not present.") },
        FactoryCase { extra_fragments: EXTRA_FIELDS, ..case("corsPreflightFailure", StorageErrorFactory::corsPreflightFailure(Some(REQUEST_ID), Some(extra_messages())), 403, "CorsPreflightFailure", "CORS not enabled or no matching rule found for this request.") },
        FactoryCase { extra_fragments: EXTRA_FIELDS, ..case("getCannotVerifyCopySource", StorageErrorFactory::getCannotVerifyCopySource(REQUEST_ID, 418, DYNAMIC_MESSAGE, Some(extra_messages())), 418, "CannotVerifyCopySource", DYNAMIC_MESSAGE) },
        case("getInvalidResourceName", StorageErrorFactory::getInvalidResourceName(Some(REQUEST_ID)), 400, "InvalidResourceName", "The specified resource name contains invalid characters."),
        case("getOutOfRangeName", StorageErrorFactory::getOutOfRangeName(Some(REQUEST_ID)), 400, "OutOfRangeInput", "The specified resource name length is not within the permissible limits."),
        FactoryCase { extra_fragments: COPY_STATUS_FIELDS, ..case("getUnexpectedSyncCopyStatus", StorageErrorFactory::getUnexpectedSyncCopyStatus(REQUEST_ID, "pending"), 409, "UnexpectedSyncCopyStatus", "Expected copyStatus to be \"success\" but got different status.") },
        case("getInvalidMetadata", StorageErrorFactory::getInvalidMetadata(REQUEST_ID), 400, "InvalidMetadata", "The metadata specified is invalid. It has characters that are not permitted."),
        case("getEmptyTagName", StorageErrorFactory::getEmptyTagName(REQUEST_ID), 400, "EmptyTagName", "The name of one of the tag key-value pairs is empty."),
        case("getDuplicateTagNames", StorageErrorFactory::getDuplicateTagNames(REQUEST_ID), 400, "DuplicateTagNames", "The tags specified contain duplicate names."),
        case("getTagsTooLarge", StorageErrorFactory::getTagsTooLarge(REQUEST_ID), 400, "TagsTooLarge", "The tags specified exceed the maximum permissible limit."),
        case("getInvalidTag", StorageErrorFactory::getInvalidTag(REQUEST_ID), 400, "DuplicateTagNames", "The tags specified are invalid. It contains characters that are not permitted."),
        case("getInvalidXmlDocument", StorageErrorFactory::getInvalidXmlDocument(Some(REQUEST_ID)), 400, "InvalidXmlDocument", "XML specified is not syntactically valid."),
        case("getBlobSealed", StorageErrorFactory::getBlobSealed(Some(REQUEST_ID)), 409, "BlobIsSealed", "The specified blob is sealed, and its contents can't be modified unless the blob is re-created after a delete."),
    ];

    assert_eq!(
        cases.len(),
        74,
        "update this test when factory coverage changes"
    );

    for case in cases {
        assert_storage_error_case(case);
    }
}

#[test]
fn storage_error_factory_preserves_ts_default_request_id_quirks() {
    assert_eq!(
        StorageErrorFactory::getContainerNotFound(None).storageRequestID,
        DEFAULT_BLOB_REQUEST_ID
    );
    assert_eq!(
        StorageErrorFactory::getInvalidHeaderValue(None, None).storageRequestID,
        ""
    );
    assert_eq!(
        StorageErrorFactory::getInvalidAPIVersion(None, None).storageRequestID,
        ""
    );
    assert_eq!(
        StorageErrorFactory::getInvalidResourceName(None).storageRequestID,
        ""
    );
    assert_eq!(
        StorageErrorFactory::getBlobSealed(None).storageRequestID,
        ""
    );
}

#[test]
fn simple_blob_error_types_match_ts_constructors() {
    let not_implemented = NotImplementedError::new(Some(REQUEST_ID));
    assert_storage_error_case(case(
        "NotImplementedError",
        not_implemented.into(),
        501,
        "APINotImplemented",
        "Current API is not implemented yet. Please vote your wanted features to https://github.com/azure/azurite/issues",
    ));

    let sql_not_implemented = NotImplementedinSQLError::new(Some(REQUEST_ID));
    assert_storage_error_case(case(
        "NotImplementedinSQLError",
        sql_not_implemented.into(),
        501,
        "APINotImplemented",
        "Current API is not implemented yet when use a SQL database based metadata storage. Please vote your wanted features to https://github.com/azure/azurite/issues",
    ));

    let strict = StrictModelNotSupportedError::new("SAS Encryption Scope 'ses'", Some(REQUEST_ID));
    assert_storage_error_case(case(
        "StrictModelNotSupportedError",
        strict.into(),
        500,
        "FeatureNotSupported",
        "SAS Encryption Scope 'ses' header or parameter is not supported in Azurite strict mode. Switch to loose model by Azurite command line parameter \"--loose\" or Visual Studio Code configuration \"Loose\". Please vote your wanted features to https://github.com/azure/azurite/issues",
    ));
}

#[test]
fn blob_storage_context_propagates_fields_and_request_id_aliases_like_ts() {
    let holder = Context::new_holder();
    let context = Context::from_holder(holder, "/blob", None, None);
    context.setContextId(Some(String::from("ctx-0")));

    let blob_context = BlobStorageContext::new(&context);
    blob_context.setAccount(Some(String::from("acct")));
    blob_context.setIsSecondary(Some(true));
    blob_context.setContainer(Some(String::from("container")));
    blob_context.setBlob(Some(String::from("blob")));
    blob_context.setAuthenticationPath(Some(String::from("/acct/container/blob")));
    blob_context.setDisableProductStyleUrl(Some(false));
    blob_context.setLoose(Some(true));
    blob_context.setXMsRequestID(Some(String::from("ctx-1")));

    assert_eq!(blob_context.account().as_deref(), Some("acct"));
    assert_eq!(blob_context.isSecondary(), Some(true));
    assert_eq!(blob_context.container().as_deref(), Some("container"));
    assert_eq!(blob_context.getContainer().as_deref(), Some("container"));
    assert_eq!(blob_context.blob().as_deref(), Some("blob"));
    assert_eq!(
        blob_context.authenticationPath().as_deref(),
        Some("/acct/container/blob")
    );
    assert_eq!(blob_context.disableProductStyleUrl(), Some(false));
    assert_eq!(blob_context.loose(), Some(true));
    assert_eq!(blob_context.xMsRequestID().as_deref(), Some("ctx-1"));
    assert_eq!(context.contextId().as_deref(), Some("ctx-1"));

    let wrapped_clone = BlobStorageContext::new(&context);
    assert_eq!(wrapped_clone.account().as_deref(), Some("acct"));
    assert_eq!(wrapped_clone.container().as_deref(), Some("container"));
    assert_eq!(wrapped_clone.blob().as_deref(), Some("blob"));

    blob_context.setBlob(None);
    blob_context.setContainer(None);
    assert_eq!(BlobStorageContext::new(&context).blob(), None);
    assert_eq!(BlobStorageContext::new(&context).container(), None);
}
