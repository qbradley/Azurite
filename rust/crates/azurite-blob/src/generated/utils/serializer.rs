use std::collections::BTreeMap;

use crate::generated::artifacts::mappers::Mapper;
use crate::generated::artifacts::models::{
    GeneratedBody, GeneratedObject, GeneratedResponse, GeneratedValue,
};
use crate::generated::artifacts::parameters::{OperationParameter, ParameterPath};
use crate::generated::artifacts::specifications::{OperationSpec, ResponseSpec};
use crate::generated::context::{Context, IHandlerParameters};
use crate::generated::errors::deserialization_error::DeserializationError;
use crate::generated::i_request::{GeneratedReadableStream, IRequest, RequestHeaderValue};
use crate::generated::i_response::{IResponse, ResponseHeaderValue};
use crate::generated::utils::i_logger::ILogger;
use crate::generated::utils::xml::{parseXML, stringifyXML};

pub async fn deserialize<R: IRequest, L: ILogger + ?Sized>(
    context: &Context,
    req: &mut R,
    spec: &OperationSpec,
    logger: &L,
) -> crate::generated::GeneratedResult<IHandlerParameters> {
    let mut parameters = GeneratedObject::new();

    for queryParameter in &spec.queryParameters {
        let queryKey = queryParameter
            .mapper
            .serializedName
            .clone()
            .ok_or_else(|| {
                DeserializationError::new(
                    "QueryParameter mapper doesn't include valid serializedName",
                )
            })?;
        let queryValueOriginal = req.getQuery(&queryKey);
        let queryValue = deserialize_primitive(
            &queryParameter.mapper,
            queryValueOriginal.map(GeneratedValue::String),
        );
        setParametersValue(&mut parameters, &queryParameter.parameterPath, queryValue);
    }

    for headerParameter in &spec.headerParameters {
        if let Some(prefix) = &headerParameter.mapper.headerCollectionPrefix {
            let mut dictionary = GeneratedObject::new();
            for (headerKey, value) in req.getHeaders() {
                if headerKey
                    .to_ascii_lowercase()
                    .starts_with(&prefix.to_ascii_lowercase())
                {
                    let partial = headerKey[prefix.len()..].to_owned();
                    dictionary.insert(partial, deserialize_header_value(value));
                }
            }
            setParametersValue(
                &mut parameters,
                &headerParameter.parameterPath,
                GeneratedValue::Object(dictionary),
            );
        } else {
            let headerKey = headerParameter
                .mapper
                .serializedName
                .clone()
                .ok_or_else(|| {
                    DeserializationError::new(
                        "HeaderParameter mapper doesn't include valid serializedName",
                    )
                })?;
            let headerValue = req.getHeader(&headerKey).map(GeneratedValue::String);
            let deserialized = deserialize_primitive(&headerParameter.mapper, headerValue);
            setParametersValue(
                &mut parameters,
                &headerParameter.parameterPath,
                deserialized,
            );
        }
    }

    if let Some(bodyParameter) = &spec.requestBody {
        if bodyParameter.mapper.r#type.name == "Stream" {
            setParametersValue(
                &mut parameters,
                &ParameterPath::Single(String::from("body")),
                GeneratedValue::Stream(req.getBodyStream()),
            );
        } else {
            let body = readRequestIntoText(req).await;
            logger.debug(
                &format!(
                    "deserialize(): Raw request body string is (removed all empty characters) {}",
                    body.replace(char::is_whitespace, "")
                ),
                context.contextId().as_deref(),
            );
            req.setBody(Some(body.clone()));
            let contentType = req
                .getHeader("content-type")
                .unwrap_or_default()
                .to_ascii_lowercase();
            let parsed = if contentType.contains("json") {
                serde_json::from_str::<serde_json::Value>(&body).unwrap_or(serde_json::Value::Null)
            } else {
                parseXML(&body, false).unwrap_or(serde_json::Value::Null)
            };
            let generated = GeneratedValue::from(parsed);
            setParametersValue(&mut parameters, &bodyParameter.parameterPath, generated);
            setParametersValue(
                &mut parameters,
                &ParameterPath::Single(String::from("body")),
                GeneratedValue::String(body),
            );
        }
    }

    Ok(parameters)
}

pub async fn serialize<R: IResponse, L: ILogger + ?Sized>(
    context: &Context,
    res: &mut R,
    spec: &OperationSpec,
    handlerResponse: &GeneratedResponse,
    logger: &L,
) -> crate::generated::GeneratedResult<()> {
    res.setStatusCode(handlerResponse.statusCode);
    if let Some(statusMessage) = &handlerResponse.statusMessage {
        res.setStatusMessage(statusMessage.clone());
    }

    let responseSpec = resolve_response_spec(spec, handlerResponse.statusCode)?;

    if let Some(headersMapper) = &responseSpec.headersMapper {
        for (key, mapper) in &headersMapper.r#type.modelProperties {
            if let Some(value) = handlerResponse.get_field(key) {
                if let Some(prefix) = &mapper.headerCollectionPrefix {
                    if let Some(object) = value.as_object() {
                        for (headerKey, headerValue) in object {
                            if let Some(serialized) = serialize_header_value(headerValue) {
                                res.setHeader(
                                    &format!("{}{}", prefix, headerKey),
                                    Some(serialized),
                                );
                            }
                        }
                    }
                } else if let Some(serializedName) = &mapper.serializedName {
                    if let Some(serialized) = serialize_header_value(value) {
                        res.setHeader(serializedName, Some(serialized));
                    }
                }
            }
        }
    }

    if let Some(bodyMapper) = &responseSpec.bodyMapper {
        if bodyMapper.r#type.name == "Stream" {
            if let Some(GeneratedBody::Stream(stream)) = &handlerResponse.body {
                res.getBodyStream().write_bytes(&stream.read_to_vec());
            }
        } else {
            let body = handlerResponse.body_value().to_json_value();
            if spec.isXML {
                let xmlBody = stringifyXML(
                    &body,
                    bodyMapper
                        .xmlName
                        .as_deref()
                        .or(bodyMapper.serializedName.as_deref()),
                )?;
                res.setContentType(Some(String::from("application/xml")));
                res.getBodyStream().write_text(&xmlBody);
                logger.debug(
                    &format!("Serializer: Raw response body string is {}", xmlBody),
                    context.contextId().as_deref(),
                );
            } else {
                if res.getHeader("content-type").is_none() {
                    res.setContentType(Some(String::from("application/json")));
                }
                let jsonBody = serde_json::to_string(&body)?;
                res.getBodyStream().write_text(&jsonBody);
                logger.debug(
                    &format!("Serializer: Raw response body string is {}", jsonBody),
                    context.contextId().as_deref(),
                );
            }
        }
        logger.info(
            "Serializer: Start returning stream body.",
            context.contextId().as_deref(),
        );
    }

    Ok(())
}

async fn readRequestIntoText<R: IRequest>(req: &R) -> String {
    req.getBodyStream().read_to_string()
}

fn resolve_response_spec<'a>(
    spec: &'a OperationSpec,
    statusCode: u16,
) -> crate::generated::GeneratedResult<&'a ResponseSpec> {
    spec.responses.get(&statusCode.to_string()).ok_or_else(|| {
        format!(
            "Request specification doesn't include provided response status code {}",
            statusCode
        )
        .into()
    })
}

fn deserialize_header_value(value: RequestHeaderValue) -> GeneratedValue {
    match value {
        RequestHeaderValue::Single(value) => GeneratedValue::String(value),
        RequestHeaderValue::Multi(values) => {
            GeneratedValue::Array(values.into_iter().map(GeneratedValue::String).collect())
        }
    }
}

fn deserialize_primitive(mapper: &Mapper, value: Option<GeneratedValue>) -> GeneratedValue {
    let value = if let Some(value) = value {
        value
    } else if let Some(defaultValue) = &mapper.defaultValue {
        GeneratedValue::from(defaultValue.clone())
    } else {
        GeneratedValue::Null
    };
    match mapper.r#type.name.as_str() {
        "Boolean" => value
            .as_bool()
            .map(GeneratedValue::Bool)
            .unwrap_or(GeneratedValue::Null),
        "Number" => value
            .as_number()
            .map(GeneratedValue::Number)
            .unwrap_or(GeneratedValue::Null),
        "Sequence" => match value {
            GeneratedValue::Array(values) => GeneratedValue::Array(values),
            GeneratedValue::String(value) => GeneratedValue::Array(
                value
                    .split(',')
                    .map(|item| GeneratedValue::String(item.to_owned()))
                    .collect(),
            ),
            other => other,
        },
        _ => value,
    }
}

fn serialize_header_value(value: &GeneratedValue) -> Option<ResponseHeaderValue> {
    match value {
        GeneratedValue::Null => None,
        GeneratedValue::Bool(value) => Some(ResponseHeaderValue::from(*value)),
        GeneratedValue::Number(value) => Some(ResponseHeaderValue::from(*value)),
        GeneratedValue::String(value) => Some(ResponseHeaderValue::from(value.clone())),
        GeneratedValue::Array(values) => Some(ResponseHeaderValue::Multi(
            values
                .iter()
                .filter_map(GeneratedValue::as_string)
                .collect(),
        )),
        GeneratedValue::Object(_) => None,
        GeneratedValue::Stream(stream) => Some(ResponseHeaderValue::from(stream.read_to_string())),
    }
}

pub fn setParametersValue(
    parameters: &mut GeneratedObject,
    parameterPath: &ParameterPath,
    parameterValue: GeneratedValue,
) {
    match parameterPath {
        ParameterPath::Single(parameterPath) => {
            parameters.insert(parameterPath.clone(), parameterValue);
        }
        ParameterPath::Many(parameterPath) => {
            set_nested_value(parameters, parameterPath, parameterValue);
        }
        ParameterPath::Map(_) => {}
    }
}

fn set_nested_value(
    parameters: &mut GeneratedObject,
    parameterPath: &[String],
    parameterValue: GeneratedValue,
) {
    if let Some((first, rest)) = parameterPath.split_first() {
        if rest.is_empty() {
            parameters.insert(first.clone(), parameterValue);
            return;
        }
        let entry = parameters
            .entry(first.clone())
            .or_insert_with(|| GeneratedValue::Object(BTreeMap::new()));
        if let GeneratedValue::Object(child) = entry {
            set_nested_value(child, rest, parameterValue);
        }
    }
}
