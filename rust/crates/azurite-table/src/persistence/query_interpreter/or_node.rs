use crate::persistence::query_interpreter::binary_operator_node::BinaryOperatorNode;
use crate::persistence::query_interpreter::i_query_context::IQueryContext;
use crate::persistence::query_interpreter::i_query_node::IQueryNode;
use crate::persistence::query_interpreter::query_error::QueryError;
use crate::persistence::query_interpreter::query_value::{QueryStaticType, QueryValue};

pub struct OrNode {
    inner: BinaryOperatorNode,
}

impl OrNode {
    pub fn new(left: Box<dyn IQueryNode>, right: Box<dyn IQueryNode>) -> Self {
        Self {
            inner: BinaryOperatorNode::new("or", left, right),
        }
    }
}

impl IQueryNode for OrNode {
    fn name(&self) -> &'static str {
        self.inner.name
    }

    fn evaluate(&self, context: &dyn IQueryContext) -> Result<QueryValue, QueryError> {
        let left = self.inner.left.evaluate(context)?;
        if left.is_truthy() {
            return Ok(left);
        }

        self.inner.right.evaluate(context)
    }

    fn left(&self) -> Option<&dyn IQueryNode> {
        Some(self.inner.left.as_ref())
    }

    fn right(&self) -> Option<&dyn IQueryNode> {
        Some(self.inner.right.as_ref())
    }

    fn static_type(&self) -> Option<QueryStaticType> {
        Some(QueryStaticType::Boolean)
    }
}
