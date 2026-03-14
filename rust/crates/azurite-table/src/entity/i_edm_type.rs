// Ported from src/table/entity/IEdmType.ts

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EdmType {
    Binary = 0,
    Boolean = 1,
    DateTime = 2,
    Double = 3,
    Guid = 4,
    Int32 = 5,
    Int64 = 6,
    String = 7,
    Null = 8,
}

impl fmt::Display for EdmType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EdmType::Binary => write!(f, "Edm.Binary"),
            EdmType::Boolean => write!(f, "Edm.Boolean"),
            EdmType::DateTime => write!(f, "Edm.DateTime"),
            EdmType::Double => write!(f, "Edm.Double"),
            EdmType::Guid => write!(f, "Edm.Guid"),
            EdmType::Int32 => write!(f, "Edm.Int32"),
            EdmType::Int64 => write!(f, "Edm.Int64"),
            EdmType::String => write!(f, "Edm.String"),
            EdmType::Null => write!(f, "Edm.Null"),
        }
    }
}

pub fn get_edm_type(type_str: &str) -> Result<EdmType, String> {
    match type_str {
        "Edm.Binary" => Ok(EdmType::Binary),
        "Edm.Boolean" => Ok(EdmType::Boolean),
        "Edm.DateTime" => Ok(EdmType::DateTime),
        "Edm.Double" => Ok(EdmType::Double),
        "Edm.Guid" => Ok(EdmType::Guid),
        "Edm.Int32" => Ok(EdmType::Int32),
        "Edm.Int64" => Ok(EdmType::Int64),
        "Edm.String" => Ok(EdmType::String),
        "Edm.Null" => Ok(EdmType::Null),
        _ => Err(format!("{} is not a valid Edm Type.", type_str)),
    }
}

// Forward declaration - AnnotationLevel from EntityProperty
pub use super::entity_property::AnnotationLevel;

pub trait IEdmType: Send + Sync {
    fn to_json_property_value_pair(&self, name: &str) -> Option<(String, serde_json::Value)>;

    fn to_json_property_value_string(&self, name: &str) -> Option<String>;

    fn to_json_property_type_pair(
        &self,
        name: &str,
        annotation_level: AnnotationLevel,
        is_system_property: bool,
        force: Option<bool>,
    ) -> Option<(String, String)>;

    fn to_json_property_type_string(
        &self,
        name: &str,
        annotation_level: AnnotationLevel,
        is_system_property: bool,
    ) -> Option<String>;
}
