use std::ops::Deref;

use crate::authentication::i_authentication_context::IAuthenticationContext;
use crate::generated::artifacts::models::GeneratedValue;
use crate::generated::context::Context;

const ACCOUNT_KEY: &str = "account";
const IS_SECONDARY_KEY: &str = "isSecondary";
const QUEUE_KEY: &str = "queue";
const MESSAGE_KEY: &str = "message";
const MESSAGE_ID_KEY: &str = "messageId";
const AUTHENTICATION_PATH_KEY: &str = "authenticationPath";

#[allow(non_snake_case)]
#[derive(Debug, Clone, Default)]
pub struct QueueStorageContext {
    pub context: Context,
}

#[allow(non_snake_case)]
impl QueueStorageContext {
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

    pub fn isSecondary(&self) -> Option<bool> {
        self.get_bool(IS_SECONDARY_KEY)
    }

    pub fn setIsSecondary(&self, isSecondary: Option<bool>) {
        self.set_bool(IS_SECONDARY_KEY, isSecondary);
    }

    pub fn queue(&self) -> Option<String> {
        self.get_string(QUEUE_KEY)
    }

    pub fn setQueue(&self, queue: Option<String>) {
        self.set_string(QUEUE_KEY, queue);
    }

    pub fn message(&self) -> Option<String> {
        self.get_string(MESSAGE_KEY)
    }

    pub fn setMessage(&self, message: Option<String>) {
        self.set_string(MESSAGE_KEY, message);
    }

    pub fn messageId(&self) -> Option<String> {
        self.get_string(MESSAGE_ID_KEY)
    }

    pub fn setMessageId(&self, messageId: Option<String>) {
        self.set_string(MESSAGE_ID_KEY, messageId);
    }

    pub fn authenticationPath(&self) -> Option<String> {
        self.get_string(AUTHENTICATION_PATH_KEY)
    }

    pub fn setAuthenticationPath(&self, path: Option<String>) {
        self.set_string(AUTHENTICATION_PATH_KEY, path);
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

impl Deref for QueueStorageContext {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl IAuthenticationContext for QueueStorageContext {
    fn account(&self) -> Option<String> {
        QueueStorageContext::account(self)
    }
}
