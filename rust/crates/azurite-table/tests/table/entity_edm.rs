#![allow(non_snake_case)]
//! Parity tests for Table EDM (Entity Data Model) types.
//! Validates type parsing, validation, and JSON serialization for all 9 EDM types.

use azurite_table::entity::{
    EdmBinary, EdmBoolean, EdmDateTime, EdmDouble, EdmGuid, EdmInt32, EdmInt64,
};
use azurite_table::entity::i_edm_type::IEdmType;
use azurite_table::entity::edm_double::EdmDoubleValue;
use azurite_table::entity::AnnotationLevel;
use serde_json::json;

// ── EdmString ──────────────────────────────────────────────

#[test]
fn edm_string_validates_string_value() {
    let val = azurite_table::entity::edm_string::EdmString::validate(&json!("hello"));
    assert!(val.is_ok());
    assert_eq!(val.unwrap(), "hello");
}

#[test]
fn edm_string_rejects_number() {
    let val = azurite_table::entity::edm_string::EdmString::validate(&json!(42));
    assert!(val.is_err());
}

#[test]
fn edm_string_serializes_to_json() {
    let s = azurite_table::entity::edm_string::EdmString::new(json!("test")).unwrap();
    let (key, val) = s.to_json_property_value_pair("Name").unwrap();
    assert_eq!(key, "Name");
    assert_eq!(val, json!("test"));
}

// ── EdmInt32 ───────────────────────────────────────────────

#[test]
fn edm_int32_validates_number() {
    assert_eq!(EdmInt32::validate(&json!(42)).unwrap(), 42);
}

#[test]
fn edm_int32_validates_string_number() {
    assert_eq!(EdmInt32::validate(&json!("123")).unwrap(), 123);
}

#[test]
fn edm_int32_validates_negative() {
    assert_eq!(EdmInt32::validate(&json!("-5")).unwrap(), -5);
}

#[test]
fn edm_int32_rejects_float_string() {
    assert!(EdmInt32::validate(&json!("3.14")).is_err());
}

#[test]
fn edm_int32_rejects_overflow() {
    assert!(EdmInt32::validate(&json!("99999999999")).is_err());
}

#[test]
fn edm_int32_rejects_non_numeric_string() {
    assert!(EdmInt32::validate(&json!("abc")).is_err());
}

#[test]
fn edm_int32_serializes_as_number() {
    let i = EdmInt32::new(json!(7)).unwrap();
    let (_, val) = i.to_json_property_value_pair("count").unwrap();
    assert_eq!(val, json!(7));
}

// ── EdmInt64 ───────────────────────────────────────────────

#[test]
fn edm_int64_accepts_any_string() {
    // EdmInt64 stores the raw string value (matching TS behavior)
    let val = azurite_table::entity::edm_int64::EdmInt64::validate(&json!("9007199254740993"));
    assert!(val.is_ok());
}

#[test]
fn edm_int64_rejects_non_string() {
    let val = azurite_table::entity::edm_int64::EdmInt64::validate(&json!(42));
    assert!(val.is_err());
}

// ── EdmDouble ──────────────────────────────────────────────

#[test]
fn edm_double_validates_number() {
    let val = EdmDouble::validate(&json!(3.14));
    assert!(val.is_ok());
}

#[test]
fn edm_double_validates_nan_string() {
    let val = EdmDouble::validate(&json!("NaN"));
    assert!(val.is_ok());
    assert!(matches!(val.unwrap(), EdmDoubleValue::Special(s) if s == "NaN"));
}

#[test]
fn edm_double_validates_infinity() {
    let val = EdmDouble::validate(&json!("Infinity"));
    assert!(matches!(val.unwrap(), EdmDoubleValue::Special(s) if s == "Infinity"));
}

#[test]
fn edm_double_validates_negative_infinity() {
    let val = EdmDouble::validate(&json!("-Infinity"));
    assert!(matches!(val.unwrap(), EdmDoubleValue::Special(s) if s == "-Infinity"));
}

#[test]
fn edm_double_rejects_non_numeric() {
    assert!(EdmDouble::validate(&json!("notanumber")).is_err());
}

#[test]
fn edm_double_integer_serializes_with_decimal() {
    let d = EdmDouble::new(json!(5.0)).unwrap();
    let s = d.to_json_property_value_string("val").unwrap();
    assert_eq!(s, "\"val\":5.0");
}

#[test]
fn edm_double_fractional_serializes_raw() {
    let d = EdmDouble::new(json!(3.14)).unwrap();
    let s = d.to_json_property_value_string("val").unwrap();
    assert_eq!(s, "\"val\":3.14");
}

#[test]
fn edm_double_nan_emits_type_annotation_at_minimal() {
    let d = EdmDouble::new(json!("NaN")).unwrap();
    let pair = d.to_json_property_type_pair("x", AnnotationLevel::MINIMAL, false, None);
    assert!(pair.is_some(), "NaN should produce type annotation at MINIMAL");
    assert_eq!(pair.unwrap().1, "Edm.Double");
}

#[test]
fn edm_double_normal_skips_type_annotation_at_minimal() {
    let d = EdmDouble::new(json!(1.5)).unwrap();
    let pair = d.to_json_property_type_pair("x", AnnotationLevel::MINIMAL, false, None);
    assert!(pair.is_none(), "Normal double should not produce type annotation");
}

// ── EdmBoolean ─────────────────────────────────────────────

#[test]
fn edm_boolean_validates_bool() {
    let val = EdmBoolean::validate(&json!(true));
    assert!(val.is_ok());
    assert!(val.unwrap());
}

#[test]
fn edm_boolean_validates_string_true() {
    let val = EdmBoolean::validate(&json!("true"));
    assert!(val.is_ok());
    assert!(val.unwrap());
}

#[test]
fn edm_boolean_rejects_random_string() {
    assert!(EdmBoolean::validate(&json!("maybe")).is_err());
}

// ── EdmDateTime ────────────────────────────────────────────

#[test]
fn edm_datetime_validates_iso_string() {
    let val = EdmDateTime::validate(&json!("2023-01-15T12:00:00.000Z"));
    assert!(val.is_ok());
}

#[test]
fn edm_datetime_rejects_non_string() {
    assert!(EdmDateTime::validate(&json!(12345)).is_err());
}

// ── EdmGuid ────────────────────────────────────────────────

#[test]
fn edm_guid_validates_guid_string() {
    let val = EdmGuid::validate(&json!("550e8400-e29b-41d4-a716-446655440000"));
    assert!(val.is_ok());
    let (raw, encoded) = val.unwrap();
    assert_eq!(raw, "550e8400-e29b-41d4-a716-446655440000");
    assert!(!encoded.is_empty(), "should produce base64 encoding");
}

#[test]
fn edm_guid_rejects_non_string() {
    assert!(EdmGuid::validate(&json!(42)).is_err());
}

#[test]
fn edm_guid_emits_type_annotation_at_minimal() {
    let g = EdmGuid::new(json!("550e8400-e29b-41d4-a716-446655440000")).unwrap();
    let pair = g.to_json_property_type_pair("id", AnnotationLevel::MINIMAL, false, None);
    assert!(pair.is_some());
    assert_eq!(pair.unwrap().1, "Edm.Guid");
}

#[test]
fn edm_guid_no_annotation_at_nometadata() {
    let g = EdmGuid::new(json!("550e8400-e29b-41d4-a716-446655440000")).unwrap();
    let pair = g.to_json_property_type_pair("id", AnnotationLevel::NO, false, None);
    assert!(pair.is_none());
}

// ── EdmBinary ──────────────────────────────────────────────

#[test]
fn edm_binary_validates_base64() {
    let val = EdmBinary::validate(&json!("SGVsbG8="));
    assert!(val.is_ok());
}

#[test]
fn edm_binary_rejects_non_string() {
    assert!(EdmBinary::validate(&json!(42)).is_err());
}
