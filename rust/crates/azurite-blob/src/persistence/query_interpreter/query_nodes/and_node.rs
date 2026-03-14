use super::super::i_query_context::IQueryContext;
use super::binary_operator_node::BinaryOperatorNode;
use super::i_query_node::{IQueryNode, TagContent};

/// Mirrors TypeScript `AndNode`.
#[derive(Debug)]
pub struct AndNode {
    pub inner: BinaryOperatorNode,
}

impl AndNode {
    pub fn new(left: Box<dyn IQueryNode>, right: Box<dyn IQueryNode>) -> Self {
        Self {
            inner: BinaryOperatorNode::new(left, right),
        }
    }
}

impl IQueryNode for AndNode {
    fn name(&self) -> &str {
        "and"
    }

    fn evaluate(&self, context: &IQueryContext) -> Vec<TagContent> {
        let left_content = self.inner.left.evaluate(context);
        let right_content = self.inner.right.evaluate(context);
        if !left_content.is_empty() && !right_content.is_empty() {
            let mut result = left_content;
            result.extend(right_content);
            result
        } else {
            vec![]
        }
    }

    fn to_string_repr(&self) -> String {
        self.inner.to_string_repr_with_name("and")
    }
}
