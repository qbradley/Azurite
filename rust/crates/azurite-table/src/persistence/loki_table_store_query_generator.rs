use crate::errors::{StorageError, StorageErrorFactory};
use crate::generated::{artifacts::models, context::Context};

use super::{
    query_interpreter::{execute_query, parse_query, validate_query_tree, IQueryNode},
    Entity, Table,
};

pub type EntityPredicate = Box<dyn Fn(&Entity) -> bool + Send + Sync + 'static>;
pub type TablePredicate = Box<dyn Fn(&Table) -> bool + Send + Sync + 'static>;

/// Handles query generation for the Loki-style table persistence layer.
pub struct LokiTableStoreQueryGenerator;

#[allow(non_snake_case)]
impl LokiTableStoreQueryGenerator {
    pub fn generateQueryForPersistenceLayer(
        queryOptions: &models::QueryOptions,
        context: &Context,
    ) -> Result<EntityPredicate, StorageError> {
        Self::generateQueryEntityWhereFunction(queryOptions.filter.as_deref())
            .map_err(|_| StorageErrorFactory::getQueryConditionInvalid(context))
    }

    pub fn generateQueryTableWhereFunction(query: Option<&str>) -> Result<TablePredicate, String> {
        if matches!(query, None | Some("" | "true")) {
            return Ok(Box::new(|_| true));
        }

        if matches!(query, Some("false")) {
            return Ok(Box::new(|_| false));
        }

        let query_tree = parse_query(query.unwrap()).map_err(|err| err.to_string())?;
        validate_query_tree(query_tree.as_ref()).map_err(|err| err.to_string())?;
        Ok(Self::predicate_from_query_tree(query_tree))
    }

    pub fn transformTableQuery(query: &str) -> Result<String, String> {
        let _ = parse_query(query).map_err(|err| err.to_string())?;
        Ok(query.to_string())
    }

    fn generateQueryEntityWhereFunction(query: Option<&str>) -> Result<EntityPredicate, String> {
        if matches!(query, None | Some("" | "true")) {
            return Ok(Box::new(|_| true));
        }

        if matches!(query, Some("false")) {
            return Ok(Box::new(|_| false));
        }

        let query_tree = parse_query(query.unwrap()).map_err(|err| err.to_string())?;
        validate_query_tree(query_tree.as_ref()).map_err(|err| err.to_string())?;
        Ok(Self::predicate_from_query_tree(query_tree))
    }

    fn predicate_from_query_tree<T>(
        query_tree: Box<dyn IQueryNode>,
    ) -> Box<dyn Fn(&T) -> bool + Send + Sync + 'static>
    where
        T: Send + Sync + crate::persistence::query_interpreter::IQueryContext + 'static,
    {
        Box::new(move |value| execute_query(value, query_tree.as_ref()))
    }
}

#[cfg(test)]
mod tests {
    use crate::generated::{artifacts::models::QueryOptions, context::Context};

    use super::LokiTableStoreQueryGenerator;
    use crate::persistence::Entity;

    #[test]
    fn true_filter_matches_everything() {
        let predicate = LokiTableStoreQueryGenerator::generateQueryForPersistenceLayer(
            &QueryOptions {
                filter: Some("true".to_string()),
                ..QueryOptions::default()
            },
            &Context::default(),
        )
        .unwrap();

        assert!(predicate(&Entity::default()));
    }

    #[test]
    fn false_filter_matches_nothing() {
        let predicate =
            LokiTableStoreQueryGenerator::generateQueryTableWhereFunction(Some("false")).unwrap();

        assert!(!predicate(&crate::persistence::Table::default()));
    }
}
