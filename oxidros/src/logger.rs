//! Logging utilities.

pub use oxidros_core::logging::LoggingBuilder;

/// Extension trait that adds backend-specific default layers to a
/// [`LoggingBuilder`].
///
/// The implementation is selected at compile time via feature flags:
/// - `rcl` — adds the RCL layer (routes to rcutils)
/// - `zenoh` — adds the Zenoh layer + stderr fmt output
pub trait LoggingBuilderExt {
    /// Add the default logging layers for the active backend.
    fn with_default_layers(self) -> Self;
}

impl LoggingBuilderExt for LoggingBuilder {
    fn with_default_layers(self) -> Self {
        #[cfg(feature = "rcl")]
        {
            oxidros_wrapper::logger::with_default_layers(self)
        }
        #[cfg(feature = "zenoh")]
        {
            oxidros_zenoh::logger::with_default_layers(self)
        }
        #[cfg(not(any(feature = "rcl", feature = "zenoh")))]
        {
            self
        }
    }
}

#[cfg(feature = "rcl")]
pub use oxidros_wrapper::logger::init_ros_logging;

#[cfg(feature = "zenoh")]
pub use oxidros_zenoh::logger::init_ros_logging;
