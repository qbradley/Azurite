use crate::persistence::query_interpreter::and_node::AndNode;
use crate::persistence::query_interpreter::big_number_node::BigNumberNode;
use crate::persistence::query_interpreter::binary_data_node::BinaryDataNode;
use crate::persistence::query_interpreter::constant_node::ConstantNode;
use crate::persistence::query_interpreter::date_time_node::DateTimeNode;
use crate::persistence::query_interpreter::equals_node::EqualsNode;
use crate::persistence::query_interpreter::greater_than_equal_node::GreaterThanEqualNode;
use crate::persistence::query_interpreter::greater_than_node::GreaterThanNode;
use crate::persistence::query_interpreter::guid_node::GuidNode;
use crate::persistence::query_interpreter::i_query_node::IQueryNode;
use crate::persistence::query_interpreter::identifier_node::IdentifierNode;
use crate::persistence::query_interpreter::less_than_equal_node::LessThanEqualNode;
use crate::persistence::query_interpreter::less_than_node::LessThanNode;
use crate::persistence::query_interpreter::not_equals_node::NotEqualsNode;
use crate::persistence::query_interpreter::not_node::NotNode;
use crate::persistence::query_interpreter::or_node::OrNode;
use crate::persistence::query_interpreter::query_error::QueryError;
use crate::persistence::query_interpreter::query_lexer::{
    ComparisonOperator, LogicOperator, QueryLexer, QueryTokenKind, Token, TypeHint, UnaryOperator,
};

pub fn parse_query(query: &str) -> Result<Box<dyn IQueryNode>, QueryError> {
    QueryParser::new(query).visit()
}

pub struct QueryParser {
    tokens: QueryLexer,
}

impl QueryParser {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            tokens: QueryLexer::new(query),
        }
    }

    pub fn visit(&mut self) -> Result<Box<dyn IQueryNode>, QueryError> {
        self.visit_query()
    }

    fn visit_query(&mut self) -> Result<Box<dyn IQueryNode>, QueryError> {
        let tree = self.visit_expression()?;
        self.tokens
            .next_if(|token| token.kind() == QueryTokenKind::EndOfQuery)
            .ok_or_else(|| self.unexpected_token(&[QueryTokenKind::EndOfQuery]))?;
        Ok(tree)
    }

    fn visit_expression(&mut self) -> Result<Box<dyn IQueryNode>, QueryError> {
        self.visit_or()
    }

    fn visit_or(&mut self) -> Result<Box<dyn IQueryNode>, QueryError> {
        let left = self.visit_and()?;
        if self
            .tokens
            .next_if(|token| {
                matches!(
                    token,
                    Token::LogicOperator {
                        operator: LogicOperator::Or,
                        ..
                    }
                )
            })
            .is_some()
        {
            let right = self.visit_or()?;
            Ok(Box::new(OrNode::new(left, right)))
        } else {
            Ok(left)
        }
    }

    fn visit_and(&mut self) -> Result<Box<dyn IQueryNode>, QueryError> {
        let left = self.visit_unary()?;
        if self
            .tokens
            .next_if(|token| {
                matches!(
                    token,
                    Token::LogicOperator {
                        operator: LogicOperator::And,
                        ..
                    }
                )
            })
            .is_some()
        {
            let right = self.visit_and()?;
            Ok(Box::new(AndNode::new(left, right)))
        } else {
            Ok(left)
        }
    }

    fn visit_unary(&mut self) -> Result<Box<dyn IQueryNode>, QueryError> {
        let has_not = self
            .tokens
            .next_if(|token| {
                matches!(
                    token,
                    Token::UnaryOperator {
                        operator: UnaryOperator::Not,
                        ..
                    }
                )
            })
            .is_some();
        let right = self.visit_expression_group()?;
        if has_not {
            Ok(Box::new(NotNode::new(right)))
        } else {
            Ok(right)
        }
    }

    fn visit_expression_group(&mut self) -> Result<Box<dyn IQueryNode>, QueryError> {
        if self
            .tokens
            .next_if(|token| token.kind() == QueryTokenKind::OpenParen)
            .is_some()
        {
            let child = self.visit_expression()?;
            self.tokens
                .next_if(|token| token.kind() == QueryTokenKind::CloseParen)
                .ok_or_else(|| self.unexpected_token(&[QueryTokenKind::CloseParen]))?;
            Ok(child)
        } else {
            self.visit_binary()
        }
    }

    fn visit_binary(&mut self) -> Result<Box<dyn IQueryNode>, QueryError> {
        let left = self.visit_identifier_or_constant()?;
        let Some(operator) = self
            .tokens
            .next_if(|token| token.kind() == QueryTokenKind::ComparisonOperator)
        else {
            return Ok(left);
        };

        let right = self.visit_identifier_or_constant()?;
        match operator {
            Token::ComparisonOperator {
                operator: ComparisonOperator::Eq,
                ..
            } => Ok(Box::new(EqualsNode::new(left, right))),
            Token::ComparisonOperator {
                operator: ComparisonOperator::Ne,
                ..
            } => Ok(Box::new(NotEqualsNode::new(left, right))),
            Token::ComparisonOperator {
                operator: ComparisonOperator::Ge,
                ..
            } => Ok(Box::new(GreaterThanEqualNode::new(left, right))),
            Token::ComparisonOperator {
                operator: ComparisonOperator::Gt,
                ..
            } => Ok(Box::new(GreaterThanNode::new(left, right))),
            Token::ComparisonOperator {
                operator: ComparisonOperator::Le,
                ..
            } => Ok(Box::new(LessThanEqualNode::new(left, right))),
            Token::ComparisonOperator {
                operator: ComparisonOperator::Lt,
                ..
            } => Ok(Box::new(LessThanNode::new(left, right))),
            token => Err(QueryError::UnexpectedOperator {
                operator: token.display_value(),
                position: token.position(),
            }),
        }
    }

    fn visit_identifier_or_constant(&mut self) -> Result<Box<dyn IQueryNode>, QueryError> {
        match self.tokens.peek() {
            Token::Identifier { value, .. } => {
                self.tokens.next();
                Ok(Box::new(IdentifierNode::new(value)))
            }
            Token::Bool { value, .. } => {
                self.tokens.next();
                Ok(Box::new(ConstantNode::boolean(value)))
            }
            Token::String { value, .. } => {
                self.tokens.next();
                Ok(Box::new(ConstantNode::string(value)))
            }
            Token::Number { .. } => self.visit_number(),
            Token::TypeHint { .. } => self.visit_type_hint(),
            _ => Err(self.unexpected_token(&[
                QueryTokenKind::Identifier,
                QueryTokenKind::Bool,
                QueryTokenKind::String,
                QueryTokenKind::Number,
                QueryTokenKind::TypeHint,
            ])),
        }
    }

    fn visit_type_hint(&mut self) -> Result<Box<dyn IQueryNode>, QueryError> {
        let type_hint = self
            .tokens
            .next_if(|token| token.kind() == QueryTokenKind::TypeHint)
            .ok_or_else(|| self.unexpected_token(&[QueryTokenKind::TypeHint]))?;
        let value = self
            .tokens
            .next_if(|token| token.kind() == QueryTokenKind::String)
            .ok_or_else(|| self.unexpected_token(&[QueryTokenKind::String]))?;

        let value = match value {
            Token::String { value, .. } => value,
            _ => unreachable!(),
        };

        match type_hint {
            Token::TypeHint {
                hint: TypeHint::DateTime,
                ..
            } => Ok(Box::new(DateTimeNode::new(value))),
            Token::TypeHint {
                hint: TypeHint::Guid,
                ..
            } => Ok(Box::new(GuidNode::new(value))),
            Token::TypeHint {
                hint: TypeHint::Binary | TypeHint::X,
                ..
            } => Ok(Box::new(BinaryDataNode::new(value))),
            token => Err(QueryError::UnexpectedTypeHint {
                type_hint: token.display_value(),
                position: token.position(),
            }),
        }
    }

    fn visit_number(&mut self) -> Result<Box<dyn IQueryNode>, QueryError> {
        let token = self
            .tokens
            .next_if(|token| token.kind() == QueryTokenKind::Number)
            .ok_or_else(|| self.unexpected_token(&[QueryTokenKind::Number]))?;
        let Token::Number { value, .. } = token else {
            unreachable!();
        };

        if let Some(value) = value.strip_suffix('L') {
            Ok(Box::new(BigNumberNode::new(value.to_string())))
        } else {
            let parsed = value
                .parse::<f64>()
                .map_err(|err| QueryError::Parse(err.to_string()))?;
            Ok(Box::new(ConstantNode::number(parsed)))
        }
    }

    fn unexpected_token(&mut self, expected: &[QueryTokenKind]) -> QueryError {
        let actual = self.tokens.peek();
        QueryError::unexpected_token(
            actual.kind(),
            actual.display_value(),
            actual.position(),
            actual.length(),
            expected,
        )
    }
}
