//! Logging utilities.

#[cfg(feature = "rcl")]
pub use oxidros_wrapper::logger::init_ros_logging;

#[cfg(feature = "zenoh")]
pub use oxidros_zenoh::logger::init_ros_logging;
