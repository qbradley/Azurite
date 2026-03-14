use std::ops::Deref;

use crate::authentication::i_authentication_context::IAuthenticationContext;
use crate::generated::artifacts::models::GeneratedValue;
use crate::generated::context::Context;

const ACCOUNT_KEY: &str = "account";
const TABLE_NAME_KEY: &str = "tableName";
const AUTHENTICATION_PATH_KEY: &str = "authenticationPath";
const PARTITION_KEY_KEY: &str = "partitionKey";
const ROW_KEY_KEY: &str = "rowKey";
const ACCEPT_KEY: &str = "accept";
const IS_SECONDARY_KEY: &str = "isSecondary";
const BATCH_ID_KEY: &str = "batchId";

#[allow(non_snake_case)]
#[derive(Debug, Clone, Default)]
pub struct TableStorageContext {
    pub context: Context,
}

#[allow(non_snake_case)]
impl TableStorageContext {
    pub fn new(context: &Context) -> Self {
        Self {
            context: Context::new(context),
        }
    }

    pub fn account(&self) -> Option<String> {
        self.get_string(ACCOUNT_KEY)
    }

    pub fn setAccount(&self, account: Option<String>) {
        self.set_string(ACCOUNT_KEY, account);
    }

    pub fn tableName(&self) -> Option<String> {
        self.get_string(TABLE_NAME_KEY)
    }

    pub fn setTableName(&self, tableName: Option<String>) {
        self.set_string(TABLE_NAME_KEY, tableName);
    }

    pub fn authenticationPath(&self) -> Option<String> {
        self.get_string(AUTHENTICATION_PATH_KEY)
    }

    pub fn setAuthenticationPath(&self, path: Option<String>) {
        self.set_string(AUTHENTICATION_PATH_KEY, path);
    }

    pub fn partitionKey(&self) -> Option<String> {
        self.get_string(PARTITION_KEY_KEY)
    }

    pub fn setPartitionKey(&self, partitionKey: Option<String>) {
        self.set_string(PARTITION_KEY_KEY, partitionKey);
    }

    pub fn rowKey(&self) -> Option<String> {
        self.get_string(ROW_KEY_KEY)
    }

    pub fn setRowKey(&self, rowKey: Option<String>) {
        self.set_string(ROW_KEY_KEY, rowKey);
    }

    pub fn accept(&self) -> Option<String> {
        self.get_string(ACCEPT_KEY)
    }

    pub fn setAccept(&self, accept: Option<String>) {
        self.set_string(ACCEPT_KEY, accept);
    }

    pub fn isSecondary(&self) -> Option<bool> {
        self.get_bool(IS_SECONDARY_KEY)
    }

    pub fn setIsSecondary(&self, isSecondary: Option<bool>) {
        self.set_bool(IS_SECONDARY_KEY, isSecondary);
    }

    pub fn batchId(&self) -> Option<String> {
        self.get_string(BATCH_ID_KEY)
    }

    pub fn setBatchId(&self, batchId: Option<String>) {
        self.set_string(BATCH_ID_KEY, batchId);
    }

    pub fn xMsRequestID(&self) -> Option<String> {
        self.context.contextId()
    }

    pub fn setXMsRequestID(&self, xMsRequestID: Option<String>) {
        self.context.setContextId(xMsRequestID);
    }

    fn get_string(&self, key: &str) -> Option<String> {
        self.context
            .extras()
            .get(key)
            .and_then(GeneratedValue::as_string)
    }

    fn set_string(&self, key: &str, value: Option<String>) {
        self.context.insertExtra(
            key,
            match value {
                Some(value) => GeneratedValue::String(value),
                None => GeneratedValue::Null,
            },
        );
    }

    fn get_bool(&self, key: &str) -> Option<bool> {
        self.context
            .extras()
            .get(key)
            .and_then(GeneratedValue::as_bool)
    }

    fn set_bool(&self, key: &str, value: Option<bool>) {
        self.context.insertExtra(
            key,
            match value {
                Some(value) => GeneratedValue::Bool(value),
                None => GeneratedValue::Null,
            },
        );
    }
}

impl Deref for TableStorageContext {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl IAuthenticationContext for TableStorageContext {
    fn account(&self) -> Option<String> {
        TableStorageContext::account(self)
    }
}
