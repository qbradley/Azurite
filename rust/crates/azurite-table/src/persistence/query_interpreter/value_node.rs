use crate::persistence::query_interpreter::i_query_context::IQueryContext;
use crate::persistence::query_interpreter::i_query_node::IQueryNode;
use crate::persistence::query_interpreter::query_error::QueryError;
use crate::persistence::query_interpreter::query_value::{
    QueryComparison, QueryStaticType, QueryValue,
};

pub struct ValueNode {
    pub name: &'static str,
    pub value: String,
    pub static_type: QueryStaticType,
}

impl ValueNode {
    pub fn new(name: &'static str, value: String, static_type: QueryStaticType) -> Self {
        Self {
            name,
            value,
            static_type,
        }
    }

    pub fn evaluate(&self) -> QueryValue {
        QueryValue::String(self.value.clone())
    }

    pub fn compare(
        &self,
        context: &dyn IQueryContext,
        other: &dyn IQueryNode,
    ) -> Result<QueryComparison, QueryError> {
        let this_value = self.evaluate();
        let other_value = other.evaluate(context)?;
        if this_value.is_undefined() || other_value.is_undefined() || other_value.is_null() {
            return Ok(QueryComparison::Nan);
        }

        Ok(QueryComparison::from_ordering(
            this_value.relational_compare(&other_value),
        ))
    }
}
