use crate::persistence::query_interpreter::i_query_context::IQueryContext;
use crate::persistence::query_interpreter::i_query_node::IQueryNode;

pub fn execute_query(context: &dyn IQueryContext, query_tree: &dyn IQueryNode) -> bool {
    query_tree
        .evaluate(context)
        .map(|value| value.is_truthy())
        .unwrap_or(false)
}
