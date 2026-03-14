use super::super::i_query_context::IQueryContext;
use super::i_query_node::{IQueryNode, TagContent};

/// Mirrors TypeScript `ExpressionNode`.
#[derive(Debug)]
pub struct ExpressionNode {
    pub child: Box<dyn IQueryNode>,
}

impl ExpressionNode {
    pub fn new(child: Box<dyn IQueryNode>) -> Self {
        Self { child }
    }
}

impl IQueryNode for ExpressionNode {
    fn name(&self) -> &str {
        "expression"
    }

    fn evaluate(&self, context: &IQueryContext) -> Vec<TagContent> {
        self.child.evaluate(context)
    }

    fn to_string_repr(&self) -> String {
        format!("({})", self.child.to_string_repr())
    }
}
