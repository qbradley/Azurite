use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum QueueSASPermission {
    #[serde(rename = "r")]
    Read,
    #[serde(rename = "a")]
    Add,
    #[serde(rename = "u")]
    Update,
    #[serde(rename = "p")]
    Process,
}

impl QueueSASPermission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "r",
            Self::Add => "a",
            Self::Update => "u",
            Self::Process => "p",
        }
    }
}

impl Display for QueueSASPermission {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
