use std::{collections::HashSet, sync::LazyLock};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use regex::Regex;

pub const AZURITE_ACCOUNTS_ENV: &str = "AZURITE_ACCOUNTS";
pub const DEFAULT_ACCOUNTS_REFRESH_INTERVAL: u64 = 60 * 1000;
pub const DEFAULT_FD_CACHE_NUMBER: u64 = 100;
pub const FD_CACHE_NUMBER_MIN: u64 = 1;
pub const FD_CACHE_NUMBER_MAX: u64 = 100;
pub const DEFAULT_MAX_EXTENT_SIZE: u64 = 64 * 1024 * 1024;
pub const DEFAULT_READ_CONCURRENCY: u64 = 100;
pub const DEFAULT_EXTENT_GC_PROTECT_TIME_IN_MS: u64 = 10 * 60 * 1000;
pub const DEFAULT_SQL_CHARSET: &str = "utf8mb4";
pub const DEFAULT_SQL_COLLATE: &str = "utf8mb4_bin";
pub const BEARER_TOKEN_PREFIX: &str = "Bearer";
pub const HTTPS: &str = "https";
pub const EMULATOR_ACCOUNT_NAME: &str = "devstoreaccount1";
pub const EMULATOR_ACCOUNT_KEY_STR: &str =
    "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==";
pub const VALID_ISSUE_PREFIXES: [&str; 4] = [
    "https://sts.windows.net/",
    "https://sts.microsoftonline.de/",
    "https://sts.chinacloudapi.cn/",
    "https://sts.windows-ppe.net",
];

pub static IP_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:[0-9]{1,3}\.){3}[0-9]{1,3}$").unwrap());
pub static NO_ACCOUNT_HOST_NAMES: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["host.docker.internal"]));
pub static EMULATOR_ACCOUNT_KEY: LazyLock<Vec<u8>> =
    LazyLock::new(|| STANDARD.decode(EMULATOR_ACCOUNT_KEY_STR).unwrap());
pub static VALID_CSHARP_IDENTIFIER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap());

#[allow(non_snake_case)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SqlPoolOptions {
    pub max: u64,
    pub min: u64,
    pub acquire: u64,
    pub idle: u64,
}

#[allow(non_snake_case)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SqlDialectOptions {
    pub timezone: &'static str,
}

#[allow(non_snake_case)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SqlOptions {
    pub logging: bool,
    pub pool: SqlPoolOptions,
    pub charset: &'static str,
    pub collate: &'static str,
    pub dialectOptions: SqlDialectOptions,
}

pub const DEFAULT_SQL_OPTIONS: SqlOptions = SqlOptions {
    logging: false,
    pool: SqlPoolOptions {
        max: 20,
        min: 0,
        acquire: 30000,
        idle: 10000,
    },
    charset: DEFAULT_SQL_CHARSET,
    collate: DEFAULT_SQL_COLLATE,
    dialectOptions: SqlDialectOptions { timezone: "+00:00" },
};
