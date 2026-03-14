use std::{collections::BTreeMap, sync::Arc};

use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{request::Parts, HeaderName, HeaderValue, Method, Request, Response, StatusCode},
    routing::any,
    Router,
};
use chrono::Utc;
use url::form_urlencoded;

use azurite_common::{
    configuration_base::AccessLogWriteStream, i_account_data_store::IAccountDataStore,
    i_logger::ILogger, i_request_listener_factory::IRequestListenerFactory,
    logger::logger as global_logger, models::OAuthLevel, server_base::RequestListener,
};

use crate::{
    authentication::{
        AccountSASAuthenticator, BlobSASAuthenticator, BlobSharedKeyAuthenticator,
        BlobTokenAuthenticator, PublicAccessAuthenticator,
    },
    context::BlobStorageContext,
    generated::{
        context::Context,
        handlers::{
            IAppendBlobHandler, IBlobHandler, IBlockBlobHandler, IContainerHandler, IHandlers,
            IPageBlobHandler, IServiceHandler,
        },
        i_request::{
            GeneratedHttpRequest, GeneratedReadableStream, HttpMethod, IRequest, RequestHeaderValue,
        },
        i_response::{GeneratedHttpResponse, IResponse, ResponseHeaderValue},
        middleware::{
            deserializer::deserializer_middleware, dispatch::dispatch_middleware,
            end::end_middleware, error::error_middleware,
            handler_middleware_factory::HandlerMiddlewareFactory,
            serializer::serializer_middleware,
        },
    },
    handlers::{
        append_blob_handler::AppendBlobHandler,
        base_handler::{BaseHandler, SharedBlobMetadataStore, SharedExtentStore, SharedLogger},
        blob_handler::BlobHandler,
        block_blob_handler::BlockBlobHandler,
        container_handler::ContainerHandler,
        page_blob_handler::PageBlobHandler,
        page_blob_ranges_manager::PageBlobRangesManager,
        service_handler::ServiceHandler,
    },
    middlewares::{
        createStorageBlobContextMiddleware, internalBlobStorageContextMiddleware,
        AuthenticationMiddleware, AuthenticationMiddlewareFactory,
        BlobStorageContextMiddlewareOptions, BoxedMiddlewareError, CorsRequestMiddleware,
        OptionsHandlerMiddleware, PreflightMiddlewareFactory, SharedAuthenticator,
        StrictModelMiddleware, StrictModelMiddlewareFactory, TelemetryMiddleware,
        TelemetryMiddlewareFactory, UnsupportedHeadersBlocker, UnsupportedParametersBlocker,
    },
    utils::constants::DEFAULT_CONTEXT_PATH,
};

#[derive(Default)]
struct SharedGlobalLogger;

impl ILogger for SharedGlobalLogger {
    fn error(&self, message: &str, contextID: Option<&str>) {
        global_logger.error(message, contextID);
    }

    fn warn(&self, message: &str, contextID: Option<&str>) {
        global_logger.warn(message, contextID);
    }

    fn info(&self, message: &str, contextID: Option<&str>) {
        global_logger.info(message, contextID);
    }

    fn verbose(&self, message: &str, contextID: Option<&str>) {
        global_logger.verbose(message, contextID);
    }

    fn debug(&self, message: &str, contextID: Option<&str>) {
        global_logger.debug(message, contextID);
    }
}

#[derive(Clone)]
struct RequestListenerHandlers {
    service: Arc<ServiceHandler>,
    container: Arc<ContainerHandler>,
    blob: Arc<BlobHandler>,
    page: Arc<PageBlobHandler>,
    append: Arc<AppendBlobHandler>,
    block: Arc<BlockBlobHandler>,
}

impl IHandlers for RequestListenerHandlers {
    fn serviceHandler(&self) -> &(dyn IServiceHandler + Send + Sync) {
        self.service.as_ref()
    }

    fn containerHandler(&self) -> &(dyn IContainerHandler + Send + Sync) {
        self.container.as_ref()
    }

    fn blobHandler(&self) -> &(dyn IBlobHandler + Send + Sync) {
        self.blob.as_ref()
    }

    fn pageBlobHandler(&self) -> &(dyn IPageBlobHandler + Send + Sync) {
        self.page.as_ref()
    }

    fn appendBlobHandler(&self) -> &(dyn IAppendBlobHandler + Send + Sync) {
        self.append.as_ref()
    }

    fn blockBlobHandler(&self) -> &(dyn IBlockBlobHandler + Send + Sync) {
        self.block.as_ref()
    }
}

#[derive(Clone)]
struct RequestListenerState {
    enableAccessLog: bool,
    accessLogWriteStream: Option<AccessLogWriteStream>,
    logger: SharedLogger,
    handlers: Arc<RequestListenerHandlers>,
    blobStorageContextOptions: BlobStorageContextMiddlewareOptions,
    authenticationMiddleware: AuthenticationMiddleware,
    strictModelMiddleware: Option<StrictModelMiddleware>,
    optionsHandlerMiddleware: OptionsHandlerMiddleware,
    corsErrorRequestMiddleware: CorsRequestMiddleware,
    corsRequestMiddleware: CorsRequestMiddleware,
    telemetryMiddleware: TelemetryMiddleware,
}

impl RequestListenerState {
    async fn handle_request(&self, request: Request<Body>) -> Response<Body> {
        let (parts, body) = request.into_parts();
        let body_bytes = match to_bytes(body, usize::MAX).await {
            Ok(body_bytes) => body_bytes,
            Err(error) => {
                return simple_text_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Failed to read request body: {error}"),
                );
            }
        };

        let mut generated_request = build_generated_request(&parts, &body_bytes);
        let mut generated_response = GeneratedHttpResponse::default();
        let holder = Context::new_holder();
        let context = Context::from_holder(
            holder,
            DEFAULT_CONTEXT_PATH,
            Some(generated_request.clone()),
            Some(generated_response.clone()),
        );
        let request_host = request_host(&parts);
        let request_path = generated_request.getPath();

        let mut pending_error: Option<BoxedMiddlewareError> = internalBlobStorageContextMiddleware(
            &context,
            &generated_request,
            &mut generated_response,
            &request_host,
            &request_path,
            self.logger.as_ref(),
            self.blobStorageContextOptions.skipApiVersionCheck,
            self.blobStorageContextOptions.disableProductStyleUrl,
            self.blobStorageContextOptions.loose,
        )
        .err()
        .map(|error| Box::new(error) as BoxedMiddlewareError);

        let blob_context = BlobStorageContext::new(&context);

        if pending_error.is_none() {
            if let Err(error) =
                dispatch_middleware(&context, &generated_request, self.logger.as_ref())
            {
                pending_error = Some(error);
            }
        }

        if pending_error.is_none() {
            if let Some(strictModelMiddleware) = &self.strictModelMiddleware {
                if let Err(error) = strictModelMiddleware.apply(&generated_request, &context) {
                    pending_error = Some(Box::new(error));
                }
            }
        }

        if pending_error.is_none() {
            if let Err(error) = self
                .authenticationMiddleware
                .apply(&blob_context, &generated_request, &generated_response)
                .await
            {
                pending_error = Some(Box::new(error));
            }
        }

        if pending_error.is_none() {
            if let Err(error) =
                deserializer_middleware(&context, &mut generated_request, self.logger.as_ref())
                    .await
            {
                pending_error = Some(error);
            }
        }

        if pending_error.is_none() {
            let handlerMiddlewareFactory =
                HandlerMiddlewareFactory::new(Arc::clone(&self.handlers), Arc::clone(&self.logger));
            if let Err(error) = handlerMiddlewareFactory.call(&context).await {
                pending_error = Some(error);
            }
        }

        if let Some(error) = pending_error.take() {
            pending_error = self
                .corsErrorRequestMiddleware
                .apply(
                    Some(error),
                    &blob_context,
                    &generated_request,
                    &mut generated_response,
                )
                .await;
        }

        if pending_error.is_none() {
            pending_error = self
                .corsRequestMiddleware
                .apply(
                    None,
                    &blob_context,
                    &generated_request,
                    &mut generated_response,
                )
                .await;
        }

        if pending_error.is_none() {
            if let Err(error) =
                serializer_middleware(&context, &mut generated_response, self.logger.as_ref()).await
            {
                pending_error = Some(error);
            }
        }

        if let Some(error) = pending_error.take() {
            pending_error = self
                .optionsHandlerMiddleware
                .apply(
                    error,
                    &blob_context,
                    &generated_request,
                    &mut generated_response,
                )
                .await;
        }

        if let Some(error) = pending_error.as_ref() {
            let _ = error_middleware(
                &context,
                error.as_ref(),
                &generated_request,
                &mut generated_response,
                self.logger.as_ref(),
            );
        }

        context.setResponse(Some(generated_response.clone()));
        self.telemetryMiddleware.apply(&context);
        end_middleware(&context, &mut generated_response, self.logger.as_ref());

        let status_code = if generated_response.getStatusCode() == 0 {
            200
        } else {
            generated_response.getStatusCode()
        };
        if self.enableAccessLog {
            self.write_access_log(&generated_request, status_code);
        }

        generated_response_to_axum(generated_response)
    }

    fn write_access_log(&self, request: &GeneratedHttpRequest, status_code: u16) {
        let content_length = request.getBodyStream().read_to_vec().len();
        let line = format!(
            "- - [{}] \"{} {} HTTP/1.1\" {} {}\n",
            Utc::now().format("%d/%b/%Y:%H:%M:%S %z"),
            request.getMethod(),
            request.getPath(),
            status_code,
            content_length,
        );

        if let Some(stream) = &self.accessLogWriteStream {
            let mut guard = stream.lock().unwrap();
            let _ = guard.write_all(line.as_bytes());
            let _ = guard.flush();
        } else {
            self.logger.info(line.trim_end(), None);
        }
    }
}

#[derive(Clone)]
pub struct BlobRequestListenerFactory {
    metadataStore: SharedBlobMetadataStore,
    extentStore: SharedExtentStore,
    accountDataStore: Arc<dyn IAccountDataStore + Send + Sync>,
    enableAccessLog: bool,
    accessLogWriteStream: Option<AccessLogWriteStream>,
    loose: Option<bool>,
    skipApiVersionCheck: Option<bool>,
    oauth: Option<OAuthLevel>,
    disableProductStyleUrl: Option<bool>,
}

impl BlobRequestListenerFactory {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        metadataStore: SharedBlobMetadataStore,
        extentStore: SharedExtentStore,
        accountDataStore: Arc<dyn IAccountDataStore + Send + Sync>,
        enableAccessLog: bool,
        accessLogWriteStream: Option<AccessLogWriteStream>,
        loose: Option<bool>,
        skipApiVersionCheck: Option<bool>,
        oauth: Option<OAuthLevel>,
        disableProductStyleUrl: Option<bool>,
    ) -> Self {
        Self {
            metadataStore,
            extentStore,
            accountDataStore,
            enableAccessLog,
            accessLogWriteStream,
            loose,
            skipApiVersionCheck,
            oauth,
            disableProductStyleUrl,
        }
    }

    #[allow(non_snake_case)]
    pub fn createRequestListener(&self) -> RequestListener {
        let logger: SharedLogger = Arc::new(SharedGlobalLogger);
        let loose = self.loose.unwrap_or(false);
        let oauth = self.oauth.map(oauth_level_to_string);
        let baseHandler = BaseHandler::new(
            Arc::clone(&self.metadataStore),
            Arc::clone(&self.extentStore),
            Arc::clone(&logger),
            loose,
        );
        let pageBlobRangesManager = Arc::new(PageBlobRangesManager::new());
        let handlers = Arc::new(RequestListenerHandlers {
            append: Arc::new(AppendBlobHandler::new(baseHandler.clone())),
            blob: Arc::new(BlobHandler::new(
                baseHandler.clone(),
                pageBlobRangesManager.clone(),
            )),
            block: Arc::new(BlockBlobHandler::new(baseHandler.clone())),
            container: Arc::new(ContainerHandler::new(
                Arc::clone(&self.accountDataStore),
                oauth.clone(),
                baseHandler.clone(),
                self.disableProductStyleUrl,
            )),
            page: Arc::new(PageBlobHandler::new(
                baseHandler.clone(),
                pageBlobRangesManager,
            )),
            service: Arc::new(ServiceHandler::new(
                Arc::clone(&self.accountDataStore),
                oauth.clone(),
                baseHandler,
                self.disableProductStyleUrl,
            )),
        });

        let authenticationMiddlewareFactory =
            AuthenticationMiddlewareFactory::new(Arc::clone(&logger));
        let mut authenticators: Vec<SharedAuthenticator> = vec![
            Arc::new(PublicAccessAuthenticator::new(
                Arc::clone(&self.metadataStore),
                Arc::clone(&logger),
            )) as SharedAuthenticator,
            Arc::new(BlobSharedKeyAuthenticator::new(
                Arc::clone(&self.accountDataStore),
                Arc::clone(&logger),
            )) as SharedAuthenticator,
            Arc::new(AccountSASAuthenticator::new(
                Arc::clone(&self.accountDataStore),
                Arc::clone(&self.metadataStore),
                Arc::clone(&logger),
            )) as SharedAuthenticator,
            Arc::new(BlobSASAuthenticator::new(
                Arc::clone(&self.accountDataStore),
                Arc::clone(&self.metadataStore),
                Arc::clone(&logger),
            )) as SharedAuthenticator,
        ];
        if let Some(oauth) = self.oauth {
            authenticators.push(Arc::new(BlobTokenAuthenticator::new(
                Arc::clone(&self.accountDataStore),
                oauth,
                Arc::clone(&logger),
            )) as SharedAuthenticator);
        }

        let preflightMiddlewareFactory = PreflightMiddlewareFactory::new(Arc::clone(&logger));
        let telemetryMiddlewareFactory =
            TelemetryMiddlewareFactory::new(DEFAULT_CONTEXT_PATH.to_string());
        let strictModelMiddlewareFactory = StrictModelMiddlewareFactory::new(
            Arc::clone(&logger),
            vec![UnsupportedHeadersBlocker, UnsupportedParametersBlocker],
        );

        let state = Arc::new(RequestListenerState {
            enableAccessLog: self.enableAccessLog,
            accessLogWriteStream: self.accessLogWriteStream.clone(),
            logger,
            handlers,
            blobStorageContextOptions: createStorageBlobContextMiddleware(
                self.skipApiVersionCheck,
                self.disableProductStyleUrl,
                self.loose,
            ),
            authenticationMiddleware: authenticationMiddlewareFactory
                .createAuthenticationMiddleware(authenticators),
            strictModelMiddleware: if loose {
                None
            } else {
                Some(strictModelMiddlewareFactory.createStrictModelMiddleware())
            },
            optionsHandlerMiddleware: preflightMiddlewareFactory
                .createOptionsHandlerMiddleware(Arc::clone(&self.metadataStore)),
            corsErrorRequestMiddleware: preflightMiddlewareFactory
                .createCorsRequestMiddleware(Arc::clone(&self.metadataStore), true),
            corsRequestMiddleware: preflightMiddlewareFactory
                .createCorsRequestMiddleware(Arc::clone(&self.metadataStore), false),
            telemetryMiddleware: telemetryMiddlewareFactory.createTelemetryMiddleware(),
        });

        Router::new()
            .route("/", any(blob_request_listener))
            .route("/{*path}", any(blob_request_listener))
            .with_state(state)
    }
}

impl IRequestListenerFactory for BlobRequestListenerFactory {
    fn createRequestListener(&self) -> RequestListener {
        BlobRequestListenerFactory::createRequestListener(self)
    }
}

async fn blob_request_listener(
    State(state): State<Arc<RequestListenerState>>,
    request: Request<Body>,
) -> Response<Body> {
    state.handle_request(request).await
}

fn build_generated_request(parts: &Parts, body: &[u8]) -> GeneratedHttpRequest {
    let protocol = request_protocol(parts);
    let host = request_host(parts);
    let path = parts.uri.path().to_string();
    let url = format!(
        "{}://{}{}",
        protocol,
        host,
        parts
            .uri
            .path_and_query()
            .map(|value| value.as_str())
            .unwrap_or(parts.uri.path())
    );
    let endpoint = format!("{}://{}", protocol, host);

    GeneratedHttpRequest {
        method: to_generated_method(parts.method.clone()),
        url,
        endpoint,
        path,
        bodyStream: GeneratedReadableStream::from_bytes(body.to_vec()),
        body: if body.is_empty() {
            None
        } else {
            Some(String::from_utf8_lossy(body).into_owned())
        },
        rawHeaders: raw_headers(&parts.headers),
        headers: header_map_to_generated(&parts.headers),
        query: form_urlencoded::parse(parts.uri.query().unwrap_or_default().as_bytes())
            .into_owned()
            .collect::<BTreeMap<_, _>>(),
        protocol,
    }
}

fn raw_headers(headers: &axum::http::HeaderMap) -> Vec<String> {
    let mut values = Vec::new();
    for (name, value) in headers.iter() {
        values.push(name.as_str().to_string());
        values.push(value.to_str().unwrap_or_default().to_string());
    }
    values
}

fn header_map_to_generated(
    headers: &axum::http::HeaderMap,
) -> BTreeMap<String, RequestHeaderValue> {
    let mut generated = BTreeMap::new();
    for (name, value) in headers.iter() {
        let key = name.as_str().to_ascii_lowercase();
        let value = value.to_str().unwrap_or_default().to_string();
        match generated.remove(&key) {
            Some(RequestHeaderValue::Single(existing)) => {
                generated.insert(key, RequestHeaderValue::Multi(vec![existing, value]));
            }
            Some(RequestHeaderValue::Multi(mut existing)) => {
                existing.push(value);
                generated.insert(key, RequestHeaderValue::Multi(existing));
            }
            None => {
                generated.insert(key, RequestHeaderValue::Single(value));
            }
        }
    }
    generated
}

fn request_protocol(parts: &Parts) -> String {
    parts
        .headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .unwrap_or("http")
        .to_string()
}

fn request_host(parts: &Parts) -> String {
    parts
        .headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .map(strip_port)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            parts
                .uri
                .authority()
                .map(|authority| strip_port(authority.as_str()))
        })
        .unwrap_or_else(|| "127.0.0.1".to_string())
}

fn strip_port(host: &str) -> String {
    host.rsplit_once(':')
        .filter(|(left, right)| !left.contains(':') && right.chars().all(|ch| ch.is_ascii_digit()))
        .map(|(left, _)| left.to_string())
        .unwrap_or_else(|| host.to_string())
}

fn to_generated_method(method: Method) -> HttpMethod {
    method
        .as_str()
        .parse::<HttpMethod>()
        .unwrap_or(HttpMethod::GET)
}

fn generated_response_to_axum(response: GeneratedHttpResponse) -> Response<Body> {
    let status = StatusCode::from_u16(if response.statusCode == 0 {
        200
    } else {
        response.statusCode
    })
    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut axum_response = Response::new(Body::from(response.bodyStream.bytes()));
    *axum_response.status_mut() = status;

    for (name, value) in response.headers {
        if let Ok(header_name) = HeaderName::from_bytes(name.as_bytes()) {
            match value {
                ResponseHeaderValue::Single(value) => {
                    if let Ok(header_value) = HeaderValue::from_str(&value) {
                        axum_response
                            .headers_mut()
                            .insert(header_name, header_value);
                    }
                }
                ResponseHeaderValue::Multi(values) => {
                    for value in values {
                        if let Ok(header_value) = HeaderValue::from_str(&value) {
                            axum_response
                                .headers_mut()
                                .append(header_name.clone(), header_value);
                        }
                    }
                }
            }
        }
    }

    axum_response
}

fn simple_text_response(status: StatusCode, body: &str) -> Response<Body> {
    let mut response = Response::new(Body::from(body.to_string()));
    *response.status_mut() = status;
    response
}

fn oauth_level_to_string(level: OAuthLevel) -> String {
    match level {
        OAuthLevel::BASIC => String::from("basic"),
    }
}
