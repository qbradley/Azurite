use super::super::i_query_context::IQueryContext;
use super::i_query_node::{IQueryNode, TagContent};

/// Mirrors TypeScript `ConstantNode`.
#[derive(Debug)]
pub struct ConstantNode {
    value: String,
}

impl ConstantNode {
    pub fn new(value: String) -> Self {
        Self { value }
    }
}

impl IQueryNode for ConstantNode {
    fn name(&self) -> &str {
        "constant"
    }

    fn evaluate(&self, _context: &IQueryContext) -> Vec<TagContent> {
        vec![TagContent {
            key: None,
            value: Some(self.value.clone()),
        }]
    }

    fn to_string_repr(&self) -> String {
        // Mirrors JSON.stringify(this.value)
        format!("\"{}\"", self.value)
    }
}
