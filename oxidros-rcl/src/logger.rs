//! Logger of ROS2.
//!
//! # Examples
//!
//! ## Basics
//!
//! ```
//! use oxidros_rcl::{logger::Logger, pr_debug, pr_error, pr_fatal, pr_info, pr_warn};
//!
//! let logger = Logger::new("my_logger");
//! let some_value = 100;
//!
//! pr_debug!(logger, "debug: {some_value}");
//! pr_info!(logger, "information: {some_value}");
//! pr_warn!(logger, "warning: {some_value}");
//! pr_error!(logger, "error: {some_value}");
//! pr_fatal!(logger, "fatal: {some_value}");
//! ```
//!
//! ## Callback Functions of Single Threaded Execution
//!
//! ```
//! use std::{rc::Rc, time::Duration};
//! use oxidros_rcl::{context::Context, logger::Logger, pr_error, pr_info};
//!
//! let ctx = Context::new().unwrap();
//! let mut selector = ctx.create_selector().unwrap();
//!
//! // Use Rc to share the logger by multiple callback functions.
//! let logger = Logger::new("my_logger");
//! let logger = Rc::new(logger);
//! let logger1 = logger.clone();
//!
//! selector.add_wall_timer(
//!     "timer1", // name of the timer
//!     Duration::from_millis(100),
//!     Box::new(move || pr_info!(logger1, "some information")),
//! );
//!
//! selector.add_wall_timer(
//!     "timer2", // name of the timer
//!     Duration::from_millis(150),
//!     Box::new(move || pr_error!(logger, "some error")),
//! );
//! ```
//!
//! ## Multi Threaded
//!
//! ```
//! use oxidros_rcl::{logger::Logger, pr_info, pr_warn};
//! use std::sync::Arc;
//!
//! let logger = Logger::new("my_logger");
//!
//! // Use Arc to share a logger by multiple threads.
//! let logger = Arc::new(logger);
//! let logger1 = logger.clone();
//!
//! let th1 = std::thread::spawn(move || pr_info!(logger1, "some information"));
//! let th2 = std::thread::spawn(move || pr_warn!(logger, "some warning"));
//!
//! th1.join().unwrap();
//! th2.join().unwrap();
//! ```

use crate::{error::Result, rcl};
use num_derive::{FromPrimitive, ToPrimitive};
use oxidros_core::{Error, RclError};
use std::ffi::CString;

static INITIALIZER: std::sync::OnceLock<std::result::Result<(), RclError>> =
    std::sync::OnceLock::new();

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

/// Print information.
#[macro_export]
macro_rules! pr_info {
    ($logger:expr, $($arg:tt)*) => {{
        let res = format!($($arg)*);
        let _ = $logger.write_info(&res, $crate::function!(), file!(), line!() as u64);
    }}
}

macro_rules! pr_info_in {
    ($logger:expr, $($arg:tt)*) => {{
        let res = std::format!($($arg)*);
        let _ = $logger.write_info(&res, $crate::function!(), std::file!(), std::line!() as u64);
    }}
}
pub(crate) use pr_info_in;

/// Print warning.
#[macro_export]
macro_rules! pr_warn {
    ($logger:expr, $($arg:tt)*) => {{
        let res = format!($($arg)*);
        let _ = $logger.write_warn(&res, $crate::function!(), file!(), line!() as u64);
    }}
}

/// Print error.
#[macro_export]
macro_rules! pr_error {
    ($logger:expr, $($arg:tt)*) => {{
        let res = format!($($arg)*);
        let _ = $logger.write_error(&res, $crate::function!(), file!(), line!() as u64);
    }}
}

macro_rules! pr_error_in {
    ($logger:expr, $($arg:tt)*) => {{
        let res = std::format!($($arg)*);
        let _ = $logger.write_error(&res, crate::function!(), std::file!(), std::line!() as u64);
    }}
}
pub(crate) use pr_error_in;

/// Print fatal.
#[macro_export]
macro_rules! pr_fatal {
    ($logger:expr, $($arg:tt)*) => {{
        let res = format!($($arg)*);
        let _ = $logger.write_fatal(&res, $crate::function!(), file!(), line!() as u64);
    }}
}

macro_rules! pr_fatal_in {
    ($logger:expr, $($arg:tt)*) => {{
        let res = std::format!($($arg)*);
        let _ = $logger.write_error(&res, crate::function!(), std::file!(), std::line!() as u64);
    }}
}
pub(crate) use pr_fatal_in;

/// Print debug.
/// Debug messages is not printed by default.
/// To enable debug print, type as follows.
///
/// ```text
/// ros2 run logging_demo logging_demo_main --ros-args --log-level debug
/// ```
#[macro_export]
macro_rules! pr_debug {
    ($logger:expr, $($arg:tt)*) => {{
        let res = format!($($arg)*);
        let _ = $logger.write_debug(&res, $crate::function!(), file!(), line!() as u64);
    }}
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

/// Logger of ROS2.
/// The methods of Logger are called by pr_* macros.
/// Use these macros instead of the methods.
#[derive(Debug)]
pub struct Logger {
    name: CString,
}

impl Logger {
    /// Create a new logger.
    pub fn new(name: &str) -> Self {
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
        init_once()?; // first of all, initialize the logging system

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

    /// Print information.
    /// Use `pr_info!` macro instead of this.
    pub fn write_info(
        &self,
        msg: &str,
        function_name: &str,
        file_name: &str,
        line_number: u64,
    ) -> Result<()> {
        self.write(msg, Severity::Info, function_name, file_name, line_number)
    }

    /// Print warning.
    /// Use `pr_warn!` macro instead of this.
    pub fn write_warn(
        &self,
        msg: &str,
        function_name: &str,
        file_name: &str,
        line_number: u64,
    ) -> Result<()> {
        self.write(msg, Severity::Warn, function_name, file_name, line_number)
    }

    /// Print error.
    /// Use `pr_error!` macro instead of this.
    pub fn write_error(
        &self,
        msg: &str,
        function_name: &str,
        file_name: &str,
        line_number: u64,
    ) -> Result<()> {
        self.write(msg, Severity::Error, function_name, file_name, line_number)
    }

    /// Print fatal.
    /// Use `pr_fatal!` macro instead of this.
    pub fn write_fatal(
        &self,
        msg: &str,
        function_name: &str,
        file_name: &str,
        line_number: u64,
    ) -> Result<()> {
        self.write(msg, Severity::Fatal, function_name, file_name, line_number)
    }

    /// Print debug.
    /// Use `pr_debug!` macro instead of this.
    pub fn write_debug(
        &self,
        msg: &str,
        function_name: &str,
        file_name: &str,
        line_number: u64,
    ) -> Result<()> {
        self.write(msg, Severity::Debug, function_name, file_name, line_number)
    }

    fn is_enable_for(&self, severity: Severity) -> bool {
        let guard = rcl::MT_UNSAFE_LOG_FN.lock();
        guard.rcutils_logging_logger_is_enabled_for(self.name.as_ptr(), severity.to_i32())
    }
}

fn init_once() -> std::result::Result<(), RclError> {
    *INITIALIZER.get_or_init(|| {
        // initialize
        let guard = rcl::MT_UNSAFE_LOG_FN.lock();
        match guard.rcutils_logging_initialize() {
            Ok(v) => Ok(v),
            Err(Error::Rcl(e)) => Err(e),
            _ => Err(RclError::InvalidRetVal),
        }
    })
}

// ============================================================================
// Tracing-based logging (modern API)
// ============================================================================

use std::sync::OnceLock;
use tracing::Subscriber;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

static TRACING_INITIALIZED: OnceLock<()> = OnceLock::new();

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
    TRACING_INITIALIZED.get_or_init(|| {
        // Initialize rcutils logging first
        let _ = init_once();

        // Set up log -> tracing bridge
        tracing_log::LogTracer::init().ok();

        // Create the subscriber with our RCL layer
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

        tracing_subscriber::registry()
            .with(filter)
            .with(RclLayer::new(name))
            .try_init()
            .ok();
    });
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
