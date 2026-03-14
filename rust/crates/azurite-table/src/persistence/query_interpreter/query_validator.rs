use crate::persistence::query_interpreter::i_query_node::IQueryNode;
use crate::persistence::query_interpreter::query_error::QueryError;
use crate::persistence::query_interpreter::query_value::QueryStaticType;

pub fn validate_query_tree(query_tree: &dyn IQueryNode) -> Result<(), QueryError> {
    if count_identifier_references(query_tree) == 0 {
        return Err(QueryError::Validation(
            "Invalid Query, no identifier references found.".to_string(),
        ));
    }

    validate_node(query_tree)
}

fn count_identifier_references(query_tree: &dyn IQueryNode) -> usize {
    let mut count = usize::from(query_tree.identifier_reference().is_some());
    if let Some(left) = query_tree.left() {
        count += count_identifier_references(left);
    }
    if let Some(right) = query_tree.right() {
        count += count_identifier_references(right);
    }
    count
}

fn validate_node(node: &dyn IQueryNode) -> Result<(), QueryError> {
    if matches!(node.name(), "and" | "or" | "not") {
        validate_boolean_operand(node.right(), node.name())?;
        if node.name() != "not" {
            validate_boolean_operand(node.left(), node.name())?;
        }
    }

    if let (Some(left), Some(right)) = (node.left(), node.right()) {
        validate_comparison_types(node.name(), left.static_type(), right.static_type())?;
        validate_node(left)?;
        validate_node(right)?;
    } else if let Some(right) = node.right() {
        validate_node(right)?;
    }

    Ok(())
}

fn validate_boolean_operand(
    node: Option<&dyn IQueryNode>,
    operator: &str,
) -> Result<(), QueryError> {
    if let Some(QueryStaticType::Boolean) | None = node.and_then(IQueryNode::static_type) {
        return Ok(());
    }

    Err(QueryError::Validation(format!(
        "Invalid Query, operator '{operator}' expects a boolean expression."
    )))
}

fn validate_comparison_types(
    operator: &str,
    left: Option<QueryStaticType>,
    right: Option<QueryStaticType>,
) -> Result<(), QueryError> {
    let (Some(left), Some(right)) = (left, right) else {
        return Ok(());
    };

    if left == right {
        return Ok(());
    }

    let compatible = matches!(
        (left, right),
        (QueryStaticType::String, QueryStaticType::Long)
            | (QueryStaticType::Long, QueryStaticType::String)
            | (QueryStaticType::String, QueryStaticType::DateTime)
            | (QueryStaticType::DateTime, QueryStaticType::String)
            | (QueryStaticType::String, QueryStaticType::Guid)
            | (QueryStaticType::Guid, QueryStaticType::String)
            | (QueryStaticType::String, QueryStaticType::Binary)
            | (QueryStaticType::Binary, QueryStaticType::String)
    );

    if compatible {
        return Ok(());
    }

    if matches!(operator, "eq" | "ne" | "gt" | "ge" | "lt" | "le") {
        return Err(QueryError::Validation(format!(
            "Invalid Query, operator '{operator}' cannot compare values of type '{left}' and '{right}'."
        )));
    }

    Ok(())
}
