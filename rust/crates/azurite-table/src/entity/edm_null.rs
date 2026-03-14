// Ported from src/table/entity/EdmNull.ts

use super::entity_property::AnnotationLevel;
use super::i_edm_type::IEdmType;
use serde_json::Value;

pub struct EdmNull;

impl EdmNull {
    pub fn validate(value: &Value) -> Result<(), String> {
        if value.is_null() {
            Ok(())
        } else {
            Err("Not a valid EdmNull string.".to_string())
        }
    }

    pub fn new(value: Value) -> Result<Self, String> {
        Self::validate(&value)?;
        Ok(Self)
    }
}

impl IEdmType for EdmNull {
    fn to_json_property_value_pair(&self, _name: &str) -> Option<(String, Value)> {
        None
    }

    fn to_json_property_value_string(&self, _name: &str) -> Option<String> {
        None
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
