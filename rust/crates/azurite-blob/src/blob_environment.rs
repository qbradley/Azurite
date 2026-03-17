use std::{
    env,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use azurite_common::{
    environment::Environment as CommonEnvironment, i_environment::IEnvironment,
    storage_error::StorageError,
};
use clap::{builder::ValueParser, Arg, ArgAction, Command};
use tokio::fs;

use crate::{
    i_blob_environment::IBlobEnvironment,
    utils::constants::{
        DEFAULT_BLOB_KEEP_ALIVE_TIMEOUT, DEFAULT_BLOB_LISTENING_PORT, DEFAULT_BLOB_SERVER_HOST_NAME,
    },
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
struct BlobEnvironmentFlags {
    blobHost: String,
    blobPort: u16,
    blobKeepAliveTimeout: u64,
    location: Option<String>,
    silent: bool,
    loose: bool,
    skipApiVersionCheck: bool,
    cert: Option<String>,
    key: Option<String>,
    pwd: Option<String>,
    debug: Option<String>,
    oauth: Option<String>,
    disableProductStyleUrl: bool,
    bugForBugCompatibility: bool,
    inMemoryPersistence: bool,
    extentMemoryLimit: Option<f64>,
    disableTelemetry: bool,
}

#[derive(Clone, Debug)]
pub struct BlobEnvironment {
    pub args: Vec<String>,
    flags: BlobEnvironmentFlags,
}

impl BlobEnvironment {
    pub fn new(args: Vec<String>) -> Result<Self, StorageError> {
        let flags = Self::parse_flags(args.clone())?;
        Self::validate_flags(&flags)?;
        Ok(Self { args, flags })
    }

    pub fn from_process() -> Result<Self, StorageError> {
        Self::new(env::args().collect())
    }

    fn normalize_args(args: Vec<String>) -> Vec<String> {
        if args.first().map(|arg| arg.starts_with('-')).unwrap_or(true) {
            let mut normalized = vec!["azurite-blob".to_string()];
            normalized.extend(args);
            normalized
        } else {
            args
        }
    }

    fn command() -> Command {
        Command::new("azurite-blob")
            .disable_help_subcommand(true)
            .args_override_self(true)
            .arg(
                Arg::new("blobHost")
                    .long("blobHost")
                    .help("Optional. Customize listening address for blob")
                    .default_value(DEFAULT_BLOB_SERVER_HOST_NAME),
            )
            .arg(
                Arg::new("blobPort")
                    .long("blobPort")
                    .help("Optional. Customize listening port for blob")
                    .value_parser(ValueParser::from(clap::value_parser!(u16)))
                    .default_value("10000"),
            )
            .arg(
                Arg::new("blobKeepAliveTimeout")
                    .long("blobKeepAliveTimeout")
                    .help("Optional. Customize http keep alive timeout for blob")
                    .value_parser(ValueParser::from(clap::value_parser!(u64)))
                    .default_value("5"),
            )
            .arg(
                Arg::new("location")
                    .short('l')
                    .long("location")
                    .help("Optional. Use an existing folder as workspace path, default is current working directory")
                    .default_value("<cwd>"),
            )
            .arg(
                Arg::new("silent")
                    .short('s')
                    .long("silent")
                    .help("Optional. Disable access log displayed in console")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("loose")
                    .short('L')
                    .long("loose")
                    .help("Optional. Enable loose mode which ignores unsupported headers and parameters")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("skipApiVersionCheck")
                    .long("skipApiVersionCheck")
                    .help("Optional. Skip the request API version check, request with all Api versions will be allowed")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("oauth")
                    .long("oauth")
                    .help("Optional. OAuth level. Candidate values: \"basic\""),
            )
            .arg(
                Arg::new("cert")
                    .long("cert")
                    .help("Optional. Path to certificate file"),
            )
            .arg(
                Arg::new("key")
                    .long("key")
                    .help("Optional. Path to certificate key .pem file"),
            )
            .arg(
                Arg::new("inMemoryPersistence")
                    .long("inMemoryPersistence")
                    .help("Optional. Disable persisting any data to disk. If the Azurite process is terminated, all data is lost")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("extentMemoryLimit")
                    .long("extentMemoryLimit")
                    .help("Optional. The number of megabytes to limit in-memory extent storage to. Only used with the --inMemoryPersistence option. Defaults to 50% of total memory")
                    .allow_hyphen_values(true)
                    .default_value("-1"),
            )
            .arg(
                Arg::new("debug")
                    .short('d')
                    .long("debug")
                    .help("Optional. Enable debug log by providing a valid local file path as log destination")
                    .num_args(0..=1)
                    .default_missing_value("true"),
            )
            .arg(
                Arg::new("pwd")
                    .long("pwd")
                    .help("Optional. Password for .pfx file"),
            )
            .arg(
                Arg::new("disableProductStyleUrl")
                    .long("disableProductStyleUrl")
                    .help("Optional. Disable getting account name from the host of request URI, always get account name from the first path segment of request URI")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("disableBugForBugCompatibility")
                    .long("disableBugForBugCompatibility")
                    .help("Optional. Disable Blob_Query bug-for-bug compatibility mode and return the semantically correct 501 response")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("disableTelemetry")
                    .long("disableTelemetry")
                    .help("Optional. Disable telemetry data collection of this Azurite execution. By default, Azurite will collect telemetry data to help improve the product")
                    .action(ArgAction::SetTrue),
            )
    }

    fn parse_flags(args: Vec<String>) -> Result<BlobEnvironmentFlags, StorageError> {
        let normalized = Self::normalize_args(args);
        let matches = Self::command()
            .try_get_matches_from(normalized)
            .map_err(|error| StorageError::new(error.to_string()))?;

        let location = matches.get_one::<String>("location").and_then(|value| {
            if value == "<cwd>" {
                None
            } else {
                Some(value.to_string())
            }
        });
        let extentMemoryLimit = matches
            .get_one::<String>("extentMemoryLimit")
            .and_then(|value| {
                if value == "-1" {
                    None
                } else {
                    Some(value.parse::<f64>().unwrap_or(f64::NAN))
                }
            });

        Ok(BlobEnvironmentFlags {
            blobHost: matches
                .get_one::<String>("blobHost")
                .cloned()
                .unwrap_or_else(|| DEFAULT_BLOB_SERVER_HOST_NAME.to_string()),
            blobPort: *matches
                .get_one::<u16>("blobPort")
                .unwrap_or(&DEFAULT_BLOB_LISTENING_PORT),
            blobKeepAliveTimeout: *matches
                .get_one::<u64>("blobKeepAliveTimeout")
                .unwrap_or(&DEFAULT_BLOB_KEEP_ALIVE_TIMEOUT),
            location,
            silent: matches.get_flag("silent"),
            loose: matches.get_flag("loose"),
            skipApiVersionCheck: matches.get_flag("skipApiVersionCheck"),
            cert: matches.get_one::<String>("cert").cloned(),
            key: matches.get_one::<String>("key").cloned(),
            pwd: matches.get_one::<String>("pwd").cloned(),
            debug: matches.get_one::<String>("debug").cloned(),
            oauth: matches.get_one::<String>("oauth").cloned(),
            disableProductStyleUrl: matches.get_flag("disableProductStyleUrl"),
            bugForBugCompatibility: !matches.get_flag("disableBugForBugCompatibility"),
            inMemoryPersistence: matches.get_flag("inMemoryPersistence"),
            extentMemoryLimit,
            disableTelemetry: matches.get_flag("disableTelemetry"),
        })
    }

    fn validate_flags(flags: &BlobEnvironmentFlags) -> Result<(), StorageError> {
        if flags.inMemoryPersistence && flags.location.is_some() {
            return Err(StorageError::new(
                "The --inMemoryPersistence option is not supported when the --location option is set.",
            ));
        }

        if !flags.inMemoryPersistence && flags.extentMemoryLimit.is_some() {
            return Err(StorageError::new(
                "The --extentMemoryLimit option is only supported when the --inMemoryPersistence option is set.",
            ));
        }

        Ok(())
    }

    async fn ensure_dir_access(path: &Path) -> Result<(), StorageError> {
        fs::create_dir_all(path)
            .await
            .map_err(|error| StorageError::new(error.to_string()))?;
        fs::metadata(path)
            .await
            .map_err(|error| StorageError::new(error.to_string()))?;
        Ok(())
    }

    fn debug_parent(debugFilePath: &str) -> PathBuf {
        Path::new(debugFilePath)
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or(Path::new("."))
            .to_path_buf()
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl IBlobEnvironment for BlobEnvironment {
    fn blobHost(&self) -> Option<String> {
        Some(self.flags.blobHost.clone())
    }

    fn blobPort(&self) -> Option<u16> {
        Some(self.flags.blobPort)
    }

    fn blobKeepAliveTimeout(&self) -> Option<u64> {
        Some(self.flags.blobKeepAliveTimeout)
    }

    async fn location(&self) -> Result<String, StorageError> {
        let location = if let Some(location) = &self.flags.location {
            PathBuf::from(location)
        } else {
            env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        };
        Self::ensure_dir_access(&location).await?;
        Ok(location.display().to_string())
    }

    fn silent(&self) -> bool {
        self.flags.silent
    }

    fn loose(&self) -> bool {
        self.flags.loose
    }

    fn skipApiVersionCheck(&self) -> bool {
        self.flags.skipApiVersionCheck
    }

    fn cert(&self) -> Option<String> {
        self.flags.cert.clone()
    }

    fn key(&self) -> Option<String> {
        self.flags.key.clone()
    }

    fn pwd(&self) -> Option<String> {
        self.flags.pwd.clone()
    }

    async fn debug(&self) -> Result<Option<String>, StorageError> {
        if let Some(debugFilePath) = &self.flags.debug {
            if debugFilePath == "true" {
                return Err(StorageError::new(
                    "Must provide a debug log file path for parameter -d or --debug",
                ));
            }
            let parent = Self::debug_parent(debugFilePath);
            Self::ensure_dir_access(&parent).await?;
            return Ok(Some(debugFilePath.clone()));
        }

        Ok(None)
    }

    fn oauth(&self) -> Option<String> {
        self.flags.oauth.clone()
    }

    fn disableProductStyleUrl(&self) -> bool {
        self.flags.disableProductStyleUrl
    }

    fn bugForBugCompatibility(&self) -> bool {
        self.flags.bugForBugCompatibility
    }

    fn inMemoryPersistence(&self) -> bool {
        if self.flags.inMemoryPersistence {
            if self.flags.location.is_some() {
                panic!("The --inMemoryPersistence option is not supported when the --location option is set.");
            }
            return true;
        }

        if self.flags.extentMemoryLimit.is_some() {
            panic!("The --extentMemoryLimit option is only supported when the --inMemoryPersistence option is set.");
        }

        false
    }

    fn extentMemoryLimit(&self) -> Option<f64> {
        self.flags.extentMemoryLimit
    }

    fn disableTelemetry(&self) -> bool {
        self.flags.disableTelemetry
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl IBlobEnvironment for CommonEnvironment {
    fn blobHost(&self) -> Option<String> {
        IEnvironment::blobHost(self)
    }

    fn blobPort(&self) -> Option<u16> {
        IEnvironment::blobPort(self)
    }

    fn blobKeepAliveTimeout(&self) -> Option<u64> {
        IEnvironment::blobKeepAliveTimeout(self)
    }

    async fn location(&self) -> Result<String, StorageError> {
        IEnvironment::location(self).await
    }

    fn silent(&self) -> bool {
        IEnvironment::silent(self)
    }

    fn loose(&self) -> bool {
        IEnvironment::loose(self)
    }

    fn skipApiVersionCheck(&self) -> bool {
        IEnvironment::skipApiVersionCheck(self)
    }

    fn cert(&self) -> Option<String> {
        IEnvironment::cert(self)
    }

    fn key(&self) -> Option<String> {
        IEnvironment::key(self)
    }

    fn pwd(&self) -> Option<String> {
        IEnvironment::pwd(self)
    }

    async fn debug(&self) -> Result<Option<String>, StorageError> {
        IEnvironment::debug(self).await
    }

    fn oauth(&self) -> Option<String> {
        IEnvironment::oauth(self)
    }

    fn disableProductStyleUrl(&self) -> bool {
        IEnvironment::disableProductStyleUrl(self)
    }

    fn bugForBugCompatibility(&self) -> bool {
        IEnvironment::bugForBugCompatibility(self)
    }

    fn inMemoryPersistence(&self) -> bool {
        IEnvironment::inMemoryPersistence(self)
    }

    fn extentMemoryLimit(&self) -> Option<f64> {
        IEnvironment::extentMemoryLimit(self)
    }

    fn disableTelemetry(&self) -> bool {
        IEnvironment::disableTelemetry(self)
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl IEnvironment for BlobEnvironment {
    fn blobHost(&self) -> Option<String> {
        IBlobEnvironment::blobHost(self)
    }

    fn blobPort(&self) -> Option<u16> {
        IBlobEnvironment::blobPort(self)
    }

    fn blobKeepAliveTimeout(&self) -> Option<u64> {
        IBlobEnvironment::blobKeepAliveTimeout(self)
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
        IBlobEnvironment::location(self).await
    }

    fn silent(&self) -> bool {
        IBlobEnvironment::silent(self)
    }

    fn loose(&self) -> bool {
        IBlobEnvironment::loose(self)
    }

    fn skipApiVersionCheck(&self) -> bool {
        IBlobEnvironment::skipApiVersionCheck(self)
    }

    fn disableProductStyleUrl(&self) -> bool {
        IBlobEnvironment::disableProductStyleUrl(self)
    }

    fn bugForBugCompatibility(&self) -> bool {
        IBlobEnvironment::bugForBugCompatibility(self)
    }

    fn cert(&self) -> Option<String> {
        IBlobEnvironment::cert(self)
    }

    fn key(&self) -> Option<String> {
        IBlobEnvironment::key(self)
    }

    fn pwd(&self) -> Option<String> {
        IBlobEnvironment::pwd(self)
    }

    async fn debug(&self) -> Result<Option<String>, StorageError> {
        IBlobEnvironment::debug(self).await
    }

    fn oauth(&self) -> Option<String> {
        IBlobEnvironment::oauth(self)
    }

    fn inMemoryPersistence(&self) -> bool {
        IBlobEnvironment::inMemoryPersistence(self)
    }

    fn extentMemoryLimit(&self) -> Option<f64> {
        IBlobEnvironment::extentMemoryLimit(self)
    }

    fn disableTelemetry(&self) -> bool {
        IBlobEnvironment::disableTelemetry(self)
    }
}
