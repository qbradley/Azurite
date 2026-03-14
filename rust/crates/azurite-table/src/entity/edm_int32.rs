// Ported from src/table/entity/EdmInt32.ts

use super::entity_property::AnnotationLevel;
use super::i_edm_type::IEdmType;
use serde_json::Value;

pub struct EdmInt32 {
    pub typed_value: i32,
}

impl EdmInt32 {
    pub fn validate(value: &Value) -> Result<i32, String> {
        match value {
            Value::String(s) => {
                // Check for non-digits other than +/- in the int string
                // as parseInt will just return the first number it finds
                let re = regex::Regex::new(r"^[+-]?\d+$").unwrap();
                if !re.is_match(s) {
                    return Err("Not a valid integer.".to_string());
                }

                match s.parse::<i32>() {
                    Ok(intval) => Ok(intval),
                    Err(_) => Err("Not a valid EdmInt32 string.".to_string()),
                }
            }
            Value::Number(n) => n
                .as_i64()
                .and_then(|i| i32::try_from(i).ok())
                .ok_or_else(|| "Not a valid EdmInt32 string.".to_string()),
            _ => Err("Not a valid EdmInt32 string.".to_string()),
        }
    }

    pub fn new(value: Value) -> Result<Self, String> {
        let typed_value = Self::validate(&value)?;
        Ok(Self { typed_value })
    }
}

impl IEdmType for EdmInt32 {
    fn to_json_property_value_pair(&self, name: &str) -> Option<(String, Value)> {
        Some((name.to_string(), Value::Number(self.typed_value.into())))
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
