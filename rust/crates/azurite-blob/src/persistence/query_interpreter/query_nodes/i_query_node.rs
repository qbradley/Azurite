use super::super::i_query_context::IQueryContext;

/// Mirrors TypeScript `TagContent` interface.
#[derive(Clone, Debug, Default)]
pub struct TagContent {
    pub key: Option<String>,
    pub value: Option<String>,
}

/// Mirrors TypeScript `IQueryNode` interface.
///
/// Each node can `evaluate` itself against an `IQueryContext` (tag map)
/// and produce a `Vec<TagContent>` witness array.
pub trait IQueryNode: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn evaluate(&self, context: &IQueryContext) -> Vec<TagContent>;
    fn to_string_repr(&self) -> String;
}
