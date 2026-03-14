use std::cmp::Ordering;
use std::fmt;

#[derive(Clone, Debug)]
pub enum QueryValue {
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    String(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryComparison {
    Less,
    Equal,
    Greater,
    Nan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryStaticType {
    Boolean,
    Number,
    String,
    Long,
    DateTime,
    Guid,
    Binary,
}

impl QueryComparison {
    pub fn from_ordering(ordering: Option<Ordering>) -> Self {
        match ordering {
            Some(Ordering::Less) => Self::Less,
            Some(Ordering::Equal) => Self::Equal,
            Some(Ordering::Greater) => Self::Greater,
            None => Self::Nan,
        }
    }
}

impl QueryValue {
    pub fn from_json(value: &serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(value) => Self::Bool(*value),
            serde_json::Value::Number(value) => Self::Number(value.as_f64().unwrap_or(f64::NAN)),
            serde_json::Value::String(value) => Self::String(value.clone()),
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                Self::String(value.to_string())
            }
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Undefined | Self::Null => false,
            Self::Bool(value) => *value,
            Self::Number(value) => *value != 0.0 && !value.is_nan(),
            Self::String(value) => !value.is_empty(),
        }
    }

    pub fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined)
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn strict_equal(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Undefined, Self::Undefined) | (Self::Null, Self::Null) => true,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::Number(left), Self::Number(right)) => {
                !left.is_nan() && !right.is_nan() && left == right
            }
            (Self::String(left), Self::String(right)) => left == right,
            _ => false,
        }
    }

    pub fn relational_compare(&self, other: &Self) -> Option<Ordering> {
        if matches!(self, Self::Undefined) || matches!(other, Self::Undefined) {
            return None;
        }

        if let (Self::String(left), Self::String(right)) = (self, other) {
            return Some(left.cmp(right));
        }

        let left = self.to_js_number()?;
        let right = other.to_js_number()?;
        if left.is_nan() || right.is_nan() {
            return None;
        }

        left.partial_cmp(&right)
    }

    pub fn to_js_number(&self) -> Option<f64> {
        match self {
            Self::Undefined => None,
            Self::Null => Some(0.0),
            Self::Bool(value) => Some(if *value { 1.0 } else { 0.0 }),
            Self::Number(value) => Some(*value),
            Self::String(value) => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    Some(0.0)
                } else {
                    trimmed.parse::<f64>().ok()
                }
            }
        }
    }

    pub fn to_string_value(&self) -> Option<String> {
        match self {
            Self::Undefined => None,
            Self::Null => Some("null".to_string()),
            Self::Bool(value) => Some(value.to_string()),
            Self::Number(value) => Some(format_number(*value)),
            Self::String(value) => Some(value.clone()),
        }
    }
}

impl fmt::Display for QueryStaticType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Boolean => write!(f, "bool"),
            Self::Number => write!(f, "number"),
            Self::String => write!(f, "string"),
            Self::Long => write!(f, "long"),
            Self::DateTime => write!(f, "datetime"),
            Self::Guid => write!(f, "guid"),
            Self::Binary => write!(f, "binary"),
        }
    }
}

fn format_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}
