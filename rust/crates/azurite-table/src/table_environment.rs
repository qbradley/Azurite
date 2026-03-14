use std::{
    env,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use azurite_common::{i_environment::IEnvironment, storage_error::StorageError};
use clap::{builder::ValueParser, Arg, ArgAction, Command};
use tokio::fs;

use crate::{
    i_table_environment::ITableEnvironment,
    utils::constants::{
        DEFAULT_TABLE_KEEP_ALIVE_TIMEOUT, DEFAULT_TABLE_LISTENING_PORT,
        DEFAULT_TABLE_SERVER_HOST_NAME,
    },
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default)]
struct TableEnvironmentFlags {
    tableHost: String,
    tablePort: u16,
    tableKeepAliveTimeout: u64,
    location: Option<String>,
    silent: bool,
    loose: bool,
    skipApiVersionCheck: bool,
    disableProductStyleUrl: bool,
    cert: Option<String>,
    key: Option<String>,
    pwd: Option<String>,
    debug: Option<String>,
    oauth: Option<String>,
    inMemoryPersistence: bool,
    disableTelemetry: bool,
}

#[derive(Clone, Debug)]
pub struct TableEnvironment {
    pub args: Vec<String>,
    flags: TableEnvironmentFlags,
}

impl TableEnvironment {
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
            let mut normalized = vec!["azurite-table".to_string()];
            normalized.extend(args);
            normalized
        } else {
            args
        }
    }

    fn command() -> Command {
        Command::new("azurite-table")
            .disable_help_subcommand(true)
            .args_override_self(true)
            .arg(
                Arg::new("tableHost")
                    .long("tableHost")
                    .help("Optional. Customize listening address for table")
                    .default_value(DEFAULT_TABLE_SERVER_HOST_NAME),
            )
            .arg(
                Arg::new("tablePort")
                    .long("tablePort")
                    .help("Optional. Customize listening port for table")
                    .value_parser(ValueParser::from(clap::value_parser!(u16)))
                    .default_value("10002"),
            )
            .arg(
                Arg::new("tableKeepAliveTimeout")
                    .long("tableKeepAliveTimeout")
                    .help("Optional. Customize http keep alive timeout for table")
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
                Arg::new("disableProductStyleUrl")
                    .long("disableProductStyleUrl")
                    .help("Optional. Disable getting account name from the host of request URI, always get account name from the first path segment of request URI")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("inMemoryPersistence")
                    .long("inMemoryPersistence")
                    .help("Optional. Disable persisting any data to disk. If the Azurite process is terminated, all data is lost")
                    .action(ArgAction::SetTrue),
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
                Arg::new("pwd")
                    .long("pwd")
                    .help("Optional. Password for .pfx file"),
            )
            .arg(
                Arg::new("disableTelemetry")
                    .long("disableTelemetry")
                    .help("Optional. Disable telemetry data collection of this Azurite execution. By default, Azurite will collect telemetry data to help improve the product")
                    .action(ArgAction::SetTrue),
            )
    }

    fn parse_flags(args: Vec<String>) -> Result<TableEnvironmentFlags, StorageError> {
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

        Ok(TableEnvironmentFlags {
            tableHost: matches
                .get_one::<String>("tableHost")
                .cloned()
                .unwrap_or_else(|| DEFAULT_TABLE_SERVER_HOST_NAME.to_string()),
            tablePort: *matches
                .get_one::<u16>("tablePort")
                .unwrap_or(&DEFAULT_TABLE_LISTENING_PORT),
            tableKeepAliveTimeout: *matches
                .get_one::<u64>("tableKeepAliveTimeout")
                .unwrap_or(&DEFAULT_TABLE_KEEP_ALIVE_TIMEOUT),
            location,
            silent: matches.get_flag("silent"),
            loose: matches.get_flag("loose"),
            skipApiVersionCheck: matches.get_flag("skipApiVersionCheck"),
            disableProductStyleUrl: matches.get_flag("disableProductStyleUrl"),
            cert: matches.get_one::<String>("cert").cloned(),
            key: matches.get_one::<String>("key").cloned(),
            pwd: matches.get_one::<String>("pwd").cloned(),
            debug: matches.get_one::<String>("debug").cloned(),
            oauth: matches.get_one::<String>("oauth").cloned(),
            inMemoryPersistence: matches.get_flag("inMemoryPersistence"),
            disableTelemetry: matches.get_flag("disableTelemetry"),
        })
    }

    fn validate_flags(flags: &TableEnvironmentFlags) -> Result<(), StorageError> {
        if flags.inMemoryPersistence && flags.location.is_some() {
            return Err(StorageError::new(
                "The --inMemoryPersistence option is not supported when the --location option is set.",
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
impl ITableEnvironment for TableEnvironment {
    fn tableHost(&self) -> Option<String> {
        Some(self.flags.tableHost.clone())
    }

    fn tablePort(&self) -> Option<u16> {
        Some(self.flags.tablePort)
    }

    fn tableKeepAliveTimeout(&self) -> Option<u64> {
        Some(self.flags.tableKeepAliveTimeout)
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

    fn disableProductStyleUrl(&self) -> bool {
        self.flags.disableProductStyleUrl
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

    fn inMemoryPersistence(&self) -> bool {
        self.flags.inMemoryPersistence
    }

    fn disableTelemetry(&self) -> bool {
        self.flags.disableTelemetry
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl IEnvironment for TableEnvironment {
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
        ITableEnvironment::tableHost(self)
    }

    fn tablePort(&self) -> Option<u16> {
        ITableEnvironment::tablePort(self)
    }

    fn tableKeepAliveTimeout(&self) -> Option<u64> {
        ITableEnvironment::tableKeepAliveTimeout(self)
    }

    async fn location(&self) -> Result<String, StorageError> {
        ITableEnvironment::location(self).await
    }

    fn silent(&self) -> bool {
        ITableEnvironment::silent(self)
    }

    fn loose(&self) -> bool {
        ITableEnvironment::loose(self)
    }

    fn skipApiVersionCheck(&self) -> bool {
        ITableEnvironment::skipApiVersionCheck(self)
    }

    fn disableProductStyleUrl(&self) -> bool {
        ITableEnvironment::disableProductStyleUrl(self)
    }

    fn cert(&self) -> Option<String> {
        ITableEnvironment::cert(self)
    }

    fn key(&self) -> Option<String> {
        ITableEnvironment::key(self)
    }

    fn pwd(&self) -> Option<String> {
        ITableEnvironment::pwd(self)
    }

    async fn debug(&self) -> Result<Option<String>, StorageError> {
        ITableEnvironment::debug(self).await
    }

    fn oauth(&self) -> Option<String> {
        ITableEnvironment::oauth(self)
    }

    fn inMemoryPersistence(&self) -> bool {
        ITableEnvironment::inMemoryPersistence(self)
    }

    fn extentMemoryLimit(&self) -> Option<f64> {
        None
    }

    fn disableTelemetry(&self) -> bool {
        ITableEnvironment::disableTelemetry(self)
    }
}
