// Ported from src/table/entity/EdmDouble.ts

use super::entity_property::AnnotationLevel;
use super::i_edm_type::IEdmType;
use serde_json::Value;

const ODATA_TYPE: &str = "@odata.type";

#[derive(Clone)]
pub enum EdmDoubleValue {
    Number(f64),
    Special(String), // "NaN", "Infinity", "-Infinity"
}

pub struct EdmDouble {
    pub value: Value, // Raw value as stored
    pub typed_value: EdmDoubleValue,
}

impl EdmDouble {
    pub fn validate(value: &Value) -> Result<EdmDoubleValue, String> {
        match value {
            Value::String(s) => {
                // Special string values
                if s == "NaN" || s == "Infinity" || s == "-Infinity" {
                    return Ok(EdmDoubleValue::Special(s.clone()));
                }

                // Try to parse as float
                match s.parse::<f64>() {
                    Ok(val) => {
                        // Test overflow - matches TS behavior
                        if val.is_infinite() {
                            Err("InvalidInput".to_string())
                        } else {
                            Ok(EdmDoubleValue::Number(val))
                        }
                    }
                    Err(_) => Err("Not a valid EdmDouble string.".to_string()),
                }
            }
            Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    Ok(EdmDoubleValue::Number(f))
                } else {
                    Err("Not a valid EdmDouble string.".to_string())
                }
            }
            _ => Err("Not a valid EdmDouble string.".to_string()),
        }
    }

    pub fn new(value: Value) -> Result<Self, String> {
        let typed_value = Self::validate(&value)?;
        Ok(Self {
            value: value.clone(),
            typed_value,
        })
    }
}

impl IEdmType for EdmDouble {
    fn to_json_property_value_pair(&self, name: &str) -> Option<(String, Value)> {
        // TS uses this.value (raw), not this.typedValue
        Some((name.to_string(), self.value.clone()))
    }

    fn to_json_property_value_string(&self, name: &str) -> Option<String> {
        match &self.typed_value {
            EdmDoubleValue::Number(n) => {
                // If integer, use .toFixed(1), else raw value
                if n.fract() == 0.0 {
                    Some(format!("\"{}\":{:.1}", name, n))
                } else {
                    Some(format!("\"{}\":{}", name, n))
                }
            }
            EdmDoubleValue::Special(s) => Some(format!(
                "\"{}\":{}",
                name,
                serde_json::to_string(s).unwrap()
            )),
        }
    }

    fn to_json_property_type_pair(
        &self,
        name: &str,
        annotation_level: AnnotationLevel,
        is_system_property: bool,
        force: Option<bool>,
    ) -> Option<(String, String)> {
        if is_system_property {
            panic!("EdmDouble type shouldn't be a system property.");
        }

        let force = force.unwrap_or(false);

        if force {
            return Some((format!("{}{}", name, ODATA_TYPE), "Edm.Double".to_string()));
        }

        // Only add type annotation for special float values at MINIMAL or FULL
        if matches!(&self.typed_value, EdmDoubleValue::Special(_))
            && (annotation_level == AnnotationLevel::MINIMAL
                || annotation_level == AnnotationLevel::FULL)
        {
            Some((format!("{}{}", name, ODATA_TYPE), "Edm.Double".to_string()))
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
