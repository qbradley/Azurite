use super::i_query_node::IQueryNode;

/// Mirrors TypeScript abstract `BinaryOperatorNode`.
///
/// A binary operator node with left and right children.
/// Concrete subclasses implement `evaluate()` and `name()`.
#[derive(Debug)]
pub struct BinaryOperatorNode {
    pub left: Box<dyn IQueryNode>,
    pub right: Box<dyn IQueryNode>,
}

impl BinaryOperatorNode {
    pub fn new(left: Box<dyn IQueryNode>, right: Box<dyn IQueryNode>) -> Self {
        Self { left, right }
    }

    pub fn to_string_repr_with_name(&self, name: &str) -> String {
        format!(
            "({} {} {})",
            self.left.to_string_repr(),
            name,
            self.right.to_string_repr()
        )
    }
}
