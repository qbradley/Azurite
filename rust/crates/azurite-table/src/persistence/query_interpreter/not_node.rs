use crate::persistence::query_interpreter::i_query_context::IQueryContext;
use crate::persistence::query_interpreter::i_query_node::IQueryNode;
use crate::persistence::query_interpreter::query_error::QueryError;
use crate::persistence::query_interpreter::query_value::{QueryStaticType, QueryValue};

pub struct NotNode {
    right: Box<dyn IQueryNode>,
}

impl NotNode {
    pub fn new(right: Box<dyn IQueryNode>) -> Self {
        Self { right }
    }
}

impl IQueryNode for NotNode {
    fn name(&self) -> &'static str {
        "not"
    }

    fn evaluate(&self, context: &dyn IQueryContext) -> Result<QueryValue, QueryError> {
        Ok(QueryValue::Bool(!self.right.evaluate(context)?.is_truthy()))
    }

    fn right(&self) -> Option<&dyn IQueryNode> {
        Some(self.right.as_ref())
    }

    fn static_type(&self) -> Option<QueryStaticType> {
        Some(QueryStaticType::Boolean)
    }
}
