use crate::persistence::query_interpreter::i_query_context::IQueryContext;
use crate::persistence::query_interpreter::query_error::QueryError;
use crate::persistence::query_interpreter::query_value::{
    QueryComparison, QueryStaticType, QueryValue,
};

pub trait IQueryNode: Send + Sync {
    fn name(&self) -> &'static str;
    fn evaluate(&self, context: &dyn IQueryContext) -> Result<QueryValue, QueryError>;

    fn compare(
        &self,
        _context: &dyn IQueryContext,
        _other: &dyn IQueryNode,
    ) -> Result<Option<QueryComparison>, QueryError> {
        Ok(None)
    }

    fn left(&self) -> Option<&dyn IQueryNode> {
        None
    }

    fn right(&self) -> Option<&dyn IQueryNode> {
        None
    }

    fn identifier_reference(&self) -> Option<&str> {
        None
    }

    fn static_type(&self) -> Option<QueryStaticType> {
        None
    }
}
