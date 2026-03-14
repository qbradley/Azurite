// Ported from src/table/entity/EdmBinary.ts

use super::entity_property::AnnotationLevel;
use super::i_edm_type::IEdmType;
use serde_json::Value;

const ODATA_TYPE: &str = "@odata.type";

pub struct EdmBinary {
    pub value: String, // Raw value
    pub typed_value: String,
}

impl EdmBinary {
    pub fn validate(value: &Value) -> Result<String, String> {
        match value {
            Value::String(s) => Ok(s.clone()),
            _ => Err("Not a valid EdmBinary string.".to_string()),
        }
    }

    pub fn new(value: Value) -> Result<Self, String> {
        let typed_value = Self::validate(&value)?;
        Ok(Self {
            value: typed_value.clone(),
            typed_value,
        })
    }
}

impl IEdmType for EdmBinary {
    fn to_json_property_value_pair(&self, name: &str) -> Option<(String, Value)> {
        // TS uses this.value (not this.typedValue)
        Some((name.to_string(), Value::String(self.value.clone())))
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
            panic!("EdmBinary type shouldn't be a system property.");
        }

        if annotation_level == AnnotationLevel::MINIMAL || annotation_level == AnnotationLevel::FULL
        {
            Some((format!("{}{}", name, ODATA_TYPE), "Edm.Binary".to_string()))
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
