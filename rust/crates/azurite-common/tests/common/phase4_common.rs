#![allow(non_snake_case)]
#![allow(clippy::incompatible_msrv)]
#![allow(clippy::assertions_on_constants)]
#![allow(clippy::await_holding_lock)]

use std::{
    any::Any,
    fs,
    io::Read,
    panic::{self, AssertUnwindSafe},
    sync::{Arc, LazyLock, Mutex},
};

use async_trait::async_trait;
use axum::Router;
use azurite_common::{
    account_data_store::AccountDataStore,
    configuration_base::{setExtentMemoryLimit, CertMaterial, CertOptions, ConfigurationBase},
    environment::Environment,
    i_account_data_store::IAccountDataStore,
    i_cleaner::ICleaner,
    i_data_store::IDataStore,
    i_environment::IEnvironment,
    i_logger::ILogger,
    i_logger_strategy::{ILoggerStrategy, LogLevels},
    logger::{configLogger, logger},
    models::OAuthLevel,
    server_base::{RequestListener, ServerBase, ServerStatus},
    storage_error::StorageError,
    utils::{
        buffer_stream::BufferStream,
        constants::{
            AZURITE_ACCOUNTS_ENV, BEARER_TOKEN_PREFIX, DEFAULT_ACCOUNTS_REFRESH_INTERVAL,
            DEFAULT_EXTENT_GC_PROTECT_TIME_IN_MS, DEFAULT_FD_CACHE_NUMBER, DEFAULT_MAX_EXTENT_SIZE,
            DEFAULT_READ_CONCURRENCY, DEFAULT_SQL_CHARSET, DEFAULT_SQL_COLLATE,
            DEFAULT_SQL_OPTIONS, EMULATOR_ACCOUNT_KEY, EMULATOR_ACCOUNT_KEY_STR,
            EMULATOR_ACCOUNT_NAME, FD_CACHE_NUMBER_MAX, FD_CACHE_NUMBER_MIN, HTTPS, IP_REGEX,
            NO_ACCOUNT_HOST_NAMES, VALID_CSHARP_IDENTIFIER_REGEX, VALID_ISSUE_PREFIXES,
        },
        utils::{
            computeHMACSHA256, convertDateTimeStringMsTo7Digital, convertRawHeadersToMetadata,
            getMD5FromStream, getMD5FromString, getURLQueries, lfsa, minDate, newEtag,
            truncatedISO8061Date,
        },
    },
    winston_logger_strategy::WinstonLoggerStrategy,
};
use bytes::Bytes;
use chrono::{Duration, TimeZone, Utc};
use tempfile::tempdir;

static ACCOUNT_ENV_GUARD: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
static LOGGER_GUARD: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[derive(Clone, Debug, Eq, PartialEq)]
struct RecordedLog {
    level: LogLevels,
    message: String,
    context_id: Option<String>,
}

#[derive(Default)]
struct RecordingLogger {
    entries: Mutex<Vec<RecordedLog>>,
}

impl RecordingLogger {
    fn entries(&self) -> Vec<RecordedLog> {
        self.entries
            .lock()
            .expect("recording logger mutex poisoned")
            .clone()
    }
}

impl ILogger for RecordingLogger {
    fn error(&self, message: &str, contextID: Option<&str>) {
        self.entries
            .lock()
            .expect("recording logger mutex poisoned")
            .push(RecordedLog {
                level: LogLevels::Error,
                message: message.to_owned(),
                context_id: contextID.map(str::to_owned),
            });
    }

    fn warn(&self, message: &str, contextID: Option<&str>) {
        self.entries
            .lock()
            .expect("recording logger mutex poisoned")
            .push(RecordedLog {
                level: LogLevels::Warn,
                message: message.to_owned(),
                context_id: contextID.map(str::to_owned),
            });
    }

    fn info(&self, message: &str, contextID: Option<&str>) {
        self.entries
            .lock()
            .expect("recording logger mutex poisoned")
            .push(RecordedLog {
                level: LogLevels::Info,
                message: message.to_owned(),
                context_id: contextID.map(str::to_owned),
            });
    }

    fn verbose(&self, message: &str, contextID: Option<&str>) {
        self.entries
            .lock()
            .expect("recording logger mutex poisoned")
            .push(RecordedLog {
                level: LogLevels::Verbose,
                message: message.to_owned(),
                context_id: contextID.map(str::to_owned),
            });
    }

    fn debug(&self, message: &str, contextID: Option<&str>) {
        self.entries
            .lock()
            .expect("recording logger mutex poisoned")
            .push(RecordedLog {
                level: LogLevels::Debug,
                message: message.to_owned(),
                context_id: contextID.map(str::to_owned),
            });
    }
}

#[derive(Default)]
struct ExtentLimitEnvironment {
    in_memory_persistence: bool,
    extent_memory_limit: Option<f64>,
}

#[async_trait]
impl IEnvironment for ExtentLimitEnvironment {
    fn blobHost(&self) -> Option<String> {
        None
    }

    fn blobPort(&self) -> Option<u16> {
        None
    }

    fn blobKeepAliveTimeout(&self) -> Option<u64> {
        None
    }

    fn queueHost(&self) -> Option<String> {
        None
    }

    fn queuePort(&self) -> Option<u16> {
        None
    }

    fn queueKeepAliveTimeout(&self) -> Option<u64> {
        None
    }

    fn tableHost(&self) -> Option<String> {
        None
    }

    fn tablePort(&self) -> Option<u16> {
        None
    }

    fn tableKeepAliveTimeout(&self) -> Option<u64> {
        None
    }

    async fn location(&self) -> Result<String, StorageError> {
        Ok(String::new())
    }

    fn silent(&self) -> bool {
        false
    }

    fn loose(&self) -> bool {
        false
    }

    fn skipApiVersionCheck(&self) -> bool {
        false
    }

    fn disableProductStyleUrl(&self) -> bool {
        false
    }

    fn cert(&self) -> Option<String> {
        None
    }

    fn key(&self) -> Option<String> {
        None
    }

    fn pwd(&self) -> Option<String> {
        None
    }

    async fn debug(&self) -> Result<Option<String>, StorageError> {
        Ok(None)
    }

    fn oauth(&self) -> Option<String> {
        None
    }

    fn inMemoryPersistence(&self) -> bool {
        self.in_memory_persistence
    }

    fn extentMemoryLimit(&self) -> Option<f64> {
        self.extent_memory_limit
    }

    fn disableTelemetry(&self) -> bool {
        false
    }
}

struct TestRequestListenerFactory;

impl azurite_common::i_request_listener_factory::IRequestListenerFactory
    for TestRequestListenerFactory
{
    fn createRequestListener(&self) -> RequestListener {
        Router::new()
    }
}

struct ScopedEnvVar {
    key: &'static str,
    previous: Option<String>,
}

impl ScopedEnvVar {
    fn set(key: &'static str, value: Option<&str>) -> Self {
        let previous = std::env::var(key).ok();
        unsafe {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        Self { key, previous }
    }
}

impl Drop for ScopedEnvVar {
    fn drop(&mut self) {
        unsafe {
            if let Some(previous) = self.previous.as_deref() {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }
}

fn panic_message(payload: Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else {
        "non-string panic payload".to_owned()
    }
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn phase4_constants_match_typescript_values() {
    assert_eq!(AZURITE_ACCOUNTS_ENV, "AZURITE_ACCOUNTS");
    assert_eq!(DEFAULT_ACCOUNTS_REFRESH_INTERVAL, 60 * 1000);
    assert_eq!(DEFAULT_FD_CACHE_NUMBER, 100);
    assert_eq!(FD_CACHE_NUMBER_MIN, 1);
    assert_eq!(FD_CACHE_NUMBER_MAX, 100);
    assert_eq!(DEFAULT_MAX_EXTENT_SIZE, 64 * 1024 * 1024);
    assert_eq!(DEFAULT_READ_CONCURRENCY, 100);
    assert_eq!(DEFAULT_EXTENT_GC_PROTECT_TIME_IN_MS, 10 * 60 * 1000);
    assert_eq!(DEFAULT_SQL_CHARSET, "utf8mb4");
    assert_eq!(DEFAULT_SQL_COLLATE, "utf8mb4_bin");
    assert_eq!(DEFAULT_SQL_OPTIONS.charset, DEFAULT_SQL_CHARSET);
    assert_eq!(DEFAULT_SQL_OPTIONS.collate, DEFAULT_SQL_COLLATE);
    assert!(!DEFAULT_SQL_OPTIONS.logging);
    assert_eq!(DEFAULT_SQL_OPTIONS.pool.max, 20);
    assert_eq!(DEFAULT_SQL_OPTIONS.pool.min, 0);
    assert_eq!(DEFAULT_SQL_OPTIONS.pool.acquire, 30000);
    assert_eq!(DEFAULT_SQL_OPTIONS.pool.idle, 10000);
    assert_eq!(DEFAULT_SQL_OPTIONS.dialectOptions.timezone, "+00:00");
    assert_eq!(BEARER_TOKEN_PREFIX, "Bearer");
    assert_eq!(HTTPS, "https");
    assert_eq!(EMULATOR_ACCOUNT_NAME, "devstoreaccount1");
    assert_eq!(
        EMULATOR_ACCOUNT_KEY_STR,
        "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw=="
    );
    assert_eq!(EMULATOR_ACCOUNT_KEY.len(), 64);
    assert!(IP_REGEX.is_match("127.0.0.1"));
    assert!(IP_REGEX.is_match("256.1.1.1"));
    assert!(!IP_REGEX.is_match("example.com"));
    assert!(NO_ACCOUNT_HOST_NAMES.contains("host.docker.internal"));
    assert_eq!(
        VALID_ISSUE_PREFIXES,
        [
            "https://sts.windows.net/",
            "https://sts.microsoftonline.de/",
            "https://sts.chinacloudapi.cn/",
            "https://sts.windows-ppe.net",
        ]
    );
    assert!(VALID_CSHARP_IDENTIFIER_REGEX.is_match("_metadata42"));
    assert!(!VALID_CSHARP_IDENTIFIER_REGEX.is_match("42metadata"));
}

#[test]
fn phase4_utility_helpers_match_typescript_edge_cases() {
    let earlier = Utc.with_ymd_and_hms(2024, 1, 2, 3, 4, 5).unwrap();
    let later = Utc.with_ymd_and_hms(2024, 1, 2, 3, 4, 6).unwrap();
    assert_eq!(minDate(later, earlier), earlier);
    assert_eq!(lfsa, "lokijs/src/loki-fs-structured-adapter.js");
    assert_eq!(
        convertDateTimeStringMsTo7Digital("2024-01-02T03:04:05.678ZtailZ"),
        "2024-01-02T03:04:05.6780000ZtailZ"
    );

    let raw_headers = vec![
        "x-ms-meta-name".to_owned(),
        "first".to_owned(),
        "Content-Type".to_owned(),
        "ignored".to_owned(),
        "x-ms-meta-name".to_owned(),
        "second".to_owned(),
        "x-ms-meta-empty".to_owned(),
    ];
    let metadata = convertRawHeadersToMetadata(&raw_headers, "ctx-1")
        .unwrap()
        .expect("metadata should be present");
    assert_eq!(metadata.get("name"), Some(&"first,second".to_owned()));
    assert_eq!(metadata.get("empty"), Some(&String::new()));
    assert_eq!(convertRawHeadersToMetadata(&[], "ctx-1").unwrap(), None);

    let error = convertRawHeadersToMetadata(
        &["x-ms-meta-1invalid".to_owned(), "value".to_owned()],
        "ctx-2",
    )
    .unwrap_err();
    assert_eq!(
        error.message,
        "The metadata specified is invalid. It has characters that are not permitted."
    );

    let etag = newEtag();
    assert!(etag.starts_with("\"0x"));
    assert!(etag.ends_with('"'));
    let hex = &etag[3..etag.len() - 1];
    assert!(hex.len() >= 15);
    assert!(hex
        .chars()
        .all(|character| character.is_ascii_digit() || matches!(character, 'A'..='F')));

    assert_eq!(
        computeHMACSHA256("to-sign", b"key-value"),
        "v4kEJ1gSDpddPUV2igLlJYnd6b6Lo7HU5nb/GJYS1Lw="
    );

    let date = Utc.with_ymd_and_hms(2024, 1, 2, 3, 4, 5).unwrap() + Duration::milliseconds(678);
    assert_eq!(
        truncatedISO8061Date(date, true, false),
        "2024-01-02T03:04:05.6780000Z"
    );
    assert_eq!(
        truncatedISO8061Date(date, false, false),
        "2024-01-02T03:04:05Z"
    );
    let hrtime = truncatedISO8061Date(date, false, true);
    assert!(hrtime.starts_with("2024-01-02T03:04:05.678"));
    assert!(hrtime.ends_with('Z'));
    assert_eq!(hrtime.len(), "2024-01-02T03:04:05.6780000Z".len());

    let queries = getURLQueries(
        "https://example.invalid/path?first=1&second=two=three&&=ignored&third=3#fragment",
    );
    assert_eq!(queries.len(), 2);
    assert_eq!(queries.get("first"), Some(&"1".to_owned()));
    assert_eq!(queries.get("third"), Some(&"3".to_owned()));
    assert_eq!(
        getURLQueries("https://example.invalid/path"),
        Default::default()
    );
}

#[tokio::test]
async fn phase4_md5_helpers_return_raw_digest_bytes() {
    let expected = "5eb63bbbe01eeed093cb22bb8f5acdc3";
    assert_eq!(to_hex(&getMD5FromString("hello world").await), expected);
    assert_eq!(
        to_hex(
            &getMD5FromStream(BufferStream::from_vec(b"hello world".to_vec()))
                .await
                .unwrap()
        ),
        expected
    );
}

#[test]
fn phase4_buffer_stream_sync_reads_use_typescript_chunk_boundaries() {
    let payload: Vec<u8> = (0..(64 * 1024 + 17))
        .map(|index| (index % 251) as u8)
        .collect();
    let mut stream = BufferStream::from_vec(payload.clone());

    let mut first_chunk = vec![0u8; 128 * 1024];
    let first_read = stream.read(&mut first_chunk).unwrap();
    assert_eq!(first_read, 64 * 1024);
    assert_eq!(&first_chunk[..first_read], &payload[..first_read]);

    let mut remainder = Vec::new();
    std::io::Read::read_to_end(&mut stream, &mut remainder).unwrap();

    let mut reconstructed = first_chunk[..first_read].to_vec();
    reconstructed.extend(remainder);
    assert_eq!(reconstructed, payload);
}

#[tokio::test]
async fn phase4_buffer_stream_async_reads_empty_and_multi_chunk_payloads() {
    let mut empty_stream = BufferStream::new(Bytes::new());
    let mut empty = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut empty_stream, &mut empty)
        .await
        .unwrap();
    assert!(empty.is_empty());

    let payload: Vec<u8> = (0..(64 * 1024 * 2 + 9))
        .map(|index| (index % 199) as u8)
        .collect();
    let mut stream = BufferStream::from_vec(payload.clone());
    let mut output = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut stream, &mut output)
        .await
        .unwrap();
    assert_eq!(output, payload);
}

#[test]
fn phase4_winston_logger_strategy_preserves_tab_default_and_level_filtering() {
    let temp = tempdir().unwrap();
    let log_path = temp.path().join("phase4-winston.log");
    let strategy = WinstonLoggerStrategy::new(
        LogLevels::Info,
        Some(log_path.to_string_lossy().into_owned()),
    );

    strategy.log(LogLevels::Debug, "ignored", None);
    strategy.log(LogLevels::Info, "hello", None);
    strategy.log(LogLevels::Warn, "warned", Some("ctx-7"));

    let lines: Vec<String> = fs::read_to_string(&log_path)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains(" \t info: hello"));
    assert!(lines[1].contains(" ctx-7 warn: warned"));
    assert!(!lines.iter().any(|line| line.contains("ignored")));
}

#[test]
fn phase4_global_logger_strategy_swapping_matches_typescript() {
    let _guard = LOGGER_GUARD.lock().unwrap();
    let temp = tempdir().unwrap();
    let log_path = temp.path().join("phase4-global.log");

    configLogger(true, Some(log_path.to_string_lossy().into_owned()));
    logger.info("enabled", Some("ctx-phase4"));
    configLogger(false, None);
    logger.info("disabled", Some("ctx-phase4"));

    let contents = fs::read_to_string(&log_path).unwrap();
    assert!(contents.contains("ctx-phase4 info: enabled"));
    assert!(!contents.contains("disabled"));
}

#[test]
fn phase4_configuration_base_helpers_match_typescript_behavior() {
    let temp = tempdir().unwrap();
    let cert_path = temp.path().join("cert.pem");
    let key_path = temp.path().join("key.pem");
    fs::write(&cert_path, b"certificate-bytes").unwrap();
    fs::write(&key_path, b"key-bytes").unwrap();

    let pem = ConfigurationBase::new(
        "127.0.0.1".to_owned(),
        10000,
        5,
        false,
        None,
        false,
        None,
        false,
        false,
        cert_path.to_string_lossy().into_owned(),
        key_path.to_string_lossy().into_owned(),
        "pfx-password".to_owned(),
        Some("BASIC".to_owned()),
        false,
    );
    assert_eq!(pem.hasCert(), CertOptions::PEM);
    assert_eq!(pem.getOAuthLevel(), Some(OAuthLevel::BASIC));
    assert_eq!(pem.getHttpServerAddress(), "https://127.0.0.1:10000");
    match pem.getCert(CertOptions::PEM).unwrap().unwrap() {
        CertMaterial::PEM { cert, key } => {
            assert_eq!(cert, b"certificate-bytes".to_vec());
            assert_eq!(key, b"key-bytes".to_vec());
        }
        other => panic!("expected PEM material, got {other:?}"),
    }

    let pfx = ConfigurationBase::new(
        "127.0.0.1".to_owned(),
        10001,
        5,
        false,
        None,
        false,
        None,
        false,
        false,
        cert_path.to_string_lossy().into_owned(),
        String::new(),
        "secret".to_owned(),
        Some("unsupported".to_owned()),
        false,
    );
    assert_eq!(pfx.hasCert(), CertOptions::PFX);
    assert_eq!(pfx.getOAuthLevel(), None);
    match pfx.getCert(CertOptions::PFX).unwrap().unwrap() {
        CertMaterial::PFX { pfx, passphrase } => {
            assert_eq!(pfx, b"certificate-bytes".to_vec());
            assert_eq!(passphrase, "secret");
        }
        other => panic!("expected PFX material, got {other:?}"),
    }

    assert_eq!(
        ConfigurationBase::default()
            .getCert(CertOptions::Default)
            .unwrap(),
        None
    );
}

#[test]
fn phase4_set_extent_memory_limit_rejects_negative_values() {
    let environment = ExtentLimitEnvironment {
        in_memory_persistence: true,
        extent_memory_limit: Some(-1.0),
    };

    let error = setExtentMemoryLimit(&environment, false).unwrap_err();
    assert_eq!(
        error.message,
        "A negative value of '-1' is not allowed for the extent memory limit."
    );
}

#[tokio::test]
async fn phase4_server_base_start_close_and_clean_match_typescript_lifecycle() {
    let mut server = ServerBase::new(
        "127.0.0.1".to_owned(),
        0,
        Arc::new(TestRequestListenerFactory),
        ConfigurationBase::default(),
    );

    assert_eq!(server.getStatus(), ServerStatus::Closed);
    assert_eq!(server.getHttpServerAddress(), "");

    server.start().await.unwrap();
    assert_eq!(server.getStatus(), ServerStatus::Running);
    let address = server.getHttpServerAddress();
    assert!(address.starts_with("http://127.0.0.1:"));
    assert_ne!(address, "http://127.0.0.1:0");

    server.clean().await.unwrap();
    server.close().await.unwrap();
    assert_eq!(server.getStatus(), ServerStatus::Closed);
}

#[tokio::test]
async fn phase4_server_base_rejects_invalid_state_transitions() {
    let mut server = ServerBase::new(
        "127.0.0.1".to_owned(),
        0,
        Arc::new(TestRequestListenerFactory),
        ConfigurationBase::default(),
    );

    let close_error = server.close().await.unwrap_err();
    assert_eq!(close_error.message, "Cannot close server in status Closed");

    server.start().await.unwrap();
    let start_error = server.start().await.unwrap_err();
    assert_eq!(start_error.message, "Cannot start server in status Running");

    server.close().await.unwrap();
}

#[tokio::test]
async fn phase4_account_data_store_parses_accounts_and_masks_logs() {
    let _guard = ACCOUNT_ENV_GUARD.lock().unwrap();
    let env_value = "alpha:AQID;beta:BAUG:BwgJ;";
    let _env = ScopedEnvVar::set(AZURITE_ACCOUNTS_ENV, Some(env_value));
    let recorder = Arc::new(RecordingLogger::default());
    let mut store = AccountDataStore::new(recorder.clone());

    store.init().await.unwrap();
    assert!(store.isInitialized());

    let alpha = store
        .getAccount("alpha")
        .expect("alpha account should exist");
    assert_eq!(alpha.name, "alpha");
    assert_eq!(alpha.key1, vec![1, 2, 3]);
    assert_eq!(alpha.key2, None);

    let beta = store.getAccount("beta").expect("beta account should exist");
    assert_eq!(beta.name, "beta");
    assert_eq!(beta.key1, vec![4, 5, 6]);
    assert_eq!(beta.key2, Some(vec![7, 8, 9]));

    let entries = recorder.entries();
    assert!(entries
        .iter()
        .any(|entry| entry.message.contains("value *****")));
    assert!(!entries
        .iter()
        .any(|entry| entry.message.contains(env_value)));

    store.close().await.unwrap();
    assert!(store.isClosed());
}

#[tokio::test]
async fn phase4_account_data_store_falls_back_to_emulator_account_on_invalid_env() {
    let _guard = ACCOUNT_ENV_GUARD.lock().unwrap();
    let _env = ScopedEnvVar::set(AZURITE_ACCOUNTS_ENV, Some("broken-format"));
    let recorder = Arc::new(RecordingLogger::default());
    let mut store = AccountDataStore::new(recorder.clone());

    store.init().await.unwrap();
    let emulator = store
        .getAccount(EMULATOR_ACCOUNT_NAME)
        .expect("default emulator account should be restored");
    assert_eq!(emulator.name, EMULATOR_ACCOUNT_NAME);
    assert_eq!(emulator.key1, EMULATOR_ACCOUNT_KEY.clone());
    assert_eq!(emulator.key2, None);
    assert_eq!(store.getAccount("broken-format"), None);
    assert!(recorder.entries().iter().any(|entry| {
        entry.level == LogLevels::Error
            && entry
                .message
                .contains("Fallback to default emulator account devstoreaccount1")
    }));

    store.close().await.unwrap();
}

#[test]
fn phase4_environment_duplicate_cli_arguments_use_last_value() {
    let environment = Environment::new(vec![
        "--blobHost".to_owned(),
        "127.0.0.1".to_owned(),
        "--blobHost".to_owned(),
        "127.0.0.2".to_owned(),
        "--blobPort".to_owned(),
        "10000".to_owned(),
        "--blobPort".to_owned(),
        "12000".to_owned(),
    ]);

    assert_eq!(environment.blobHost(), Some("127.0.0.2".to_owned()));
    assert_eq!(environment.blobPort(), Some(12000));
}

#[tokio::test]
async fn phase4_environment_defaults_sentinels_and_bare_debug_match_typescript() {
    let defaults = Environment::default();
    assert_eq!(defaults.blobHost(), Some("127.0.0.1".to_owned()));
    assert_eq!(defaults.blobPort(), Some(10000));
    assert_eq!(defaults.queueHost(), Some("127.0.0.1".to_owned()));
    assert_eq!(defaults.queuePort(), Some(10001));
    assert_eq!(defaults.tableHost(), Some("127.0.0.1".to_owned()));
    assert_eq!(defaults.tablePort(), Some(10002));
    assert_eq!(defaults.blobKeepAliveTimeout(), Some(5));
    assert_eq!(defaults.queueKeepAliveTimeout(), Some(5));
    assert_eq!(defaults.tableKeepAliveTimeout(), Some(5));
    assert!(!defaults.silent());
    assert!(!defaults.loose());
    assert!(!defaults.skipApiVersionCheck());
    assert!(!defaults.disableProductStyleUrl());
    assert!(!defaults.disableTelemetry());
    assert_eq!(defaults.extentMemoryLimit(), None);

    let sentinel = Environment::new(vec![
        "--location".to_owned(),
        "<cwd>".to_owned(),
        "--extentMemoryLimit".to_owned(),
        "-1".to_owned(),
    ]);
    assert_eq!(
        sentinel.location().await.unwrap(),
        std::env::current_dir().unwrap().display().to_string()
    );
    assert_eq!(sentinel.extentMemoryLimit(), None);

    let debug_error = Environment::new(vec!["--debug".to_owned()])
        .debug()
        .await
        .unwrap_err();
    assert_eq!(
        debug_error.message,
        "Must provide a debug log file path for parameter -d or --debug"
    );
}

#[test]
fn phase4_environment_validation_remains_lazy_and_getter_triggered() {
    let with_location = Environment::new(vec![
        "--inMemoryPersistence".to_owned(),
        "--location".to_owned(),
        "/tmp/azurite".to_owned(),
    ]);
    let panic = panic::catch_unwind(AssertUnwindSafe(|| with_location.inMemoryPersistence()))
        .expect_err("validation should panic only when inMemoryPersistence() is called");
    assert!(panic_message(panic).contains(
        "The --inMemoryPersistence option is not supported when the --location option is set."
    ));

    let without_in_memory =
        Environment::new(vec!["--extentMemoryLimit".to_owned(), "256".to_owned()]);
    let panic = panic::catch_unwind(AssertUnwindSafe(|| without_in_memory.inMemoryPersistence()))
        .expect_err("extentMemoryLimit should only fail when inMemoryPersistence() is queried");
    assert!(panic_message(panic).contains(
        "The --extentMemoryLimit option is only supported when the --inMemoryPersistence option is set."
    ));
}
