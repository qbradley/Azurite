use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ContainerSASPermission {
    #[serde(rename = "r")]
    Read,
    #[serde(rename = "a")]
    Add,
    #[serde(rename = "c")]
    Create,
    #[serde(rename = "w")]
    Write,
    #[serde(rename = "d")]
    Delete,
    #[serde(rename = "l")]
    List,
    #[serde(rename = "f")]
    Filter,
    #[serde(rename = "AnyPermission")]
    Any,
}

impl ContainerSASPermission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "r",
            Self::Add => "a",
            Self::Create => "c",
            Self::Write => "w",
            Self::Delete => "d",
            Self::List => "l",
            Self::Filter => "f",
            Self::Any => "AnyPermission",
        }
    }
}

impl Display for ContainerSASPermission {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
