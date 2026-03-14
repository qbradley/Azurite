// Ported from src/table/entity/EdmDateTime.ts

use super::entity_property::AnnotationLevel;
use super::i_edm_type::IEdmType;
use serde_json::Value;

const ODATA_TYPE: &str = "@odata.type";

pub struct EdmDateTime {
    pub typed_value: String,
}

impl EdmDateTime {
    pub fn validate(value: &Value) -> Result<String, String> {
        match value {
            Value::String(s) => Ok(s.clone()),
            _ => Err("Not a valid EdmDateTime string.".to_string()),
        }
    }

    pub fn new(value: Value) -> Result<Self, String> {
        // Azure Server will take time string like "2012-01-02T23:00:00" as UTC time,
        // so Azurite needs to be aligned by adding suffix "Z"
        let value_str = match &value {
            Value::String(s) => s.clone(),
            _ => return Err("Not a valid EdmDateTime string.".to_string()),
        };

        let utc_time_string = format!("{}Z", value_str);

        // Try parsing with Z suffix
        if chrono::DateTime::parse_from_rfc3339(&utc_time_string).is_ok() {
            // When adding suffix "Z" is still a valid date string, use the string with suffix "Z"
            Ok(Self {
                typed_value: utc_time_string,
            })
        } else {
            // Else use original string
            let typed_value = Self::validate(&value)?;
            Ok(Self { typed_value })
        }
    }
}

impl IEdmType for EdmDateTime {
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
        if annotation_level == AnnotationLevel::MINIMAL || annotation_level == AnnotationLevel::FULL
        {
            // Special case: MINIMAL + system property → no annotation
            if annotation_level == AnnotationLevel::MINIMAL && is_system_property {
                return None;
            }
            Some((
                format!("{}{}", name, ODATA_TYPE),
                "Edm.DateTime".to_string(),
            ))
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
