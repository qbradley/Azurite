use std::sync::Arc;

use azurite_common::i_logger::ILogger;

use crate::{
    authentication::IAuthenticator,
    context::TableStorageContext,
    errors::{StorageError, StorageErrorFactory},
    generated::{i_request::GeneratedHttpRequest, i_response::GeneratedHttpResponse},
};

pub type SharedAuthenticator = Arc<dyn IAuthenticator + Send + Sync>;
pub type SharedMiddlewareLogger = Arc<dyn ILogger + Send + Sync>;

#[derive(Clone)]
pub struct AuthenticationMiddleware {
    logger: SharedMiddlewareLogger,
    authenticators: Arc<Vec<SharedAuthenticator>>,
}

impl AuthenticationMiddleware {
    pub async fn apply(
        &self,
        context: &TableStorageContext,
        req: &GeneratedHttpRequest,
        res: &GeneratedHttpResponse,
    ) -> Result<(), StorageError> {
        let pass = self
            .authenticate(context, req, res, self.authenticators.as_slice())
            .await?;
        if pass {
            Ok(())
        } else {
            Err(StorageErrorFactory::getAuthorizationFailure(context))
        }
    }

    pub async fn authenticate(
        &self,
        context: &TableStorageContext,
        req: &GeneratedHttpRequest,
        _res: &GeneratedHttpResponse,
        authenticators: &[SharedAuthenticator],
    ) -> Result<bool, StorageError> {
        self.logger.verbose(
            "AuthenticationMiddlewareFactory:createAuthenticationMiddleware() Validating authentications.",
            context.contextId().as_deref(),
        );

        for authenticator in authenticators {
            match authenticator.validate(req, context).await? {
                Some(true) => return Ok(true),
                // Continue to next authenticator on false or None
                _ => continue,
            }
        }

        Ok(false)
    }
}

#[derive(Clone)]
pub struct AuthenticationMiddlewareFactory {
    logger: SharedMiddlewareLogger,
}

impl AuthenticationMiddlewareFactory {
    pub fn new(logger: SharedMiddlewareLogger) -> Self {
        Self { logger }
    }

    #[allow(non_snake_case)]
    pub fn createAuthenticationMiddleware(
        &self,
        authenticators: Vec<SharedAuthenticator>,
    ) -> AuthenticationMiddleware {
        AuthenticationMiddleware {
            logger: Arc::clone(&self.logger),
            authenticators: Arc::new(authenticators),
        }
    }

    pub async fn authenticate(
        &self,
        context: &TableStorageContext,
        req: &GeneratedHttpRequest,
        res: &GeneratedHttpResponse,
        authenticators: &[SharedAuthenticator],
    ) -> Result<bool, StorageError> {
        AuthenticationMiddleware {
            logger: Arc::clone(&self.logger),
            authenticators: Arc::new(authenticators.to_vec()),
        }
        .authenticate(context, req, res, authenticators)
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use azurite_common::i_logger::ILogger;

    use crate::{
        authentication::IAuthenticator,
        context::TableStorageContext,
        generated::{
            context::Context, i_request::GeneratedHttpRequest, i_response::GeneratedHttpResponse,
        },
    };

    use super::{AuthenticationMiddlewareFactory, SharedAuthenticator};

    #[derive(Default)]
    struct TestLogger;

    impl ILogger for TestLogger {
        fn error(&self, _message: &str, _contextID: Option<&str>) {}
        fn warn(&self, _message: &str, _contextID: Option<&str>) {}
        fn info(&self, _message: &str, _contextID: Option<&str>) {}
        fn verbose(&self, _message: &str, _contextID: Option<&str>) {}
        fn debug(&self, _message: &str, _contextID: Option<&str>) {}
    }

    struct StubAuthenticator {
        name: &'static str,
        result: Option<bool>,
        calls: Arc<Mutex<Vec<&'static str>>>,
    }

    #[async_trait]
    impl IAuthenticator for StubAuthenticator {
        async fn validate(
            &self,
            _req: &GeneratedHttpRequest,
            _context: &crate::generated::context::Context,
        ) -> Result<Option<bool>, crate::errors::StorageError> {
            self.calls.lock().unwrap().push(self.name);
            Ok(self.result)
        }
    }

    #[tokio::test]
    async fn authentication_stops_after_first_success() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let factory = AuthenticationMiddlewareFactory::new(Arc::new(TestLogger));
        let middleware = factory.createAuthenticationMiddleware(vec![
            Arc::new(StubAuthenticator {
                name: "shared-key",
                result: None,
                calls: Arc::clone(&calls),
            }) as SharedAuthenticator,
            Arc::new(StubAuthenticator {
                name: "account-sas",
                result: Some(true),
                calls: Arc::clone(&calls),
            }) as SharedAuthenticator,
            Arc::new(StubAuthenticator {
                name: "table-sas",
                result: Some(true),
                calls: Arc::clone(&calls),
            }) as SharedAuthenticator,
        ]);

        let context = Context::default();
        let table_context = TableStorageContext::new(&context);
        table_context.setAccount(Some(String::from("devstoreaccount1")));

        middleware
            .apply(
                &table_context,
                &GeneratedHttpRequest::default(),
                &GeneratedHttpResponse::default(),
            )
            .await
            .unwrap();

        assert_eq!(*calls.lock().unwrap(), vec!["shared-key", "account-sas"]);
    }

    #[tokio::test]
    async fn authentication_continues_past_explicit_failure() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let factory = AuthenticationMiddlewareFactory::new(Arc::new(TestLogger));
        let middleware = factory.createAuthenticationMiddleware(vec![
            Arc::new(StubAuthenticator {
                name: "shared-key",
                result: None,
                calls: Arc::clone(&calls),
            }) as SharedAuthenticator,
            Arc::new(StubAuthenticator {
                name: "account-sas",
                result: Some(false),
                calls: Arc::clone(&calls),
            }) as SharedAuthenticator,
            Arc::new(StubAuthenticator {
                name: "table-sas",
                result: Some(true),
                calls: Arc::clone(&calls),
            }) as SharedAuthenticator,
        ]);

        let context = Context::default();
        let table_context = TableStorageContext::new(&context);
        table_context.setAccount(Some(String::from("devstoreaccount1")));

        middleware
            .apply(
                &table_context,
                &GeneratedHttpRequest::default(),
                &GeneratedHttpResponse::default(),
            )
            .await
            .unwrap();

        assert_eq!(
            *calls.lock().unwrap(),
            vec!["shared-key", "account-sas", "table-sas"]
        );
    }
}
