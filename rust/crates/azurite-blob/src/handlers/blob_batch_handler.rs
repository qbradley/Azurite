use std::sync::Arc;

use azurite_common::i_account_data_store::IAccountDataStore;
use azurite_common::models::OAuthLevel;
use chrono::Utc;
use uuid::Uuid;

use crate::authentication::{
    AccountSASAuthenticator, BlobSASAuthenticator, BlobSharedKeyAuthenticator,
    BlobTokenAuthenticator, IAuthenticator, PublicAccessAuthenticator,
};
use crate::context::BlobStorageContext;
use crate::errors::{StorageError, StorageErrorFactory};
use crate::generated::artifacts::operation::Operation;
use crate::generated::context::Context;
use crate::generated::errors::middleware_error::MiddlewareError;
use crate::generated::handlers::IHandlers;
use crate::generated::i_request::{GeneratedHttpRequest, IRequest, RequestHeaderValue};
use crate::generated::i_response::{IResponse, ResponseHeaderValue};
use crate::generated::middleware::deserializer::deserializer_middleware;
use crate::generated::middleware::dispatch::dispatch_middleware;
use crate::generated::middleware::end::end_middleware;
use crate::generated::middleware::error::error_middleware;
use crate::generated::middleware::handler_middleware_factory::HandlerMiddlewareFactory;
use crate::generated::middleware::serializer::serializer_middleware;

use super::base_handler::{SharedBlobMetadataStore, SharedExtentStore, SharedLogger};
use super::blob_batch_sub_request::BlobBatchSubRequest;
use super::blob_batch_sub_response::BlobBatchSubResponse;

// Constants ported from src/blob/utils/constants.ts
const HTTP_LINE_ENDING: &str = "\r\n";
const HTTP_HEADER_DELIMITER: &str = ": ";
const DEFAULT_CONTEXT_PATH: &str = "azurite_blob_context";

// Max batch body size: 4 MiB (matches TS streamToBuffer2 buffer alloc)
const BATCH_BODY_MAX_BYTES: usize = 4 * 1024 * 1024;
// Azure Storage maximum subrequest count per batch
const MAX_BATCH_SUBREQUEST_COUNT: usize = 256;

const SECONDARY_SUFFIX: &str = "-secondary";
const HOST_DOCKER_INTERNAL: &str = "host.docker.internal";

/// Minimal percent-decode: replaces `%XX` sequences with the corresponding byte.
/// Only needed for ASCII-safe characters that appear in blob/container names.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push(((h << 4) | l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_owned())
}

/// Parsed result of a URL path: (account, container, blob, isSecondary)
/// Ported from `extractStoragePartsFromPath` in blobStorageContext.middleware.ts.
fn extract_storage_parts_from_path(
    hostname: &str,
    path: &str,
    disable_product_style_url: Option<bool>,
) -> (Option<String>, Option<String>, Option<String>, bool) {
    let decoded = percent_decode(path);

    let normalized = decoded.strip_prefix('/').unwrap_or(&decoded);
    let parts: Vec<&str> = normalized.split('/').collect();

    let is_ip_address = hostname.split('.').all(|seg| seg.parse::<u8>().is_ok())
        && hostname.split('.').count() == 4;
    let is_no_account_hostname =
        hostname.eq_ignore_ascii_case(HOST_DOCKER_INTERNAL) || hostname == "localhost";
    let first_dot = hostname.find('.');

    let mut url_part_index: usize = 0;
    let mut account: Option<String>;

    if !disable_product_style_url.unwrap_or(false)
        && !is_ip_address
        && !is_no_account_hostname
        && first_dot.map(|i| i > 0).unwrap_or(false)
    {
        account = first_dot.map(|i| hostname[..i].to_owned());
    } else {
        account = parts.get(url_part_index).map(|s| s.to_string());
        url_part_index += 1;
    }

    let container = parts.get(url_part_index).map(|s| s.to_string());
    url_part_index += 1;

    let blob = if url_part_index < parts.len() {
        let joined = parts[url_part_index..].join("/").replace('\\', "/");
        if joined.is_empty() {
            None
        } else {
            Some(joined)
        }
    } else {
        None
    };

    let mut is_secondary = false;
    if let Some(ref acc) = account {
        if acc.ends_with(SECONDARY_SUFFIX) {
            account = Some(acc[..acc.len() - SECONDARY_SUFFIX.len()].to_owned());
            is_secondary = true;
        }
    }

    (account, container, blob, is_secondary)
}

/// Set up BlobStorageContext fields on a freshly-created Context for a subrequest.
/// Replicates `internalBlobStorageContextMiddleware` with `skipApiVersionCheck = true`.
fn setup_sub_request_context(
    context: &Context,
    req: &BlobBatchSubRequest,
    request_id: &str,
    loose: bool,
    disable_product_style: Option<bool>,
) {
    let blob_context = BlobStorageContext::new(context);
    blob_context.setStartTime(Some(Utc::now()));
    blob_context.setXMsRequestID(Some(request_id.to_owned()));
    blob_context.setLoose(Some(loose));
    blob_context.setDisableProductStyleUrl(disable_product_style);

    // Parse account/container/blob from the subrequest URL.
    // BlobBatchSubRequest::getEndpoint() returns "scheme://host" and
    // getPath() returns the path component; use those directly.
    let parsed_host: String = {
        let endpoint = req.getEndpoint();
        // Strip "scheme://" prefix to get bare host
        endpoint
            .find("://")
            .map(|i| endpoint[i + 3..].to_owned())
            .unwrap_or(endpoint)
    };
    let parsed_path: String = req.getPath();

    let (account, container, blob, is_secondary) =
        extract_storage_parts_from_path(&parsed_host, &parsed_path, disable_product_style);

    blob_context.setAccount(account);
    blob_context.setContainer(container.clone());
    blob_context.setBlob(blob.clone());
    blob_context.setIsSecondary(Some(is_secondary));

    let dispatch_pattern = match (&container, &blob) {
        (Some(_), Some(b)) if !b.is_empty() => "/container/blob".to_owned(),
        (Some(_), _) => "/container".to_owned(),
        _ => "/".to_owned(),
    };
    context.setDispatchPattern(Some(dispatch_pattern));

    let auth_path = parsed_path.clone();
    blob_context.setAuthenticationPath(Some(auth_path));
}

/// Convert a BlobBatchSubRequest into a GeneratedHttpRequest for use with
/// IAuthenticator::validate and context storage.
fn to_generated_request(req: &BlobBatchSubRequest) -> GeneratedHttpRequest {
    // Parse query parameters from the URL so SAS authenticators can read them
    // via getQuery() on the GeneratedHttpRequest.
    let query: std::collections::BTreeMap<String, String> = url::Url::parse(&req.getUrl())
        .map(|u| {
            u.query_pairs()
                .map(|(k, v)| (k.into_owned(), v.into_owned()))
                .collect()
        })
        .unwrap_or_default();

    GeneratedHttpRequest {
        method: req.getMethod(),
        url: req.getUrl(),
        endpoint: req.getEndpoint(),
        path: req.getPath(),
        headers: req.getHeaders(),
        query,
        ..GeneratedHttpRequest::default()
    }
}

/// BlobBatchHandler handles Azure Storage batch requests.
///
/// Ported from `BlobBatchHandler.ts`.  The TypeScript handler builds an internal
/// Express-style middleware pipeline; here we call the Rust middleware functions
/// directly and in the same order.
///
/// ## Supported operations
/// * `Blob_Delete`
/// * `Blob_SetTier`
///
/// ## Constraints (preserved from TypeScript)
/// * Batch body is capped at 4 MiB before parsing.
/// * At most 256 subrequests per batch.
/// * All subrequests in a single batch must use the same operation.
/// * No subrequest body parsing (only headers are read today).
pub struct BlobBatchHandler<H: IHandlers> {
    handlers: Arc<H>,
    authenticators: Vec<Box<dyn IAuthenticator + Send + Sync>>,
    logger: SharedLogger,
    loose: bool,
    disable_product_style: Option<bool>,
}

impl<H: IHandlers + 'static> BlobBatchHandler<H> {
    pub fn new(
        account_data_store: Arc<dyn IAccountDataStore + Send + Sync>,
        oauth: Option<OAuthLevel>,
        metadata_store: SharedBlobMetadataStore,
        _extent_store: SharedExtentStore,
        logger: SharedLogger,
        loose: bool,
        disable_product_style: Option<bool>,
        handlers: Arc<H>,
    ) -> Self {
        let mut authenticators: Vec<Box<dyn IAuthenticator + Send + Sync>> = vec![
            Box::new(PublicAccessAuthenticator::new(
                Arc::clone(&metadata_store),
                Arc::clone(&logger) as Arc<dyn azurite_common::i_logger::ILogger + Send + Sync>,
            )),
            Box::new(BlobSharedKeyAuthenticator::new(
                Arc::clone(&account_data_store),
                Arc::clone(&logger) as Arc<dyn azurite_common::i_logger::ILogger + Send + Sync>,
            )),
            Box::new(AccountSASAuthenticator::new(
                Arc::clone(&account_data_store),
                Arc::clone(&metadata_store),
                Arc::clone(&logger) as Arc<dyn azurite_common::i_logger::ILogger + Send + Sync>,
            )),
            Box::new(BlobSASAuthenticator::new(
                Arc::clone(&account_data_store),
                Arc::clone(&metadata_store),
                Arc::clone(&logger) as Arc<dyn azurite_common::i_logger::ILogger + Send + Sync>,
            )),
        ];

        if let Some(oauth_level) = oauth {
            authenticators.push(Box::new(BlobTokenAuthenticator::new(
                Arc::clone(&account_data_store),
                oauth_level,
                Arc::clone(&logger) as Arc<dyn azurite_common::i_logger::ILogger + Send + Sync>,
            )));
        }

        Self {
            handlers,
            authenticators,
            logger,
            loose,
            disable_product_style,
        }
    }

    // -----------------------------------------------------------------------
    // Body helpers
    // -----------------------------------------------------------------------

    /// Read `body_bytes` into a String, rejecting inputs larger than BATCH_BODY_MAX_BYTES.
    /// Ported from `streamToBuffer2` + `requestBodyToString` in BlobBatchHandler.ts.
    fn body_bytes_to_string(body_bytes: &[u8]) -> Result<String, String> {
        if body_bytes.len() > BATCH_BODY_MAX_BYTES {
            return Err(format!(
                "Stream exceeds buffer size. Buffer size: {}",
                BATCH_BODY_MAX_BYTES
            ));
        }
        String::from_utf8(body_bytes.to_vec())
            .map_err(|e| format!("Batch body is not valid UTF-8: {e}"))
    }

    // -----------------------------------------------------------------------
    // Operation detection (context + dispatch only, no auth/handler)
    // -----------------------------------------------------------------------

    /// Runs just context-setup + dispatch middleware on a subrequest to learn
    /// which `Operation` it targets. Ported from `getSubRequestOperation`.
    async fn get_sub_request_operation(
        &self,
        req: &BlobBatchSubRequest,
        request_id: &str,
    ) -> Result<Operation, Box<dyn std::error::Error + Send + Sync>> {
        let holder = Context::new_holder();
        let gen_req = to_generated_request(req);
        let context = Context::from_holder(holder, DEFAULT_CONTEXT_PATH, Some(gen_req), None);

        setup_sub_request_context(
            &context,
            req,
            request_id,
            self.loose,
            self.disable_product_style,
        );

        dispatch_middleware(&context, req, self.logger.as_ref())?;

        context
            .operation()
            .ok_or_else(|| "Dispatch did not set an operation".into())
    }

    // -----------------------------------------------------------------------
    // Subrequest parsing
    // -----------------------------------------------------------------------

    /// Parse the raw multipart batch body into a list of `BlobBatchSubRequest`s.
    /// Ported from `parseSubRequests` in BlobBatchHandler.ts.
    async fn parse_sub_requests(
        &self,
        common_request_id: &str,
        per_request_prefix: &str,
        batch_request_ending: &str,
        sub_request_path_prefix: &str,
        batch_request: &BlobBatchSubRequest,
        body: &str,
    ) -> Result<Vec<BlobBatchSubRequest>, Box<dyn std::error::Error + Send + Sync>> {
        // Split off everything after the closing boundary
        let before_ending = body.split(batch_request_ending).next().unwrap_or(body);

        // Split on per-request boundary to get individual subrequest blocks
        let blocks: Vec<&str> = before_ending.split(per_request_prefix).collect();
        // First element is whatever precedes the first boundary – skip it
        let sub_request_blocks = &blocks[1..];

        let mut results: Vec<BlobBatchSubRequest> = Vec::new();
        let mut previous_operation: Option<Operation> = None;

        for block in sub_request_blocks {
            let lines: Vec<&str> = block.split(HTTP_LINE_ENDING).collect();

            // Minimum: at least the MIME headers, blank line, and request line
            // Content-Type, Content-ID, (optional headers), <blank>, <request line>
            if lines.len() < 5 {
                return Err("Bad request".into());
            }

            // --- Parse MIME wrapper headers to extract Content-ID ----------
            let mut line_index = 0usize;
            let mut content_id: Option<u32> = None;

            while line_index < lines.len() {
                if lines[line_index].is_empty() {
                    break;
                }
                let header_parts: Vec<&str> =
                    lines[line_index].splitn(2, HTTP_HEADER_DELIMITER).collect();
                if header_parts.len() != 2 {
                    return Err("Bad Request".into());
                }
                if header_parts[0].eq_ignore_ascii_case("content-id") {
                    content_id = header_parts[1].trim().parse::<u32>().ok();
                }
                line_index += 1;
            }

            let content_id = content_id.ok_or("Bad request: missing Content-ID")?;

            // Skip the blank line that separates MIME headers from the HTTP request line
            line_index += 1;

            // --- Parse HTTP request line: "DELETE /container/blob HTTP/1.1" -
            if line_index >= lines.len() {
                return Err("Bad request: missing HTTP request line".into());
            }
            let operation_parts: Vec<&str> = lines[line_index].splitn(3, ' ').collect();
            if operation_parts.len() < 3 {
                return Err("Bad request: malformed HTTP request line".into());
            }

            let raw_path = operation_parts[1];
            let request_path = if raw_path.starts_with('/') {
                raw_path.to_owned()
            } else {
                format!("/{raw_path}")
            };

            if !request_path.starts_with(sub_request_path_prefix) {
                return Err("Request from a different container".into());
            }

            let method_str = operation_parts[0];
            let method: crate::generated::i_request::HttpMethod = method_str
                .parse()
                .map_err(|_| format!("Unknown HTTP method: {method_str}"))?;

            let protocol_with_version = operation_parts[2].to_owned();
            let url = format!("{}{}", batch_request.getEndpoint(), request_path);

            let mut sub_req = BlobBatchSubRequest::new(
                content_id,
                url,
                method,
                protocol_with_version,
                Default::default(),
            );

            // --- Parse HTTP request headers ---------------------------------
            line_index += 1;
            while line_index < lines.len() {
                if lines[line_index].is_empty() {
                    break;
                }
                let header_parts: Vec<&str> =
                    lines[line_index].splitn(2, HTTP_HEADER_DELIMITER).collect();
                if header_parts.len() != 2 {
                    return Err("Bad Request: malformed header".into());
                }
                sub_req.setHeader(
                    header_parts[0].to_owned(),
                    Some(RequestHeaderValue::Single(header_parts[1].to_owned())),
                );
                line_index += 1;
            }

            // --- Validate and record operation ------------------------------
            let operation = self
                .get_sub_request_operation(&sub_req, common_request_id)
                .await?;

            if operation != Operation::Blob_Delete && operation != Operation::Blob_SetTier {
                return Err("Not supported operation".into());
            }

            match previous_operation {
                None => {
                    previous_operation = Some(operation);
                }
                Some(prev) if prev != operation => {
                    return Err(Box::new(StorageError::new(
                        400,
                        "AllBatchSubRequestsShouldBeSameApi",
                        "All batch subrequests should be the same api.",
                        common_request_id,
                        StorageError::empty_extra(),
                    )));
                }
                _ => {}
            }

            results.push(sub_req);
        }

        if results.is_empty() {
            return Err("Bad Request: no subrequests found".into());
        }

        Ok(results)
    }

    // -----------------------------------------------------------------------
    // Response serialisation
    // -----------------------------------------------------------------------

    /// Serialize all subrequest responses into a single multipart response body.
    /// Ported from `serializeSubResponse` in BlobBatchHandler.ts.
    fn serialize_sub_response(
        sub_response_prefix: &str,
        response_ending: &str,
        sub_responses: &[BlobBatchSubResponse],
    ) -> String {
        let mut body = String::new();

        for sub_response in sub_responses {
            body.push_str(sub_response_prefix);
            body.push_str("Content-Type: application/http");
            body.push_str(HTTP_LINE_ENDING);
            if let Some(cid) = sub_response.content_id {
                body.push_str("Content-ID");
                body.push_str(HTTP_HEADER_DELIMITER);
                body.push_str(&cid.to_string());
                body.push_str(HTTP_LINE_ENDING);
            }
            body.push_str(HTTP_LINE_ENDING);

            // HTTP status line
            body.push_str(&sub_response.protocolWithVersion);
            body.push(' ');
            body.push_str(&sub_response.getStatusCode().to_string());
            body.push(' ');
            body.push_str(&sub_response.getStatusMessage());
            body.push_str(HTTP_LINE_ENDING);

            // Response headers
            for (key, value) in sub_response.getHeaders() {
                let value_str = match &value {
                    ResponseHeaderValue::Single(v) => v.clone(),
                    ResponseHeaderValue::Multi(vs) => vs.join(", "),
                };
                body.push_str(&key);
                body.push_str(HTTP_HEADER_DELIMITER);
                body.push_str(&value_str);
                body.push_str(HTTP_LINE_ENDING);
            }

            // Optional body content
            let body_content = sub_response.getBodyContent();
            if !body_content.is_empty() {
                body.push_str(HTTP_LINE_ENDING);
                body.push_str(&body_content);
                body.push_str(HTTP_LINE_ENDING);
            }

            body.push_str(HTTP_LINE_ENDING);
        }

        body.push_str(response_ending);
        body
    }

    // -----------------------------------------------------------------------
    // Single subrequest execution
    // -----------------------------------------------------------------------

    /// Run the full middleware pipeline for one subrequest.
    /// Ported from `HandleOneSubRequest` in BlobBatchHandler.ts.
    async fn handle_one_sub_request(
        &self,
        req: &mut BlobBatchSubRequest,
        res: &mut BlobBatchSubResponse,
    ) {
        let request_id = Uuid::new_v4().to_string();
        let holder = Context::new_holder();
        let gen_req = to_generated_request(req);
        let context =
            Context::from_holder(holder, DEFAULT_CONTEXT_PATH, Some(gen_req.clone()), None);

        setup_sub_request_context(
            &context,
            req,
            &request_id,
            self.loose,
            self.disable_product_style,
        );

        // 1. Dispatch
        if let Err(err) = dispatch_middleware(&context, req, self.logger.as_ref()) {
            let _ = error_middleware(&context, err.as_ref(), req, res, self.logger.as_ref());
            end_middleware(&context, res, self.logger.as_ref());
            return;
        }

        // 2. Authenticate
        let auth_result = self.authenticate_sub_request(&gen_req, &context).await;
        match auth_result {
            Ok(false) => {
                let auth_err = StorageErrorFactory::getAuthorizationFailure(&request_id);
                let _ = error_middleware(&context, &auth_err, req, res, self.logger.as_ref());
                end_middleware(&context, res, self.logger.as_ref());
                return;
            }
            Err(err) => {
                let _ = error_middleware(&context, &err, req, res, self.logger.as_ref());
                end_middleware(&context, res, self.logger.as_ref());
                return;
            }
            Ok(true) => {}
        }

        // 3. Deserialise
        if let Err(err) = deserializer_middleware(&context, req, self.logger.as_ref()).await {
            let _ = error_middleware(&context, err.as_ref(), req, res, self.logger.as_ref());
            end_middleware(&context, res, self.logger.as_ref());
            return;
        }

        // 4. Handle
        let handler_mf =
            HandlerMiddlewareFactory::new(Arc::clone(&self.handlers), Arc::clone(&self.logger));
        if let Err(err) = handler_mf.call(&context).await {
            let _ = error_middleware(&context, err.as_ref(), req, res, self.logger.as_ref());
            end_middleware(&context, res, self.logger.as_ref());
            return;
        }

        // 5. Serialise
        if let Err(err) = serializer_middleware(&context, res, self.logger.as_ref()).await {
            let _ = error_middleware(&context, err.as_ref(), req, res, self.logger.as_ref());
        }

        // 6. End
        end_middleware(&context, res, self.logger.as_ref());
    }

    /// Run only the error middleware for a whole-batch failure.
    /// Ported from `HandleOneFailedRequest` in BlobBatchHandler.ts.
    async fn handle_one_failed_request(
        &self,
        err: &(dyn std::error::Error + Send + Sync + 'static),
        req: &BlobBatchSubRequest,
        res: &mut BlobBatchSubResponse,
    ) {
        let holder = Context::new_holder();
        let gen_req = to_generated_request(req);
        let context = Context::from_holder(holder, DEFAULT_CONTEXT_PATH, Some(gen_req), None);
        let _ = error_middleware(&context, err, req, res, self.logger.as_ref());
        end_middleware(&context, res, self.logger.as_ref());
    }

    // -----------------------------------------------------------------------
    // Authentication helper
    // -----------------------------------------------------------------------

    /// Try each authenticator in order; return the first definitive pass/fail.
    /// Returns `Ok(true)` if authenticated, `Ok(false)` if denied, `Err` on error.
    async fn authenticate_sub_request(
        &self,
        req: &GeneratedHttpRequest,
        context: &Context,
    ) -> Result<bool, StorageError> {
        for authenticator in self.authenticators.iter() {
            match authenticator.validate(req, context).await {
                Ok(Some(true)) => return Ok(true),
                Ok(Some(false)) | Ok(None) => continue,
                Err(e) => return Err(e),
            }
        }
        // No authenticator matched – deny
        Ok(false)
    }

    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    /// Process a batch request and return the serialised multipart response body.
    /// Ported from `submitBatch` in BlobBatchHandler.ts.
    pub async fn submitBatch(
        &self,
        body_bytes: &[u8],
        request_batch_boundary: &str,
        sub_request_path_prefix: &str,
        batch_request: &BlobBatchSubRequest,
        context_id: &str,
    ) -> String {
        let per_request_prefix = format!("--{request_batch_boundary}{HTTP_LINE_ENDING}");
        let batch_request_ending = format!("--{request_batch_boundary}--");

        // --- Read body ------------------------------------------------------
        let request_body = match Self::body_bytes_to_string(body_bytes) {
            Ok(s) => s,
            Err(msg) => {
                self.logger
                    .error(&format!("BlobBatchHandler: {msg}"), Some(context_id));
                let err = StorageError::new(
                    400,
                    "InvalidInput",
                    "One of the request inputs is not valid.",
                    context_id,
                    StorageError::empty_extra(),
                );
                let mut error_response = BlobBatchSubResponse::new(None, "HTTP/1.1".to_owned());
                self.handle_one_failed_request(&err, batch_request, &mut error_response)
                    .await;
                error_response.end();
                return Self::serialize_sub_response(
                    &per_request_prefix,
                    &batch_request_ending,
                    &[error_response],
                );
            }
        };

        // --- Parse subrequests ----------------------------------------------
        let parse_result = self
            .parse_sub_requests(
                context_id,
                &per_request_prefix,
                &batch_request_ending,
                sub_request_path_prefix,
                batch_request,
                &request_body,
            )
            .await;

        let (sub_requests, batch_error) = match parse_result {
            Ok(reqs) => (Some(reqs), None),
            Err(err) => {
                // Distinguish StorageError (already has error code) from generic errors
                let storage_err = if err.downcast_ref::<StorageError>().is_some()
                    || err.downcast_ref::<MiddlewareError>().is_some()
                {
                    None // handled below generically after downcast
                } else {
                    Some(StorageError::new(
                        400,
                        "InvalidInput",
                        "One of the request inputs is not valid.",
                        context_id,
                        StorageError::empty_extra(),
                    ))
                };
                // If the original error is already a StorageError, use it directly
                let final_err: Box<dyn std::error::Error + Send + Sync> =
                    if let Ok(se) = err.downcast::<StorageError>() {
                        se
                    } else if let Some(se) = storage_err {
                        Box::new(se)
                    } else {
                        Box::new(StorageError::new(
                            400,
                            "InvalidInput",
                            "One of the request inputs is not valid.",
                            context_id,
                            StorageError::empty_extra(),
                        ))
                    };
                (None, Some(final_err))
            }
        };

        // --- Enforce max sub-request count ----------------------------------
        let batch_error = batch_error.or_else(|| {
            if sub_requests
                .as_ref()
                .map(|r| r.len() > MAX_BATCH_SUBREQUEST_COUNT)
                .unwrap_or(false)
            {
                Some(Box::new(StorageError::new(
                    400,
                    "ExceedsMaxBatchRequestCount",
                    "The batch operation exceeds maximum number of allowed subrequests.",
                    context_id,
                    StorageError::empty_extra(),
                ))
                    as Box<dyn std::error::Error + Send + Sync>)
            } else {
                None
            }
        });

        // --- Execute ---------------------------------------------------------
        let mut sub_responses: Vec<BlobBatchSubResponse> = Vec::new();

        if let Some(err) = batch_error {
            self.logger
                .error(&format!("BlobBatchHandler: {}", err), Some(context_id));
            let mut error_response = BlobBatchSubResponse::new(None, "HTTP/1.1".to_owned());
            self.handle_one_failed_request(err.as_ref(), batch_request, &mut error_response)
                .await;
            error_response.end();
            sub_responses.push(error_response);
        } else if let Some(reqs) = sub_requests {
            for mut sub_req in reqs {
                self.logger.info(
                    &format!(
                        "BlobBatchHandler: starting on subrequest {}",
                        sub_req.content_id
                    ),
                    Some(context_id),
                );
                let mut sub_response = BlobBatchSubResponse::new(
                    Some(sub_req.content_id),
                    sub_req.protocolWithVersion.clone(),
                );
                self.handle_one_sub_request(&mut sub_req, &mut sub_response)
                    .await;
                sub_response.end();
                let req_id = sub_response
                    .getHeader("x-ms-request-id")
                    .and_then(|v| v.as_single())
                    .unwrap_or_default();
                self.logger.info(
                    &format!(
                        "BlobBatchHandler: completed on subrequest {} {req_id}",
                        sub_req.content_id
                    ),
                    Some(context_id),
                );
                sub_responses.push(sub_response);
            }
        }

        Self::serialize_sub_response(&per_request_prefix, &batch_request_ending, &sub_responses)
    }
}
