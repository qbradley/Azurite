// Ported from src/table/entity/EdmGuid.ts

use super::entity_property::AnnotationLevel;
use super::i_edm_type::IEdmType;
use base64::{engine::general_purpose, Engine as _};
use serde_json::Value;

const ODATA_TYPE: &str = "@odata.type";

pub struct EdmGuid {
    pub value: String,       // Store raw value for backwards compat check
    pub typed_value: String, // Base64 encoded
}

impl EdmGuid {
    /// Stores value as base64
    pub fn validate(value: &Value) -> Result<(String, String), String> {
        match value {
            Value::String(s) => {
                // Store GUID in base64 to avoid finding with a string query
                let guid_bytes = s.as_bytes();
                let base64_value = general_purpose::STANDARD.encode(guid_bytes);
                Ok((s.clone(), base64_value))
            }
            _ => Err("Not a valid EdmGuid string.".to_string()),
        }
    }

    pub fn new(value: Value) -> Result<Self, String> {
        let (raw_value, typed_value) = Self::validate(&value)?;
        Ok(Self {
            value: raw_value,
            typed_value,
        })
    }

    /// Check if value is base64 encoded
    fn is_base64_encoded(value: &str) -> bool {
        let re = regex::Regex::new(
            r"^([A-Za-z0-9+/]{4})*([A-Za-z0-9+/]{3}=|[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{1}=)?$",
        )
        .unwrap();
        if let Some(captures) = re.captures(value) {
            let matches_len = captures.len();
            let second_match = captures.get(2);
            matches_len == 3
                && (second_match.is_none() || second_match.unwrap().as_str().len() == 4)
        } else {
            false
        }
    }
}

impl IEdmType for EdmGuid {
    fn to_json_property_value_pair(&self, name: &str) -> Option<(String, Value)> {
        Some((name.to_string(), Value::String(self.typed_value.clone())))
    }

    /// We store GUIDs as base64 encoded strings to stop them being found
    /// by simple string searches.
    /// We must support backwards compatibility, so cover both cases.
    fn to_json_property_value_string(&self, name: &str) -> Option<String> {
        if Self::is_base64_encoded(&self.value) {
            // Decode from base64
            if let Ok(bin_data) = general_purpose::STANDARD.decode(&self.value) {
                if let Ok(decoded) = String::from_utf8(bin_data) {
                    return Some(format!(
                        "\"{}\":{}",
                        name,
                        serde_json::to_string(&decoded).unwrap()
                    ));
                }
            }
        }
        Some(format!(
            "\"{}\":{}",
            name,
            serde_json::to_string(&self.value).unwrap()
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
            panic!("EdmGuid type shouldn't be a system property.");
        }

        if annotation_level == AnnotationLevel::MINIMAL || annotation_level == AnnotationLevel::FULL
        {
            Some((format!("{}{}", name, ODATA_TYPE), "Edm.Guid".to_string()))
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
