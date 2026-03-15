use std::{collections::BTreeMap, sync::Arc};

use azurite_common::i_logger::ILogger;

use crate::{
    context::QueueStorageContext,
    errors::{StorageError, StorageErrorFactory},
    generated::{
        artifacts::{
            models::{GeneratedObject, GeneratedValue},
            specifications::specification,
        },
        errors::MiddlewareError,
        i_request::{GeneratedHttpRequest, IRequest},
        i_response::{GeneratedHttpResponse, IResponse, ResponseHeaderValue},
    },
    persistence::{IQueueMetadataStore, ServicePropertiesModel},
    utils::constants::{HeaderConstants, MethodConstants},
};

pub type BoxedMiddlewareError = Box<dyn std::error::Error + Send + Sync>;
pub type SharedPreflightLogger = Arc<dyn ILogger + Send + Sync>;
pub type SharedQueueMetadataStore = Arc<dyn IQueueMetadataStore + Send + Sync>;

#[derive(Clone)]
pub struct OptionsHandlerMiddleware {
    factory: PreflightMiddlewareFactory,
    metadataStore: SharedQueueMetadataStore,
}

impl OptionsHandlerMiddleware {
    pub async fn apply(
        &self,
        err: BoxedMiddlewareError,
        context: &QueueStorageContext,
        req: &GeneratedHttpRequest,
        res: &mut GeneratedHttpResponse,
    ) -> Option<BoxedMiddlewareError> {
        self.factory
            .apply_options(err, Arc::clone(&self.metadataStore), context, req, res)
            .await
    }
}

#[derive(Clone)]
pub struct CorsRequestMiddleware {
    factory: PreflightMiddlewareFactory,
    metadataStore: SharedQueueMetadataStore,
    blockErrorRequest: bool,
}

impl CorsRequestMiddleware {
    pub async fn apply(
        &self,
        err: Option<BoxedMiddlewareError>,
        context: &QueueStorageContext,
        req: &GeneratedHttpRequest,
        res: &mut GeneratedHttpResponse,
    ) -> Option<BoxedMiddlewareError> {
        self.factory
            .apply_cors_request(
                err,
                Arc::clone(&self.metadataStore),
                context,
                req,
                res,
                self.blockErrorRequest,
            )
            .await
    }
}

#[derive(Clone)]
pub struct PreflightMiddlewareFactory {
    logger: SharedPreflightLogger,
}

impl PreflightMiddlewareFactory {
    pub fn new(logger: SharedPreflightLogger) -> Self {
        Self { logger }
    }

    #[allow(non_snake_case)]
    pub fn createOptionsHandlerMiddleware(
        &self,
        metadataStore: SharedQueueMetadataStore,
    ) -> OptionsHandlerMiddleware {
        OptionsHandlerMiddleware {
            factory: self.clone(),
            metadataStore,
        }
    }

    #[allow(non_snake_case)]
    pub fn createCorsRequestMiddleware(
        &self,
        metadataStore: SharedQueueMetadataStore,
        blockErrorRequest: bool,
    ) -> CorsRequestMiddleware {
        CorsRequestMiddleware {
            factory: self.clone(),
            metadataStore,
            blockErrorRequest,
        }
    }

    async fn apply_options(
        &self,
        err: BoxedMiddlewareError,
        metadataStore: SharedQueueMetadataStore,
        context: &QueueStorageContext,
        req: &GeneratedHttpRequest,
        res: &mut GeneratedHttpResponse,
    ) -> Option<BoxedMiddlewareError> {
        if req.getMethod().to_string() != MethodConstants.OPTIONS {
            return Some(err);
        }

        let requestId = context.contextId().unwrap_or_default();
        let account = match context.account().filter(|value| !value.is_empty()) {
            Some(account) => account,
            None => return Some(err),
        };

        self.logger.info(
            "PreflightMiddlewareFactory.createOptionsHandlerMiddleware(): OPTIONS request.",
            Some(&requestId),
        );

        let origin = match req.getHeader(HeaderConstants.ORIGIN) {
            Some(origin) if !origin.is_empty() => origin,
            other => {
                return Some(Box::new(StorageErrorFactory::getInvalidCorsHeaderValue(
                    Some(&requestId),
                    Some(message_details(&format!(
                        "Invalid required CORS header Origin {:?}",
                        other
                    ))),
                )));
            }
        };

        let requestMethod = match req.getHeader(HeaderConstants.ACCESS_CONTROL_REQUEST_METHOD) {
            Some(method) if !method.is_empty() => method,
            other => {
                return Some(Box::new(StorageErrorFactory::getInvalidCorsHeaderValue(
                    Some(&requestId),
                    Some(message_details(&format!(
                        "Invalid required CORS header Access-Control-Request-Method {:?}",
                        other
                    ))),
                )));
            }
        };

        let requestHeaders = req.getHeader(HeaderConstants.ACCESS_CONTROL_REQUEST_HEADERS);

        let properties = match metadataStore.getServiceProperties(&account).await {
            Ok(properties) => properties,
            Err(error) => return Some(Box::new(error)),
        };

        let Some(properties) = properties else {
            return Some(Box::new(StorageErrorFactory::corsPreflightFailure(
                Some(&requestId),
                Some(message_details("No CORS rules matches this request")),
            )));
        };

        for cors in cors_rules(&properties) {
            let allowedOrigins = field_string(&cors, "allowedOrigins").unwrap_or_default();
            let allowedMethods = field_string(&cors, "allowedMethods").unwrap_or_default();
            if !self.checkOrigin(Some(&origin), &allowedOrigins)
                || !self.checkMethod(&requestMethod, &allowedMethods)
            {
                continue;
            }

            if let Some(headers) = requestHeaders.as_deref() {
                let allowedHeaders = field_string(&cors, "allowedHeaders").unwrap_or_default();
                if !self.checkHeaders(headers, &allowedHeaders) {
                    continue;
                }
            }

            res.setStatusCode(200);
            res.setHeader(
                HeaderConstants.ACCESS_CONTROL_ALLOW_ORIGIN,
                Some(ResponseHeaderValue::from(origin.clone())),
            );
            res.setHeader(
                HeaderConstants.ACCESS_CONTROL_ALLOW_METHODS,
                Some(ResponseHeaderValue::from(requestMethod.clone())),
            );
            if let Some(headers) = requestHeaders {
                res.setHeader(
                    HeaderConstants.ACCESS_CONTROL_ALLOW_HEADERS,
                    Some(ResponseHeaderValue::from(headers)),
                );
            }
            if let Some(maxAge) = field_string(&cors, "maxAgeInSeconds") {
                res.setHeader(
                    HeaderConstants.ACCESS_CONTROL_MAX_AGE,
                    Some(ResponseHeaderValue::from(maxAge)),
                );
            }
            res.setHeader(
                HeaderConstants.ACCESS_CONTROL_ALLOW_CREDENTIALS,
                Some(ResponseHeaderValue::from("true")),
            );
            return None;
        }

        Some(Box::new(StorageErrorFactory::corsPreflightFailure(
            Some(&requestId),
            Some(message_details("No CORS rules matches this request")),
        )))
    }

    async fn apply_cors_request(
        &self,
        err: Option<BoxedMiddlewareError>,
        metadataStore: SharedQueueMetadataStore,
        context: &QueueStorageContext,
        req: &GeneratedHttpRequest,
        res: &mut GeneratedHttpResponse,
        blockErrorRequest: bool,
    ) -> Option<BoxedMiddlewareError> {
        if req.getMethod().to_string() == MethodConstants.OPTIONS {
            return err;
        }

        if blockErrorRequest && err.is_none() {
            return None;
        }

        let account = match context.account().filter(|value| !value.is_empty()) {
            Some(account) => account,
            None => return err,
        };
        let Some(origin) = req.getHeader(HeaderConstants.ORIGIN) else {
            return err;
        };
        let method = req.getMethod().to_string();

        let properties = match metadataStore.getServiceProperties(&account).await {
            Ok(properties) => properties,
            Err(error) => return Some(Box::new(error)),
        };
        let Some(properties) = properties else {
            return err;
        };

        let corsSet = cors_rules(&properties);
        let responseHeaders = self.getResponseHeaders(res, context, err.as_deref());

        for cors in &corsSet {
            let allowedOrigins = field_string(cors, "allowedOrigins").unwrap_or_default();
            let allowedMethods = field_string(cors, "allowedMethods").unwrap_or_default();
            if self.checkOrigin(Some(&origin), &allowedOrigins)
                && self.checkMethod(&method, &allowedMethods)
            {
                let exposedHeaders = self.getExposedHeaders(
                    &responseHeaders,
                    &field_string(cors, "exposedHeaders").unwrap_or_default(),
                );
                res.setHeader(
                    HeaderConstants.ACCESS_CONTROL_EXPOSE_HEADERS,
                    Some(ResponseHeaderValue::from(exposedHeaders)),
                );
                res.setHeader(
                    HeaderConstants.ACCESS_CONTROL_ALLOW_ORIGIN,
                    Some(ResponseHeaderValue::from(if allowedOrigins.trim() == "*" {
                        "*".to_string()
                    } else {
                        origin.clone()
                    })),
                );
                if allowedOrigins.trim() != "*" {
                    res.setHeader(
                        HeaderConstants.VARY,
                        Some(ResponseHeaderValue::from("Origin")),
                    );
                    res.setHeader(
                        HeaderConstants.ACCESS_CONTROL_ALLOW_CREDENTIALS,
                        Some(ResponseHeaderValue::from("true")),
                    );
                }
                return err;
            }
        }

        if !corsSet.is_empty() {
            res.setHeader(
                HeaderConstants.VARY,
                Some(ResponseHeaderValue::from("Origin")),
            );
        }
        err
    }

    fn checkOrigin(&self, origin: Option<&str>, allowedOrigin: &str) -> bool {
        if allowedOrigin.trim() == "*" {
            return true;
        }
        let Some(origin) = origin else {
            return false;
        };

        allowedOrigin
            .split(',')
            .any(|allowed| origin.trim().eq_ignore_ascii_case(allowed.trim()))
    }

    fn checkMethod(&self, method: &str, allowedMethod: &str) -> bool {
        allowedMethod
            .split(',')
            .any(|allowed| method.trim().eq_ignore_ascii_case(allowed.trim()))
    }

    fn checkHeaders(&self, headers: &str, allowedHeaders: &str) -> bool {
        let allowedHeadersArray = allowedHeaders.split(',').collect::<Vec<_>>();
        for header in headers.split(',') {
            let trimmedHeader = header.trim().to_ascii_lowercase();
            let mut matched = false;
            for allowedHeader in &allowedHeadersArray {
                let trimmedAllowedHeader = allowedHeader.trim().to_ascii_lowercase();
                if trimmedHeader == trimmedAllowedHeader
                    || (trimmedAllowedHeader.ends_with('*')
                        && trimmedHeader.starts_with(
                            trimmedAllowedHeader[..trimmedAllowedHeader.len().saturating_sub(1)]
                                .trim_end(),
                        ))
                {
                    matched = true;
                    break;
                }
            }
            if !matched {
                return false;
            }
        }
        true
    }

    fn getResponseHeaders(
        &self,
        res: &GeneratedHttpResponse,
        context: &QueueStorageContext,
        err: Option<&(dyn std::error::Error + Send + Sync + 'static)>,
    ) -> Vec<String> {
        let mut responseHeaderSet = Vec::new();

        if let (Some(handlerResponse), Some(operation)) =
            (context.handlerResponses(), context.operation())
        {
            if let Some(spec) = specification(operation) {
                if let Some(responseSpec) =
                    spec.responses.get(&handlerResponse.statusCode.to_string())
                {
                    if let Some(headersMapper) = &responseSpec.headersMapper {
                        for (key, mapper) in &headersMapper.r#type.modelProperties {
                            if let Some(value) = handlerResponse.get_field(key) {
                                if let Some(prefix) = &mapper.headerCollectionPrefix {
                                    if let Some(object) = value.as_object() {
                                        for headerKey in object.keys() {
                                            responseHeaderSet
                                                .push(format!("{}{}", prefix, headerKey));
                                        }
                                    }
                                } else if let Some(serializedName) = &mapper.serializedName {
                                    responseHeaderSet.push(serializedName.clone());
                                }
                            }
                        }
                    }

                    if spec.isXML
                        && responseSpec
                            .bodyMapper
                            .as_ref()
                            .map(|mapper| mapper.r#type.name != "Stream")
                            .unwrap_or(false)
                    {
                        responseHeaderSet.push("content-type".to_string());
                        responseHeaderSet.push("content-length".to_string());
                    } else if handlerResponse.body.is_some()
                        && responseSpec
                            .bodyMapper
                            .as_ref()
                            .map(|mapper| mapper.r#type.name == "Stream")
                            .unwrap_or(false)
                    {
                        responseHeaderSet.push("content-length".to_string());
                    }
                }
            }
        }

        responseHeaderSet.extend(res.getHeaders().keys().cloned());

        if let Some(storageError) = err.and_then(|value| value.downcast_ref::<StorageError>()) {
            if let Some(headers) = &storageError.headers {
                responseHeaderSet.extend(headers.keys().cloned());
            }
        }
        if let Some(middlewareError) = err.and_then(|value| value.downcast_ref::<MiddlewareError>())
        {
            if let Some(headers) = &middlewareError.headers {
                responseHeaderSet.extend(headers.keys().cloned());
            }
        }

        responseHeaderSet.push("Date".to_string());
        responseHeaderSet.push("Connection".to_string());
        responseHeaderSet.push("Transfer-Encoding".to_string());
        responseHeaderSet.sort();
        responseHeaderSet.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
        responseHeaderSet
    }

    fn getExposedHeaders(&self, responseHeaders: &[String], exposedHeaders: &str) -> String {
        let mut prefixRules = Vec::new();
        let mut simpleHeaders = Vec::new();
        for rule in exposedHeaders
            .split(',')
            .map(str::trim)
            .filter(|rule| !rule.is_empty())
        {
            if let Some(prefix) = rule.strip_suffix('*') {
                prefixRules.push(prefix.to_ascii_lowercase());
            } else {
                simpleHeaders.push(rule.to_string());
            }
        }

        let mut result = Vec::new();
        for header in responseHeaders {
            let headerLower = header.to_ascii_lowercase();
            let mut isMatch = prefixRules
                .iter()
                .any(|rule| headerLower.starts_with(rule.as_str()));
            if !isMatch {
                isMatch = simpleHeaders
                    .iter()
                    .any(|rule| headerLower.eq_ignore_ascii_case(rule));
            }
            if isMatch {
                result.push(header.clone());
            }
        }

        for header in simpleHeaders {
            if !result
                .iter()
                .any(|value| value.eq_ignore_ascii_case(&header))
            {
                result.push(header);
            }
        }

        result.join(",")
    }
}

fn cors_rules(properties: &ServicePropertiesModel) -> Vec<GeneratedObject> {
    let cors_val = properties
        .properties
        .get("cors")
        .or_else(|| properties.properties.get("Cors"));

    let Some(cors_val) = cors_val else {
        return Vec::new();
    };

    match cors_val {
        GeneratedValue::Array(values) => values
            .iter()
            .filter_map(|value| value.as_object().cloned())
            .collect(),
        GeneratedValue::Object(obj) => {
            let inner = obj.get("CorsRule").or_else(|| obj.get("corsRule"));
            match inner {
                Some(GeneratedValue::Array(arr)) => {
                    arr.iter().filter_map(|v| v.as_object().cloned()).collect()
                }
                Some(GeneratedValue::Object(single)) => vec![single.clone()],
                _ => Vec::new(),
            }
        }
        _ => Vec::new(),
    }
}

fn field_string(value: &GeneratedObject, key: &str) -> Option<String> {
    value.get(key).and_then(GeneratedValue::as_string)
}

fn message_details(message: &str) -> BTreeMap<String, String> {
    BTreeMap::from([(String::from("MessageDetails"), message.to_string())])
}
