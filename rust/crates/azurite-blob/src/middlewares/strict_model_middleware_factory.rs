use std::sync::Arc;

use azurite_common::i_logger::ILogger;

use crate::{
    errors::{StorageError, StrictModelNotSupportedError},
    generated::{
        context::Context,
        i_request::{GeneratedHttpRequest, IRequest},
    },
    utils::constants::HeaderConstants,
};

pub type StrictModelRequestValidator =
    fn(&GeneratedHttpRequest, &Context, &(dyn ILogger + Send + Sync)) -> Result<(), StorageError>;

#[allow(non_snake_case)]
pub fn UnsupportedHeadersBlocker(
    req: &GeneratedHttpRequest,
    context: &Context,
    _logger: &(dyn ILogger + Send + Sync),
) -> Result<(), StorageError> {
    let UnsupportedHeaderKeys = [
        HeaderConstants::X_MS_CONTENT_CRC64,
        HeaderConstants::X_MS_RANGE_GET_CONTENT_CRC64,
        HeaderConstants::X_MS_ENCRYPTION_KEY,
        HeaderConstants::X_MS_ENCRYPTION_KEY_SHA256,
        HeaderConstants::X_MS_ENCRYPTION_ALGORITHM,
    ];

    for headerKey in UnsupportedHeaderKeys {
        if req.getHeader(headerKey).is_some() {
            return Err(StrictModelNotSupportedError::new(
                headerKey,
                context.contextId().as_deref(),
            )
            .into());
        }
    }

    Ok(())
}

#[allow(non_snake_case)]
pub fn UnsupportedParametersBlocker(
    req: &GeneratedHttpRequest,
    context: &Context,
    _logger: &(dyn ILogger + Send + Sync),
) -> Result<(), StorageError> {
    let UnsupportedParameterKeys: [&str; 0] = [];

    for parameterKey in UnsupportedParameterKeys {
        if req.getQuery(parameterKey).is_some() {
            return Err(StrictModelNotSupportedError::new(
                parameterKey,
                context.contextId().as_deref(),
            )
            .into());
        }
    }

    Ok(())
}

#[derive(Clone)]
pub struct StrictModelMiddleware {
    logger: Arc<dyn ILogger + Send + Sync>,
    validators: Arc<Vec<StrictModelRequestValidator>>,
}

impl StrictModelMiddleware {
    pub fn apply(&self, req: &GeneratedHttpRequest, context: &Context) -> Result<(), StorageError> {
        self.validate(req, context)
    }

    fn validate(&self, req: &GeneratedHttpRequest, context: &Context) -> Result<(), StorageError> {
        for validator in self.validators.iter() {
            validator(req, context, self.logger.as_ref())?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct StrictModelMiddlewareFactory {
    logger: Arc<dyn ILogger + Send + Sync>,
    validators: Vec<StrictModelRequestValidator>,
}

impl StrictModelMiddlewareFactory {
    pub fn new(
        logger: Arc<dyn ILogger + Send + Sync>,
        validators: Vec<StrictModelRequestValidator>,
    ) -> Self {
        Self { logger, validators }
    }

    #[allow(non_snake_case)]
    pub fn createStrictModelMiddleware(&self) -> StrictModelMiddleware {
        StrictModelMiddleware {
            logger: Arc::clone(&self.logger),
            validators: Arc::new(self.validators.clone()),
        }
    }
}
