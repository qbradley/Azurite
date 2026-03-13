use crate::i_logger_strategy::{ILoggerStrategy, LogLevels};

#[derive(Debug, Default)]
pub struct NoLoggerStrategy;

impl ILoggerStrategy for NoLoggerStrategy {
    fn log(&self, _level: LogLevels, _message: &str, _contextID: Option<&str>) {}
}
