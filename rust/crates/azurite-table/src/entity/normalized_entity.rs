// Ported from src/table/entity/NormalizedEntity.ts

use super::edm_string::EdmString;
use super::entity_property::{parse_entity_property, AnnotationLevel, Entity, EntityProperty};
use serde_json::Value;
use std::collections::HashMap;

const ODATA_TYPE: &str = "@odata.type";

pub struct NormalizedEntity {
    pub ref_entity: Entity, // 'ref' is reserved in Rust
    pub properties: Vec<EntityProperty>,
    pub properties_map: HashMap<String, usize>, // Map name -> index in properties vec
}

impl NormalizedEntity {
    pub fn new(mut entity: Entity) -> Result<Self, String> {
        let mut properties = Vec::new();
        let mut properties_map = HashMap::new();

        // PartitionKey
        let partition_key = entity
            .properties
            .get("PartitionKey")
            .and_then(|v| v.as_str())
            .ok_or("Missing PartitionKey")?
            .to_string();

        let partition_key_property = EntityProperty::new(
            "PartitionKey".to_string(),
            Box::new(()),
            Box::new(EdmString::new(Value::String(partition_key))?),
            true,
        );
        properties_map.insert("PartitionKey".to_string(), properties.len());
        properties.push(partition_key_property);

        // RowKey
        let row_key = entity
            .properties
            .get("RowKey")
            .and_then(|v| v.as_str())
            .ok_or("Missing RowKey")?
            .to_string();

        let row_key_property = EntityProperty::new(
            "RowKey".to_string(),
            Box::new(()),
            Box::new(EdmString::new(Value::String(row_key))?),
            true,
        );
        properties_map.insert("RowKey".to_string(), properties.len());
        properties.push(row_key_property);

        // Sync Timestamp from entity last modified time
        // Placeholder for truncatedISO8061Date - for now use current time
        let timestamp_value =
            if let Some(Value::String(s)) = entity.properties.get("lastModifiedTime") {
                if s.is_empty() {
                    chrono::Utc::now().to_rfc3339()
                } else {
                    s.clone()
                }
            } else {
                chrono::Utc::now().to_rfc3339()
            };

        entity
            .properties
            .insert("Timestamp".to_string(), Value::String(timestamp_value));
        entity.properties.insert(
            format!("Timestamp{}", ODATA_TYPE),
            Value::String("Edm.DateTime".to_string()),
        );

        // Iterate over entity properties
        let keys: Vec<String> = entity.properties.keys().cloned().collect();
        for key in keys {
            if properties_map.contains_key(&key) {
                continue;
            }

            if key.ends_with(ODATA_TYPE) {
                continue;
            }

            let element = entity
                .properties
                .get(&key)
                .ok_or(format!("Missing property {}", key))?
                .clone();
            let type_key = format!("{}{}", key, ODATA_TYPE);
            let type_value = entity.properties.get(&type_key);

            let edm_type = if let Some(type_val) = type_value {
                if !type_val.is_string() {
                    return Err(format!(
                        "Invalid EdmType value:{:?} for key:{}",
                        type_val, type_key
                    ));
                }
                type_val
                    .as_str()
                    .and_then(|s| super::i_edm_type::get_edm_type(s).ok())
            } else {
                None
            };

            // Validate system property
            let is_system_property = key == "Timestamp";
            let property =
                parse_entity_property(key.clone(), element, edm_type, is_system_property)?;

            properties_map.insert(key, properties.len());
            properties.push(property);
        }

        Ok(Self {
            ref_entity: entity,
            properties,
            properties_map,
        })
    }

    /// Convert to HTTP response payload string
    pub fn to_response_string(
        &self,
        annotation_level: AnnotationLevel,
        injections: HashMap<String, String>,
        includes: Option<&std::collections::HashSet<String>>,
    ) -> String {
        let mut pairs: Vec<String> = Vec::new();

        // Add injections first
        for (key, value) in injections {
            pairs.push(format!(
                "\"{}\":{}",
                key,
                serde_json::to_string(&value).unwrap()
            ));
        }

        // Add properties
        for pair in &self.properties {
            if includes.is_none() || includes.unwrap().contains(&pair.name) {
                let str = pair.to_response_string(annotation_level);
                if !str.is_empty() {
                    pairs.push(str);
                }
            }
        }

        format!("{{{}}}", pairs.join(","))
    }

    pub fn normalize(mut self) -> Entity {
        self.ref_entity.properties.clear();
        for entity in &self.properties {
            entity.normalize(&mut self.ref_entity);
        }
        self.ref_entity
    }
}
