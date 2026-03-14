use std::fmt;
use std::sync::OnceLock;

use regex::Regex;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryTokenKind {
    Identifier,
    Bool,
    Number,
    TypeHint,
    String,
    OpenParen,
    CloseParen,
    UnaryOperator,
    ComparisonOperator,
    LogicOperator,
    EndOfQuery,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComparisonOperator {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnaryOperator {
    Not,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogicOperator {
    And,
    Or,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypeHint {
    DateTime,
    Guid,
    Binary,
    X,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Token {
    Identifier {
        position: usize,
        length: usize,
        value: String,
    },
    Bool {
        position: usize,
        length: usize,
        value: bool,
        lexeme: String,
    },
    Number {
        position: usize,
        length: usize,
        value: String,
    },
    TypeHint {
        position: usize,
        length: usize,
        hint: TypeHint,
        lexeme: String,
    },
    String {
        position: usize,
        length: usize,
        value: String,
    },
    OpenParen {
        position: usize,
    },
    CloseParen {
        position: usize,
    },
    UnaryOperator {
        position: usize,
        length: usize,
        operator: UnaryOperator,
        lexeme: String,
    },
    ComparisonOperator {
        position: usize,
        length: usize,
        operator: ComparisonOperator,
        lexeme: String,
    },
    LogicOperator {
        position: usize,
        length: usize,
        operator: LogicOperator,
        lexeme: String,
    },
    EndOfQuery {
        position: usize,
    },
}

impl Token {
    pub fn kind(&self) -> QueryTokenKind {
        match self {
            Self::Identifier { .. } => QueryTokenKind::Identifier,
            Self::Bool { .. } => QueryTokenKind::Bool,
            Self::Number { .. } => QueryTokenKind::Number,
            Self::TypeHint { .. } => QueryTokenKind::TypeHint,
            Self::String { .. } => QueryTokenKind::String,
            Self::OpenParen { .. } => QueryTokenKind::OpenParen,
            Self::CloseParen { .. } => QueryTokenKind::CloseParen,
            Self::UnaryOperator { .. } => QueryTokenKind::UnaryOperator,
            Self::ComparisonOperator { .. } => QueryTokenKind::ComparisonOperator,
            Self::LogicOperator { .. } => QueryTokenKind::LogicOperator,
            Self::EndOfQuery { .. } => QueryTokenKind::EndOfQuery,
        }
    }

    pub fn position(&self) -> usize {
        match self {
            Self::Identifier { position, .. }
            | Self::Bool { position, .. }
            | Self::Number { position, .. }
            | Self::TypeHint { position, .. }
            | Self::String { position, .. }
            | Self::UnaryOperator { position, .. }
            | Self::ComparisonOperator { position, .. }
            | Self::LogicOperator { position, .. }
            | Self::OpenParen { position }
            | Self::CloseParen { position }
            | Self::EndOfQuery { position } => *position,
        }
    }

    pub fn length(&self) -> usize {
        match self {
            Self::Identifier { length, .. }
            | Self::Bool { length, .. }
            | Self::Number { length, .. }
            | Self::TypeHint { length, .. }
            | Self::String { length, .. }
            | Self::UnaryOperator { length, .. }
            | Self::ComparisonOperator { length, .. }
            | Self::LogicOperator { length, .. } => *length,
            Self::OpenParen { .. } | Self::CloseParen { .. } => 1,
            Self::EndOfQuery { .. } => 0,
        }
    }

    pub fn display_value(&self) -> String {
        match self {
            Self::Identifier { value, .. }
            | Self::Number { value, .. }
            | Self::String { value, .. } => value.clone(),
            Self::Bool { lexeme, .. }
            | Self::TypeHint { lexeme, .. }
            | Self::UnaryOperator { lexeme, .. }
            | Self::ComparisonOperator { lexeme, .. }
            | Self::LogicOperator { lexeme, .. } => lexeme.clone(),
            Self::OpenParen { .. } => "(".to_string(),
            Self::CloseParen { .. } => ")".to_string(),
            Self::EndOfQuery { .. } => String::new(),
        }
    }
}

impl fmt::Display for QueryTokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identifier => write!(f, "identifier"),
            Self::Bool => write!(f, "bool"),
            Self::Number => write!(f, "number"),
            Self::TypeHint => write!(f, "type-hint"),
            Self::String => write!(f, "string"),
            Self::OpenParen => write!(f, "open-paren"),
            Self::CloseParen => write!(f, "close-paren"),
            Self::UnaryOperator => write!(f, "unary-operator"),
            Self::ComparisonOperator => write!(f, "comparison-operator"),
            Self::LogicOperator => write!(f, "logic-operator"),
            Self::EndOfQuery => write!(f, "end-of-query"),
        }
    }
}

pub struct QueryLexer {
    query: String,
    token_position: usize,
}

impl QueryLexer {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            token_position: 0,
        }
    }

    pub fn next(&mut self) -> Token {
        let token = self.peek();
        self.token_position = token.position() + token.length();
        token
    }

    pub fn next_if(&mut self, predicate: impl Fn(&Token) -> bool) -> Option<Token> {
        let token = self.peek();
        if predicate(&token) {
            self.token_position = token.position() + token.length();
            Some(token)
        } else {
            None
        }
    }

    pub fn peek(&mut self) -> Token {
        self.skip_whitespace();

        let bytes = self.query.as_bytes();
        if self.token_position >= bytes.len() {
            return Token::EndOfQuery {
                position: self.token_position,
            };
        }

        match bytes[self.token_position] as char {
            '(' => Token::OpenParen {
                position: self.token_position,
            },
            ')' => Token::CloseParen {
                position: self.token_position,
            },
            '\'' | '"' => self.peek_string(),
            _ => self.peek_word(),
        }
    }

    fn peek_string(&self) -> Token {
        let start = self.token_position;
        let bytes = self.query.as_bytes();
        let open = bytes[start] as char;
        let mut position = start + 1;
        let mut closed = false;

        while position < bytes.len() {
            let current = bytes[position] as char;
            if current == open && position + 1 < bytes.len() && bytes[position + 1] as char == open
            {
                position += 2;
            } else if current == open {
                position += 1;
                closed = true;
                break;
            } else {
                position += 1;
            }
        }

        let end = if closed {
            position.saturating_sub(1)
        } else {
            position
        };
        let raw = &self.query[start + 1..end];
        Token::String {
            position: start,
            length: position - start,
            value: raw.replace(&format!("{0}{0}", open), &open.to_string()),
        }
    }

    fn peek_word(&self) -> Token {
        let start = self.token_position;
        let bytes = self.query.as_bytes();
        let mut position = start;
        while position < bytes.len() {
            let current = bytes[position] as char;
            if current.is_whitespace() || matches!(current, '(' | ')' | '\'' | '"') {
                break;
            }
            position += 1;
        }

        let value = self.query[start..position].to_string();
        let lower = value.to_ascii_lowercase();
        let next_char = bytes.get(position).map(|value| *value as char);

        if let Some(operator) = match lower.as_str() {
            "and" => Some(LogicOperator::And),
            "or" => Some(LogicOperator::Or),
            _ => None,
        } {
            return Token::LogicOperator {
                position: start,
                length: value.len(),
                operator,
                lexeme: value,
            };
        }

        if lower == "not" {
            return Token::UnaryOperator {
                position: start,
                length: value.len(),
                operator: UnaryOperator::Not,
                lexeme: value,
            };
        }

        if let Some(operator) = match lower.as_str() {
            "eq" => Some(ComparisonOperator::Eq),
            "ne" => Some(ComparisonOperator::Ne),
            "gt" => Some(ComparisonOperator::Gt),
            "ge" => Some(ComparisonOperator::Ge),
            "lt" => Some(ComparisonOperator::Lt),
            "le" => Some(ComparisonOperator::Le),
            _ => None,
        } {
            return Token::ComparisonOperator {
                position: start,
                length: value.len(),
                operator,
                lexeme: value,
            };
        }

        if lower == "true" || lower == "false" {
            return Token::Bool {
                position: start,
                length: value.len(),
                value: lower == "true",
                lexeme: value,
            };
        }

        if let Some(hint) = match lower.as_str() {
            "datetime" if matches!(next_char, Some('\'' | '"')) => Some(TypeHint::DateTime),
            "guid" if matches!(next_char, Some('\'' | '"')) => Some(TypeHint::Guid),
            "binary" if matches!(next_char, Some('\'' | '"')) => Some(TypeHint::Binary),
            "x" if matches!(next_char, Some('\'' | '"')) => Some(TypeHint::X),
            _ => None,
        } {
            return Token::TypeHint {
                position: start,
                length: value.len(),
                hint,
                lexeme: value,
            };
        }

        if number_regex().is_match(&value) {
            return Token::Number {
                position: start,
                length: value.len(),
                value,
            };
        }

        Token::Identifier {
            position: start,
            length: value.len(),
            value,
        }
    }

    fn skip_whitespace(&mut self) {
        let bytes = self.query.as_bytes();
        while self.token_position < bytes.len()
            && (bytes[self.token_position] as char).is_whitespace()
        {
            self.token_position += 1;
        }
    }
}

fn number_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"^[+-]?[0-9]+(\.[0-9]+)?L?$").unwrap())
}
