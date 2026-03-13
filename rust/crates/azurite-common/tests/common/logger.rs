use std::sync::{Arc, Mutex};

use azurite_common::{
    i_logger::ILogger,
    i_logger_strategy::{ILoggerStrategy, LogLevels},
    logger::Logger,
};
use pretty_assertions::assert_eq;

#[derive(Debug, Clone, Eq, PartialEq)]
struct RecordedLog {
    level: LogLevels,
    message: String,
    context_id: Option<String>,
}

#[derive(Default)]
struct RecordingStrategy {
    entries: Mutex<Vec<RecordedLog>>,
}

impl RecordingStrategy {
    fn entries(&self) -> Vec<RecordedLog> {
        self.entries
            .lock()
            .expect("recording mutex poisoned")
            .clone()
    }
}

impl ILoggerStrategy for RecordingStrategy {
    fn log(&self, level: LogLevels, message: &str, context_id: Option<&str>) {
        self.entries
            .lock()
            .expect("recording mutex poisoned")
            .push(RecordedLog {
                level,
                message: message.to_owned(),
                context_id: context_id.map(str::to_owned),
            });
    }
}

#[test]
fn logger_forwards_levels_and_context_to_strategy() {
    let strategy = Arc::new(RecordingStrategy::default());
    let logger = Logger::new(strategy.clone());

    logger.error("error", Some("ctx-1"));
    logger.warn("warn", Some("ctx-2"));
    logger.info("info", None);
    logger.verbose("verbose", Some("ctx-3"));
    logger.debug("debug", None);

    assert_eq!(
        strategy.entries(),
        vec![
            RecordedLog {
                level: LogLevels::Error,
                message: "error".to_owned(),
                context_id: Some("ctx-1".to_owned()),
            },
            RecordedLog {
                level: LogLevels::Warn,
                message: "warn".to_owned(),
                context_id: Some("ctx-2".to_owned()),
            },
            RecordedLog {
                level: LogLevels::Info,
                message: "info".to_owned(),
                context_id: None,
            },
            RecordedLog {
                level: LogLevels::Verbose,
                message: "verbose".to_owned(),
                context_id: Some("ctx-3".to_owned()),
            },
            RecordedLog {
                level: LogLevels::Debug,
                message: "debug".to_owned(),
                context_id: None,
            },
        ]
    );
}

#[test]
fn default_logger_uses_noop_strategy() {
    let logger = Logger::default();

    logger.info("no-op", Some("ctx"));
    logger.debug("still no-op", None);
}

#[test]
fn logger_can_swap_strategies_after_construction() {
    let initial = Arc::new(RecordingStrategy::default());
    let replacement = Arc::new(RecordingStrategy::default());
    let mut logger = Logger::new(initial.clone());

    logger.info("before swap", Some("ctx-1"));
    logger.set_strategy(replacement.clone());
    logger.info("after swap", Some("ctx-2"));

    assert_eq!(
        initial.entries(),
        vec![RecordedLog {
            level: LogLevels::Info,
            message: "before swap".to_owned(),
            context_id: Some("ctx-1".to_owned()),
        }]
    );
    assert_eq!(
        replacement.entries(),
        vec![RecordedLog {
            level: LogLevels::Info,
            message: "after swap".to_owned(),
            context_id: Some("ctx-2".to_owned()),
        }]
    );
}
