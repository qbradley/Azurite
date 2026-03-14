use std::sync::Arc;

use azurite_common::i_logger::ILogger;

use crate::{
    authentication::IAuthenticator,
    context::BlobStorageContext,
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
        context: &BlobStorageContext,
        req: &GeneratedHttpRequest,
        res: &GeneratedHttpResponse,
    ) -> Result<(), StorageError> {
        let pass = self
            .authenticate(context, req, res, self.authenticators.as_slice())
            .await?;
        if pass {
            Ok(())
        } else {
            Err(StorageErrorFactory::getAuthorizationFailure(
                context.contextId().as_deref().unwrap_or_default(),
            ))
        }
    }

    pub async fn authenticate(
        &self,
        context: &BlobStorageContext,
        req: &GeneratedHttpRequest,
        _res: &GeneratedHttpResponse,
        authenticators: &[SharedAuthenticator],
    ) -> Result<bool, StorageError> {
        self.logger.verbose(
            "AuthenticationMiddlewareFactory:createAuthenticationMiddleware() Validating authentications.",
            context.contextId().as_deref(),
        );

        for authenticator in authenticators {
            if let Some(pass) = authenticator.validate(req, context).await? {
                return Ok(pass);
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
        context: &BlobStorageContext,
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
