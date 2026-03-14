use std::ops::Deref;

use crate::authentication::i_authentication_context::IAuthenticationContext;
use crate::generated::artifacts::models::GeneratedValue;
use crate::generated::context::Context;

const ACCOUNT_KEY: &str = "account";
const IS_SECONDARY_KEY: &str = "isSecondary";
const CONTAINER_KEY: &str = "container";
const BLOB_KEY: &str = "blob";
const AUTHENTICATION_PATH_KEY: &str = "authenticationPath";
const DISABLE_PRODUCT_STYLE_URL_KEY: &str = "disableProductStyleUrl";
const LOOSE_KEY: &str = "loose";

#[derive(Debug, Clone)]
pub struct BlobStorageContext {
    pub context: Context,
}

impl BlobStorageContext {
    pub fn new(context: &Context) -> Self {
        Self {
            context: Context::new(context),
        }
    }

    pub fn getContainer(&self) -> Option<String> {
        self.container()
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

    pub fn container(&self) -> Option<String> {
        self.get_string(CONTAINER_KEY)
    }

    pub fn setContainer(&self, container: Option<String>) {
        self.set_string(CONTAINER_KEY, container);
    }

    pub fn blob(&self) -> Option<String> {
        self.get_string(BLOB_KEY)
    }

    pub fn setBlob(&self, blob: Option<String>) {
        self.set_string(BLOB_KEY, blob);
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

    pub fn disableProductStyleUrl(&self) -> Option<bool> {
        self.get_bool(DISABLE_PRODUCT_STYLE_URL_KEY)
    }

    pub fn setDisableProductStyleUrl(&self, disableProductStyleUrl: Option<bool>) {
        self.set_bool(DISABLE_PRODUCT_STYLE_URL_KEY, disableProductStyleUrl);
    }

    pub fn loose(&self) -> Option<bool> {
        self.get_bool(LOOSE_KEY)
    }

    pub fn setLoose(&self, loose: Option<bool>) {
        self.set_bool(LOOSE_KEY, loose);
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

impl Deref for BlobStorageContext {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl IAuthenticationContext for BlobStorageContext {
    fn account(&self) -> Option<String> {
        self.account()
    }
}
