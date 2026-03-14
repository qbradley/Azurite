use std::{collections::BTreeMap, sync::LazyLock};

use azurite_common::persistence::i_extent_store::{
    IExtentChunk, IStoreDestinationConfigure, StoreDestinationArray,
};
use chrono::{DateTime, Utc};
use regex::Regex;

pub const VERSION: &str = "3.35.0";
pub const QUEUE_API_VERSION: &str = "2025-11-05";
pub const DEFAULT_QUEUE_SERVER_HOST_NAME: &str = "127.0.0.1";
pub const DEFAULT_QUEUE_LISTENING_PORT: u16 = 10001;
pub static IS_PRODUCTION: LazyLock<bool> =
    LazyLock::new(|| std::env::var("NODE_ENV").is_ok_and(|value| value == "production"));
pub const DEFAULT_QUEUE_LOKI_DB_PATH: &str = "__azurite_db_queue__.json";
pub const DEFAULT_QUEUE_EXTENT_LOKI_DB_PATH: &str = "__azurite_db_queue_extent__.json";
pub const DEFAULT_QUEUE_PERSISTENCE_PATH: &str = "__queuestorage__";
pub const DEFAULT_DEBUG_LOG_PATH: &str = "./debug.log";
pub const DEFAULT_ENABLE_DEBUG_LOG: bool = true;
pub const DEFAULT_ACCESS_LOG_PATH: &str = "./access.log";
pub const DEFAULT_ENABLE_ACCESS_LOG: bool = true;
pub const DEFAULT_QUEUE_CONTEXT_PATH: &str = "azurite_queue_context";
pub static LOGGER_CONFIGS: LazyLock<BTreeMap<String, String>> = LazyLock::new(BTreeMap::new);
pub const DEFAULT_GC_INTERVAL_MS: u64 = 60 * 1000;
pub static NEVER_EXPIRE_DATE: LazyLock<DateTime<Utc>> = LazyLock::new(|| {
    DateTime::parse_from_rfc3339("9999-12-31T23:59:59.999Z")
        .unwrap()
        .with_timezone(&Utc)
});
pub const QUEUE_SERVICE_PERMISSION: &str = "raup";
pub const LIST_QUEUE_MAXRESULTS_MIN: i32 = 1;
pub const LIST_QUEUE_MAXRESULTS_MAX: i32 = 2147483647;
pub const DEFAULT_DEQUEUE_VISIBILITYTIMEOUT: i32 = 30;
pub const DEQUEUE_VISIBILITYTIMEOUT_MIN: i32 = 1;
pub const DEQUEUE_VISIBILITYTIMEOUT_MAX: i32 = 604800;
pub const DEQUEUE_NUMOFMESSAGES_MIN: i32 = 1;
pub const DEQUEUE_NUMOFMESSAGES_MAX: i32 = 32;
pub const MESSAGETEXT_LENGTH_MAX: usize = 65536;
pub const DEFAULT_MESSAGETTL: i32 = 604800;
pub const ENQUEUE_VISIBILITYTIMEOUT_MIN: i32 = 0;
pub const ENQUEUE_VISIBILITYTIMEOUT_MAX: i32 = 604800;
pub const MESSAGETTL_MIN: i32 = 1;
pub const DEFAULT_UPDATE_VISIBILITYTIMEOUT: i32 = 30;
pub const UPDATE_VISIBILITYTIMEOUT_MIN: i32 = 0;
pub const UPDATE_VISIBILITYTIMEOUT_MAX: i32 = 604800;

pub const DEFAULT_QUEUE_KEEP_ALIVE_TIMEOUT: u64 = 5;

pub static EMPTY_EXTENT_CHUNK: LazyLock<IExtentChunk> = LazyLock::new(|| IExtentChunk {
    id: String::new(),
    offset: 0,
    count: 0,
});

#[allow(non_snake_case)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MethodConstantsType {
    pub OPTIONS: &'static str,
}

pub const MethodConstants: MethodConstantsType = MethodConstantsType { OPTIONS: "OPTIONS" };

#[allow(non_snake_case)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HeaderConstantsType {
    pub AUTHORIZATION: &'static str,
    pub AUTHORIZATION_SCHEME: &'static str,
    pub CONTENT_ENCODING: &'static str,
    pub CONTENT_LANGUAGE: &'static str,
    pub CONTENT_LENGTH: &'static str,
    pub CONTENT_MD5: &'static str,
    pub CONTENT_TYPE: &'static str,
    pub COOKIE: &'static str,
    pub DATE: &'static str,
    pub IF_MATCH: &'static str,
    pub IF_MODIFIED_SINCE: &'static str,
    pub IF_NONE_MATCH: &'static str,
    pub IF_UNMODIFIED_SINCE: &'static str,
    pub PREFIX_FOR_STORAGE: &'static str,
    pub RANGE: &'static str,
    pub USER_AGENT: &'static str,
    pub X_MS_CLIENT_REQUEST_ID: &'static str,
    pub X_MS_DATE: &'static str,
    pub SERVER: &'static str,
    pub X_MS_META: &'static str,
    pub ORIGIN: &'static str,
    pub VARY: &'static str,
    pub ACCESS_CONTROL_EXPOSE_HEADERS: &'static str,
    pub ACCESS_CONTROL_ALLOW_ORIGIN: &'static str,
    pub ACCESS_CONTROL_ALLOW_CREDENTIALS: &'static str,
    pub ACCESS_CONTROL_ALLOW_METHODS: &'static str,
    pub ACCESS_CONTROL_ALLOW_HEADERS: &'static str,
    pub ACCESS_CONTROL_MAX_AGE: &'static str,
    pub ACCESS_CONTROL_REQUEST_METHOD: &'static str,
    pub ACCESS_CONTROL_REQUEST_HEADERS: &'static str,
    pub X_MS_VERSION: &'static str,
}

pub const HeaderConstants: HeaderConstantsType = HeaderConstantsType {
    AUTHORIZATION: "authorization",
    AUTHORIZATION_SCHEME: "Bearer",
    CONTENT_ENCODING: "content-encoding",
    CONTENT_LANGUAGE: "content-language",
    CONTENT_LENGTH: "content-length",
    CONTENT_MD5: "content-md5",
    CONTENT_TYPE: "content-type",
    COOKIE: "Cookie",
    DATE: "date",
    IF_MATCH: "if-match",
    IF_MODIFIED_SINCE: "if-modified-since",
    IF_NONE_MATCH: "if-none-match",
    IF_UNMODIFIED_SINCE: "if-unmodified-since",
    PREFIX_FOR_STORAGE: "x-ms-",
    RANGE: "Range",
    USER_AGENT: "User-Agent",
    X_MS_CLIENT_REQUEST_ID: "x-ms-client-request-id",
    X_MS_DATE: "x-ms-date",
    SERVER: "Server",
    X_MS_META: "x-ms-meta-",
    ORIGIN: "origin",
    VARY: "Vary",
    ACCESS_CONTROL_EXPOSE_HEADERS: "Access-Control-Expose-Headers",
    ACCESS_CONTROL_ALLOW_ORIGIN: "Access-Control-Allow-Origin",
    ACCESS_CONTROL_ALLOW_CREDENTIALS: "Access-Control-Allow-Credentials",
    ACCESS_CONTROL_ALLOW_METHODS: "Access-Control-Allow-Methods",
    ACCESS_CONTROL_ALLOW_HEADERS: "Access-Control-Allow-Headers",
    ACCESS_CONTROL_MAX_AGE: "Access-Control-Max-Age",
    ACCESS_CONTROL_REQUEST_METHOD: "access-control-request-method",
    ACCESS_CONTROL_REQUEST_HEADERS: "access-control-request-headers",
    X_MS_VERSION: "x-ms-version",
};

pub const SECONDARY_SUFFIX: &str = "-secondary";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum QUEUE_STATUSCODE {
    CREATED = 201,
    NOCONTENT = 204,
}

pub static DEFAULT_QUEUE_PERSISTENCE_ARRAY: LazyLock<StoreDestinationArray> = LazyLock::new(|| {
    vec![IStoreDestinationConfigure {
        locationId: String::from("Default"),
        locationPath: String::from(DEFAULT_QUEUE_PERSISTENCE_PATH),
        maxConcurrency: 1,
    }]
});

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

pub static VALID_QUEUE_AUDIENCES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"^https://storage\.azure\.com[/]?$").unwrap(),
        Regex::new(r"^e406a681-f3d4-42a8-90b6-c2b029497af1$").unwrap(),
        Regex::new(r"^https://(.*)\.queue\.core\.windows\.net[/]?$").unwrap(),
        Regex::new(r"^https://(.*)\.queue\.core\.chinacloudapi\.cn[/]?$").unwrap(),
        Regex::new(r"^https://(.*)\.queue\.core\.usgovcloudapi\.net[/]?$").unwrap(),
        Regex::new(r"^https://(.*)\.queue\.core\.cloudapi\.de[/]?$").unwrap(),
    ]
});
