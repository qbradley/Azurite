use crate::generated::artifacts::operation::ALL_OPERATIONS;
use crate::generated::artifacts::specifications::{specification, OperationSpec};
use crate::generated::context::Context;
use crate::generated::errors::unsupported_request_error::UnsupportedRequestError;
use crate::generated::i_request::IRequest;
use crate::generated::utils::i_logger::ILogger;
use crate::generated::utils::utils::isURITemplateMatch;

pub fn dispatch_middleware<R: IRequest, L: ILogger + ?Sized>(
    context: &Context,
    req: &R,
    logger: &L,
) -> crate::generated::GeneratedResult<()> {
    logger.verbose(
        "DispatchMiddleware: Dispatching request...",
        context.contextId().as_deref(),
    );
    let mut conditionsMet: i32 = -1;

    for operation in ALL_OPERATIONS {
        if let Some(spec) = specification(operation) {
            let (isMatch, metConditions) =
                is_request_against_operation(req, spec, context.dispatchPattern().as_deref());
            if isMatch && metConditions > conditionsMet {
                context.setOperation(Some(operation));
                conditionsMet = metConditions;
            }
        }
    }

    if context.operation().is_none() {
        let handlerError = UnsupportedRequestError::new();
        logger.error(
            &format!("DispatchMiddleware: {}", handlerError.message),
            context.contextId().as_deref(),
        );
        return Err(Box::new(handlerError));
    }

    logger.info(
        &format!(
            "DispatchMiddleware: Operation={}",
            context.operation().unwrap()
        ),
        context.contextId().as_deref(),
    );
    Ok(())
}

fn is_request_against_operation<R: IRequest>(
    req: &R,
    spec: &OperationSpec,
    dispatchPathPattern: Option<&str>,
) -> (bool, i32) {
    let mut metConditionsNum = 0;
    let mut method = req.getMethod().to_string();
    if let Some(xHttpMethod) = req.getHeader("X-HTTP-Method") {
        let value = xHttpMethod.trim().to_owned();
        if matches!(value.as_str(), "GET" | "MERGE" | "PATCH" | "DELETE") {
            method = value;
        }
    }

    if method != spec.httpMethod {
        return (false, metConditionsNum + 1);
    }

    let path = spec
        .path
        .clone()
        .map(|path| {
            if path.starts_with('/') {
                path
            } else {
                format!("/{path}")
            }
        })
        .unwrap_or_else(|| String::from("/"));
    if !isURITemplateMatch(dispatchPathPattern.unwrap_or(&req.getPath()), &path) {
        return (false, metConditionsNum + 1);
    }

    for queryParameter in &spec.queryParameters {
        if queryParameter.mapper.required {
            let queryValue = req.getQuery(
                queryParameter
                    .mapper
                    .serializedName
                    .as_deref()
                    .unwrap_or(""),
            );
            if queryValue.is_none() {
                return (false, metConditionsNum);
            }
            if queryParameter.mapper.r#type.name == "Enum" {
                if let Some(queryValue) = &queryValue {
                    if !queryParameter
                        .mapper
                        .r#type
                        .allowedValues
                        .iter()
                        .any(|value| value == queryValue)
                    {
                        return (false, metConditionsNum);
                    }
                }
            }
            if queryParameter.mapper.isConstant {
                if let Some(defaultValue) = &queryParameter.mapper.defaultValue {
                    if queryValue.as_deref() != defaultValue.as_str() {
                        return (false, metConditionsNum);
                    }
                }
            }
            metConditionsNum += 1;
        }
    }

    for headerParameter in &spec.headerParameters {
        if headerParameter.mapper.required {
            let headerValue = req.getHeader(
                headerParameter
                    .mapper
                    .serializedName
                    .as_deref()
                    .unwrap_or(""),
            );
            if headerValue.is_none() {
                return (false, metConditionsNum);
            }
            if headerParameter.mapper.r#type.name == "Enum" {
                if let Some(headerValue) = &headerValue {
                    if !headerParameter
                        .mapper
                        .r#type
                        .allowedValues
                        .iter()
                        .any(|value| value == headerValue)
                    {
                        return (false, metConditionsNum);
                    }
                }
            }
            if headerParameter.mapper.isConstant {
                if let Some(defaultValue) = &headerParameter.mapper.defaultValue {
                    if defaultValue
                        .as_str()
                        .map(|value| value.to_ascii_lowercase())
                        != headerValue.map(|value| value.to_ascii_lowercase())
                    {
                        return (false, metConditionsNum);
                    }
                }
            }
            metConditionsNum += 1;
        }
    }

    (true, metConditionsNum)
}
