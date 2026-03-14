// Entity type system for Azure Table Storage
// Ported from src/table/entity/

pub mod edm_binary;
pub mod edm_boolean;
pub mod edm_date_time;
pub mod edm_double;
pub mod edm_guid;
pub mod edm_int32;
pub mod edm_int64;
pub mod edm_null;
pub mod edm_string;
pub mod entity_property;
pub mod i_edm_type;
pub mod normalized_entity;

pub use entity_property::{
    parse_entity_property, to_annotation_level, AnnotationLevel, EntityProperty,
};
pub use i_edm_type::{get_edm_type, EdmType, IEdmType};
pub use normalized_entity::NormalizedEntity;

// Re-export all EDM types
pub use edm_binary::EdmBinary;
pub use edm_boolean::EdmBoolean;
pub use edm_date_time::EdmDateTime;
pub use edm_double::EdmDouble;
pub use edm_guid::EdmGuid;
pub use edm_int32::EdmInt32;
pub use edm_int64::EdmInt64;
pub use edm_null::EdmNull;
pub use edm_string::EdmString;
