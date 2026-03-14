use chrono::Utc;
use url::Url;
use uuid::Uuid;

use azurite_common::{
    i_logger::ILogger,
    utils::constants::{IP_REGEX, NO_ACCOUNT_HOST_NAMES},
};

use crate::{
    context::BlobStorageContext,
    errors::{StorageError, StorageErrorFactory},
    generated::{
        context::Context,
        i_request::{GeneratedHttpRequest, IRequest},
        i_response::{GeneratedHttpResponse, IResponse, ResponseHeaderValue},
    },
    utils::{
        constants::{HeaderConstants, ValidAPIVersions, SECONDARY_SUFFIX, VERSION},
        utils::{checkApiVersion, validateContainerName},
    },
};

#[allow(non_snake_case)]
#[derive(Debug, Clone, Copy, Default)]
pub struct BlobStorageContextMiddlewareOptions {
    pub skipApiVersionCheck: bool,
    pub disableProductStyleUrl: bool,
    pub loose: bool,
}

#[allow(non_snake_case)]
pub fn createStorageBlobContextMiddleware(
    skipApiVersionCheck: Option<bool>,
    disableProductStyleUrl: Option<bool>,
    loose: Option<bool>,
) -> BlobStorageContextMiddlewareOptions {
    BlobStorageContextMiddlewareOptions {
        skipApiVersionCheck: skipApiVersionCheck.unwrap_or(false),
        disableProductStyleUrl: disableProductStyleUrl.unwrap_or(false),
        loose: loose.unwrap_or(false),
    }
}

#[allow(non_snake_case)]
pub fn blobStorageContextMiddleware(
    context: &Context,
    req: &GeneratedHttpRequest,
    res: &mut GeneratedHttpResponse,
    logger: &(dyn ILogger + Send + Sync),
    options: BlobStorageContextMiddlewareOptions,
) -> Result<(), StorageError> {
    let reqHost = extract_request_host(req);
    let reqPath = req.getPath();
    internalBlobStorageContextMiddleware(
        context,
        req,
        res,
        &reqHost,
        &reqPath,
        logger,
        options.skipApiVersionCheck,
        options.disableProductStyleUrl,
        options.loose,
    )
}

#[allow(non_snake_case)]
pub fn internalBlobStorageContextMiddleware(
    context: &Context,
    req: &GeneratedHttpRequest,
    res: &mut GeneratedHttpResponse,
    reqHost: &str,
    reqPath: &str,
    logger: &(dyn ILogger + Send + Sync),
    skipApiVersionCheck: bool,
    disableProductStyleUrl: bool,
    loose: bool,
) -> Result<(), StorageError> {
    res.setHeader(
        HeaderConstants::SERVER,
        Some(ResponseHeaderValue::from(format!("Azurite-Blob/{VERSION}"))),
    );
    let requestID = Uuid::new_v4().to_string();

    if !skipApiVersionCheck {
        if let Some(apiVersion) = req.getHeader(HeaderConstants::X_MS_VERSION) {
            checkApiVersion(&apiVersion, &ValidAPIVersions, &requestID)?;
        }
    }

    let blobContext = BlobStorageContext::new(context);
    blobContext.setStartTime(Some(Utc::now()));
    blobContext.setDisableProductStyleUrl(Some(disableProductStyleUrl));
    blobContext.setLoose(Some(loose));
    blobContext.setXMsRequestID(Some(requestID.clone()));

    logger.info(
        &format!(
            "BlobStorageContextMiddleware: RequestMethod={} RequestURL={} RequestHeaders={:?} ClientIP={} Protocol={}",
            req.getMethod(),
            req.getUrl(),
            req.getHeaders(),
            req.getEndpoint(),
            req.getProtocol(),
        ),
        Some(&requestID),
    );

    let (account, container, blob, isSecondary) =
        extractStoragePartsFromPath(reqHost, reqPath, Some(disableProductStyleUrl));

    blobContext.setAccount(account.clone());
    blobContext.setContainer(container.clone());
    blobContext.setBlob(blob.clone());
    blobContext.setIsSecondary(Some(isSecondary));

    let dispatchPattern = match (
        container.as_deref().filter(|value| !value.is_empty()),
        blob.as_deref().filter(|value| !value.is_empty()),
    ) {
        (Some(_), Some(_)) => "/container/blob",
        (Some(_), None) => "/container",
        _ => "/",
    };
    context.setDispatchPattern(Some(dispatchPattern.to_string()));

    let mut authenticationPath = reqPath.to_string();
    if isSecondary {
        if let Some(pos) = authenticationPath.find(SECONDARY_SUFFIX) {
            authenticationPath = format!(
                "{}{}",
                &authenticationPath[..pos],
                &authenticationPath[pos + SECONDARY_SUFFIX.len()..]
            );
        }
    }
    blobContext.setAuthenticationPath(Some(authenticationPath));

    let account_is_valid = account
        .as_deref()
        .map(|value| !value.is_empty())
        .unwrap_or(false);
    if !account_is_valid {
        let handlerError =
            StorageErrorFactory::getInvalidQueryParameterValue(Some(&requestID), None, None, None);
        logger.error(
            &format!(
                "BlobStorageContextMiddleware: BlobStorageContextMiddleware: {}",
                handlerError.message
            ),
            Some(&requestID),
        );
        return Err(handlerError);
    }

    if let Some(container) = container
        .as_deref()
        .filter(|value| !value.is_empty() && !value.starts_with('$'))
    {
        validateContainerName(&requestID, container)?;
    }

    logger.info(
        &format!(
            "BlobStorageContextMiddleware: Account={} Container={} Blob={}",
            account.unwrap_or_default(),
            container.unwrap_or_default(),
            blob.unwrap_or_default(),
        ),
        Some(&requestID),
    );

    Ok(())
}

#[allow(non_snake_case)]
pub fn extractStoragePartsFromPath(
    hostname: &str,
    path: &str,
    disableProductStyleUrl: Option<bool>,
) -> (Option<String>, Option<String>, Option<String>, bool) {
    let mut account: Option<String>;

    let mut blob: Option<String> = None;
    let mut isSecondary = false;

    let decodedPath = percent_decode_path(path);
    let normalizedPath = decodedPath.strip_prefix('/').unwrap_or(&decodedPath);
    let parts = normalizedPath.split('/').collect::<Vec<_>>();

    let mut urlPartIndex = 0usize;
    let isIPAddress = IP_REGEX.is_match(hostname);
    let hostname_lower = hostname.to_ascii_lowercase();
    let isNoAccountHostName = NO_ACCOUNT_HOST_NAMES.contains(hostname_lower.as_str());
    let firstDotIndex = hostname.find('.');

    if !disableProductStyleUrl.unwrap_or(false)
        && !isIPAddress
        && !isNoAccountHostName
        && firstDotIndex.map(|index| index > 0).unwrap_or(false)
    {
        account = firstDotIndex.map(|index| hostname[..index].to_string());
    } else {
        account = parts.get(urlPartIndex).map(|value| (*value).to_string());
        urlPartIndex += 1;
    }

    let container: Option<String> = parts.get(urlPartIndex).map(|value| (*value).to_string());
    urlPartIndex += 1;

    if urlPartIndex <= parts.len() {
        let joined = parts[urlPartIndex..].join("/").replace('\\', "/");
        if !joined.is_empty() {
            blob = Some(joined);
        }
    }

    if let Some(account_name) = account.as_deref() {
        if account_name.ends_with(SECONDARY_SUFFIX) {
            account = Some(
                account_name[..account_name.len().saturating_sub(SECONDARY_SUFFIX.len())]
                    .to_string(),
            );
            isSecondary = true;
        }
    }

    (account, container, blob, isSecondary)
}

fn extract_request_host(req: &GeneratedHttpRequest) -> String {
    Url::parse(&req.getUrl())
        .ok()
        .and_then(|url| url.host_str().map(|host| host.to_string()))
        .or_else(|| {
            Url::parse(&req.getEndpoint())
                .ok()
                .and_then(|url| url.host_str().map(|host| host.to_string()))
        })
        .or_else(|| {
            req.getEndpoint()
                .split_once("://")
                .map(|(_, value)| value.to_string())
        })
        .unwrap_or_default()
}

fn percent_decode_path(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hi = (bytes[index + 1] as char).to_digit(16);
            let lo = (bytes[index + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                output.push(((hi << 4) | lo) as u8);
                index += 3;
                continue;
            }
        }
        output.push(bytes[index]);
        index += 1;
    }

    String::from_utf8(output).unwrap_or_else(|_| value.to_string())
}
