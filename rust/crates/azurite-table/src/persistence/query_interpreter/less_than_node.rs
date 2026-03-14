use crate::persistence::query_interpreter::binary_operator_node::BinaryOperatorNode;
use crate::persistence::query_interpreter::i_query_context::IQueryContext;
use crate::persistence::query_interpreter::i_query_node::IQueryNode;
use crate::persistence::query_interpreter::query_error::QueryError;
use crate::persistence::query_interpreter::query_value::{
    QueryComparison, QueryStaticType, QueryValue,
};

pub struct LessThanNode {
    inner: BinaryOperatorNode,
}

impl LessThanNode {
    pub fn new(left: Box<dyn IQueryNode>, right: Box<dyn IQueryNode>) -> Self {
        Self {
            inner: BinaryOperatorNode::new("lt", left, right),
        }
    }
}

impl IQueryNode for LessThanNode {
    fn name(&self) -> &'static str {
        self.inner.name
    }

    fn evaluate(&self, context: &dyn IQueryContext) -> Result<QueryValue, QueryError> {
        if let Some(result) = self
            .inner
            .left
            .compare(context, self.inner.right.as_ref())?
        {
            return Ok(QueryValue::Bool(matches!(result, QueryComparison::Less)));
        }

        if let Some(result) = self
            .inner
            .right
            .compare(context, self.inner.left.as_ref())?
        {
            return Ok(QueryValue::Bool(matches!(result, QueryComparison::Greater)));
        }

        let left = self.inner.left.evaluate(context)?;
        let right = self.inner.right.evaluate(context)?;
        Ok(QueryValue::Bool(matches!(
            left.relational_compare(&right),
            Some(std::cmp::Ordering::Less)
        )))
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
