//! Parameter types.

#[cfg(feature = "rcl")]
pub use oxidros_wrapper::ParameterServer;

#[cfg(feature = "zenoh")]
pub use oxidros_zenoh::parameter::ParameterServer;
