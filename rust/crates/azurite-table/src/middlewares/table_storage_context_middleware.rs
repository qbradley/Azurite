use std::{collections::BTreeMap, sync::LazyLock};

use chrono::Utc;
use regex::Regex;
use uuid::Uuid;

use azurite_common::{
    i_logger::ILogger,
    utils::constants::{IP_REGEX, NO_ACCOUNT_HOST_NAMES},
};

use crate::{
    context::TableStorageContext,
    errors::{StorageError, StorageErrorFactory},
    generated::{
        context::Context,
        i_request::{GeneratedHttpRequest, IRequest},
        i_response::{GeneratedHttpResponse, IResponse, ResponseHeaderValue},
    },
    utils::{
        constants::{HeaderConstants, SECONDARY_SUFFIX, VERSION},
        utils::{checkApiVersion, validateTableName},
    },
};

static QUOTED_KEY_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"'([^']|'')*'").unwrap());

#[allow(non_snake_case)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TableStorageContextMiddlewareOptions {
    pub skipApiVersionCheck: bool,
    pub disableProductStyleUrl: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ParsedTableSection {
    dispatchSection: Option<String>,
    tableName: Option<String>,
    partitionKey: Option<String>,
    rowKey: Option<String>,
}

#[allow(non_snake_case)]
pub fn createTableStorageContextMiddleware(
    skipApiVersionCheck: Option<bool>,
    disableProductStyleUrl: Option<bool>,
) -> TableStorageContextMiddlewareOptions {
    TableStorageContextMiddlewareOptions {
        skipApiVersionCheck: skipApiVersionCheck.unwrap_or(false),
        disableProductStyleUrl: disableProductStyleUrl.unwrap_or(false),
    }
}

#[allow(non_snake_case)]
pub fn tableStorageContextMiddleware(
    context: &Context,
    req: &GeneratedHttpRequest,
    res: &mut GeneratedHttpResponse,
    logger: &(dyn ILogger + Send + Sync),
    options: TableStorageContextMiddlewareOptions,
) -> Result<(), StorageError> {
    let reqHost = extract_request_host(req);
    let reqPath = req.getPath();
    internalTableStorageContextMiddleware(
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
pub fn internalTableStorageContextMiddleware(
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
            "Azurite-Table/{VERSION}"
        ))),
    );

    let requestID = Uuid::new_v4().to_string();
    let tableContext = TableStorageContext::new(context);
    tableContext.setAccept(req.getHeader(HeaderConstants.ACCEPT));
    tableContext.setStartTime(Some(Utc::now()));
    tableContext.setXMsRequestID(Some(requestID.clone()));

    if !skipApiVersionCheck {
        if let Some(apiVersion) = req.getHeader(HeaderConstants.X_MS_VERSION) {
            checkApiVersion(
                &apiVersion,
                &crate::utils::constants::ValidAPIVersions,
                &tableContext,
            )?;
        }
    }

    logger.info(
        &format!(
            "TableStorageContextMiddleware: RequestMethod={} RequestURL={} RequestHeaders={:?} ClientIP={} Protocol={}",
            req.getMethod(),
            req.getUrl(),
            req.getHeaders(),
            req.getEndpoint(),
            req.getProtocol(),
        ),
        Some(&requestID),
    );

    let (account, tableSection, isSecondary) =
        extractStoragePartsFromPath(reqHost, reqPath, Some(disableProductStyleUrl));

    tableContext.setIsSecondary(Some(isSecondary));

    let parsed = parse_table_section(
        tableSection.as_deref(),
        req.getMethod().to_string() == "GET",
    )
    .map_err(|message| {
        logger.error(&message, Some(&requestID));
        let mut details = BTreeMap::new();
        details.insert(String::from("MessageDetails"), message);
        StorageErrorFactory::getInvalidQueryParameterValue(&tableContext, Some(details))
    })?;

    tableContext.setTableName(parsed.tableName.clone());
    tableContext.setPartitionKey(parsed.partitionKey.clone());
    tableContext.setRowKey(parsed.rowKey.clone());
    tableContext.setAccount(account.clone());

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
    tableContext.setAuthenticationPath(Some(authenticationPath));

    context.setDispatchPattern(Some(match parsed.dispatchSection.as_deref() {
        Some(dispatch) if !dispatch.is_empty() => format!("/{dispatch}"),
        _ => String::from("/"),
    }));

    logger.debug(
        &format!(
            "tableStorageContextMiddleware: Dispatch pattern string: {}",
            context.dispatchPattern().unwrap_or_default()
        ),
        Some(&requestID),
    );

    if let Some(table_name) = tableContext.tableName() {
        if !table_name.starts_with('$') {
            validateTableName(&tableContext, &table_name)?;
        }
    }

    logger.info(
        &format!(
            "tableStorageContextMiddleware: Account={} tableName={}",
            account.unwrap_or_default(),
            tableContext.tableName().unwrap_or_default(),
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
) -> (Option<String>, Option<String>, bool) {
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

    let table = parts.get(urlPartIndex).map(|value| (*value).to_string());

    if let Some(account_name) = account.as_deref() {
        if account_name.ends_with(SECONDARY_SUFFIX) {
            account = Some(
                account_name[..account_name.len().saturating_sub(SECONDARY_SUFFIX.len())]
                    .to_string(),
            );
            isSecondary = true;
        }
    }

    (account, table, isSecondary)
}

fn parse_table_section(
    table_section: Option<&str>,
    is_get: bool,
) -> Result<ParsedTableSection, String> {
    let Some(table_section) = table_section else {
        return Ok(ParsedTableSection::default());
    };
    if table_section.is_empty() {
        return Ok(ParsedTableSection::default());
    }

    if matches!(table_section, "Tables" | "Tables()") {
        return Ok(ParsedTableSection {
            dispatchSection: Some(String::from("Tables")),
            ..ParsedTableSection::default()
        });
    }

    if table_section.starts_with("Tables('") && table_section.ends_with("')") {
        let table_name = table_section[8..table_section.len().saturating_sub(2)].to_string();
        let dispatchSection = if is_get {
            Some(format!("{table_name}()"))
        } else {
            Some(table_section.to_string())
        };
        return Ok(ParsedTableSection {
            dispatchSection,
            tableName: Some(table_name),
            ..ParsedTableSection::default()
        });
    }

    if !table_section.contains('(') && !table_section.contains(')') {
        let table_name = table_section.to_string();
        let dispatchSection = if is_get {
            Some(format!("{table_name}()"))
        } else {
            Some(table_name.clone())
        };
        return Ok(ParsedTableSection {
            dispatchSection,
            tableName: Some(table_name),
            ..ParsedTableSection::default()
        });
    }

    if table_section.contains('(')
        && table_section.contains(')')
        && table_section.contains("PartitionKey='")
        && table_section.contains("RowKey='")
    {
        let table_name = table_section
            .split_once('(')
            .map(|(name, _)| name.to_string())
            .unwrap_or_default();
        let matches = QUOTED_KEY_REGEX
            .find_iter(table_section)
            .map(|capture| decode_quoted_key(capture.as_str()))
            .collect::<Vec<_>>();
        return Ok(ParsedTableSection {
            dispatchSection: Some(format!(
                "{table_name}(PartitionKey='PLACEHOLDER',RowKey='PLACEHOLDER')"
            )),
            tableName: Some(table_name),
            partitionKey: matches.first().cloned(),
            rowKey: matches.get(1).cloned(),
        });
    }

    if let (Some(open_index), Some(close_index)) =
        (table_section.find('('), table_section.find(')'))
    {
        if close_index == open_index + 1 {
            return Ok(ParsedTableSection {
                dispatchSection: Some(table_section.to_string()),
                tableName: Some(table_section[..open_index].to_string()),
                ..ParsedTableSection::default()
            });
        }
    }

    Err(format!(
        "tableStorageContextMiddleware: Cannot extract table name from URL path={table_section}"
    ))
}

fn decode_quoted_key(value: &str) -> String {
    value
        .strip_prefix('\'')
        .and_then(|value| value.strip_suffix('\''))
        .unwrap_or(value)
        .replace("''", "'")
}

fn extract_request_host(req: &GeneratedHttpRequest) -> String {
    extract_host_candidate(&req.getUrl())
        .or_else(|| extract_host_candidate(&req.getEndpoint()))
        .or_else(|| {
            req.getEndpoint()
                .split_once("://")
                .map(|(_, value)| value.to_string())
        })
        .unwrap_or_default()
}

fn extract_host_candidate(value: &str) -> Option<String> {
    let without_scheme = value
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(value);
    let authority = without_scheme.split('/').next().unwrap_or_default();
    let authority = authority.rsplit('@').next().unwrap_or(authority);
    if authority.is_empty() {
        return None;
    }

    if authority.starts_with('[') {
        return authority
            .find(']')
            .map(|end| authority[1..end].to_string())
            .or_else(|| Some(authority.to_string()));
    }

    Some(authority.split(':').next().unwrap_or(authority).to_string())
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
    use azurite_common::i_logger::ILogger;

    use crate::{
        context::TableStorageContext,
        generated::{
            context::Context,
            i_request::{GeneratedHttpRequest, HttpMethod, RequestHeaderValue},
            i_response::{GeneratedHttpResponse, IResponse},
        },
        utils::constants::{HeaderConstants, VERSION},
    };

    use super::{
        createTableStorageContextMiddleware, extractStoragePartsFromPath,
        internalTableStorageContextMiddleware, TableStorageContextMiddlewareOptions,
    };

    #[derive(Default)]
    struct TestLogger;

    impl ILogger for TestLogger {
        fn error(&self, _message: &str, _contextID: Option<&str>) {}
        fn warn(&self, _message: &str, _contextID: Option<&str>) {}
        fn info(&self, _message: &str, _contextID: Option<&str>) {}
        fn verbose(&self, _message: &str, _contextID: Option<&str>) {}
        fn debug(&self, _message: &str, _contextID: Option<&str>) {}
    }

    #[test]
    fn extracts_path_style_table_paths() {
        let (account, table, is_secondary) =
            extractStoragePartsFromPath("127.0.0.1", "/devstoreaccount1/Tables('mytable')", None);

        assert_eq!(account.as_deref(), Some("devstoreaccount1"));
        assert_eq!(table.as_deref(), Some("Tables('mytable')"));
        assert!(!is_secondary);
    }

    #[test]
    fn extracts_product_style_table_paths() {
        let (account, table, is_secondary) =
            extractStoragePartsFromPath("devstoreaccount1.table.localhost", "/mytable()", None);

        assert_eq!(account.as_deref(), Some("devstoreaccount1"));
        assert_eq!(table.as_deref(), Some("mytable()"));
        assert!(!is_secondary);
    }

    #[test]
    fn detects_secondary_accounts() {
        let (account, table, is_secondary) =
            extractStoragePartsFromPath("127.0.0.1", "/devstoreaccount1-secondary/mytable", None);

        assert_eq!(account.as_deref(), Some("devstoreaccount1"));
        assert_eq!(table.as_deref(), Some("mytable"));
        assert!(is_secondary);
    }

    #[test]
    fn options_default_to_disabled_flags() {
        let options = TableStorageContextMiddlewareOptions::default();
        let _ = Context::default();
        assert!(!options.skipApiVersionCheck);
        assert!(!options.disableProductStyleUrl);

        let created = createTableStorageContextMiddleware(None, None);
        assert_eq!(created, options);
    }

    #[test]
    fn internal_middleware_parses_entity_keys_and_dispatch_pattern() {
        let context = Context::default();
        let path = "/devstoreaccount1/mytable(PartitionKey='pk',RowKey='r''k')";
        let mut request = GeneratedHttpRequest::with_details(
            HttpMethod::GET,
            format!("http://127.0.0.1:10002{path}"),
            "http://127.0.0.1:10002",
            path,
        );
        request.headers.insert(
            String::from("accept"),
            RequestHeaderValue::Single(String::from("application/json")),
        );
        let mut response = GeneratedHttpResponse::default();

        internalTableStorageContextMiddleware(
            &context,
            &request,
            &mut response,
            "127.0.0.1",
            path,
            &TestLogger,
            false,
            false,
        )
        .unwrap();

        let table_context = TableStorageContext::new(&context);
        assert_eq!(table_context.account().as_deref(), Some("devstoreaccount1"));
        assert_eq!(table_context.tableName().as_deref(), Some("mytable"));
        assert_eq!(table_context.partitionKey().as_deref(), Some("pk"));
        assert_eq!(table_context.rowKey().as_deref(), Some("r'k"));
        assert_eq!(
            context.dispatchPattern().as_deref(),
            Some("/mytable(PartitionKey='PLACEHOLDER',RowKey='PLACEHOLDER')")
        );
        assert_eq!(table_context.authenticationPath().as_deref(), Some(path));
        assert_eq!(table_context.accept().as_deref(), Some("application/json"));
        assert!(context.contextId().is_some());
        assert_eq!(
            response
                .getHeader(HeaderConstants.SERVER)
                .and_then(|value| value.as_single()),
            Some(format!("Azurite-Table/{VERSION}"))
        );
    }

    #[test]
    fn internal_middleware_rewrites_get_table_lookup_to_query_dispatch_pattern() {
        let context = Context::default();
        let path = "/devstoreaccount1/Tables('mytable')";
        let request = GeneratedHttpRequest::with_details(
            HttpMethod::GET,
            format!("http://127.0.0.1:10002{path}"),
            "http://127.0.0.1:10002",
            path,
        );
        let mut response = GeneratedHttpResponse::default();

        internalTableStorageContextMiddleware(
            &context,
            &request,
            &mut response,
            "127.0.0.1",
            path,
            &TestLogger,
            false,
            false,
        )
        .unwrap();

        let table_context = TableStorageContext::new(&context);
        assert_eq!(table_context.tableName().as_deref(), Some("mytable"));
        assert_eq!(context.dispatchPattern().as_deref(), Some("/mytable()"));
    }

    #[test]
    fn internal_middleware_rejects_invalid_api_version() {
        let context = Context::default();
        let path = "/devstoreaccount1/Tables";
        let mut request = GeneratedHttpRequest::with_details(
            HttpMethod::GET,
            format!("http://127.0.0.1:10002{path}"),
            "http://127.0.0.1:10002",
            path,
        );
        request.headers.insert(
            String::from("x-ms-version"),
            RequestHeaderValue::Single(String::from("1900-01-01")),
        );
        let mut response = GeneratedHttpResponse::default();

        let error = internalTableStorageContextMiddleware(
            &context,
            &request,
            &mut response,
            "127.0.0.1",
            path,
            &TestLogger,
            false,
            false,
        )
        .unwrap_err();

        assert_eq!(error.storageErrorCode, "InvalidHeaderValue");
    }
}
