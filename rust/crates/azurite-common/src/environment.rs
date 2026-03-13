use std::{env, path::PathBuf};

use async_trait::async_trait;
use clap::{builder::ValueParser, Arg, ArgAction, Command};

use crate::{i_environment::IEnvironment, storage_error::StorageError};

const DEFAULT_BLOB_SERVER_HOST_NAME: &str = "127.0.0.1";
const DEFAULT_BLOB_LISTENING_PORT: u16 = 10000;
const DEFAULT_BLOB_KEEP_ALIVE_TIMEOUT: u64 = 5;
const DEFAULT_QUEUE_SERVER_HOST_NAME: &str = "127.0.0.1";
const DEFAULT_QUEUE_LISTENING_PORT: u16 = 10001;
const DEFAULT_QUEUE_KEEP_ALIVE_TIMEOUT: u64 = 5;
const DEFAULT_TABLE_SERVER_HOST_NAME: &str = "127.0.0.1";
const DEFAULT_TABLE_LISTENING_PORT: u16 = 10002;
const DEFAULT_TABLE_KEEP_ALIVE_TIMEOUT: u64 = 5;

#[allow(non_snake_case)]
#[derive(Clone, Debug)]
struct EnvironmentFlags {
    blobHost: String,
    blobPort: u16,
    blobKeepAliveTimeout: u64,
    queueHost: String,
    queuePort: u16,
    queueKeepAliveTimeout: u64,
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
    oauth: Option<String>,
    inMemoryPersistence: bool,
    extentMemoryLimit: Option<f64>,
    debug: Option<String>,
    disableTelemetry: bool,
}

impl Default for EnvironmentFlags {
    fn default() -> Self {
        Self {
            blobHost: DEFAULT_BLOB_SERVER_HOST_NAME.to_string(),
            blobPort: DEFAULT_BLOB_LISTENING_PORT,
            blobKeepAliveTimeout: DEFAULT_BLOB_KEEP_ALIVE_TIMEOUT,
            queueHost: DEFAULT_QUEUE_SERVER_HOST_NAME.to_string(),
            queuePort: DEFAULT_QUEUE_LISTENING_PORT,
            queueKeepAliveTimeout: DEFAULT_QUEUE_KEEP_ALIVE_TIMEOUT,
            tableHost: DEFAULT_TABLE_SERVER_HOST_NAME.to_string(),
            tablePort: DEFAULT_TABLE_LISTENING_PORT,
            tableKeepAliveTimeout: DEFAULT_TABLE_KEEP_ALIVE_TIMEOUT,
            location: None,
            silent: false,
            loose: false,
            skipApiVersionCheck: false,
            disableProductStyleUrl: false,
            cert: None,
            key: None,
            pwd: None,
            oauth: None,
            inMemoryPersistence: false,
            extentMemoryLimit: None,
            debug: None,
            disableTelemetry: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Environment {
    pub args: Vec<String>,
    flags: EnvironmentFlags,
}

impl Environment {
    pub fn new(args: Vec<String>) -> Self {
        let flags = Self::parse_flags(args.clone());
        Self { args, flags }
    }

    fn normalize_args(args: Vec<String>) -> Vec<String> {
        if args.first().map(|arg| arg.starts_with('-')).unwrap_or(true) {
            let mut normalized = vec!["azurite".to_string()];
            normalized.extend(args);
            normalized
        } else {
            args
        }
    }

    fn command() -> Command {
        let command = Command::new("azurite")
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
                Arg::new("queueHost")
                    .long("queueHost")
                    .help("Optional. Customize listening address for queue")
                    .default_value(DEFAULT_QUEUE_SERVER_HOST_NAME),
            )
            .arg(
                Arg::new("queuePort")
                    .long("queuePort")
                    .help("Optional. Customize listening port for queue")
                    .value_parser(ValueParser::from(clap::value_parser!(u16)))
                    .default_value("10001"),
            )
            .arg(
                Arg::new("queueKeepAliveTimeout")
                    .long("queueKeepAliveTimeout")
                    .help("Optional. Customize http keep alive timeout for queue")
                    .value_parser(ValueParser::from(clap::value_parser!(u64)))
                    .default_value("5"),
            )
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
            );

        command
            .mut_arg("disableProductStyleUrl", |arg| {
                arg.help("Optional. Disable getting account name from the host of request Uri, always get account name from the first path segment of request Uri")
            })
            .arg(
                Arg::new("disableTelemetry")
                    .long("disableTelemetry")
                    .help("Optional. Disable telemtry collection of Azurite. If not specify this parameter Azurite will collect telemetry data by default.")
                    .action(ArgAction::SetTrue),
            )
    }

    fn parse_flags(args: Vec<String>) -> EnvironmentFlags {
        let normalized = Self::normalize_args(args);
        let matches = Self::command()
            .try_get_matches_from(normalized)
            .unwrap_or_else(|error| panic!("{error}"));

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

        EnvironmentFlags {
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
            queueHost: matches
                .get_one::<String>("queueHost")
                .cloned()
                .unwrap_or_else(|| DEFAULT_QUEUE_SERVER_HOST_NAME.to_string()),
            queuePort: *matches
                .get_one::<u16>("queuePort")
                .unwrap_or(&DEFAULT_QUEUE_LISTENING_PORT),
            queueKeepAliveTimeout: *matches
                .get_one::<u64>("queueKeepAliveTimeout")
                .unwrap_or(&DEFAULT_QUEUE_KEEP_ALIVE_TIMEOUT),
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
            oauth: matches.get_one::<String>("oauth").cloned(),
            inMemoryPersistence: matches.get_flag("inMemoryPersistence"),
            extentMemoryLimit,
            debug: matches.get_one::<String>("debug").cloned(),
            disableTelemetry: matches.get_flag("disableTelemetry"),
        }
    }
}

#[allow(non_snake_case)]
#[async_trait]
impl IEnvironment for Environment {
    fn blobHost(&self) -> Option<String> {
        Some(self.flags.blobHost.clone())
    }

    fn blobPort(&self) -> Option<u16> {
        Some(self.flags.blobPort)
    }

    fn blobKeepAliveTimeout(&self) -> Option<u64> {
        Some(self.flags.blobKeepAliveTimeout)
    }

    fn queueHost(&self) -> Option<String> {
        Some(self.flags.queueHost.clone())
    }

    fn queuePort(&self) -> Option<u16> {
        Some(self.flags.queuePort)
    }

    fn queueKeepAliveTimeout(&self) -> Option<u64> {
        Some(self.flags.queueKeepAliveTimeout)
    }

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
        if let Some(location) = &self.flags.location {
            Ok(location.to_string())
        } else {
            Ok(env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .display()
                .to_string())
        }
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
        if let Some(debug) = &self.flags.debug {
            if debug == "true" {
                return Err(StorageError::new(
                    "Must provide a debug log file path for parameter -d or --debug",
                ));
            }
            return Ok(Some(debug.to_string()));
        }
        Ok(None)
    }

    fn oauth(&self) -> Option<String> {
        self.flags.oauth.clone()
    }

    fn inMemoryPersistence(&self) -> bool {
        if self.flags.inMemoryPersistence {
            if self.flags.location.is_some() {
                panic!("The --inMemoryPersistence option is not supported when the --location option is set.");
            }
            return true;
        }

        if self.extentMemoryLimit().is_some() {
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
