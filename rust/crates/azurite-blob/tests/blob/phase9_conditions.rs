use azurite_blob::conditions::{
    validate_sequence_number_write_conditions, ConditionResourceAdapter, ConditionalHeadersAdapter,
    IConditionalHeadersValidator, ReadConditionalHeadersValidator,
    WriteConditionalHeadersValidator,
};
use azurite_blob::generated::artifacts::models::{
    GeneratedObject, GeneratedValue, ModifiedAccessConditions, SequenceNumberAccessConditions,
};
use azurite_blob::generated::context::Context;
use azurite_blob::persistence::BlobModel;
use chrono::{TimeZone, Utc};
use pretty_assertions::assert_eq;

const CONTEXT_ID: &str = "phase9-context";

fn context() -> Context {
    let holder = Context::new_holder();
    let context = Context::from_holder(holder, "/blob/phase9", None, None);
    context.setContextId(Some(CONTEXT_ID.to_string()));
    context
}

fn make_conditions(entries: Vec<(&str, GeneratedValue)>) -> ModifiedAccessConditions {
    let mut conditions = ModifiedAccessConditions::default();
    for (key, value) in entries {
        conditions.insert(key.to_string(), value);
    }
    conditions
}

fn make_blob_properties(entries: Vec<(&str, GeneratedValue)>) -> GeneratedObject {
    let mut props = GeneratedObject::default();
    for (key, value) in entries {
        props.insert(key.to_string(), value);
    }
    props
}

// ============================================================================
// Phase 9.1: ConditionalHeadersAdapter - Quote Stripping & Date Truncation
// ============================================================================

#[test]
fn conditional_headers_adapter_strips_quotes_from_etag_list() {
    let ctx = context();
    let conditions = make_conditions(vec![(
        "ifMatch",
        GeneratedValue::String(r#""e1","e2","e3""#.to_string()),
    )]);

    let adapted = ConditionalHeadersAdapter::new(&ctx, &conditions);

    assert_eq!(
        adapted.ifMatch,
        Some(vec!["e1".to_string(), "e2".to_string(), "e3".to_string()])
    );
}

#[test]
fn conditional_headers_adapter_trims_before_stripping_quotes() {
    let ctx = context();
    // Space before quote gets trimmed, then quotes stripped
    let conditions = make_conditions(vec![(
        "ifNoneMatch",
        GeneratedValue::String(r#" "e1""#.to_string()),
    )]);

    let adapted = ConditionalHeadersAdapter::new(&ctx, &conditions);

    // Rust trims, so space removed, then quotes stripped
    assert_eq!(adapted.ifNoneMatch, Some(vec!["e1".to_string()]));
}

#[test]
fn conditional_headers_adapter_handles_wildcard_etag() {
    let ctx = context();
    let conditions = make_conditions(vec![("ifMatch", GeneratedValue::String("*".to_string()))]);

    let adapted = ConditionalHeadersAdapter::new(&ctx, &conditions);

    assert_eq!(adapted.ifMatch, Some(vec!["*".to_string()]));
}

#[test]
fn conditional_headers_adapter_truncates_milliseconds_on_dates() {
    let ctx = context();
    let conditions = make_conditions(vec![
        (
            "ifModifiedSince",
            GeneratedValue::String("2024-01-15T10:30:45.789Z".to_string()),
        ),
        (
            "ifUnmodifiedSince",
            GeneratedValue::String("2024-01-15T11:00:00.999Z".to_string()),
        ),
    ]);

    let adapted = ConditionalHeadersAdapter::new(&ctx, &conditions);

    // Milliseconds truncated to 0
    let expected_modified = Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 45).unwrap();
    let expected_unmodified = Utc.with_ymd_and_hms(2024, 1, 15, 11, 0, 0).unwrap();

    assert_eq!(adapted.ifModifiedSince, Some(expected_modified));
    assert_eq!(adapted.ifUnmodifiedSince, Some(expected_unmodified));
}

#[test]
fn conditional_headers_adapter_handles_empty_ifmatch() {
    let ctx = context();
    let conditions = make_conditions(vec![("ifMatch", GeneratedValue::String("".to_string()))]);

    let adapted = ConditionalHeadersAdapter::new(&ctx, &conditions);

    // Empty string splits to single empty element
    assert_eq!(adapted.ifMatch, Some(vec!["".to_string()]));
}

// ============================================================================
// Phase 9.2: ConditionResourceAdapter - Resource Validation
// ============================================================================

#[test]
fn condition_resource_adapter_treats_uncommitted_blob_as_nonexistent() {
    let blob = BlobModel {
        properties: make_blob_properties(vec![
            ("etag", GeneratedValue::String(r#""blob-etag""#.to_string())),
            (
                "lastModified",
                GeneratedValue::String("2024-01-15T10:00:00Z".to_string()),
            ),
            ("isCommitted", GeneratedValue::Bool(false)),
        ]),
        isCommitted: Some(false),
        ..Default::default()
    };

    let resource = ConditionResourceAdapter::from_blob(Some(&blob));

    assert_eq!(resource.exist, false);
    assert_eq!(resource.etag, "NONEXISTENT_RESOURCE_ETAG");
    assert_eq!(resource.lastModified, None);
}

#[test]
fn condition_resource_adapter_treats_committed_blob_as_existent() {
    let blob = BlobModel {
        name: Some("test.txt".to_string()),
        containerName: "mycontainer".to_string(),
        properties: make_blob_properties(vec![
            ("etag", GeneratedValue::String(r#""blob-etag""#.to_string())),
            (
                "lastModified",
                GeneratedValue::String("2024-01-15T10:00:00Z".to_string()),
            ),
            ("isCommitted", GeneratedValue::Bool(true)),
        ]),
        isCommitted: Some(true),
        ..Default::default()
    };

    let resource = ConditionResourceAdapter::from_blob(Some(&blob));

    assert_eq!(resource.exist, true);
    assert_eq!(resource.etag, "blob-etag");
    assert!(resource.lastModified.is_some());
}

#[test]
fn condition_resource_adapter_strips_quotes_from_etag() {
    let blob = BlobModel {
        properties: make_blob_properties(vec![(
            "etag",
            GeneratedValue::String(r#""abc123""#.to_string()),
        )]),
        isCommitted: Some(true),
        ..Default::default()
    };

    let resource = ConditionResourceAdapter::from_blob(Some(&blob));

    assert_eq!(resource.etag, "abc123");
}

#[test]
fn condition_resource_adapter_handles_none_blob() {
    let resource = ConditionResourceAdapter::from_blob(None);

    assert_eq!(resource.exist, false);
    assert_eq!(resource.etag, "NONEXISTENT_RESOURCE_ETAG");
}

// ============================================================================
// Phase 9.3: ReadConditionalHeadersValidator - Wildcard & Date Logic
// ============================================================================

#[test]
fn read_validator_wildcard_in_if_none_match_throws_unsatisfiable_condition_for_nonexistent() {
    let ctx = context();
    let conditions = make_conditions(vec![(
        "ifNoneMatch",
        GeneratedValue::String("*".to_string()),
    )]);
    let adapted = ConditionalHeadersAdapter::new(&ctx, &conditions);
    let resource = ConditionResourceAdapter::from_blob(None);

    let result = ReadConditionalHeadersValidator.validate(&ctx, &adapted, &resource, None);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.statusCode, 400); // UnsatisfiableCondition is 400
    assert_eq!(err.storageErrorCode, "UnsatisfiableCondition");
}

#[test]
fn read_validator_wildcard_in_if_none_match_throws_unsatisfiable_condition_for_existing() {
    let ctx = context();
    let conditions = make_conditions(vec![(
        "ifNoneMatch",
        GeneratedValue::String("*".to_string()),
    )]);
    let adapted = ConditionalHeadersAdapter::new(&ctx, &conditions);

    let blob = BlobModel {
        properties: make_blob_properties(vec![(
            "etag",
            GeneratedValue::String(r#""etag123""#.to_string()),
        )]),
        isCommitted: Some(true),
        ..Default::default()
    };
    let resource = ConditionResourceAdapter::from_blob(Some(&blob));

    let result = ReadConditionalHeadersValidator.validate(&ctx, &adapted, &resource, None);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.statusCode, 400); // UnsatisfiableCondition is 400
    assert_eq!(err.storageErrorCode, "UnsatisfiableCondition");
}

#[test]
fn read_validator_if_match_passes_when_etag_matches() {
    let ctx = context();
    let conditions = make_conditions(vec![(
        "ifMatch",
        GeneratedValue::String(r#""etag123""#.to_string()),
    )]);
    let adapted = ConditionalHeadersAdapter::new(&ctx, &conditions);

    let blob = BlobModel {
        properties: make_blob_properties(vec![(
            "etag",
            GeneratedValue::String(r#""etag123""#.to_string()),
        )]),
        isCommitted: Some(true),
        ..Default::default()
    };
    let resource = ConditionResourceAdapter::from_blob(Some(&blob));

    let result = ReadConditionalHeadersValidator.validate(&ctx, &adapted, &resource, None);

    assert!(result.is_ok());
}

#[test]
fn read_validator_if_match_fails_when_etag_does_not_match() {
    let ctx = context();
    let conditions = make_conditions(vec![(
        "ifMatch",
        GeneratedValue::String(r#""etag-wrong""#.to_string()),
    )]);
    let adapted = ConditionalHeadersAdapter::new(&ctx, &conditions);

    let blob = BlobModel {
        properties: make_blob_properties(vec![(
            "etag",
            GeneratedValue::String(r#""etag123""#.to_string()),
        )]),
        isCommitted: Some(true),
        ..Default::default()
    };
    let resource = ConditionResourceAdapter::from_blob(Some(&blob));

    let result = ReadConditionalHeadersValidator.validate(&ctx, &adapted, &resource, None);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.storageErrorCode, "ConditionNotMet");
}

#[test]
fn read_validator_if_modified_since_passes_when_resource_newer() {
    let ctx = context();
    let since = Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap();
    let conditions = make_conditions(vec![(
        "ifModifiedSince",
        GeneratedValue::String(since.to_rfc3339()),
    )]);
    let adapted = ConditionalHeadersAdapter::new(&ctx, &conditions);

    let last_modified = Utc.with_ymd_and_hms(2024, 1, 15, 11, 0, 0).unwrap();
    let blob = BlobModel {
        properties: make_blob_properties(vec![
            ("etag", GeneratedValue::String(r#""etag123""#.to_string())),
            (
                "lastModified",
                GeneratedValue::String(last_modified.to_rfc3339()),
            ),
        ]),
        isCommitted: Some(true),
        ..Default::default()
    };
    let resource = ConditionResourceAdapter::from_blob(Some(&blob));

    let result = ReadConditionalHeadersValidator.validate(&ctx, &adapted, &resource, None);

    assert!(result.is_ok());
}

#[test]
fn read_validator_if_modified_since_with_if_none_match_validates_independently() {
    let ctx = context();
    let since = Utc.with_ymd_and_hms(2024, 1, 15, 11, 0, 0).unwrap();
    let conditions = make_conditions(vec![
        (
            "ifModifiedSince",
            GeneratedValue::String(since.to_rfc3339()),
        ),
        (
            "ifNoneMatch",
            GeneratedValue::String(r#""etag999""#.to_string()),
        ), // Different etag
    ]);
    let adapted = ConditionalHeadersAdapter::new(&ctx, &conditions);

    // Same time but different etag
    let last_modified = Utc.with_ymd_and_hms(2024, 1, 15, 11, 0, 0).unwrap();
    let blob = BlobModel {
        properties: make_blob_properties(vec![
            ("etag", GeneratedValue::String(r#""etag123""#.to_string())),
            (
                "lastModified",
                GeneratedValue::String(last_modified.to_rfc3339()),
            ),
        ]),
        isCommitted: Some(true),
        ..Default::default()
    };
    let resource = ConditionResourceAdapter::from_blob(Some(&blob));

    let result = ReadConditionalHeadersValidator.validate(&ctx, &adapted, &resource, None);

    // When both are present, ifNoneMatch passes (different etag),
    // but ifModifiedSince fails (not modified). Logic shows they both must pass
    // or it returns NotModified. Actually - looking at code line 114-120:
    // if ifNoneMatchPass == Some(false) && isModifiedSincePass != Some(true) → NotModified
    // if isModifiedSincePass == Some(false) && ifNoneMatchPass != Some(true) → NotModified
    // In our case: ifNoneMatchPass = Some(true), isModifiedSincePass = Some(false)
    // Second condition: Some(false) && Some(true) != Some(true) → NotModified
    // Wait, that's wrong. Let me re-read:
    // if isModifiedSincePass == Some(false) && ifNoneMatchPass != Some(true)
    // isModifiedSincePass = Some(false), ifNoneMatchPass = Some(true)
    // Some(false) && !(Some(true)) → false, no error
    // So should pass!
    assert!(result.is_ok());
}

#[test]
fn read_validator_if_unmodified_since_passes_when_resource_older_or_equal() {
    let ctx = context();
    let since = Utc.with_ymd_and_hms(2024, 1, 15, 11, 0, 0).unwrap();
    let conditions = make_conditions(vec![(
        "ifUnmodifiedSince",
        GeneratedValue::String(since.to_rfc3339()),
    )]);
    let adapted = ConditionalHeadersAdapter::new(&ctx, &conditions);

    // Same time - should pass (<=, not <)
    let last_modified = Utc.with_ymd_and_hms(2024, 1, 15, 11, 0, 0).unwrap();
    let blob = BlobModel {
        properties: make_blob_properties(vec![
            ("etag", GeneratedValue::String(r#""etag123""#.to_string())),
            (
                "lastModified",
                GeneratedValue::String(last_modified.to_rfc3339()),
            ),
        ]),
        isCommitted: Some(true),
        ..Default::default()
    };
    let resource = ConditionResourceAdapter::from_blob(Some(&blob));

    let result = ReadConditionalHeadersValidator.validate(&ctx, &adapted, &resource, None);

    assert!(result.is_ok());
}

// ============================================================================
// Phase 9.4: WriteConditionalHeadersValidator - 2-Header-Max & Wildcard
// ============================================================================

#[test]
fn write_validator_wildcard_in_if_none_match_for_existing_blob_returns_ok() {
    let ctx = context();
    let conditions = make_conditions(vec![(
        "ifNoneMatch",
        GeneratedValue::String("*".to_string()),
    )]);
    let adapted = ConditionalHeadersAdapter::new(&ctx, &conditions);

    let blob = BlobModel {
        properties: make_blob_properties(vec![(
            "etag",
            GeneratedValue::String(r#""etag123""#.to_string()),
        )]),
        isCommitted: Some(true),
        ..Default::default()
    };
    let resource = ConditionResourceAdapter::from_blob(Some(&blob));

    // Critical: TS returns Ok (not 412) - special handling for Put Blob/Commit Block List
    let result = WriteConditionalHeadersValidator.validate(&ctx, &adapted, &resource, None);

    assert!(result.is_ok());
}

#[test]
fn write_validator_if_match_and_if_unmodified_since_valid_combination() {
    let ctx = context();
    let conditions = make_conditions(vec![
        (
            "ifMatch",
            GeneratedValue::String(r#""etag123""#.to_string()),
        ),
        (
            "ifUnmodifiedSince",
            GeneratedValue::String("2024-01-15T11:00:00Z".to_string()),
        ),
    ]);
    let adapted = ConditionalHeadersAdapter::new(&ctx, &conditions);

    let blob = BlobModel {
        properties: make_blob_properties(vec![(
            "etag",
            GeneratedValue::String(r#""etag123""#.to_string()),
        )]),
        isCommitted: Some(true),
        ..Default::default()
    };
    let resource = ConditionResourceAdapter::from_blob(Some(&blob));

    // Valid: If-Match + If-Unmodified-Since allowed
    let result = WriteConditionalHeadersValidator.validate(&ctx, &adapted, &resource, None);

    assert!(result.is_ok());
}

#[test]
fn write_validator_if_none_match_and_if_modified_since_combination_depends_on_values() {
    let ctx = context();
    let conditions = make_conditions(vec![
        (
            "ifNoneMatch",
            GeneratedValue::String(r#""etag999""#.to_string()),
        ), // Different etag
        (
            "ifModifiedSince",
            GeneratedValue::String("2024-01-15T10:00:00Z".to_string()),
        ),
    ]);
    let adapted = ConditionalHeadersAdapter::new(&ctx, &conditions);

    let blob = BlobModel {
        properties: make_blob_properties(vec![
            ("etag", GeneratedValue::String(r#""etag123""#.to_string())),
            (
                "lastModified",
                GeneratedValue::String("2024-01-15T11:00:00Z".to_string()),
            ),
        ]),
        isCommitted: Some(true),
        ..Default::default()
    };
    let resource = ConditionResourceAdapter::from_blob(Some(&blob));

    // If-None-Match processes first and passes (etag different), so returns Ok
    // (If-None-Match + If-Modified-Since is technically allowed in write,
    // just If-None-Match takes precedence per spec)
    let result = WriteConditionalHeadersValidator.validate(&ctx, &adapted, &resource, None);

    // Should succeed because ifNoneMatch passes (different etag) and stops processing
    assert!(result.is_ok());
}

#[test]
fn write_validator_multiple_etag_values_in_if_match_rejected() {
    let ctx = context();
    let conditions = make_conditions(vec![(
        "ifMatch",
        GeneratedValue::String(r#""etag1","etag2""#.to_string()),
    )]);
    let adapted = ConditionalHeadersAdapter::new(&ctx, &conditions);

    let blob = BlobModel {
        properties: make_blob_properties(vec![(
            "etag",
            GeneratedValue::String(r#""etag1""#.to_string()),
        )]),
        isCommitted: Some(true),
        ..Default::default()
    };
    let resource = ConditionResourceAdapter::from_blob(Some(&blob));

    // Multiple ETag values not allowed
    let result = WriteConditionalHeadersValidator.validate(&ctx, &adapted, &resource, None);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.storageErrorCode, "MultipleConditionHeadersNotSupported");
}

#[test]
fn write_validator_more_than_two_headers_rejected() {
    let ctx = context();
    let conditions = make_conditions(vec![
        (
            "ifMatch",
            GeneratedValue::String(r#""etag123""#.to_string()),
        ),
        (
            "ifNoneMatch",
            GeneratedValue::String(r#""etag456""#.to_string()),
        ),
        (
            "ifModifiedSince",
            GeneratedValue::String("2024-01-15T10:00:00Z".to_string()),
        ),
    ]);
    let adapted = ConditionalHeadersAdapter::new(&ctx, &conditions);

    let blob = BlobModel {
        properties: make_blob_properties(vec![(
            "etag",
            GeneratedValue::String(r#""etag123""#.to_string()),
        )]),
        isCommitted: Some(true),
        ..Default::default()
    };
    let resource = ConditionResourceAdapter::from_blob(Some(&blob));

    // More than 2 headers not allowed
    let result = WriteConditionalHeadersValidator.validate(&ctx, &adapted, &resource, None);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.storageErrorCode, "MultipleConditionHeadersNotSupported");
}

// ============================================================================
// Phase 9.5: Sequence Number Conditions
// ============================================================================

#[test]
fn sequence_number_less_than_or_equal_to_passes_when_condition_gte_blob() {
    let ctx = context();
    let mut conditions = SequenceNumberAccessConditions::default();
    conditions.insert(
        "ifSequenceNumberLessThanOrEqualTo".to_string(),
        GeneratedValue::Number(100.0),
    );

    let blob = BlobModel {
        properties: make_blob_properties(vec![(
            "blobSequenceNumber",
            GeneratedValue::Number(50.0),
        )]),
        ..Default::default()
    };

    let result = validate_sequence_number_write_conditions(&ctx, Some(&conditions), Some(&blob));

    assert!(result.is_ok());
}

#[test]
fn sequence_number_less_than_or_equal_to_fails_when_condition_lt_blob() {
    let ctx = context();
    let mut conditions = SequenceNumberAccessConditions::default();
    conditions.insert(
        "ifSequenceNumberLessThanOrEqualTo".to_string(),
        GeneratedValue::Number(30.0),
    );

    let blob = BlobModel {
        properties: make_blob_properties(vec![(
            "blobSequenceNumber",
            GeneratedValue::Number(50.0),
        )]),
        ..Default::default()
    };

    let result = validate_sequence_number_write_conditions(&ctx, Some(&conditions), Some(&blob));

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.storageErrorCode, "SequenceNumberConditionNotMet");
}

#[test]
fn sequence_number_less_than_passes_when_condition_gt_blob() {
    let ctx = context();
    let mut conditions = SequenceNumberAccessConditions::default();
    conditions.insert(
        "ifSequenceNumberLessThan".to_string(),
        GeneratedValue::Number(100.0),
    );

    let blob = BlobModel {
        properties: make_blob_properties(vec![(
            "blobSequenceNumber",
            GeneratedValue::Number(50.0),
        )]),
        ..Default::default()
    };

    let result = validate_sequence_number_write_conditions(&ctx, Some(&conditions), Some(&blob));

    assert!(result.is_ok());
}

#[test]
fn sequence_number_less_than_fails_when_condition_lte_blob() {
    let ctx = context();
    let mut conditions = SequenceNumberAccessConditions::default();
    conditions.insert(
        "ifSequenceNumberLessThan".to_string(),
        GeneratedValue::Number(50.0), // Equal to blob sequence
    );

    let blob = BlobModel {
        properties: make_blob_properties(vec![(
            "blobSequenceNumber",
            GeneratedValue::Number(50.0),
        )]),
        ..Default::default()
    };

    let result = validate_sequence_number_write_conditions(&ctx, Some(&conditions), Some(&blob));

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.storageErrorCode, "SequenceNumberConditionNotMet");
}

#[test]
fn sequence_number_equal_to_passes_when_values_match() {
    let ctx = context();
    let mut conditions = SequenceNumberAccessConditions::default();
    conditions.insert(
        "ifSequenceNumberEqualTo".to_string(),
        GeneratedValue::Number(50.0),
    );

    let blob = BlobModel {
        properties: make_blob_properties(vec![(
            "blobSequenceNumber",
            GeneratedValue::Number(50.0),
        )]),
        ..Default::default()
    };

    let result = validate_sequence_number_write_conditions(&ctx, Some(&conditions), Some(&blob));

    assert!(result.is_ok());
}

#[test]
fn sequence_number_equal_to_fails_when_values_differ() {
    let ctx = context();
    let mut conditions = SequenceNumberAccessConditions::default();
    conditions.insert(
        "ifSequenceNumberEqualTo".to_string(),
        GeneratedValue::Number(50.0),
    );

    let blob = BlobModel {
        properties: make_blob_properties(vec![(
            "blobSequenceNumber",
            GeneratedValue::Number(51.0),
        )]),
        ..Default::default()
    };

    let result = validate_sequence_number_write_conditions(&ctx, Some(&conditions), Some(&blob));

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.storageErrorCode, "SequenceNumberConditionNotMet");
}
