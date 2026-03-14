use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BlobSASResourceType {
    #[serde(rename = "c")]
    Container,
    #[serde(rename = "b")]
    Blob,
    #[serde(rename = "bs")]
    BlobSnapshot,
}

impl BlobSASResourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Container => "c",
            Self::Blob => "b",
            Self::BlobSnapshot => "bs",
        }
    }
}

impl Display for BlobSASResourceType {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
