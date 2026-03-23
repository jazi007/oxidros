//! Composable logging builder for oxidros.
//!
//! Provides a [`LoggingBuilder`] that assembles a `tracing` subscriber from
//! layers. Backend crates (`oxidros-rcl`, `oxidros-zenoh`) add their own
//! layers via `with_layer()`, and the `oxidros` facade exposes a
//! `with_default_layers()` extension that picks the right backend
//! automatically.
//!
//! # Examples
//!
//! ```ignore
//! use oxidros_core::logging::LoggingBuilder;
//!
//! // Minimal: just fmt output
//! LoggingBuilder::new("my_node")
//!     .with_fmt_layer()
//!     .init();
//!
//! // Custom filter + extra layer
//! LoggingBuilder::new("my_node")
//!     .with_filter("debug")
//!     .with_layer(my_custom_layer)
//!     .init();
//! ```

use std::sync::OnceLock;
use tracing_subscriber::{
    EnvFilter, Layer, Registry, fmt, layer::SubscriberExt, util::SubscriberInitExt,
};

static TRACING_INITIALIZED: OnceLock<()> = OnceLock::new();

/// A builder for composing `tracing` subscriber layers.
///
/// Collects layers and initializes the global tracing subscriber once via
/// [`init`](LoggingBuilder::init). Subsequent calls to `init` are no-ops.
pub struct LoggingBuilder {
    name: String,
    filter: Option<String>,
    layers: Vec<Box<dyn Layer<Registry> + Send + Sync>>,
    fmt_layer: bool,
    log_bridge: bool,
}

impl LoggingBuilder {
    /// Create a new builder with the given logger name.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            filter: None,
            layers: Vec::new(),
            fmt_layer: false,
            log_bridge: true,
        }
    }

    /// Returns the logger name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the `EnvFilter` directive string (e.g. `"debug"`, `"oxidros=trace"`).
    ///
    /// If not called, defaults to `RUST_LOG` env var or `"info"`.
    pub fn with_filter(mut self, filter: &str) -> Self {
        self.filter = Some(filter.to_string());
        self
    }

    /// Add an arbitrary [`Layer`] to the subscriber.
    pub fn with_layer<L>(mut self, layer: L) -> Self
    where
        L: Layer<Registry> + Send + Sync + 'static,
    {
        self.layers.push(Box::new(layer));
        self
    }

    /// Add a `tracing_subscriber::fmt` layer that writes to stderr.
    pub fn with_fmt_layer(mut self) -> Self {
        self.fmt_layer = true;
        self
    }

    /// Enable or disable the `log` → `tracing` bridge (default: enabled).
    pub fn with_log_bridge(mut self, enabled: bool) -> Self {
        self.log_bridge = enabled;
        self
    }

    /// Initialize the global tracing subscriber.
    ///
    /// Assembles all configured layers on top of a [`Registry`] and calls
    /// `try_init()`. This is guarded by a `OnceLock` — only the first call
    /// takes effect; subsequent calls are silently ignored.
    pub fn init(self) {
        TRACING_INITIALIZED.get_or_init(|| {
            if self.log_bridge {
                tracing_log::LogTracer::init().ok();
            }

            let filter = match self.filter {
                Some(f) => EnvFilter::try_new(f).unwrap_or_else(|_| EnvFilter::new("info")),
                None => {
                    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
                }
            };

            // Collect everything into a single Vec<Box<dyn Layer<Registry>>>
            // so that .with(layers) produces Registry -> Layered<Vec<...>, Registry>
            // which implements Subscriber + Into<Dispatch>.
            let mut layers: Vec<Box<dyn Layer<Registry> + Send + Sync>> = Vec::new();

            layers.push(Box::new(filter));

            for layer in self.layers {
                layers.push(layer);
            }

            if self.fmt_layer {
                let fmt = fmt::layer()
                    .with_target(true)
                    .with_thread_ids(false)
                    .with_thread_names(false)
                    .with_file(false)
                    .with_line_number(false)
                    .with_writer(std::io::stderr);
                layers.push(Box::new(fmt));
            }

            tracing_subscriber::registry().with(layers).try_init().ok();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_creation() {
        let builder = LoggingBuilder::new("test_node");
        assert_eq!(builder.name(), "test_node");
    }

    #[test]
    fn test_builder_with_filter() {
        let builder = LoggingBuilder::new("test_node").with_filter("debug");
        assert_eq!(builder.filter.as_deref(), Some("debug"));
    }

    #[test]
    fn test_builder_chaining() {
        let builder = LoggingBuilder::new("test_node")
            .with_filter("trace")
            .with_fmt_layer()
            .with_log_bridge(false);
        assert!(builder.fmt_layer);
        assert!(!builder.log_bridge);
    }
}
