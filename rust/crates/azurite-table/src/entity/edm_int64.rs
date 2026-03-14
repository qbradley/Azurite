// Ported from src/table/entity/EdmInt64.ts

use super::entity_property::AnnotationLevel;
use super::i_edm_type::IEdmType;
use serde_json::Value;

const ODATA_TYPE: &str = "@odata.type";

pub struct EdmInt64 {
    pub typed_value: String,
}

impl EdmInt64 {
    pub fn validate(value: &Value) -> Result<String, String> {
        match value {
            Value::String(s) => Ok(s.clone()),
            _ => Err("Not a valid EdmInt64 string.".to_string()),
        }
    }

    pub fn new(value: Value) -> Result<Self, String> {
        let typed_value = Self::validate(&value)?;
        Ok(Self { typed_value })
    }
}

impl IEdmType for EdmInt64 {
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
        name: &str,
        annotation_level: AnnotationLevel,
        is_system_property: bool,
        _force: Option<bool>,
    ) -> Option<(String, String)> {
        if is_system_property {
            // Matches TS: throw RangeError
            panic!("EdmInt64 type shouldn't be a system property.");
        }

        if annotation_level == AnnotationLevel::MINIMAL || annotation_level == AnnotationLevel::FULL
        {
            Some((format!("{}{}", name, ODATA_TYPE), "Edm.Int64".to_string()))
        } else {
            None
        }
    }

    fn to_json_property_type_string(
        &self,
        name: &str,
        annotation_level: AnnotationLevel,
        is_system_property: bool,
    ) -> Option<String> {
        self.to_json_property_type_pair(name, annotation_level, is_system_property, None)
            .map(|(key, value)| format!("\"{}\":\"{}\"", key, value))
    }
}
