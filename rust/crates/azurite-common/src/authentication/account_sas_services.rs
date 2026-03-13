use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::storage_error::StorageError;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AccountSASService {
    #[serde(rename = "b")]
    Blob,
    #[serde(rename = "f")]
    File,
    #[serde(rename = "q")]
    Queue,
    #[serde(rename = "t")]
    Table,
}

impl AccountSASService {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Blob => "b",
            Self::File => "f",
            Self::Queue => "q",
            Self::Table => "t",
        }
    }
}

impl Display for AccountSASService {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AccountSASServices {
    pub blob: bool,
    pub file: bool,
    pub queue: bool,
    pub table: bool,
}

#[allow(non_snake_case)]
impl AccountSASServices {
    pub fn parse(services: &str) -> Result<AccountSASServices, StorageError> {
        let mut accountSASServices = AccountSASServices::default();

        for c in services.chars() {
            match c {
                'b' => {
                    if accountSASServices.blob {
                        return Err(StorageError::new(format!(
                            "Duplicated permission character: {c}"
                        )));
                    }
                    accountSASServices.blob = true;
                }
                'f' => {
                    if accountSASServices.file {
                        return Err(StorageError::new(format!(
                            "Duplicated permission character: {c}"
                        )));
                    }
                    accountSASServices.file = true;
                }
                'q' => {
                    if accountSASServices.queue {
                        return Err(StorageError::new(format!(
                            "Duplicated permission character: {c}"
                        )));
                    }
                    accountSASServices.queue = true;
                }
                't' => {
                    if accountSASServices.table {
                        return Err(StorageError::new(format!(
                            "Duplicated permission character: {c}"
                        )));
                    }
                    accountSASServices.table = true;
                }
                _ => {
                    return Err(StorageError::new(format!("Invalid service character: {c}")));
                }
            }
        }

        Ok(accountSASServices)
    }

    pub fn toString(&self) -> String {
        let mut services = Vec::new();
        if self.blob {
            services.push(AccountSASService::Blob.as_str());
        }
        if self.table {
            services.push(AccountSASService::Table.as_str());
        }
        if self.queue {
            services.push(AccountSASService::Queue.as_str());
        }
        if self.file {
            services.push(AccountSASService::File.as_str());
        }
        services.join("")
    }
}

impl Display for AccountSASServices {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.toString())
    }
}

#[cfg(test)]
mod tests {
    use super::AccountSASServices;

    #[test]
    fn parse_accepts_unordered_services_but_serializes_canonically() {
        let services = AccountSASServices::parse("fbtq").unwrap();

        assert!(services.blob);
        assert!(services.file);
        assert!(services.queue);
        assert!(services.table);
        assert_eq!(services.toString(), "btqf");
    }
}
