use std::collections::BTreeMap;

use chrono::DateTime;

use crate::generated::artifacts::mappers::{get_mapper, Mapper};
use crate::generated::artifacts::models::{
    GeneratedBody, GeneratedObject, GeneratedResponse, GeneratedValue,
};
use crate::generated::artifacts::parameters::ParameterPath;
use crate::generated::artifacts::specifications::{OperationSpec, ResponseSpec};
use crate::generated::context::{Context, IHandlerParameters};
use crate::generated::errors::deserialization_error::DeserializationError;
use crate::generated::i_request::{IRequest, RequestHeaderValue};
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
            let mut parsed = if contentType.contains("json") {
                serde_json::from_str::<serde_json::Value>(&body).unwrap_or(serde_json::Value::Null)
            } else {
                parseXML(&body, false).unwrap_or(serde_json::Value::Null)
            };
            if spec.isXML && bodyParameter.mapper.r#type.name == "Sequence" {
                if let Some(element_name) = &bodyParameter.mapper.xmlElementName {
                    if let serde_json::Value::Object(values) = &parsed {
                        if let Some(value) = values.get(element_name) {
                            parsed = value.clone();
                        }
                    }
                }
            }
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
                    let value = if mapper.r#type.name == "DateTimeRfc1123" {
                        &ensure_rfc1123(value)
                    } else {
                        value
                    };
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
                let xmlReadyBody = apply_model_mapping(&body, bodyMapper);
                let xmlReadyBody =
                    if bodyMapper.r#type.name == "Sequence" && xmlReadyBody.is_array() {
                        if let Some(element_name) = bodyMapper.xmlElementName.as_deref() {
                            let mut wrapper = serde_json::Map::new();
                            wrapper.insert(element_name.to_string(), xmlReadyBody);
                            serde_json::Value::Object(wrapper)
                        } else {
                            xmlReadyBody
                        }
                    } else {
                        xmlReadyBody
                    };
                let xmlBody = stringifyXML(
                    &xmlReadyBody,
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

fn resolve_response_spec(
    spec: &OperationSpec,
    statusCode: u16,
) -> crate::generated::GeneratedResult<&ResponseSpec> {
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

fn apply_model_mapping(value: &serde_json::Value, mapper: &Mapper) -> serde_json::Value {
    match value {
        serde_json::Value::Object(object) => {
            if let Some(properties) = resolve_model_properties(mapper) {
                let mut result = serde_json::Map::new();
                for (key, value) in object {
                    if let Some(property_mapper) = properties.get(key) {
                        let is_unwrapped_sequence = property_mapper.r#type.name == "Sequence"
                            && property_mapper.xmlElementName.is_some()
                            && !property_mapper.xmlIsWrapped;
                        if is_unwrapped_sequence {
                            let element_name =
                                property_mapper.xmlElementName.as_ref().unwrap().clone();
                            let mapped_array = map_sequence_elements(value, property_mapper);
                            result.insert(element_name, mapped_array);
                        } else {
                            let mapped_key = mapped_name(property_mapper).unwrap_or(key).to_owned();
                            let mapped_value = apply_model_mapping(value, property_mapper);
                            result.insert(mapped_key, mapped_value);
                        }
                    } else if resolve_additional_properties(mapper).is_some() {
                        result.insert(key.clone(), value.clone());
                    }
                }
                serde_json::Value::Object(result)
            } else {
                serde_json::Value::Object(object.clone())
            }
        }
        serde_json::Value::Array(values) => {
            let mapped_values = if let Some(element_mapper) = mapper.r#type.element.as_deref() {
                values
                    .iter()
                    .map(|value| apply_model_mapping(value, element_mapper))
                    .collect()
            } else {
                values.clone()
            };
            let mapped_array = serde_json::Value::Array(mapped_values);
            if mapper.xmlIsWrapped {
                if let Some(element_name) = mapper.xmlElementName.as_ref() {
                    let mut wrapped = serde_json::Map::new();
                    wrapped.insert(element_name.clone(), mapped_array);
                    serde_json::Value::Object(wrapped)
                } else {
                    mapped_array
                }
            } else {
                mapped_array
            }
        }
        _ => value.clone(),
    }
}

fn map_sequence_elements(value: &serde_json::Value, mapper: &Mapper) -> serde_json::Value {
    match value {
        serde_json::Value::Array(values) => {
            let mapped = if let Some(element_mapper) = mapper.r#type.element.as_deref() {
                values
                    .iter()
                    .map(|v| apply_model_mapping(v, element_mapper))
                    .collect()
            } else {
                values.clone()
            };
            serde_json::Value::Array(mapped)
        }
        _ => value.clone(),
    }
}

fn resolve_model_properties(mapper: &Mapper) -> Option<&BTreeMap<String, Mapper>> {
    if !mapper.r#type.modelProperties.is_empty() {
        Some(&mapper.r#type.modelProperties)
    } else {
        mapper
            .r#type
            .className
            .as_deref()
            .and_then(get_mapper)
            .map(|resolved| &resolved.r#type.modelProperties)
            .filter(|properties| !properties.is_empty())
    }
}

fn resolve_additional_properties(_mapper: &Mapper) -> Option<&Mapper> {
    // Queue mappers don't use additionalProperties
    None
}

fn mapped_name(mapper: &Mapper) -> Option<&str> {
    mapper
        .xmlName
        .as_deref()
        .or(mapper.serializedName.as_deref())
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

/// Safety net: if a date string is in ISO 8601 format but the mapper expects
/// DateTimeRfc1123, convert it. Mirrors the TypeScript serializer behavior.
fn ensure_rfc1123(value: &GeneratedValue) -> GeneratedValue {
    if let GeneratedValue::String(s) = value {
        if s.contains(',') {
            return value.clone();
        }
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            let utc = dt.with_timezone(&chrono::Utc);
            return GeneratedValue::String(utc.format("%a, %d %b %Y %H:%M:%S GMT").to_string());
        }
        if let Ok(dt) = s.parse::<DateTime<chrono::FixedOffset>>() {
            let utc = dt.with_timezone(&chrono::Utc);
            return GeneratedValue::String(utc.format("%a, %d %b %Y %H:%M:%S GMT").to_string());
        }
    }
    value.clone()
}
