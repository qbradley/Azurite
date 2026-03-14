use thiserror::Error;

use crate::persistence::query_interpreter::query_lexer::QueryTokenKind;

#[derive(Debug, Error)]
pub enum QueryError {
    #[error(
        "Unexpected token '{actual_kind}' at {actual_value}:{position}+{length} (expected one of: {expected})."
    )]
    UnexpectedToken {
        actual_kind: QueryTokenKind,
        actual_value: String,
        position: usize,
        length: usize,
        expected: String,
    },
    #[error(
        "Got an unexpected operator '{operator}' at :{position}, expected one of: eq, ne, ge, gt, le, lt."
    )]
    UnexpectedOperator { operator: String, position: usize },
    #[error(
        "Got an unexpected type hint '{type_hint}' at :{position} (this implies that the parser is missing a match arm)."
    )]
    UnexpectedTypeHint { type_hint: String, position: usize },
    #[error("{0}")]
    Parse(String),
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Evaluation(String),
}

impl QueryError {
    pub fn unexpected_token(
        actual_kind: QueryTokenKind,
        actual_value: String,
        position: usize,
        length: usize,
        expected: &[QueryTokenKind],
    ) -> Self {
        Self::UnexpectedToken {
            actual_kind,
            actual_value,
            position,
            length,
            expected: expected
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
        }
    }
}
