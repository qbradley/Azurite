use crate::entity::NormalizedEntity;
use crate::persistence::i_table_metadata_store::{Entity, Table};
use crate::persistence::query_interpreter::query_error::QueryError;
use crate::persistence::query_interpreter::query_value::QueryValue;

pub trait IQueryContext {
    fn get_identifier(&self, identifier: &str) -> Result<QueryValue, QueryError>;
}

impl IQueryContext for NormalizedEntity {
    fn get_identifier(&self, identifier: &str) -> Result<QueryValue, QueryError> {
        if !self.properties_map.contains_key(identifier) {
            return Ok(QueryValue::Undefined);
        }

        Ok(self
            .ref_entity
            .properties
            .get(identifier)
            .map(QueryValue::from_json)
            .unwrap_or(QueryValue::Undefined))
    }
}

impl IQueryContext for Entity {
    fn get_identifier(&self, identifier: &str) -> Result<QueryValue, QueryError> {
        if identifier == "PartitionKey" {
            return Ok(QueryValue::String(self.PartitionKey.clone()));
        }
        if identifier == "RowKey" {
            return Ok(QueryValue::String(self.RowKey.clone()));
        }

        Ok(self
            .properties
            .get(identifier)
            .map(|value| QueryValue::from_json(&value.to_json_value()))
            .unwrap_or(QueryValue::Undefined))
    }
}

impl IQueryContext for Table {
    fn get_identifier(&self, identifier: &str) -> Result<QueryValue, QueryError> {
        if identifier.eq_ignore_ascii_case("tablename") {
            Ok(QueryValue::String(self.table.clone()))
        } else {
            Err(QueryError::Evaluation(
                "Property queries cannot be used in this query context.".to_string(),
            ))
        }
    }
}
