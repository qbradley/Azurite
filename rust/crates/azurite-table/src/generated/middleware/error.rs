use crate::errors::{NotImplementedError, StorageError};
use crate::generated::artifacts::models::GeneratedValue;
use crate::generated::context::Context;
use crate::generated::errors::middleware_error::MiddlewareError;
use crate::generated::i_request::IRequest;
use crate::generated::i_response::{IResponse, ResponseHeaderValue};
use crate::generated::utils::i_logger::ILogger;
use std::collections::BTreeMap;

pub fn error_middleware<RQ: IRequest, RS: IResponse, L: ILogger + ?Sized>(
    context: &Context,
    err: &(dyn std::error::Error + Send + Sync + 'static),
    req: &RQ,
    res: &mut RS,
    logger: &L,
) -> crate::generated::GeneratedResult<()> {
    if res.headersSent() {
        logger.warn(
            "Error middleware received an error, but response.headersSent is true, pass error to next middleware",
            context.contextId().as_deref(),
        );
        return Err(err.to_string().into());
    }

    if let Some(err) = err.downcast_ref::<MiddlewareError>() {
        logger.error(
            "ErrorMiddleware: Received a MiddlewareError, fill error information to HTTP response",
            context.contextId().as_deref(),
        );
        write_error_response(
            res,
            req,
            err.statusCode,
            err.statusMessage.as_deref(),
            err.headers.as_ref(),
            err.contentType.as_deref(),
            err.body.as_ref(),
        );
        return Ok(());
    }

    if let Some(err) = err.downcast_ref::<StorageError>() {
        logger.error(
            &format!("ErrorMiddleware: Received a StorageError: {}", err),
            context.contextId().as_deref(),
        );
        write_error_response(
            res,
            req,
            err.statusCode,
            err.statusMessage.as_deref(),
            err.headers.as_ref(),
            err.contentType.as_deref(),
            err.body.as_ref(),
        );
        return Ok(());
    }

    if let Some(err) = err.downcast_ref::<NotImplementedError>() {
        logger.error(
            &format!("ErrorMiddleware: Received a NotImplementedError: {}", err),
            context.contextId().as_deref(),
        );
        write_error_response(
            res,
            req,
            err.statusCode,
            err.statusMessage.as_deref(),
            err.headers.as_ref(),
            err.contentType.as_deref(),
            err.body.as_ref(),
        );
        return Ok(());
    }

    logger.error(
        &format!("ErrorMiddleware: Received an unknown error: {}", err),
        context.contextId().as_deref(),
    );
    res.setStatusCode(500);
    Ok(())
}

fn write_error_response<RQ: IRequest, RS: IResponse>(
    res: &mut RS,
    req: &RQ,
    statusCode: u16,
    statusMessage: Option<&str>,
    headers: Option<&BTreeMap<String, ResponseHeaderValue>>,
    contentType: Option<&str>,
    body: Option<&GeneratedValue>,
) {
    res.setStatusCode(statusCode);
    if let Some(statusMessage) = statusMessage {
        res.setStatusMessage(statusMessage.to_string());
    }
    if let Some(headers) = headers {
        for (key, value) in headers {
            res.setHeader(key, Some(value.clone()));
        }
    }
    if req.getMethod().to_string() != "HEAD" {
        if let Some(contentType) = contentType {
            res.setContentType(Some(contentType.to_string()));
        }
        if let Some(body) = body {
            match body {
                GeneratedValue::String(value) => res.getBodyStream().write_text(value),
                other => {
                    if let Ok(json) = serde_json::to_string(&other.to_json_value()) {
                        res.getBodyStream().write_text(&json);
                    }
                }
            }
        }
    }
}
