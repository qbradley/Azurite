#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogLevels {
    Error,
    Warn,
    Info,
    Verbose,
    Debug,
}

impl LogLevels {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Verbose => "verbose",
            Self::Debug => "debug",
        }
    }

    pub fn priority(self) -> u8 {
        match self {
            Self::Error => 0,
            Self::Warn => 1,
            Self::Info => 2,
            Self::Verbose => 4,
            Self::Debug => 5,
        }
    }
}

#[allow(non_snake_case)]
pub trait ILoggerStrategy: Send + Sync {
    fn log(&self, level: LogLevels, message: &str, contextID: Option<&str>);
}
