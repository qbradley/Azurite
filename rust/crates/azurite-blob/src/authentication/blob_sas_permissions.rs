use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BlobSASPermission {
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
    #[serde(rename = "x")]
    DeleteVersion,
    #[serde(rename = "t")]
    Tag,
    #[serde(rename = "m")]
    Move,
    #[serde(rename = "e")]
    execute,
    #[serde(rename = "i")]
    SetImmutabilityPolicy,
    #[serde(rename = "y")]
    permanentDelete,
}

impl BlobSASPermission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "r",
            Self::Add => "a",
            Self::Create => "c",
            Self::Write => "w",
            Self::Delete => "d",
            Self::DeleteVersion => "x",
            Self::Tag => "t",
            Self::Move => "m",
            Self::execute => "e",
            Self::SetImmutabilityPolicy => "i",
            Self::permanentDelete => "y",
        }
    }
}

impl Display for BlobSASPermission {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
