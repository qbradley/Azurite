use std::collections::HashMap;

use azurite_common::telemetry::{
    AzuriteTelemetryClient, TelemetryContext, TelemetryRequest, TelemetryResponse,
    TelemetryServiceType,
};

use crate::generated::{
    context::Context, i_request::RequestHeaderValue, i_response::ResponseHeaderValue,
};

#[allow(non_snake_case)]
pub fn telemetryMiddleware(context: &Context) {
    AzuriteTelemetryClient::TraceRequest(build_telemetry_context(context));
}

#[derive(Debug, Clone)]
pub struct TelemetryMiddleware {
    contextPath: String,
}

impl TelemetryMiddleware {
    pub fn apply(&self, context: &Context) {
        let _ = &self.contextPath;
        telemetryMiddleware(context);
    }
}

#[derive(Debug, Clone)]
pub struct TelemetryMiddlewareFactory {
    contextPath: String,
}

impl TelemetryMiddlewareFactory {
    pub fn new(contextPath: impl Into<String>) -> Self {
        Self {
            contextPath: contextPath.into(),
        }
    }

    #[allow(non_snake_case)]
    pub fn createTelemetryMiddleware(&self) -> TelemetryMiddleware {
        TelemetryMiddleware {
            contextPath: self.contextPath.clone(),
        }
    }
}

fn build_telemetry_context(context: &Context) -> TelemetryContext {
    TelemetryContext {
        serviceType: TelemetryServiceType::Blob,
        operation: context.operation().map(|operation| operation.to_string()),
        request: context.request().map(|request| TelemetryRequest {
            headers: request
                .headers
                .iter()
                .map(|(key, value)| (key.clone(), request_header_value_to_string(value)))
                .collect::<HashMap<_, _>>(),
            queries: request
                .query
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            method: request.method.to_string(),
            endpoint: request.endpoint,
        }),
        response: context.response().map(|response| TelemetryResponse {
            headers: response
                .headers
                .iter()
                .map(|(key, value)| (key.clone(), response_header_value_to_string(value)))
                .collect::<HashMap<_, _>>(),
            statusCode: Some(response.statusCode),
        }),
        startTime: context.startTime(),
        contextId: context.contextId(),
        contextID: context.contextId(),
    }
}

fn request_header_value_to_string(value: &RequestHeaderValue) -> String {
    match value {
        RequestHeaderValue::Single(value) => value.clone(),
        RequestHeaderValue::Multi(values) => values.join(","),
    }
}

fn response_header_value_to_string(value: &ResponseHeaderValue) -> String {
    match value {
        ResponseHeaderValue::Single(value) => value.clone(),
        ResponseHeaderValue::Multi(values) => values.join(","),
    }
}
