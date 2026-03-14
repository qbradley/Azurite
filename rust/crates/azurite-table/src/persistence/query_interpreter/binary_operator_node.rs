use crate::persistence::query_interpreter::i_query_node::IQueryNode;

pub struct BinaryOperatorNode {
    pub name: &'static str,
    pub left: Box<dyn IQueryNode>,
    pub right: Box<dyn IQueryNode>,
}

impl BinaryOperatorNode {
    pub fn new(name: &'static str, left: Box<dyn IQueryNode>, right: Box<dyn IQueryNode>) -> Self {
        Self { name, left, right }
    }
}
