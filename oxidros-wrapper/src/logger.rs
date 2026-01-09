//! Implement logger for ros2 logging
//!
use log::*;
use oxidros::{
    logger::Logger as SdLogger,
    oxidros_rcl::{pr_debug, pr_error, pr_info, pr_warn},
};
use std::sync::Arc;

use log::{Level, Metadata, Record};

struct Logger(Arc<SdLogger>);

impl Logger {
    /// [Logger] constructor
    fn new(name: &str) -> Self {
        Self(Arc::new(SdLogger::new(name)))
    }

    /// init
    fn init(self) -> Result<(), SetLoggerError> {
        set_max_level(LevelFilter::Trace);
        set_boxed_logger(Box::new(self))?;
        Ok(())
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }
    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            match record.level() {
                Level::Trace => pr_debug!(self.0, "{}", record.args()),
                Level::Debug => pr_debug!(self.0, "{}", record.args()),
                Level::Info => pr_info!(self.0, "{}", record.args()),
                Level::Warn => pr_warn!(self.0, "{}", record.args()),
                Level::Error => pr_error!(self.0, "{}", record.args()),
            }
        }
    }
    fn flush(&self) {}
}

/// Initialize the logger with the node name
pub fn init_logger(name: &str) -> Result<(), SetLoggerError> {
    Logger::new(name).init()
}
