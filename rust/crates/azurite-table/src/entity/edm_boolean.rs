// Ported from src/table/entity/EdmBoolean.ts

use super::entity_property::AnnotationLevel;
use super::i_edm_type::IEdmType;
use serde_json::Value;

pub struct EdmBoolean {
    pub typed_value: bool,
}

impl EdmBoolean {
    pub fn validate(value: &Value) -> Result<bool, String> {
        match value {
            Value::String(s) => {
                if s == "true" {
                    Ok(true)
                } else if s == "false" {
                    Ok(false)
                } else {
                    Err("Not a valid boolean.".to_string())
                }
            }
            Value::Bool(b) => Ok(*b),
            _ => Err("Not a valid boolean.".to_string()),
        }
    }

    pub fn new(value: Value) -> Result<Self, String> {
        let typed_value = Self::validate(&value)?;
        Ok(Self { typed_value })
    }
}

impl IEdmType for EdmBoolean {
    fn to_json_property_value_pair(&self, name: &str) -> Option<(String, Value)> {
        Some((name.to_string(), Value::Bool(self.typed_value)))
    }

    fn to_json_property_value_string(&self, name: &str) -> Option<String> {
        Some(format!("\"{}\":{}", name, self.typed_value))
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
