use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::storage_error::StorageError;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AccountSASResourceType {
    #[serde(rename = "s")]
    Service,
    #[serde(rename = "c")]
    Container,
    #[serde(rename = "o")]
    Object,
    #[serde(rename = "AnyResourceType")]
    Any,
}

impl AccountSASResourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Service => "s",
            Self::Container => "c",
            Self::Object => "o",
            Self::Any => "AnyResourceType",
        }
    }
}

impl Display for AccountSASResourceType {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AccountSASResourceTypes {
    pub service: bool,
    pub container: bool,
    pub object: bool,
}

#[allow(non_snake_case)]
impl AccountSASResourceTypes {
    pub fn parse(resourceTypes: &str) -> Result<AccountSASResourceTypes, StorageError> {
        let mut accountSASResourceTypes = AccountSASResourceTypes::default();

        for c in resourceTypes.chars() {
            match c {
                's' => {
                    if accountSASResourceTypes.service {
                        return Err(StorageError::new(format!(
                            "Duplicated permission character: {c}"
                        )));
                    }
                    accountSASResourceTypes.service = true;
                }
                'c' => {
                    if accountSASResourceTypes.container {
                        return Err(StorageError::new(format!(
                            "Duplicated permission character: {c}"
                        )));
                    }
                    accountSASResourceTypes.container = true;
                }
                'o' => {
                    if accountSASResourceTypes.object {
                        return Err(StorageError::new(format!(
                            "Duplicated permission character: {c}"
                        )));
                    }
                    accountSASResourceTypes.object = true;
                }
                _ => {
                    return Err(StorageError::new(format!("Invalid resource type: {c}")));
                }
            }
        }

        Ok(accountSASResourceTypes)
    }

    pub fn toString(&self) -> String {
        let mut resourceTypes = Vec::new();
        if self.service {
            resourceTypes.push(AccountSASResourceType::Service.as_str());
        }
        if self.container {
            resourceTypes.push(AccountSASResourceType::Container.as_str());
        }
        if self.object {
            resourceTypes.push(AccountSASResourceType::Object.as_str());
        }
        resourceTypes.join("")
    }
}

impl Display for AccountSASResourceTypes {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.toString())
    }
}

#[cfg(test)]
mod tests {
    use super::AccountSASResourceTypes;

    #[test]
    fn parse_accepts_unordered_resource_types_but_serializes_canonically() {
        let resourceTypes = AccountSASResourceTypes::parse("ocs").unwrap();

        assert!(resourceTypes.service);
        assert!(resourceTypes.container);
        assert!(resourceTypes.object);
        assert_eq!(resourceTypes.toString(), "sco");
    }
}
