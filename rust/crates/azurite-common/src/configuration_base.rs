use std::{
    fs,
    io::Write,
    sync::{Arc, Mutex},
};

use crate::{
    i_environment::IEnvironment,
    i_logger::ILogger,
    logger::logger,
    models::OAuthLevel,
    persistence::memory_extent_store::{SharedChunkStore, DEFAULT_EXTENT_MEMORY_LIMIT},
    storage_error::StorageError,
};

pub type AccessLogWriteStream = Arc<Mutex<dyn Write + Send + Sync>>;

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CertOptions {
    Default,
    PEM,
    PFX,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CertMaterial {
    PEM { cert: Vec<u8>, key: Vec<u8> },
    PFX { pfx: Vec<u8>, passphrase: String },
}

fn totalmem() -> u64 {
    match fs::read_to_string("/proc/meminfo") {
        Ok(content) => content
            .lines()
            .find_map(|line| {
                line.strip_prefix("MemTotal:")
                    .and_then(|value| value.split_whitespace().next())
                    .and_then(|value| value.parse::<u64>().ok())
                    .map(|value| value * 1024)
            })
            .unwrap_or(0),
        Err(_) => 0,
    }
}

#[allow(non_snake_case)]
pub fn setExtentMemoryLimit<E>(env: &E, logToConsole: bool) -> Result<(), StorageError>
where
    E: IEnvironment,
{
    if env.inMemoryPersistence() {
        let mut mb = env.extentMemoryLimit();
        if mb.is_none() {
            mb = Some(*DEFAULT_EXTENT_MEMORY_LIMIT as f64 / (1024f64 * 1024f64));
        }

        let mb = mb.unwrap_or(f64::NAN);
        if mb < 0f64 {
            return Err(StorageError::new(format!(
                "A negative value of '{mb}' is not allowed for the extent memory limit."
            )));
        }

        if mb >= 0f64 {
            let bytes = (mb * 1024f64 * 1024f64).round() as u64;
            let totalPct = if totalmem() == 0 {
                0
            } else {
                ((100f64 * bytes as f64) / totalmem() as f64).round() as u64
            };
            let message = format!(
                "In-memory extent storage is enabled with a limit of {:.2} MB ({bytes} bytes, {totalPct}% of total memory).",
                mb
            );
            if logToConsole {
                println!("{message}");
            }
            logger.info(&message, None);
            let _ = SharedChunkStore.setSizeLimit(Some(bytes));
        } else {
            let message =
                "In-memory extent storage is enabled with no limit on memory used.".to_string();
            if logToConsole {
                println!("{message}");
            }
            logger.info(&message, None);
            let _ = SharedChunkStore.setSizeLimit(None);
        }
    }

    Ok(())
}

pub struct ConfigurationBase {
    pub host: String,
    pub port: u16,
    pub keepAliveTimeout: u64,
    pub enableAccessLog: bool,
    pub accessLogWriteStream: Option<AccessLogWriteStream>,
    pub enableDebugLog: bool,
    pub debugLogFilePath: Option<String>,
    pub loose: bool,
    pub skipApiVersionCheck: bool,
    pub cert: String,
    pub key: String,
    pub pwd: String,
    pub oauth: Option<String>,
    pub disableProductStyleUrl: bool,
}

impl Default for ConfigurationBase {
    fn default() -> Self {
        Self::new(
            "127.0.0.1".to_string(),
            10000,
            5,
            false,
            None,
            false,
            None,
            false,
            false,
            String::new(),
            String::new(),
            String::new(),
            None,
            false,
        )
    }
}

impl ConfigurationBase {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host: String,
        port: u16,
        keepAliveTimeout: u64,
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
    ) -> Self {
        Self {
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
        }
    }

    pub fn hasCert(&self) -> CertOptions {
        if !self.cert.is_empty() && !self.key.is_empty() {
            return CertOptions::PEM;
        }
        if !self.cert.is_empty() && !self.pwd.is_empty() {
            return CertOptions::PFX;
        }
        CertOptions::Default
    }

    pub fn getCert(&self, option: CertOptions) -> Result<Option<CertMaterial>, StorageError> {
        match option {
            CertOptions::PEM => Ok(Some(CertMaterial::PEM {
                cert: fs::read(&self.cert).map_err(|error| StorageError::new(error.to_string()))?,
                key: fs::read(&self.key).map_err(|error| StorageError::new(error.to_string()))?,
            })),
            CertOptions::PFX => Ok(Some(CertMaterial::PFX {
                pfx: fs::read(&self.cert).map_err(|error| StorageError::new(error.to_string()))?,
                passphrase: self.pwd.to_string(),
            })),
            CertOptions::Default => Ok(None),
        }
    }

    pub fn getOAuthLevel(&self) -> Option<OAuthLevel> {
        if let Some(oauth) = &self.oauth {
            if oauth.eq_ignore_ascii_case("basic") {
                return Some(OAuthLevel::BASIC);
            }
        }
        None
    }

    pub fn getHttpServerAddress(&self) -> String {
        format!(
            "http{}://{}:{}",
            if self.hasCert() == CertOptions::Default {
                ""
            } else {
                "s"
            },
            self.host,
            self.port
        )
    }
}
