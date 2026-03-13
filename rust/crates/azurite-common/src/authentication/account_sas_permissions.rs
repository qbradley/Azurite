use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::storage_error::StorageError;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AccountSASPermission {
    #[serde(rename = "r")]
    Read,
    #[serde(rename = "w")]
    Write,
    #[serde(rename = "d")]
    Delete,
    #[serde(rename = "x")]
    DeleteVersion,
    #[serde(rename = "l")]
    List,
    #[serde(rename = "a")]
    Add,
    #[serde(rename = "c")]
    Create,
    #[serde(rename = "u")]
    Update,
    #[serde(rename = "p")]
    Process,
    #[serde(rename = "t")]
    Tag,
    #[serde(rename = "f")]
    Filter,
    #[serde(rename = "i")]
    SetImmutabilityPolicy,
    #[serde(rename = "y")]
    PermanentDelete,
    #[serde(rename = "AnyPermission")]
    Any,
}

impl AccountSASPermission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "r",
            Self::Write => "w",
            Self::Delete => "d",
            Self::DeleteVersion => "x",
            Self::List => "l",
            Self::Add => "a",
            Self::Create => "c",
            Self::Update => "u",
            Self::Process => "p",
            Self::Tag => "t",
            Self::Filter => "f",
            Self::SetImmutabilityPolicy => "i",
            Self::PermanentDelete => "y",
            Self::Any => "AnyPermission",
        }
    }
}

impl Display for AccountSASPermission {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AccountSASPermissions {
    pub read: bool,
    pub write: bool,
    pub delete: bool,
    pub deleteVersion: bool,
    pub list: bool,
    pub add: bool,
    pub create: bool,
    pub update: bool,
    pub process: bool,
    pub tag: bool,
    pub filter: bool,
    pub setImmutabilityPolicy: bool,
    pub permanentDelete: bool,
}

#[allow(non_snake_case)]
impl AccountSASPermissions {
    pub fn parse(permissions: &str) -> Result<AccountSASPermissions, StorageError> {
        let mut accountSASPermissions = AccountSASPermissions::default();

        for c in permissions.chars() {
            match c {
                'r' => {
                    if accountSASPermissions.read {
                        return Err(StorageError::new(format!(
                            "Duplicated permission character: {c}"
                        )));
                    }
                    accountSASPermissions.read = true;
                }
                'w' => {
                    if accountSASPermissions.write {
                        return Err(StorageError::new(format!(
                            "Duplicated permission character: {c}"
                        )));
                    }
                    accountSASPermissions.write = true;
                }
                'd' => {
                    if accountSASPermissions.delete {
                        return Err(StorageError::new(format!(
                            "Duplicated permission character: {c}"
                        )));
                    }
                    accountSASPermissions.delete = true;
                }
                'x' => {
                    if accountSASPermissions.deleteVersion {
                        return Err(StorageError::new(format!(
                            "Duplicated permission character: {c}"
                        )));
                    }
                    accountSASPermissions.deleteVersion = true;
                }
                'l' => {
                    if accountSASPermissions.list {
                        return Err(StorageError::new(format!(
                            "Duplicated permission character: {c}"
                        )));
                    }
                    accountSASPermissions.list = true;
                }
                'a' => {
                    if accountSASPermissions.add {
                        return Err(StorageError::new(format!(
                            "Duplicated permission character: {c}"
                        )));
                    }
                    accountSASPermissions.add = true;
                }
                'c' => {
                    if accountSASPermissions.create {
                        return Err(StorageError::new(format!(
                            "Duplicated permission character: {c}"
                        )));
                    }
                    accountSASPermissions.create = true;
                }
                'u' => {
                    if accountSASPermissions.update {
                        return Err(StorageError::new(format!(
                            "Duplicated permission character: {c}"
                        )));
                    }
                    accountSASPermissions.update = true;
                }
                'p' => {
                    if accountSASPermissions.process {
                        return Err(StorageError::new(format!(
                            "Duplicated permission character: {c}"
                        )));
                    }
                    accountSASPermissions.process = true;
                }
                't' => {
                    if accountSASPermissions.tag {
                        return Err(StorageError::new(format!(
                            "Duplicated permission character: {c}"
                        )));
                    }
                    accountSASPermissions.tag = true;
                }
                'f' => {
                    if accountSASPermissions.filter {
                        return Err(StorageError::new(format!(
                            "Duplicated permission character: {c}"
                        )));
                    }
                    accountSASPermissions.filter = true;
                }
                'i' => {
                    if accountSASPermissions.setImmutabilityPolicy {
                        return Err(StorageError::new(format!(
                            "Duplicated permission character: {c}"
                        )));
                    }
                    accountSASPermissions.setImmutabilityPolicy = true;
                }
                'y' => {
                    if accountSASPermissions.permanentDelete {
                        return Err(StorageError::new(format!(
                            "Duplicated permission character: {c}"
                        )));
                    }
                    accountSASPermissions.permanentDelete = true;
                }
                _ => {
                    return Err(StorageError::new(format!(
                        "Invalid permission character: {c}"
                    )));
                }
            }
        }

        Ok(accountSASPermissions)
    }

    pub fn toString(&self) -> String {
        let mut permissions = Vec::new();
        if self.read {
            permissions.push(AccountSASPermission::Read.as_str());
        }
        if self.write {
            permissions.push(AccountSASPermission::Write.as_str());
        }
        if self.delete {
            permissions.push(AccountSASPermission::Delete.as_str());
        }
        if self.deleteVersion {
            permissions.push(AccountSASPermission::DeleteVersion.as_str());
        }
        if self.list {
            permissions.push(AccountSASPermission::List.as_str());
        }
        if self.add {
            permissions.push(AccountSASPermission::Add.as_str());
        }
        if self.create {
            permissions.push(AccountSASPermission::Create.as_str());
        }
        if self.update {
            permissions.push(AccountSASPermission::Update.as_str());
        }
        if self.process {
            permissions.push(AccountSASPermission::Process.as_str());
        }
        if self.tag {
            permissions.push(AccountSASPermission::Tag.as_str());
        }
        if self.filter {
            permissions.push(AccountSASPermission::Filter.as_str());
        }
        if self.setImmutabilityPolicy {
            permissions.push(AccountSASPermission::SetImmutabilityPolicy.as_str());
        }
        if self.permanentDelete {
            permissions.push(AccountSASPermission::PermanentDelete.as_str());
        }
        permissions.join("")
    }
}

impl Display for AccountSASPermissions {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.toString())
    }
}

#[cfg(test)]
mod tests {
    use super::AccountSASPermissions;

    #[test]
    fn parse_accepts_unordered_permissions_but_serializes_canonically() {
        let permissions = AccountSASPermissions::parse("ytr").unwrap();

        assert!(permissions.read);
        assert!(permissions.tag);
        assert!(permissions.permanentDelete);
        assert_eq!(permissions.toString(), "rty");
    }

    #[test]
    fn parse_rejects_duplicate_permissions() {
        let error = AccountSASPermissions::parse("rr").unwrap_err();

        assert_eq!(error.message, "Duplicated permission character: r");
    }
}
