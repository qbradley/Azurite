use std::sync::{Arc, LazyLock, RwLock};

use crate::{
    i_logger::ILogger,
    i_logger_strategy::{ILoggerStrategy, LogLevels},
    no_logger_strategy::NoLoggerStrategy,
    winston_logger_strategy::WinstonLoggerStrategy,
};

#[derive(Clone)]
pub struct Logger {
    pub strategy: Arc<dyn ILoggerStrategy>,
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
    fn error(&self, message: &str, contextID: Option<&str>) {
        self.strategy.log(LogLevels::Error, message, contextID);
    }

    fn warn(&self, message: &str, contextID: Option<&str>) {
        self.strategy.log(LogLevels::Warn, message, contextID);
    }

    fn info(&self, message: &str, contextID: Option<&str>) {
        self.strategy.log(LogLevels::Info, message, contextID);
    }

    fn verbose(&self, message: &str, contextID: Option<&str>) {
        self.strategy.log(LogLevels::Verbose, message, contextID);
    }

    fn debug(&self, message: &str, contextID: Option<&str>) {
        self.strategy.log(LogLevels::Debug, message, contextID);
    }
}

pub struct GlobalLogger {
    inner: RwLock<Logger>,
}

impl Default for GlobalLogger {
    fn default() -> Self {
        Self {
            inner: RwLock::new(Logger::default()),
        }
    }
}

impl GlobalLogger {
    pub fn set_strategy(&self, strategy: Arc<dyn ILoggerStrategy>) {
        self.inner.write().unwrap().set_strategy(strategy);
    }
}

impl ILogger for GlobalLogger {
    fn error(&self, message: &str, contextID: Option<&str>) {
        self.inner.read().unwrap().error(message, contextID);
    }

    fn warn(&self, message: &str, contextID: Option<&str>) {
        self.inner.read().unwrap().warn(message, contextID);
    }

    fn info(&self, message: &str, contextID: Option<&str>) {
        self.inner.read().unwrap().info(message, contextID);
    }

    fn verbose(&self, message: &str, contextID: Option<&str>) {
        self.inner.read().unwrap().verbose(message, contextID);
    }

    fn debug(&self, message: &str, contextID: Option<&str>) {
        self.inner.read().unwrap().debug(message, contextID);
    }
}

#[allow(non_upper_case_globals)]
pub static logger: LazyLock<GlobalLogger> = LazyLock::new(GlobalLogger::default);

#[allow(non_snake_case)]
pub fn configLogger(enable: bool, logFile: Option<String>) {
    if enable {
        logger.set_strategy(Arc::new(WinstonLoggerStrategy::new(
            LogLevels::Debug,
            logFile,
        )));
    } else {
        logger.set_strategy(Arc::new(NoLoggerStrategy));
    }
}
