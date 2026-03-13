use std::{fs::OpenOptions, io::Write, path::PathBuf};

use chrono::{SecondsFormat, Utc};
use tracing::{debug, error, info, trace, warn};

use crate::i_logger_strategy::{ILoggerStrategy, LogLevels};

#[allow(non_snake_case)]
pub struct WinstonLoggerStrategy {
    level: LogLevels,
    pub consoleTransport: Option<()>,
    pub fileTransport: Option<PathBuf>,
}

impl WinstonLoggerStrategy {
    pub fn new(level: LogLevels, logfile: Option<String>) -> Self {
        match logfile {
            Some(logfile) => Self {
                level,
                consoleTransport: None,
                fileTransport: Some(PathBuf::from(logfile)),
            },
            None => Self {
                level,
                consoleTransport: Some(()),
                fileTransport: None,
            },
        }
    }

    fn enabled(&self, level: LogLevels) -> bool {
        level.priority() <= self.level.priority()
    }

    fn write_line(&self, line: &str) {
        if let Some(fileTransport) = &self.fileTransport {
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(fileTransport)
            {
                let _ = writeln!(file, "{line}");
            }
        } else {
            println!("{line}");
        }
    }
}

impl ILoggerStrategy for WinstonLoggerStrategy {
    fn log(&self, level: LogLevels, message: &str, contextID: Option<&str>) {
        if !self.enabled(level) {
            return;
        }

        let contextID = contextID.unwrap_or("\t");
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let line = format!("{timestamp} {contextID} {}: {message}", level.as_str());

        match level {
            LogLevels::Error => error!("{line}"),
            LogLevels::Warn => warn!("{line}"),
            LogLevels::Info => info!("{line}"),
            LogLevels::Verbose => trace!("{line}"),
            LogLevels::Debug => debug!("{line}"),
        }

        self.write_line(&line);
    }
}
