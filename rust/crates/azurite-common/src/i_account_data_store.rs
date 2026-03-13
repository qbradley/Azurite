use async_trait::async_trait;

use crate::{i_cleaner::ICleaner, i_data_store::IDataStore};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IAccountProperties {
    pub name: String,
    pub key1: Vec<u8>,
    pub key2: Option<Vec<u8>>,
}

#[allow(non_snake_case)]
#[async_trait]
pub trait IAccountDataStore: IDataStore + ICleaner + Send + Sync {
    fn getAccount(&self, name: &str) -> Option<IAccountProperties>;
}
