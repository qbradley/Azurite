use super::super::i_query_context::IQueryContext;
use super::i_query_node::{IQueryNode, TagContent};

/// Mirrors TypeScript `KeyNode`.
#[derive(Debug)]
pub struct KeyNode {
    identifier: String,
}

impl KeyNode {
    pub fn new(identifier: String) -> Self {
        Self { identifier }
    }
}

impl IQueryNode for KeyNode {
    fn name(&self) -> &str {
        "id"
    }

    fn evaluate(&self, context: &IQueryContext) -> Vec<TagContent> {
        vec![TagContent {
            key: Some(self.identifier.clone()),
            value: context.get(&self.identifier).cloned(),
        }]
    }

    fn to_string_repr(&self) -> String {
        self.identifier.clone()
    }
}
