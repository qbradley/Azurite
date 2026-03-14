#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use azurite_common::persistence::i_extent_store::{
    IStoreDestinationConfigure, StoreDestinationArray,
};
use regex::Regex;

pub const DEFAULT_TABLE_EXTENT_LOKI_DB_PATH: &str = "__azurite_db_table_extent__.json";
pub const DEFAULT_TABLE_LOKI_DB_PATH: &str = "__azurite_db_table__.json";
pub const DEFAULT_TABLE_SERVER_HOST_NAME: &str = "127.0.0.1";
pub const DEFAULT_TABLE_LISTENING_PORT: u16 = 10002;
pub const DEFAULT_TABLE_KEEP_ALIVE_TIMEOUT: u64 = 5;
pub const DEFAULT_ENABLE_ACCESS_LOG: bool = true;
pub const DEFAULT_ENABLE_DEBUG_LOG: bool = true;
pub const DEFAULT_TABLE_PERSISTENCE_PATH: &str = "__tablestorage__";
pub const DEFAULT_KEY_MAX_LENGTH: usize = 512;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum TABLE_STATUSCODE {
    CREATED = 201,
    NOCONTENT = 204,
}

pub const DEFAULT_TABLE_CONTEXT_PATH: &str = "azurite_table_context";
pub const TABLE_API_VERSION: &str = "2025-11-05";
pub const VERSION: &str = "3.35.0";
pub const BODY_SIZE_MAX: usize = 1024 * 1024 * 4;
pub const ENTITY_SIZE_MAX: usize = 1024 * 1024;

#[allow(non_snake_case)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HeaderConstantsType {
    pub SERVER: &'static str,
    pub APPLICATION_JSON: &'static str,
    pub AUTHORIZATION: &'static str,
    pub CONTENT_MD5: &'static str,
    pub CONTENT_TYPE: &'static str,
    pub CONTENT_LENGTH: &'static str,
    pub DATE: &'static str,
    pub X_MS_DATE: &'static str,
    pub X_MS_VERSION: &'static str,
    pub ACCEPT: &'static str,
    pub PREFER: &'static str,
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
}

#[allow(non_upper_case_globals)]
pub const HeaderConstants: HeaderConstantsType = HeaderConstantsType {
    SERVER: "Server",
    APPLICATION_JSON: "application/json",
    AUTHORIZATION: "authorization",
    CONTENT_MD5: "content-md5",
    CONTENT_TYPE: "content-type",
    CONTENT_LENGTH: "content-length",
    DATE: "date",
    X_MS_DATE: "x-ms-date",
    X_MS_VERSION: "x-ms-version",
    ACCEPT: "accept",
    PREFER: "Prefer",
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
};

#[allow(non_snake_case)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MethodConstantsType {
    pub OPTIONS: &'static str,
}

#[allow(non_upper_case_globals)]
pub const MethodConstants: MethodConstantsType = MethodConstantsType { OPTIONS: "OPTIONS" };

pub const SUPPORTED_QUERY_OPERATOR: [&str; 6] = ["eq", "gt", "ge", "lt", "le", "ne"];
pub const NO_METADATA_ACCEPT: &str = "application/json;odata=nometadata";
pub const MINIMAL_METADATA_ACCEPT: &str = "application/json;odata=minimalmetadata";
pub const FULL_METADATA_ACCEPT: &str = "application/json;odata=fullmetadata";
pub const XML_METADATA: &str = "application/atom+xml";
pub const ODATA_TYPE: &str = "@odata.type";
pub const RETURN_NO_CONTENT: &str = "return-no-content";
pub const RETURN_CONTENT: &str = "return-content";

pub static DEFAULT_TABLE_PERSISTENCE_ARRAY: LazyLock<StoreDestinationArray> = LazyLock::new(|| {
    vec![IStoreDestinationConfigure {
        locationId: "Default".to_string(),
        locationPath: DEFAULT_TABLE_PERSISTENCE_PATH.to_string(),
        maxConcurrency: 1,
    }]
});

pub const QUERY_RESULT_MAX_NUM: usize = 1000;
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

pub const TABLE_SERVICE_PERMISSION: &str = "raud";
pub const SECONDARY_SUFFIX: &str = "-secondary";

pub static VALID_TABLE_AUDIENCES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"^https://storage\.azure\.com[/]?$").unwrap(),
        Regex::new(r"^e406a681-f3d4-42a8-90b6-c2b029497af1$").unwrap(),
        Regex::new(r"^https://(.*)\.table\.core\.windows\.net[/]?$").unwrap(),
        Regex::new(r"^https://(.*)\.table\.core\.chinacloudapi\.cn[/]?$").unwrap(),
        Regex::new(r"^https://(.*)\.table\.core\.usgovcloudapi\.net[/]?$").unwrap(),
        Regex::new(r"^https://(.*)\.table\.core\.cloudapi\.de[/]?$").unwrap(),
    ]
});
