#![allow(non_snake_case)]
//! Parity tests for Table Storage query parser and lexer.
//! Validates OData $filter parsing, token production, and query tree building.

use azurite_table::query_interpreter::query_lexer::{
    QueryLexer, QueryTokenKind, Token,
};
use azurite_table::query_interpreter::query_parser::parse_query;

// ── Lexer Tests ────────────────────────────────────────────

#[test]
fn lexer_tokenizes_simple_eq_filter() {
    let mut lexer = QueryLexer::new("PartitionKey eq 'pk1'");
    let t1 = lexer.next_token();
    assert!(matches!(t1, Token::Identifier { .. }));
    assert_eq!(t1.display_value(), "PartitionKey");

    let t2 = lexer.next_token();
    assert_eq!(t2.kind(), QueryTokenKind::ComparisonOperator);

    let t3 = lexer.next_token();
    assert_eq!(t3.kind(), QueryTokenKind::String);
    assert_eq!(t3.display_value(), "pk1");
}

#[test]
fn lexer_tokenizes_and_operator() {
    let mut lexer = QueryLexer::new("A eq 1 and B eq 2");
    // A
    assert_eq!(lexer.next_token().kind(), QueryTokenKind::Identifier);
    // eq
    assert_eq!(lexer.next_token().kind(), QueryTokenKind::ComparisonOperator);
    // 1
    assert_eq!(lexer.next_token().kind(), QueryTokenKind::Number);
    // and
    let and_token = lexer.next_token();
    assert_eq!(and_token.kind(), QueryTokenKind::LogicOperator);
    // B
    assert_eq!(lexer.next_token().kind(), QueryTokenKind::Identifier);
}

#[test]
fn lexer_tokenizes_boolean_true() {
    let mut lexer = QueryLexer::new("true");
    let t = lexer.next_token();
    assert_eq!(t.kind(), QueryTokenKind::Bool);
    assert_eq!(t.display_value(), "true");
}

#[test]
fn lexer_tokenizes_boolean_false() {
    let mut lexer = QueryLexer::new("false");
    let t = lexer.next_token();
    assert_eq!(t.kind(), QueryTokenKind::Bool);
    assert_eq!(t.display_value(), "false");
}

#[test]
fn lexer_handles_parentheses() {
    let mut lexer = QueryLexer::new("(A eq 1)");
    assert_eq!(lexer.next_token().kind(), QueryTokenKind::OpenParen);
    assert_eq!(lexer.next_token().kind(), QueryTokenKind::Identifier);
    assert_eq!(lexer.next_token().kind(), QueryTokenKind::ComparisonOperator);
    assert_eq!(lexer.next_token().kind(), QueryTokenKind::Number);
    assert_eq!(lexer.next_token().kind(), QueryTokenKind::CloseParen);
}

#[test]
fn lexer_produces_eof_at_end() {
    let mut lexer = QueryLexer::new("x");
    let _ = lexer.next_token(); // x
    let eof = lexer.next_token();
    assert_eq!(eof.kind(), QueryTokenKind::EndOfQuery);
}

#[test]
fn lexer_handles_single_quoted_strings_with_escapes() {
    let mut lexer = QueryLexer::new("Name eq 'O''Brien'");
    let _ = lexer.next_token(); // Name
    let _ = lexer.next_token(); // eq
    let t = lexer.next_token();
    assert_eq!(t.kind(), QueryTokenKind::String);
    assert_eq!(t.display_value(), "O'Brien");
}

#[test]
fn lexer_handles_negative_numbers() {
    let mut lexer = QueryLexer::new("Value gt -42");
    let _ = lexer.next_token(); // Value
    let _ = lexer.next_token(); // gt
    let t = lexer.next_token();
    assert_eq!(t.kind(), QueryTokenKind::Number);
    assert_eq!(t.display_value(), "-42");
}

#[test]
fn lexer_tokenizes_all_comparison_operators() {
    for op in &["eq", "ne", "gt", "ge", "lt", "le"] {
        let mut lexer = QueryLexer::new(&format!("X {} 1", op));
        let _ = lexer.next_token(); // X
        let t = lexer.next_token();
        assert_eq!(t.kind(), QueryTokenKind::ComparisonOperator, "operator {}", op);
    }
}

#[test]
fn lexer_tokenizes_or_logic_operator() {
    let mut lexer = QueryLexer::new("A eq 1 or B eq 2");
    let _ = lexer.next_token(); // A
    let _ = lexer.next_token(); // eq
    let _ = lexer.next_token(); // 1
    let t = lexer.next_token();
    assert_eq!(t.kind(), QueryTokenKind::LogicOperator);
}

// ── Parser Tests ───────────────────────────────────────────

#[test]
fn parser_parses_simple_eq() {
    let tree = parse_query("PartitionKey eq 'pk1'");
    assert!(tree.is_ok(), "simple eq should parse: {:?}", tree.err());
    let node = tree.unwrap();
    assert_eq!(node.name(), "eq");
}

#[test]
fn parser_parses_ne() {
    let tree = parse_query("Status ne 'active'");
    assert!(tree.is_ok());
    assert_eq!(tree.unwrap().name(), "ne");
}

#[test]
fn parser_parses_gt() {
    let tree = parse_query("Count gt 5");
    assert!(tree.is_ok());
    assert_eq!(tree.unwrap().name(), "gt");
}

#[test]
fn parser_parses_lt() {
    let tree = parse_query("Count lt 10");
    assert!(tree.is_ok());
    assert_eq!(tree.unwrap().name(), "lt");
}

#[test]
fn parser_parses_ge() {
    let tree = parse_query("Count ge 5");
    assert!(tree.is_ok());
    assert_eq!(tree.unwrap().name(), "ge");
}

#[test]
fn parser_parses_le() {
    let tree = parse_query("Count le 10");
    assert!(tree.is_ok());
    assert_eq!(tree.unwrap().name(), "le");
}

#[test]
fn parser_parses_and_expression() {
    let tree = parse_query("PartitionKey eq 'pk1' and RowKey eq 'rk1'");
    assert!(tree.is_ok());
    let node = tree.unwrap();
    assert_eq!(node.name(), "and");
    assert!(node.left().is_some());
    assert!(node.right().is_some());
}

#[test]
fn parser_parses_or_expression() {
    let tree = parse_query("Status eq 'active' or Status eq 'pending'");
    assert!(tree.is_ok());
    assert_eq!(tree.unwrap().name(), "or");
}

#[test]
fn parser_parses_parenthesized_expression() {
    let tree = parse_query("(PartitionKey eq 'pk1')");
    assert!(tree.is_ok());
    assert_eq!(tree.unwrap().name(), "eq");
}

#[test]
fn parser_parses_nested_and_or() {
    let tree = parse_query("(A eq 1 and B eq 2) or C eq 3");
    assert!(tree.is_ok());
    assert_eq!(tree.unwrap().name(), "or");
}

#[test]
fn parser_parses_not_expression() {
    let tree = parse_query("not (Status eq 'deleted')");
    assert!(tree.is_ok());
    assert_eq!(tree.unwrap().name(), "not");
}

#[test]
fn parser_parses_boolean_constant_true() {
    let tree = parse_query("IsActive eq true");
    assert!(tree.is_ok());
}

#[test]
fn parser_parses_boolean_constant_false() {
    let tree = parse_query("IsActive eq false");
    assert!(tree.is_ok());
}

#[test]
fn parser_parses_numeric_comparison() {
    let tree = parse_query("Age gt 25");
    assert!(tree.is_ok());
}

#[test]
fn parser_parses_negative_number() {
    let tree = parse_query("Balance lt -100");
    assert!(tree.is_ok());
}

#[test]
fn parser_parses_datetime_type_hint() {
    let tree = parse_query("Timestamp ge datetime'2023-01-01T00:00:00Z'");
    assert!(tree.is_ok());
}

#[test]
fn parser_parses_guid_type_hint() {
    let tree = parse_query("Id eq guid'550e8400-e29b-41d4-a716-446655440000'");
    assert!(tree.is_ok());
}

#[test]
fn parser_rejects_empty_query() {
    let tree = parse_query("");
    assert!(tree.is_err(), "empty query should fail");
}

#[test]
fn parser_parses_complex_three_clause_and() {
    let tree = parse_query("PartitionKey eq 'pk' and RowKey eq 'rk' and Status eq 'ok'");
    assert!(tree.is_ok());
    let node = tree.unwrap();
    assert_eq!(node.name(), "and");
}

#[test]
fn parser_parses_string_with_spaces() {
    let tree = parse_query("Name eq 'John Doe'");
    assert!(tree.is_ok());
}
