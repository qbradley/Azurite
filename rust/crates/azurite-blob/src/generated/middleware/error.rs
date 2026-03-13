use crate::generated::artifacts::models::GeneratedValue;
use crate::generated::context::Context;
use crate::generated::errors::middleware_error::MiddlewareError;
use crate::generated::i_request::IRequest;
use crate::generated::i_response::IResponse;
use crate::generated::utils::i_logger::ILogger;

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
        res.setStatusCode(err.statusCode);
        if let Some(statusMessage) = &err.statusMessage {
            res.setStatusMessage(statusMessage.clone());
        }
        if let Some(headers) = &err.headers {
            for (key, value) in headers {
                res.setHeader(key, Some(value.clone()));
            }
        }
        if req.getMethod().to_string() != "HEAD" {
            if let Some(contentType) = &err.contentType {
                res.setContentType(Some(contentType.clone()));
            }
            if let Some(body) = &err.body {
                match body {
                    GeneratedValue::String(value) => res.getBodyStream().write_text(value),
                    other => res
                        .getBodyStream()
                        .write_text(&serde_json::to_string(&other.to_json_value())?),
                }
            }
        }
        return Ok(());
    }

    logger.error(
        "ErrorMiddleware: Received an error, fill error information to HTTP response",
        context.contextId().as_deref(),
    );
    res.setStatusCode(500);
    Ok(())
}
