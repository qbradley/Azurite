use std::collections::HashMap;

use crate::errors::{StorageError, StorageErrorFactory};
use crate::generated::context::Context;

use super::query_nodes::*;

/// Mirrors TypeScript `enum ComparisonType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ComparisonType {
    Equal,
    Greater,
    Less,
    NotEqual,
}

/// Mirrors TypeScript `interface ComparisonNode`.
#[derive(Clone, Debug)]
struct ComparisonNode {
    #[allow(dead_code)]
    key: String,
    existed_comparison: Vec<ComparisonType>,
}

/// Mirrors TypeScript `export default function parseQuery(...)`.
///
/// Parses a blob tag query string into an AST of `IQueryNode`.
/// Preserves the unimplemented unary `not` grammar production (documented but skipped).
pub fn parse_query(
    request_context: &Context,
    query: &str,
    condition_header: Option<&str>,
) -> Result<Box<dyn IQueryNode>, StorageError> {
    let mut parser = QueryParser::new(request_context, query, condition_header);
    parser.visit()
}

/// Mirrors TypeScript `class QueryParser`.
///
/// A recursive descent parser for Azure Blob filter by tags query syntax.
struct QueryParser<'a> {
    request_context: &'a Context,
    query_ctx: ParserContext<'a>,
    comparison_nodes: HashMap<String, ComparisonNode>,
    comparison_count: usize,
    condition_header: Option<&'a str>,
    query_string: &'a str,
}

impl<'a> QueryParser<'a> {
    fn new(
        request_context: &'a Context,
        query: &'a str,
        condition_header: Option<&'a str>,
    ) -> Self {
        Self {
            request_context,
            query_ctx: ParserContext::new(request_context, query, condition_header),
            comparison_nodes: HashMap::new(),
            comparison_count: 0,
            condition_header,
            query_string: query,
        }
    }

    fn validate_with_previous_comparison(
        &self,
        key: &str,
        current_comparison: ComparisonType,
    ) -> Result<(), StorageError> {
        if self.condition_header.is_some() {
            return Ok(());
        }
        if current_comparison == ComparisonType::NotEqual {
            return Ok(());
        }

        if let Some(node) = self.comparison_nodes.get(key) {
            for existing in &node.existed_comparison {
                if current_comparison == ComparisonType::Equal {
                    return Err(StorageError::new(
                        400,
                        "InvalidQueryParameterValue".to_string(),
                        "can't have multiple conditions for a single tag unless they define a range".to_string(),
                        self.request_context.contextId().unwrap_or_default(),
                        StorageError::empty_extra(),
                    ));
                }

                if current_comparison == ComparisonType::Greater
                    && (*existing == ComparisonType::Greater || *existing == ComparisonType::Equal)
                {
                    return Err(StorageError::new(
                        400,
                        "InvalidQueryParameterValue".to_string(),
                        "can't have multiple conditions for a single tag unless they define a range".to_string(),
                        self.request_context.contextId().unwrap_or_default(),
                        StorageError::empty_extra(),
                    ));
                }

                if current_comparison == ComparisonType::Less
                    && (*existing == ComparisonType::Less || *existing == ComparisonType::Equal)
                {
                    return Err(StorageError::new(
                        400,
                        "InvalidQueryParameterValue".to_string(),
                        "can't have multiple conditions for a single tag unless they define a range".to_string(),
                        self.request_context.contextId().unwrap_or_default(),
                        StorageError::empty_extra(),
                    ));
                }
            }
        }

        Ok(())
    }

    fn append_comparison_node(
        &mut self,
        key: &str,
        current_comparison: ComparisonType,
    ) -> Result<(), StorageError> {
        if self.condition_header.is_some() {
            return Ok(());
        }

        if key != "@container" && !self.comparison_nodes.contains_key(key) {
            self.comparison_count += 1;
        }

        if self.comparison_count > 10 {
            return Err(StorageError::new(
                400,
                "InvalidQueryParameterValue".to_string(),
                "Error parsing query: there can be at most 10 unique tags in a query".to_string(),
                self.request_context.contextId().unwrap_or_default(),
                {
                    let mut extra = std::collections::BTreeMap::new();
                    extra.insert("QueryParameterName".to_string(), "where".to_string());
                    extra.insert(
                        "QueryParameterValue".to_string(),
                        self.query_string.to_string(),
                    );
                    extra
                },
            ));
        }

        if let Some(node) = self.comparison_nodes.get_mut(key) {
            node.existed_comparison.push(current_comparison);
        } else {
            self.comparison_nodes.insert(
                key.to_string(),
                ComparisonNode {
                    key: key.to_string(),
                    existed_comparison: vec![current_comparison],
                },
            );
        }

        Ok(())
    }

    /// Visits the root of the query syntax tree.
    fn visit(&mut self) -> Result<Box<dyn IQueryNode>, StorageError> {
        self.visit_query()
    }

    fn visit_query(&mut self) -> Result<Box<dyn IQueryNode>, StorageError> {
        let tree = self.visit_expression()?;
        self.query_ctx.skip_whitespace();
        self.query_ctx.assert_end_of_query()?;
        Ok(tree)
    }

    fn visit_expression(&mut self) -> Result<Box<dyn IQueryNode>, StorageError> {
        self.visit_or()
    }

    fn visit_or(&mut self) -> Result<Box<dyn IQueryNode>, StorageError> {
        let left = self.visit_and()?;
        self.query_ctx.skip_whitespace();
        if self.query_ctx.consume("or", true) {
            if self.condition_header.is_none() {
                return self.query_ctx.throw_err("unexpected or");
            }
            let right = self.visit_or()?;
            Ok(Box::new(OrNode::new(left, right)))
        } else {
            Ok(left)
        }
    }

    fn visit_and(&mut self) -> Result<Box<dyn IQueryNode>, StorageError> {
        let left = self.visit_unary()?;
        self.query_ctx.skip_whitespace();
        if self.query_ctx.consume("and", true) {
            let right = self.visit_and()?;
            Ok(Box::new(AndNode::new(left, right)))
        } else {
            Ok(left)
        }
    }

    /// Visits UNARY layer. Preserves unimplemented `not` grammar — does NOT consume "not".
    fn visit_unary(&mut self) -> Result<Box<dyn IQueryNode>, StorageError> {
        self.query_ctx.skip_whitespace();
        self.visit_expression_group()
    }

    fn visit_expression_group(&mut self) -> Result<Box<dyn IQueryNode>, StorageError> {
        self.query_ctx.skip_whitespace();
        if self.query_ctx.consume("(", false) {
            let child = self.visit_expression()?;
            self.query_ctx.skip_whitespace();
            if !self.query_ctx.consume(")", false) {
                let peek = self.query_ctx.peek().unwrap_or('\0');
                return self.query_ctx.throw_err(&format!(
                    "Expected a ')' to close the expression group, but found '{}' instead.",
                    peek
                ));
            }
            Ok(Box::new(ExpressionNode::new(child)))
        } else {
            self.visit_binary()
        }
    }

    fn visit_binary(&mut self) -> Result<Box<dyn IQueryNode>, StorageError> {
        let left = self.visit_key()?;

        self.query_ctx.skip_whitespace();
        let operator = self
            .query_ctx
            .consume_one_of(true, &["=", ">=", "<=", "<>", ">", "<"]);

        if let Some(op) = operator {
            let right = self.visit_value()?;
            let left_str = left.to_string_repr();

            match op.as_str() {
                "=" => {
                    self.validate_with_previous_comparison(&left_str, ComparisonType::Equal)?;
                    self.append_comparison_node(&left_str, ComparisonType::Equal)?;
                    Ok(Box::new(EqualsNode::new(left, right)))
                }
                "<>" => {
                    if self.condition_header.is_none() {
                        return self.query_ctx.throw_err("unexpected <>");
                    }
                    self.validate_with_previous_comparison(&left_str, ComparisonType::NotEqual)?;
                    self.append_comparison_node(&left_str, ComparisonType::NotEqual)?;
                    Ok(Box::new(NotEqualsNode::new(left, right)))
                }
                ">=" => {
                    self.validate_with_previous_comparison(&left_str, ComparisonType::Greater)?;
                    self.append_comparison_node(&left_str, ComparisonType::Greater)?;
                    Ok(Box::new(GreaterThanEqualNode::new(left, right)))
                }
                ">" => {
                    self.validate_with_previous_comparison(&left_str, ComparisonType::Greater)?;
                    self.append_comparison_node(&left_str, ComparisonType::Greater)?;
                    Ok(Box::new(GreaterThanNode::new(left, right)))
                }
                "<" => {
                    self.validate_with_previous_comparison(&left_str, ComparisonType::Less)?;
                    self.append_comparison_node(&left_str, ComparisonType::Less)?;
                    Ok(Box::new(LessThanNode::new(left, right)))
                }
                "<=" => {
                    self.validate_with_previous_comparison(&left_str, ComparisonType::Less)?;
                    self.append_comparison_node(&left_str, ComparisonType::Less)?;
                    Ok(Box::new(LessThanEqualNode::new(left, right)))
                }
                _ => Ok(left),
            }
        } else {
            Ok(left)
        }
    }

    fn visit_value(&mut self) -> Result<Box<dyn IQueryNode>, StorageError> {
        self.query_ctx.skip_whitespace();
        if let Some(c) = self.query_ctx.peek() {
            if c == '\'' {
                return self.visit_string(false);
            }
        }
        self.query_ctx.throw_err("expecting tag value")
    }

    fn contains_invalid_tag_key_character(key: &str) -> bool {
        for c in key.chars() {
            if !matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_') {
                return true;
            }
        }
        false
    }

    fn validate_key(&self, key: &str) -> Result<(), StorageError> {
        if key.starts_with('@') {
            if self.condition_header.is_some() {
                return self.query_ctx.throw_err("");
            }
            if key != "@container" {
                return self
                    .query_ctx
                    .throw_err(&format!("unsupported parameter '{}'", key));
            }
            return Ok(());
        }

        if self.condition_header.is_none() && (key.is_empty() || key.len() > 128) {
            return self
                .query_ctx
                .throw_err("tag must be between 1 and 128 characters in length");
        }
        if Self::contains_invalid_tag_key_character(key) {
            return self.query_ctx.throw_err(&format!("unexpected '{}'", key));
        }
        Ok(())
    }

    fn validate_value(&self, value: &str) -> Result<(), StorageError> {
        if self.condition_header.is_none() && value.len() > 256 {
            return self
                .query_ctx
                .throw_err("tag value must be between 0 and 256 characters in length");
        }
        for c in value.chars() {
            if !matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | ' ' | '+' | '-' | '.' | '/' | ':' | '=' | '_')
            {
                return self
                    .query_ctx
                    .throw_err(&format!("'{}' not permitted in tag name or value", c));
            }
        }
        Ok(())
    }

    /// Visits the STRING layer. Supports single-quote delimiters with doubled-quote escaping.
    fn visit_string(&mut self, is_a_key: bool) -> Result<Box<dyn IQueryNode>, StorageError> {
        let open_character = self.query_ctx.take_one().unwrap_or('\'');

        // State machine for doubled-quote escaping:
        // normal + (c != quote) → normal
        // normal + (c == quote, peek == quote) → escaping
        // normal + (c == quote, peek != quote) → end
        // escaping + any → normal
        let mut content = String::new();
        let mut state = "normal";

        loop {
            let c = self.query_ctx.peek();
            let peek = self.query_ctx.peek_at(1);

            if c.is_none() {
                break;
            }
            let c = c.unwrap();

            if state == "escaping" {
                content.push(c);
                self.query_ctx.advance();
                state = "normal";
            } else if c == open_character {
                if peek == Some(open_character) {
                    content.push(c);
                    self.query_ctx.advance();
                    state = "escaping";
                } else {
                    break;
                }
            } else {
                content.push(c);
                self.query_ctx.advance();
                state = "normal";
            }
        }

        if !self.query_ctx.consume(&open_character.to_string(), false) {
            let peek = self
                .query_ctx
                .peek()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "EOF".to_string());
            return self.query_ctx.throw_err(&format!(
                "Expected a `{}` to close the string, but found {} instead.",
                open_character, peek
            ));
        }

        if is_a_key {
            let key_name = content.replace(
                &format!("{}{}", open_character, open_character),
                &open_character.to_string(),
            );
            self.validate_key(&key_name)?;
            Ok(Box::new(KeyNode::new(key_name)))
        } else {
            let value = content.replace(
                &format!("{}{}", open_character, open_character),
                &open_character.to_string(),
            );
            self.validate_value(&value)?;
            Ok(Box::new(ConstantNode::new(value)))
        }
    }

    fn visit_key(&mut self) -> Result<Box<dyn IQueryNode>, StorageError> {
        self.query_ctx.skip_whitespace();
        if let Some(c) = self.query_ctx.peek() {
            if c == '"' {
                return self.visit_string(true);
            }
        }

        let identifier = self
            .query_ctx
            .take_while(|c| !c.is_whitespace() && c != '=' && c != '>' && c != '<');

        if identifier.is_empty() {
            let peek = self
                .query_ctx
                .peek()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "EOF".to_string());
            return self.query_ctx.throw_err(&format!(
                "Expected a valid identifier, but found '{}' instead.",
                peek
            ));
        }

        self.validate_key(&identifier)?;
        Ok(Box::new(KeyNode::new(identifier)))
    }
}

/// Mirrors TypeScript `class ParserContext`.
///
/// Provides logic and helper functions for consuming tokens from a query string.
pub struct ParserContext<'a> {
    request_context: &'a Context,
    query: &'a str,
    query_chars: Vec<char>,
    token_position: usize,
    condition_header: Option<&'a str>,
}

impl<'a> ParserContext<'a> {
    pub fn new(
        request_context: &'a Context,
        query: &'a str,
        condition_header: Option<&'a str>,
    ) -> Self {
        Self {
            request_context,
            query,
            query_chars: query.chars().collect(),
            token_position: 0,
            condition_header,
        }
    }

    pub fn assert_end_of_query(&self) -> Result<(), StorageError> {
        if self.token_position < self.query_chars.len() {
            let peek = self.peek().unwrap_or('\0');
            return self.throw_err(&format!("Unexpected token '{}'.", peek));
        }
        Ok(())
    }

    pub fn peek(&self) -> Option<char> {
        self.query_chars.get(self.token_position).copied()
    }

    pub fn peek_at(&self, offset: usize) -> Option<char> {
        self.query_chars.get(self.token_position + offset).copied()
    }

    pub fn skip_whitespace(&mut self) {
        while let Some(c) = self.query_chars.get(self.token_position) {
            if c.is_whitespace() {
                self.token_position += 1;
            } else {
                break;
            }
        }
    }

    pub fn consume(&mut self, sequence: &str, ignore_case: bool) -> bool {
        let seq_chars: Vec<char> = sequence.chars().collect();
        if self.token_position + seq_chars.len() > self.query_chars.len() {
            return false;
        }

        let matches = seq_chars.iter().enumerate().all(|(i, sc)| {
            let qc = self.query_chars[self.token_position + i];
            if ignore_case {
                qc.to_lowercase().eq(sc.to_lowercase())
            } else {
                qc == *sc
            }
        });

        if matches {
            self.token_position += seq_chars.len();
            true
        } else {
            false
        }
    }

    pub fn consume_one_of(&mut self, ignore_case: bool, options: &[&str]) -> Option<String> {
        for option in options {
            if self.consume(option, ignore_case) {
                return Some(option.to_string());
            }
        }
        None
    }

    pub fn take_one(&mut self) -> Option<char> {
        if self.token_position < self.query_chars.len() {
            let c = self.query_chars[self.token_position];
            self.token_position += 1;
            Some(c)
        } else {
            None
        }
    }

    pub fn advance(&mut self) {
        if self.token_position < self.query_chars.len() {
            self.token_position += 1;
        }
    }

    pub fn take_while<F>(&mut self, predicate: F) -> String
    where
        F: Fn(char) -> bool,
    {
        let start = self.token_position;
        while let Some(&c) = self.query_chars.get(self.token_position) {
            if predicate(c) {
                self.token_position += 1;
            } else {
                break;
            }
        }
        self.query_chars[start..self.token_position]
            .iter()
            .collect()
    }

    pub fn throw_err<T>(&self, message: &str) -> Result<T, StorageError> {
        if let Some(condition_header) = self.condition_header {
            let mut additional_messages = std::collections::BTreeMap::new();
            additional_messages.insert("HeaderName".to_string(), condition_header.to_string());
            additional_messages.insert("HeaderValue".to_string(), self.query.to_string());
            Err(StorageErrorFactory::getInvalidHeaderValue(
                self.request_context.contextId().as_deref(),
                Some(additional_messages),
            ))
        } else {
            Err(StorageError::new(
                400,
                "InvalidQueryParameterValue".to_string(),
                format!(
                    "Error parsing query at or near character position {}: {}",
                    self.token_position, message
                ),
                self.request_context.contextId().unwrap_or_default(),
                {
                    let mut extra = std::collections::BTreeMap::new();
                    extra.insert("QueryParameterName".to_string(), "where".to_string());
                    extra.insert("QueryParameterValue".to_string(), self.query.to_string());
                    extra
                },
            ))
        }
    }
}
