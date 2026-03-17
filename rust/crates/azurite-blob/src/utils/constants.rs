use std::{collections::BTreeMap, env, sync::LazyLock};

use azurite_common::persistence::i_extent_store::{
    IStoreDestinationConfigure, StoreDestinationArray,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use regex::Regex;

pub const VERSION: &str = "3.35.0";
pub const BLOB_API_VERSION: &str = "2025-11-05";
pub const DEFAULT_BLOB_SERVER_HOST_NAME: &str = "127.0.0.1";
pub const DEFAULT_LIST_BLOBS_MAX_RESULTS: u32 = 5000;
pub const DEFAULT_LIST_CONTAINERS_MAX_RESULTS: u32 = 5000;
pub const DEFAULT_BLOB_LISTENING_PORT: u16 = 10000;
pub static IS_PRODUCTION: LazyLock<bool> = LazyLock::new(|| {
    env::var("NODE_ENV")
        .map(|value| value == "production")
        .unwrap_or(false)
});
pub const DEFAULT_BLOB_LOKI_DB_PATH: &str = "__azurite_db_blob__.json";
pub const DEFAULT_BLOB_EXTENT_LOKI_DB_PATH: &str = "__azurite_db_blob_extent__.json";
pub const DEFAULT_BLOB_PERSISTENCE_PATH: &str = "__blobstorage__";
pub const DEFAULT_DEBUG_LOG_PATH: &str = "./debug.log";
pub const DEFAULT_ENABLE_DEBUG_LOG: bool = true;
pub const DEFAULT_ACCESS_LOG_PATH: &str = "./access.log";
pub const DEFAULT_ENABLE_ACCESS_LOG: bool = true;
pub const DEFAULT_CONTEXT_PATH: &str = "azurite_blob_context";
pub static LOGGER_CONFIGS: LazyLock<BTreeMap<String, String>> = LazyLock::new(BTreeMap::new);
pub const DEFAULT_GC_INTERVAL_MS: u64 = 10 * 60 * 1000;
pub const DEFAULT_WRITE_CONCURRENCY_PER_LOCATION: usize = 50;
pub const EMULATOR_ACCOUNT_NAME: &str = "devstoreaccount1";
pub const EMULATOR_ACCOUNT_KEY_STR: &str =
    "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==";
pub static EMULATOR_ACCOUNT_KEY: LazyLock<Vec<u8>> = LazyLock::new(|| {
    STANDARD
        .decode(EMULATOR_ACCOUNT_KEY_STR)
        .unwrap_or_default()
});
pub const EMULATOR_ACCOUNT_SKUNAME: &str = "Standard_RAGRS";
pub const EMULATOR_ACCOUNT_KIND: &str = "StorageV2";
pub const EMULATOR_ACCOUNT_ISHIERARCHICALNAMESPACEENABLED: bool = false;
pub const DEFAULT_BLOB_KEEP_ALIVE_TIMEOUT: u64 = 5;
pub const SECONDARY_SUFFIX: &str = "-secondary";
pub static DEFAULT_BLOB_PERSISTENCE_ARRAY: LazyLock<StoreDestinationArray> = LazyLock::new(|| {
    vec![IStoreDestinationConfigure {
        locationId: "Default".to_string(),
        locationPath: DEFAULT_BLOB_PERSISTENCE_PATH.to_string(),
        maxConcurrency: DEFAULT_WRITE_CONCURRENCY_PER_LOCATION,
    }]
});
pub const MAX_APPEND_BLOB_BLOCK_SIZE: u64 = 100 * 1024 * 1024;
pub const MAX_APPEND_BLOB_BLOCK_COUNT: u32 = 50000;
pub static VALID_BLOB_AUDIENCES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"^https://storage\.azure\.com[/]?$").unwrap(),
        Regex::new(r"^e406a681-f3d4-42a8-90b6-c2b029497af1$").unwrap(),
        Regex::new(r"^https://(.*)\.blob\.core\.windows\.net[/]?$").unwrap(),
        Regex::new(r"^https://(.*)\.blob\.core\.chinacloudapi\.cn[/]?$").unwrap(),
        Regex::new(r"^https://(.*)\.blob\.core\.usgovcloudapi\.net[/]?$").unwrap(),
        Regex::new(r"^https://(.*)\.blob\.core\.cloudapi\.de[/]?$").unwrap(),
    ]
});
pub const HTTP_LINE_ENDING: &str = "\r\n";
pub const HTTP_HEADER_DELIMITER: &str = ": ";
pub const USERDELEGATIONKEY_BASIC_KEY: &str =
    "I17GKLvcJUossaebtsEDZZ2RJ8GNLwLH4m7hRMxbVbkx6wNIRAABj4Rtw0FBhFuEAgmbL4gFMzUw+AStz9Sqdg==";
pub const AUTHENTICATION_BEARERTOKEN_REQUIRED: &str =
    "Only authentication scheme Bearer is supported";
pub const ValidAPIVersions: [&str; 45] = [
    "2025-11-05",
    "2025-07-05",
    "2025-05-05",
    "2025-01-05",
    "2024-11-04",
    "2024-08-04",
    "2024-05-04",
    "2024-02-04",
    "2023-11-03",
    "2023-08-03",
    "2023-01-03",
    "2022-11-02",
    "2021-12-02",
    "2021-10-04",
    "2021-08-06",
    "2021-06-08",
    "2021-04-10",
    "2021-02-12",
    "2020-12-06",
    "2020-10-02",
    "2020-08-04",
    "2020-06-12",
    "2020-04-08",
    "2020-02-10",
    "2019-12-12",
    "2019-10-10",
    "2019-07-07",
    "2019-02-02",
    "2018-11-09",
    "2018-03-28",
    "2017-11-09",
    "2017-07-29",
    "2017-04-17",
    "2016-05-31",
    "2015-12-11",
    "2015-07-08",
    "2015-04-05",
    "2015-02-21",
    "2014-02-14",
    "2013-08-15",
    "2012-02-12",
    "2011-08-18",
    "2009-09-19",
    "2009-07-17",
    "2009-04-14",
];

pub struct HeaderConstants;

impl HeaderConstants {
    pub const AUTHORIZATION: &'static str = "authorization";
    pub const AUTHORIZATION_SCHEME: &'static str = "Bearer";
    pub const CONTENT_ENCODING: &'static str = "content-encoding";
    pub const CONTENT_LANGUAGE: &'static str = "content-language";
    pub const CONTENT_LENGTH: &'static str = "content-length";
    pub const CONTENT_MD5: &'static str = "content-md5";
    pub const CONTENT_TYPE: &'static str = "content-type";
    pub const COOKIE: &'static str = "Cookie";
    pub const DATE: &'static str = "date";
    pub const IF_MATCH: &'static str = "if-match";
    pub const IF_MODIFIED_SINCE: &'static str = "if-modified-since";
    pub const IF_NONE_MATCH: &'static str = "if-none-match";
    pub const IF_UNMODIFIED_SINCE: &'static str = "if-unmodified-since";
    pub const SOURCE_IF_MATCH: &'static str = "x-ms-source-if-match";
    pub const SOURCE_IF_MODIFIED_SINCE: &'static str = "x-ms-source-if-modified-since";
    pub const SOURCE_IF_NONE_MATCH: &'static str = "x-ms-source-if-none-match";
    pub const SOURCE_IF_UNMODIFIED_SINCE: &'static str = "x-ms-source-if-unmodified-since";
    pub const X_MS_IF_SEQUENCE_NUMBER_LE: &'static str = "x-ms-if-sequence-number-le";
    pub const X_MS_IF_SEQUENCE_NUMBER_LT: &'static str = "x-ms-if-sequence-number-lt";
    pub const X_MS_IF_SEQUENCE_NUMBER_EQ: &'static str = "x-ms-if-sequence-number-eq";
    pub const X_MS_BLOB_CONDITION_MAXSIZE: &'static str = "x-ms-blob-condition-maxsize";
    pub const X_MS_BLOB_CONDITION_APPENDPOS: &'static str = "x-ms-blob-condition-appendpos";
    pub const X_MS_SEQUENCE_NUMBER_ACTION: &'static str = "x-ms-sequence-number-action";
    pub const X_MS_BLOB_SEQUENCE_NUMBER: &'static str = "x-ms-blob-sequence-number";
    pub const X_MS_CONTENT_CRC64: &'static str = "x-ms-content-crc64";
    pub const X_MS_RANGE_GET_CONTENT_CRC64: &'static str = "x-ms-range-get-content-crc64";
    pub const X_MS_ENCRYPTION_KEY: &'static str = "x-ms-encryption-key";
    pub const X_MS_ENCRYPTION_KEY_SHA256: &'static str = "x-ms-encryption-key-sha256";
    pub const X_MS_ENCRYPTION_ALGORITHM: &'static str = "x-ms-encryption-algorithm";
    pub const PREFIX_FOR_STORAGE: &'static str = "x-ms-";
    pub const RANGE: &'static str = "Range";
    pub const USER_AGENT: &'static str = "User-Agent";
    pub const X_MS_CLIENT_REQUEST_ID: &'static str = "x-ms-client-request-id";
    pub const X_MS_DATE: &'static str = "x-ms-date";
    pub const SERVER: &'static str = "Server";
    pub const X_MS_META: &'static str = "x-ms-meta-";
    pub const X_MS_VERSION: &'static str = "x-ms-version";
    pub const ORIGIN: &'static str = "origin";
    pub const VARY: &'static str = "Vary";
    pub const ACCESS_CONTROL_EXPOSE_HEADERS: &'static str = "Access-Control-Expose-Headers";
    pub const ACCESS_CONTROL_ALLOW_ORIGIN: &'static str = "Access-Control-Allow-Origin";
    pub const ACCESS_CONTROL_ALLOW_CREDENTIALS: &'static str = "Access-Control-Allow-Credentials";
    pub const ACCESS_CONTROL_ALLOW_METHODS: &'static str = "Access-Control-Allow-Methods";
    pub const ACCESS_CONTROL_ALLOW_HEADERS: &'static str = "Access-Control-Allow-Headers";
    pub const ACCESS_CONTROL_MAX_AGE: &'static str = "Access-Control-Max-Age";
    pub const ACCESS_CONTROL_REQUEST_METHOD: &'static str = "access-control-request-method";
    pub const ACCESS_CONTROL_REQUEST_HEADERS: &'static str = "access-control-request-headers";
}

pub struct MethodConstants;

impl MethodConstants {
    pub const OPTIONS: &'static str = "OPTIONS";
}
