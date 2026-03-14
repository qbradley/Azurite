use std::{
    env, fmt,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use azurite_common::{
    configuration_base::{AccessLogWriteStream, ConfigurationBase},
    storage_error::StorageError,
    utils::constants::{AZURITE_ACCOUNTS_ENV, EMULATOR_ACCOUNT_KEY, EMULATOR_ACCOUNT_NAME},
};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};

use crate::{
    generated::artifacts::models::GeneratedObject,
    utils::constants::{
        DEFAULT_ENABLE_ACCESS_LOG, DEFAULT_ENABLE_DEBUG_LOG, DEFAULT_TABLE_KEEP_ALIVE_TIMEOUT,
        DEFAULT_TABLE_LISTENING_PORT, DEFAULT_TABLE_LOKI_DB_PATH, DEFAULT_TABLE_SERVER_HOST_NAME,
    },
};

#[derive(Clone, Eq, PartialEq)]
pub struct TableAccountKey {
    pub name: String,
    pub key1: Vec<u8>,
    pub key2: Option<Vec<u8>>,
}

impl TableAccountKey {
    pub fn new(name: impl Into<String>, key1: Vec<u8>, key2: Option<Vec<u8>>) -> Self {
        Self {
            name: name.into(),
            key1,
            key2,
        }
    }

    pub fn emulator() -> Self {
        Self::new(EMULATOR_ACCOUNT_NAME, EMULATOR_ACCOUNT_KEY.clone(), None)
    }
}

impl fmt::Debug for TableAccountKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TableAccountKey")
            .field("name", &self.name)
            .field("key1Len", &self.key1.len())
            .field("hasKey2", &self.key2.is_some())
            .finish()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TableTlsConfiguration {
    pub certPath: Option<String>,
    pub keyPath: Option<String>,
    pub password: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TableLoggingConfiguration {
    pub enableAccessLog: bool,
    pub hasAccessLogWriteStream: bool,
    pub enableDebugLog: bool,
    pub debugLogFilePath: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct TableCorsConfiguration {
    pub rules: Vec<GeneratedObject>,
}

#[allow(non_snake_case)]
pub struct TableConfiguration {
    pub base: ConfigurationBase,
    pub locationPath: String,
    pub metadataDBPath: String,
    pub tls: TableTlsConfiguration,
    pub accountKeys: Vec<TableAccountKey>,
    pub logging: TableLoggingConfiguration,
    pub cors: TableCorsConfiguration,
    pub isMemoryPersistence: bool,
}

impl TableConfiguration {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host: String,
        port: u16,
        keepAliveTimeout: u64,
        locationPath: String,
        metadataDBPath: String,
        enableAccessLog: bool,
        accessLogWriteStream: Option<AccessLogWriteStream>,
        enableDebugLog: bool,
        debugLogFilePath: Option<String>,
        loose: bool,
        skipApiVersionCheck: bool,
        cert: String,
        key: String,
        pwd: String,
        oauth: Option<String>,
        disableProductStyleUrl: bool,
        isMemoryPersistence: bool,
        accountKeys: Option<Vec<TableAccountKey>>,
        corsRules: Option<Vec<GeneratedObject>>,
    ) -> Self {
        let tls = TableTlsConfiguration {
            certPath: (!cert.is_empty()).then(|| cert.clone()),
            keyPath: (!key.is_empty()).then(|| key.clone()),
            password: (!pwd.is_empty()).then(|| pwd.clone()),
        };
        let logging = TableLoggingConfiguration {
            enableAccessLog,
            hasAccessLogWriteStream: accessLogWriteStream.is_some(),
            enableDebugLog,
            debugLogFilePath: debugLogFilePath.clone(),
        };

        Self {
            base: ConfigurationBase::new(
                host,
                port,
                keepAliveTimeout,
                enableAccessLog,
                accessLogWriteStream,
                enableDebugLog,
                debugLogFilePath,
                loose,
                skipApiVersionCheck,
                cert,
                key,
                pwd,
                oauth,
                disableProductStyleUrl,
            ),
            locationPath,
            metadataDBPath,
            tls,
            accountKeys: accountKeys.unwrap_or_else(Self::default_account_keys),
            logging,
            cors: TableCorsConfiguration {
                rules: corsRules.unwrap_or_default(),
            },
            isMemoryPersistence,
        }
    }

    pub fn default_account_keys() -> Vec<TableAccountKey> {
        match env::var(AZURITE_ACCOUNTS_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
        {
            Some(value) => Self::parse_account_keys(&value)
                .ok()
                .filter(|accounts| !accounts.is_empty())
                .unwrap_or_else(|| vec![TableAccountKey::emulator()]),
            None => vec![TableAccountKey::emulator()],
        }
    }

    fn parse_account_keys(accounts: &str) -> Result<Vec<TableAccountKey>, StorageError> {
        let mut parsed = Vec::new();
        for accountAndKeys in accounts.trim().split(';').filter(|value| !value.is_empty()) {
            let parts = accountAndKeys.split(':').collect::<Vec<_>>();
            if parts.len() < 2 || parts.len() > 3 {
                return Err(StorageError::new(format!(
                    "Invalid account configuration in {AZURITE_ACCOUNTS_ENV}: {accountAndKeys}",
                )));
            }

            let key1 = BASE64_STANDARD
                .decode(parts[1])
                .map_err(|error| StorageError::new(error.to_string()))?;
            let key2 = if parts.len() == 3 {
                Some(
                    BASE64_STANDARD
                        .decode(parts[2])
                        .map_err(|error| StorageError::new(error.to_string()))?,
                )
            } else {
                None
            };
            parsed.push(TableAccountKey::new(parts[0], key1, key2));
        }
        Ok(parsed)
    }
}

impl Clone for TableConfiguration {
    fn clone(&self) -> Self {
        Self {
            base: ConfigurationBase::new(
                self.base.host.clone(),
                self.base.port,
                self.base.keepAliveTimeout,
                self.base.enableAccessLog,
                self.base.accessLogWriteStream.clone(),
                self.base.enableDebugLog,
                self.base.debugLogFilePath.clone(),
                self.base.loose,
                self.base.skipApiVersionCheck,
                self.base.cert.clone(),
                self.base.key.clone(),
                self.base.pwd.clone(),
                self.base.oauth.clone(),
                self.base.disableProductStyleUrl,
            ),
            locationPath: self.locationPath.clone(),
            metadataDBPath: self.metadataDBPath.clone(),
            tls: self.tls.clone(),
            accountKeys: self.accountKeys.clone(),
            logging: self.logging.clone(),
            cors: self.cors.clone(),
            isMemoryPersistence: self.isMemoryPersistence,
        }
    }
}

impl Default for TableConfiguration {
    fn default() -> Self {
        let locationPath = env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .display()
            .to_string();
        Self::new(
            DEFAULT_TABLE_SERVER_HOST_NAME.to_string(),
            DEFAULT_TABLE_LISTENING_PORT,
            DEFAULT_TABLE_KEEP_ALIVE_TIMEOUT,
            locationPath.clone(),
            PathBuf::from(locationPath)
                .join(DEFAULT_TABLE_LOKI_DB_PATH)
                .display()
                .to_string(),
            DEFAULT_ENABLE_ACCESS_LOG,
            None,
            DEFAULT_ENABLE_DEBUG_LOG,
            None,
            false,
            false,
            String::new(),
            String::new(),
            String::new(),
            None,
            false,
            false,
            None,
            None,
        )
    }
}

impl Deref for TableConfiguration {
    type Target = ConfigurationBase;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for TableConfiguration {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl fmt::Debug for TableConfiguration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TableConfiguration")
            .field("host", &self.base.host)
            .field("port", &self.base.port)
            .field("keepAliveTimeout", &self.base.keepAliveTimeout)
            .field("locationPath", &self.locationPath)
            .field("metadataDBPath", &self.metadataDBPath)
            .field("tls", &self.tls)
            .field("accountKeys", &self.accountKeys)
            .field("logging", &self.logging)
            .field("corsRules", &self.cors.rules)
            .field("loose", &self.base.loose)
            .field("skipApiVersionCheck", &self.base.skipApiVersionCheck)
            .field("oauth", &self.base.oauth)
            .field("disableProductStyleUrl", &self.base.disableProductStyleUrl)
            .field("isMemoryPersistence", &self.isMemoryPersistence)
            .finish()
    }
}
