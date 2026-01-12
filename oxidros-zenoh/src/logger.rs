//! ROS2 logging integration using tracing.
//!
//! This module provides a tracing-based logging system that integrates with
//! the standard Rust `tracing` ecosystem. It also captures `log` crate calls
//! and forwards them to tracing.
//!
//! # Example
//!
//! ```ignore
//! use oxidros_zenoh::logger::init_ros_logging;
//! use tracing::{info, warn, error, debug};
//!
//! // Initialize logging (call once at startup)
//! init_ros_logging("my_node");
//!
//! // Now use tracing macros
//! info!("Node started");
//! debug!("Processing message");
//! warn!("Something unexpected");
//! error!("Failed to connect");
//!
//! // Or use log crate (also works)
//! log::info!("This also works!");
//! ```

use std::sync::OnceLock;
use tracing::Subscriber;
use tracing_subscriber::{
    EnvFilter, Layer,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

static LOGGER_INITIALIZED: OnceLock<()> = OnceLock::new();

/// Initialize ROS2 logging with tracing integration.
///
/// This sets up:
/// 1. A tracing subscriber that outputs to stderr with ROS2-style formatting
/// 2. A bridge that forwards `log` crate calls to tracing
///
/// The `name` parameter is used as the logger name prefix in output.
///
/// # Panics
///
/// This function will not panic if called multiple times - subsequent calls are ignored.
///
/// # Example
///
/// ```ignore
/// use oxidros_zenoh::logger::init_ros_logging;
/// use tracing::info;
///
/// init_ros_logging("my_node");
/// info!("Hello from ROS2!");
/// ```
pub fn init_ros_logging(name: &str) {
    LOGGER_INITIALIZED.get_or_init(|| {
        // Set up log -> tracing bridge
        tracing_log::LogTracer::init().ok();

        // Create the subscriber with our custom layer
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

        let fmt_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_file(true)
            .with_line_number(true)
            .with_span_events(FmtSpan::NONE)
            .with_writer(std::io::stderr);

        tracing_subscriber::registry()
            .with(filter)
            .with(ZenohLayer::new(name))
            .with(fmt_layer)
            .try_init()
            .ok();
    });
}

/// Custom tracing layer for Zenoh ROS2 logging.
///
/// This layer can be extended to:
/// - Publish to /rosout topic
/// - Format messages in ROS2 style
/// - Filter based on node name
struct ZenohLayer {
    #[allow(dead_code)]
    node_name: String,
}

impl ZenohLayer {
    fn new(name: &str) -> Self {
        Self {
            node_name: name.to_string(),
        }
    }
}

impl<S> Layer<S> for ZenohLayer
where
    S: Subscriber,
{
    fn on_event(
        &self,
        _event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // Future: publish to /rosout topic
        // For now, the fmt_layer handles output
    }
}

/// Re-export tracing macros for convenience.
pub use tracing::{debug, error, info, trace, warn};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_ros_logging() {
        // Should not panic when called
        init_ros_logging("test_node");

        // Should be idempotent - calling again should not panic
        init_ros_logging("test_node_2");
    }

    #[test]
    fn test_tracing_macros() {
        init_ros_logging("test_macros");

        // These should not panic
        trace!("trace message");
        debug!("debug message");
        info!("info message");
        warn!("warn message");
        error!("error message");
    }

    #[test]
    fn test_tracing_with_args() {
        init_ros_logging("test_args");

        let value = 42;
        let name = "test";

        info!(value, "message with field");
        info!("formatted: {} = {}", name, value);
        debug!(target: "custom_target", "targeted message");
    }

    #[test]
    fn test_log_crate_forwarding() {
        init_ros_logging("test_log_forward");

        // log crate macros should be forwarded to tracing
        log::info!("log crate info");
        log::warn!("log crate warn");
        log::error!("log crate error");
    }
}
