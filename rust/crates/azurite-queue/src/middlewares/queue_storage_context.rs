use std::collections::BTreeMap;

use chrono::Utc;
use url::Url;
use uuid::Uuid;

use azurite_common::{
    i_logger::ILogger,
    utils::constants::{IP_REGEX, NO_ACCOUNT_HOST_NAMES},
};

use crate::{
    context::QueueStorageContext,
    errors::{StorageError, StorageErrorFactory},
    generated::{
        context::Context,
        i_request::{GeneratedHttpRequest, IRequest},
        i_response::{GeneratedHttpResponse, IResponse, ResponseHeaderValue},
    },
    utils::{
        constants::{HeaderConstants, ValidAPIVersions, SECONDARY_SUFFIX, VERSION},
        utils::{isValidName, nameValidateCode},
    },
};

#[allow(non_snake_case)]
#[derive(Debug, Clone, Copy, Default)]
pub struct QueueStorageContextMiddlewareOptions {
    pub skipApiVersionCheck: bool,
    pub disableProductStyleUrl: bool,
}

#[allow(non_snake_case)]
pub fn createQueueStorageContextMiddleware(
    skipApiVersionCheck: Option<bool>,
    disableProductStyleUrl: Option<bool>,
) -> QueueStorageContextMiddlewareOptions {
    QueueStorageContextMiddlewareOptions {
        skipApiVersionCheck: skipApiVersionCheck.unwrap_or(false),
        disableProductStyleUrl: disableProductStyleUrl.unwrap_or(false),
    }
}

#[allow(non_snake_case)]
pub fn queueStorageContextMiddleware(
    context: &Context,
    req: &GeneratedHttpRequest,
    res: &mut GeneratedHttpResponse,
    logger: &(dyn ILogger + Send + Sync),
    options: QueueStorageContextMiddlewareOptions,
) -> Result<(), StorageError> {
    let reqHost = extract_request_host(req);
    let reqPath = req.getPath();
    internalQueueStorageContextMiddleware(
        context,
        req,
        res,
        &reqHost,
        &reqPath,
        logger,
        options.skipApiVersionCheck,
        options.disableProductStyleUrl,
    )
}

#[allow(non_snake_case)]
pub fn internalQueueStorageContextMiddleware(
    context: &Context,
    req: &GeneratedHttpRequest,
    res: &mut GeneratedHttpResponse,
    reqHost: &str,
    reqPath: &str,
    logger: &(dyn ILogger + Send + Sync),
    skipApiVersionCheck: bool,
    disableProductStyleUrl: bool,
) -> Result<(), StorageError> {
    res.setHeader(
        HeaderConstants.SERVER,
        Some(ResponseHeaderValue::from(format!(
            "Azurite-Queue/{VERSION}"
        ))),
    );
    let requestID = Uuid::new_v4().to_string();

    if !skipApiVersionCheck {
        if let Some(apiVersion) = req.getHeader(HeaderConstants.X_MS_VERSION) {
            if !ValidAPIVersions.contains(&apiVersion.as_str()) {
                return Err(StorageErrorFactory::getInvalidAPIVersion(
                    Some(&requestID),
                    Some(&apiVersion),
                ));
            }
        }
    }

    let queueContext = QueueStorageContext::new(context);
    queueContext.setStartTime(Some(Utc::now()));
    queueContext.setXMsRequestID(Some(requestID.clone()));

    logger.info(
        &format!(
            "QueueStorageContextMiddleware: RequestMethod={} RequestURL={} RequestHeaders={:?} ClientIP={} Protocol={}",
            req.getMethod(),
            req.getUrl(),
            req.getHeaders(),
            req.getEndpoint(),
            req.getProtocol(),
        ),
        Some(&requestID),
    );

    let (account, queue, message, messageId, isSecondary) =
        extractStoragePartsFromPath(reqHost, reqPath, Some(disableProductStyleUrl));

    queueContext.setAccount(account.clone());
    queueContext.setQueue(queue.clone());
    queueContext.setMessage(message.clone());
    queueContext.setMessageId(messageId.clone());
    queueContext.setIsSecondary(Some(isSecondary));

    let mut dispatchPattern = if queue.is_some() {
        if message.is_some() {
            if messageId.is_some() {
                String::from("/queue/messages/messageId")
            } else {
                String::from("/queue/messages")
            }
        } else {
            String::from("/queue")
        }
    } else {
        String::from("/")
    };
    if matches!(req.getQuery("restype").as_deref(), Some("service"))
        || matches!(req.getQuery("comp").as_deref(), Some("list"))
    {
        dispatchPattern = String::from("/");
    }
    context.setDispatchPattern(Some(dispatchPattern.clone()));

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
    queueContext.setAuthenticationPath(Some(authenticationPath));

    let account_is_valid = account
        .as_deref()
        .map(|value| !value.is_empty())
        .unwrap_or(false);
    if !account_is_valid {
        let handlerError =
            StorageErrorFactory::getInvalidQueryParameterValue(Some(&requestID), None);
        logger.error(
            &format!(
                "QueueStorageContextMiddleware: QueueStorageContextMiddleware: {}",
                handlerError.message
            ),
            Some(&requestID),
        );
        return Err(handlerError);
    }

    if dispatchPattern != "/" {
        if let Some(queue) = queue.as_deref() {
            match isValidName(queue) {
                nameValidateCode::invalidUri => {
                    let mut details = BTreeMap::new();
                    details.insert(String::from("UriPath"), format!("/{queue}"));
                    return Err(StorageErrorFactory::getInvalidUri(
                        Some(&requestID),
                        Some(details),
                    ));
                }
                nameValidateCode::outOfRange => {
                    return Err(StorageErrorFactory::getOutOfRangeName(Some(&requestID)));
                }
                nameValidateCode::invalidName => {
                    return Err(StorageErrorFactory::getInvalidResourceName(Some(
                        &requestID,
                    )));
                }
                nameValidateCode::valid => {}
            }
        }
    }

    logger.info(
        &format!(
            "QueueStorageContextMiddleware: Account={} Queue={} Message={} MessageId={}",
            account.clone().unwrap_or_default(),
            queue.clone().unwrap_or_default(),
            message.clone().unwrap_or_default(),
            messageId.clone().unwrap_or_default(),
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
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    bool,
) {
    let mut account: Option<String>;
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

    let queue = parts.get(urlPartIndex).map(|value| (*value).to_string());
    urlPartIndex += 1;
    let message = parts.get(urlPartIndex).map(|value| (*value).to_string());
    urlPartIndex += 1;
    let messageId = parts.get(urlPartIndex).map(|value| (*value).to_string());

    if let Some(account_name) = account.as_deref() {
        if account_name.ends_with(SECONDARY_SUFFIX) {
            account = Some(
                account_name[..account_name.len().saturating_sub(SECONDARY_SUFFIX.len())]
                    .to_string(),
            );
            isSecondary = true;
        }
    }

    (account, queue, message, messageId, isSecondary)
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
                .map(|(_, value)| value.split('/').next().map(strip_port).unwrap_or_default())
        })
        .unwrap_or_default()
}

fn strip_port(host: &str) -> String {
    host.rsplit_once(':')
        .filter(|(left, right)| !left.contains(':') && right.chars().all(|ch| ch.is_ascii_digit()))
        .map(|(left, _)| left.to_string())
        .unwrap_or_else(|| host.to_string())
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

#[cfg(test)]
mod tests {
    use crate::generated::context::Context;
    use crate::generated::i_request::{GeneratedHttpRequest, HttpMethod};

    use super::{
        extractStoragePartsFromPath, extract_request_host, QueueStorageContextMiddlewareOptions,
    };

    #[test]
    fn extracts_path_style_queue_message_paths() {
        let (account, queue, message, message_id, is_secondary) =
            extractStoragePartsFromPath("127.0.0.1", "/devstoreaccount1/queue/messages/abc", None);

        assert_eq!(account.as_deref(), Some("devstoreaccount1"));
        assert_eq!(queue.as_deref(), Some("queue"));
        assert_eq!(message.as_deref(), Some("messages"));
        assert_eq!(message_id.as_deref(), Some("abc"));
        assert!(!is_secondary);
    }

    #[test]
    fn extracts_product_style_queue_paths() {
        let (account, queue, message, message_id, is_secondary) = extractStoragePartsFromPath(
            "devstoreaccount1.queue.localhost",
            "/queue/messages",
            None,
        );

        assert_eq!(account.as_deref(), Some("devstoreaccount1"));
        assert_eq!(queue.as_deref(), Some("queue"));
        assert_eq!(message.as_deref(), Some("messages"));
        assert_eq!(message_id, None);
        assert!(!is_secondary);
    }

    #[test]
    fn detects_secondary_accounts() {
        let (account, queue, _, _, is_secondary) =
            extractStoragePartsFromPath("127.0.0.1", "/devstoreaccount1-secondary/queue", None);

        assert_eq!(account.as_deref(), Some("devstoreaccount1"));
        assert_eq!(queue.as_deref(), Some("queue"));
        assert!(is_secondary);
    }

    #[test]
    fn options_default_to_disabled_flags() {
        let options = QueueStorageContextMiddlewareOptions::default();
        let _ = Context::default();
        assert!(!options.skipApiVersionCheck);
        assert!(!options.disableProductStyleUrl);
    }

    #[test]
    fn extract_request_host_strips_port_from_endpoint_fallback() {
        let request = GeneratedHttpRequest::new(
            HttpMethod::PUT,
            "/devstoreaccount1/queue?timeout=30",
            "http://127.0.0.1:11014",
            "/devstoreaccount1/queue",
        );

        assert_eq!(extract_request_host(&request), "127.0.0.1");
    }

    #[test]
    fn extract_request_host_prefers_parsed_absolute_url_without_port() {
        let request = GeneratedHttpRequest::new(
            HttpMethod::PUT,
            "http://127.0.0.1:11014/devstoreaccount1/queue?timeout=30",
            "http://127.0.0.1:11014",
            "/devstoreaccount1/queue",
        );

        assert_eq!(extract_request_host(&request), "127.0.0.1");
    }
}
