// Ported from src/table/entity/EdmString.ts

use super::entity_property::AnnotationLevel;
use super::i_edm_type::IEdmType;
use serde_json::Value;

pub struct EdmString {
    pub typed_value: String,
}

impl EdmString {
    pub fn validate(value: &Value) -> Result<String, String> {
        match value {
            Value::String(s) => Ok(s.clone()),
            _ => Err("Not a valid string.".to_string()),
        }
    }

    pub fn new(value: Value) -> Result<Self, String> {
        let typed_value = Self::validate(&value)?;
        Ok(Self { typed_value })
    }
}

impl IEdmType for EdmString {
    fn to_json_property_value_pair(&self, name: &str) -> Option<(String, Value)> {
        Some((name.to_string(), Value::String(self.typed_value.clone())))
    }

    fn to_json_property_value_string(&self, name: &str) -> Option<String> {
        Some(format!(
            "\"{}\":{}",
            name,
            serde_json::to_string(&self.typed_value).unwrap()
        ))
    }

    fn to_json_property_type_pair(
        &self,
        _name: &str,
        _annotation_level: AnnotationLevel,
        _is_system_property: bool,
        _force: Option<bool>,
    ) -> Option<(String, String)> {
        None
    }

    fn to_json_property_type_string(
        &self,
        _name: &str,
        _annotation_level: AnnotationLevel,
        _is_system_property: bool,
    ) -> Option<String> {
        None
    }
}
