// Ported from src/table/entity/EntityProperty.ts

use super::i_edm_type::{EdmType, IEdmType};
use super::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotationLevel {
    FULL,
    MINIMAL,
    NO,
}

pub fn to_annotation_level(level: &str) -> Result<AnnotationLevel, String> {
    match level {
        "application/json;odata=minimalmetadata" => Ok(AnnotationLevel::MINIMAL),
        "application/json;odata=fullmetadata" => Ok(AnnotationLevel::FULL),
        "application/json;odata=nometadata" => Ok(AnnotationLevel::NO),
        _ => Err(format!("Invalid annotation level: {}", level)),
    }
}

pub struct EntityProperty {
    pub name: String,
    pub value: Box<dyn std::any::Any + Send + Sync>,
    pub edm_type: Box<dyn IEdmType>,
    pub is_system_property: bool,
}

impl EntityProperty {
    pub fn new(
        name: String,
        value: Box<dyn std::any::Any + Send + Sync>,
        edm_type: Box<dyn IEdmType>,
        is_system_property: bool,
    ) -> Self {
        Self {
            name,
            value,
            edm_type,
            is_system_property,
        }
    }

    pub fn to_json_property_value_pair(&self) -> Option<(String, serde_json::Value)> {
        self.edm_type.to_json_property_value_pair(&self.name)
    }

    pub fn to_json_property_value_string(&self) -> Option<String> {
        self.edm_type.to_json_property_value_string(&self.name)
    }

    pub fn to_json_property_type_pair(
        &self,
        annotation_level: AnnotationLevel,
        force: Option<bool>,
    ) -> Option<(String, String)> {
        self.edm_type.to_json_property_type_pair(
            &self.name,
            annotation_level,
            self.is_system_property,
            force,
        )
    }

    pub fn to_json_property_type_string(
        &self,
        annotation_level: AnnotationLevel,
    ) -> Option<String> {
        self.edm_type.to_json_property_type_string(
            &self.name,
            annotation_level,
            self.is_system_property,
        )
    }

    pub fn to_response_string(&self, annotation_level: AnnotationLevel) -> String {
        let type_string = self.to_json_property_type_string(annotation_level);
        let property_string = self.to_json_property_value_string();

        match (type_string, property_string) {
            (Some(ts), Some(ps)) => format!("{},{}", ts, ps),
            (None, Some(ps)) => ps,
            (Some(ts), None) => ts,
            (None, None) => String::new(),
        }
    }

    pub fn normalize(&self, entity: &mut Entity) {
        if let Some((key, value)) = self.to_json_property_value_pair() {
            entity.properties.insert(key, value);
        }
        if let Some((type_key, type_value)) =
            self.to_json_property_type_pair(AnnotationLevel::FULL, Some(true))
        {
            entity
                .properties
                .insert(type_key, serde_json::Value::String(type_value));
        }
    }
}

// Entity type placeholder - will be defined in generated/artifacts/models.rs
pub struct Entity {
    pub properties: HashMap<String, serde_json::Value>,
}

pub fn parse_entity_property(
    name: String,
    value: serde_json::Value,
    edm_type: Option<EdmType>,
    is_system_property: bool,
) -> Result<EntityProperty, String> {
    match edm_type {
        Some(et) => {
            let typed_value: Box<dyn IEdmType> = match et {
                EdmType::String => Box::new(edm_string::EdmString::new(value)?),
                EdmType::Int32 => Box::new(edm_int32::EdmInt32::new(value)?),
                EdmType::Int64 => Box::new(edm_int64::EdmInt64::new(value)?),
                EdmType::Double => Box::new(edm_double::EdmDouble::new(value)?),
                EdmType::Boolean => Box::new(edm_boolean::EdmBoolean::new(value)?),
                EdmType::DateTime => Box::new(edm_date_time::EdmDateTime::new(value)?),
                EdmType::Guid => Box::new(edm_guid::EdmGuid::new(value)?),
                EdmType::Binary => Box::new(edm_binary::EdmBinary::new(value)?),
                EdmType::Null => Box::new(edm_null::EdmNull::new(value)?),
            };
            Ok(EntityProperty::new(
                name,
                Box::new(()),
                typed_value,
                is_system_property,
            ))
        }
        None => {
            // Auto-detect type
            let typed_value: Box<dyn IEdmType> = match &value {
                serde_json::Value::String(_) => Box::new(edm_string::EdmString::new(value)?),
                serde_json::Value::Number(n) => {
                    if n.is_i64() || n.is_u64() {
                        Box::new(edm_int32::EdmInt32::new(value)?)
                    } else {
                        Box::new(edm_double::EdmDouble::new(value)?)
                    }
                }
                serde_json::Value::Bool(_) => Box::new(edm_boolean::EdmBoolean::new(value)?),
                serde_json::Value::Null => Box::new(edm_null::EdmNull::new(value)?),
                _ => {
                    return Err(format!(
                        "Invalid value type for auto-detection: {:?}",
                        value
                    ))
                }
            };
            Ok(EntityProperty::new(
                name,
                Box::new(()),
                typed_value,
                is_system_property,
            ))
        }
    }
}
