use super::super::i_query_context::IQueryContext;
use super::binary_operator_node::BinaryOperatorNode;
use super::i_query_node::{IQueryNode, TagContent};

/// Mirrors TypeScript `GreaterThanNode`.
#[derive(Debug)]
pub struct GreaterThanNode {
    pub inner: BinaryOperatorNode,
}

impl GreaterThanNode {
    pub fn new(left: Box<dyn IQueryNode>, right: Box<dyn IQueryNode>) -> Self {
        Self {
            inner: BinaryOperatorNode::new(left, right),
        }
    }
}

impl IQueryNode for GreaterThanNode {
    fn name(&self) -> &str {
        "gt"
    }

    fn evaluate(&self, context: &IQueryContext) -> Vec<TagContent> {
        let left_content = self.inner.left.evaluate(context);
        let right_content = self.inner.right.evaluate(context);

        if left_content[0].value.is_some()
            && right_content[0].value.is_some()
            && left_content[0].value > right_content[0].value
        {
            if left_content[0].key.is_some() {
                left_content
            } else {
                right_content
            }
        } else {
            vec![]
        }
    }

    fn to_string_repr(&self) -> String {
        self.inner.to_string_repr_with_name("gt")
    }
}
