use std::fmt::{self, Display, Formatter};

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TableSASPermission {
    Query,
    Add,
    Update,
    Delete,
}

impl TableSASPermission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Query => "r",
            Self::Add => "a",
            Self::Update => "u",
            Self::Delete => "d",
        }
    }
}

impl Display for TableSASPermission {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
