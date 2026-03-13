use std::sync::Arc;

use crate::{
    i_logger::ILogger,
    i_logger_strategy::{ILoggerStrategy, LogLevels},
};

#[derive(Debug, Default)]
pub struct NoLoggerStrategy;

impl ILoggerStrategy for NoLoggerStrategy {
    fn log(&self, _level: LogLevels, _message: &str, _context_id: Option<&str>) {}
}

#[derive(Clone)]
pub struct Logger {
    strategy: Arc<dyn ILoggerStrategy>,
}

impl Default for Logger {
    fn default() -> Self {
        Self::new(Arc::new(NoLoggerStrategy))
    }
}

impl Logger {
    pub fn new(strategy: Arc<dyn ILoggerStrategy>) -> Self {
        Self { strategy }
    }

    pub fn set_strategy(&mut self, strategy: Arc<dyn ILoggerStrategy>) {
        self.strategy = strategy;
    }
}

impl ILogger for Logger {
    fn error(&self, message: &str, context_id: Option<&str>) {
        self.strategy.log(LogLevels::Error, message, context_id);
    }

    fn warn(&self, message: &str, context_id: Option<&str>) {
        self.strategy.log(LogLevels::Warn, message, context_id);
    }

    fn info(&self, message: &str, context_id: Option<&str>) {
        self.strategy.log(LogLevels::Info, message, context_id);
    }

    fn verbose(&self, message: &str, context_id: Option<&str>) {
        self.strategy.log(LogLevels::Verbose, message, context_id);
    }

    fn debug(&self, message: &str, context_id: Option<&str>) {
        self.strategy.log(LogLevels::Debug, message, context_id);
    }
}
