//! Clock types.

#[cfg(feature = "rcl")]
pub use oxidros_wrapper::Clock;

#[cfg(feature = "zenoh")]
pub use oxidros_zenoh::clock::Clock;
