use crate::persistence::query_interpreter::i_query_context::IQueryContext;
use crate::persistence::query_interpreter::i_query_node::IQueryNode;
use crate::persistence::query_interpreter::query_error::QueryError;
use crate::persistence::query_interpreter::query_value::QueryValue;

pub struct IdentifierNode {
    identifier: String,
}

impl IdentifierNode {
    pub fn new(identifier: String) -> Self {
        Self { identifier }
    }
}

impl IQueryNode for IdentifierNode {
    fn name(&self) -> &'static str {
        "id"
    }

    fn evaluate(&self, context: &dyn IQueryContext) -> Result<QueryValue, QueryError> {
        context.get_identifier(&self.identifier)
    }

    fn identifier_reference(&self) -> Option<&str> {
        Some(&self.identifier)
    }
}
