//! ROS2 logging integration with tracing.
//!
//! This module provides integration between the `tracing` crate and ROS2's
//! rcutils logging system.
//!
//! # Usage
//!
//! Initialize logging once at startup, then use standard `tracing` macros:
//!
//! ```ignore
//! use oxidros_rcl::logger::init_ros_logging;
//! use tracing::{info, warn, error, debug};
//!
//! init_ros_logging("my_node");
//!
//! info!("Hello from ROS2!");
//! debug!("Debug message");
//! warn!("Warning message");
//! error!("Error message");
//! ```
//!
//! # log crate support
//!
//! The `log` crate macros are automatically forwarded to tracing:
//!
//! ```ignore
//! use oxidros_rcl::logger::init_ros_logging;
//!
//! init_ros_logging("my_node");
//!
//! log::info!("This also works!");
//! ```

use crate::{error::Result, rcl};
use num_derive::{FromPrimitive, ToPrimitive};
use oxidros_core::logging::LoggingBuilder;
use oxidros_core::{Error, RclError};
use std::ffi::CString;
use std::sync::OnceLock;
use tracing::Subscriber;
use tracing_subscriber::Layer;

static INITIALIZER: OnceLock<std::result::Result<(), RclError>> = OnceLock::new();

/// Get the function name called this macro.
#[macro_export]
macro_rules! function {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        &name[..name.len() - 3]
    }};
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
enum Severity {
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl From<Severity> for rcl::rcl_log_severity_t {
    fn from(value: Severity) -> Self {
        use rcl::rcl_log_severity_t::*;
        match value {
            Severity::Debug => RCUTILS_LOG_SEVERITY_DEBUG,
            Severity::Info => RCUTILS_LOG_SEVERITY_INFO,
            Severity::Warn => RCUTILS_LOG_SEVERITY_WARN,
            Severity::Error => RCUTILS_LOG_SEVERITY_ERROR,
            Severity::Fatal => RCUTILS_LOG_SEVERITY_FATAL,
        }
    }
}

impl Severity {
    fn to_i32(self) -> i32 {
        let value: rcl::rcl_log_severity_t = self.into();
        value as i32
    }
}

/// Internal logger that writes to ROS2's rcutils logging.
#[derive(Debug)]
struct Logger {
    name: CString,
}

impl Logger {
    fn new(name: &str) -> Self {
        Logger {
            name: CString::new(name).unwrap(),
        }
    }

    fn write(
        &self,
        msg: &str,
        severity: Severity,
        function_name: &str,
        file_name: &str,
        line_number: u64,
    ) -> Result<()> {
        init_once()?;

        if !self.is_enable_for(severity) {
            let msg = format!(
                "log severity is not enabled on this system: severity = {:?}",
                severity
            );
            return Err(msg.into());
        }

        let function_name = CString::new(function_name)?;
        let file_name = CString::new(file_name)?;
        let msg = CString::new(msg)?;

        let logging_location = rcl::rcutils_log_location_t {
            function_name: function_name.as_ptr(),
            file_name: file_name.as_ptr(),
            line_number: line_number as _,
        };

        let guard = rcl::MT_UNSAFE_LOG_FN.lock();
        guard.rcutils_log(
            &logging_location,
            severity.to_i32(),
            self.name.as_ptr(),
            msg.as_ptr(),
        );

        Ok(())
    }

    fn is_enable_for(&self, severity: Severity) -> bool {
        let guard = rcl::MT_UNSAFE_LOG_FN.lock();
        guard.rcutils_logging_logger_is_enabled_for(self.name.as_ptr(), severity.to_i32())
    }
}

fn init_once() -> std::result::Result<(), RclError> {
    *INITIALIZER.get_or_init(|| {
        let guard = rcl::MT_UNSAFE_LOG_FN.lock();
        match guard.rcutils_logging_initialize() {
            Ok(v) => Ok(v),
            Err(Error::Rcl(e)) => Err(e),
            _ => Err(RclError::InvalidRetVal),
        }
    })
}

// ============================================================================
// Tracing-based logging (public API)
// ============================================================================

/// Initialize ROS2 logging with tracing integration.
///
/// This sets up:
/// 1. A tracing subscriber that routes to ROS2's rcutils logging
/// 2. A bridge that forwards `log` crate calls to tracing
///
/// The `name` parameter is used as the logger name for rcutils.
///
/// # Example
///
/// ```ignore
/// use oxidros_rcl::logger::init_ros_logging;
/// use tracing::{info, warn, error, debug};
///
/// init_ros_logging("my_node");
/// info!("Hello from ROS2!");
/// debug!("Debug message");
/// ```
pub fn init_ros_logging(name: &str) {
    with_default_layers(LoggingBuilder::new(name)).init();
}

/// Add the default RCL logging layers to the given builder.
///
/// This adds the [`RclLayer`] which routes tracing events to ROS2's
/// rcutils logging system.
pub fn with_default_layers(builder: LoggingBuilder) -> LoggingBuilder {
    let name = builder.name().to_string();
    builder.with_layer(RclLayer::new(&name))
}

/// Custom tracing layer that routes to ROS2's rcutils logging.
struct RclLayer {
    logger: Logger,
}

impl RclLayer {
    fn new(name: &str) -> Self {
        Self {
            logger: Logger::new(name),
        }
    }
}

impl<S> Layer<S> for RclLayer
where
    S: Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // Extract message from event
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let metadata = event.metadata();
        let level = metadata.level();

        // Map tracing level to our Severity
        let severity = match *level {
            tracing::Level::TRACE => Severity::Debug,
            tracing::Level::DEBUG => Severity::Debug,
            tracing::Level::INFO => Severity::Info,
            tracing::Level::WARN => Severity::Warn,
            tracing::Level::ERROR => Severity::Error,
        };

        // Get location info
        let file = metadata.file().unwrap_or("<unknown>");
        let line = metadata.line().unwrap_or(0) as u64;
        let module = metadata.module_path().unwrap_or("<unknown>");

        let _ = self
            .logger
            .write(&visitor.message, severity, module, file, line);
    }
}

/// Visitor to extract message from tracing event.
#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" || self.message.is_empty() {
            self.message = format!("{:?}", value);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" || self.message.is_empty() {
            self.message = value.to_string();
        }
    }
}

/// Re-export tracing macros for convenience.
pub use tracing::{debug, error, info, trace, warn};

#[cfg(test)]
mod test {
    use super::{Logger, Severity};

    #[test]
    fn test_logger() {
        let logger = Logger::new("test_logger");
        logger
            .write(
                "info message",
                Severity::Info,
                function!(),
                file!(),
                line!() as u64,
            )
            .unwrap();

        logger
            .write(
                "warn message",
                Severity::Warn,
                function!(),
                file!(),
                line!() as u64,
            )
            .unwrap();

        logger
            .write(
                "error message",
                Severity::Error,
                function!(),
                file!(),
                line!() as u64,
            )
            .unwrap();

        logger
            .write(
                "fatal message",
                Severity::Fatal,
                function!(),
                file!(),
                line!() as u64,
            )
            .unwrap();
    }

    #[test]
    fn test_init_ros_logging() {
        use super::init_ros_logging;

        // Should not panic when called
        init_ros_logging("test_tracing_node");

        // Should be idempotent - calling again should not panic
        init_ros_logging("test_tracing_node_2");
    }

    #[test]
    fn test_tracing_macros() {
        use super::init_ros_logging;
        use tracing::{debug, error, info, trace, warn};

        init_ros_logging("test_tracing_macros");

        // These should not panic and should route to rcutils
        trace!("trace message");
        debug!("debug message");
        info!("info message");
        warn!("warn message");
        error!("error message");
    }

    #[test]
    fn test_tracing_with_args() {
        use super::init_ros_logging;
        use tracing::{debug, info};

        init_ros_logging("test_tracing_args");

        let value = 42;
        let name = "test";

        info!(value, "message with field");
        info!("formatted: {} = {}", name, value);
        debug!(target: "custom_target", "targeted message");
    }

    #[test]
    fn test_log_crate_forwarding() {
        use super::init_ros_logging;

        init_ros_logging("test_log_forward");

        // log crate macros should be forwarded to tracing then to rcutils
        log::info!("log crate info");
        log::warn!("log crate warn");
        log::error!("log crate error");
    }
}
