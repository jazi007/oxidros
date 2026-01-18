//! Error types for oxidros-zenoh.
//!
//! This module re-exports the unified error types from `oxidros-core`
//! and provides Zenoh-specific conversions.

// Re-export the unified error types from oxidros-core
pub use oxidros_core::error::{Error, Result};

// ============================================================================
// Zenoh-specific conversion helpers
// ============================================================================

/// Extension trait to convert Result types with ros2args errors.
pub trait Ros2ArgsResultExt<T> {
    /// Convert a ros2args error to an oxidros Error.
    fn map_name_err(self) -> Result<T>;
}

impl<T> Ros2ArgsResultExt<T> for std::result::Result<T, ros2args::Ros2ArgsError> {
    fn map_name_err(self) -> Result<T> {
        self.map_err(|e| Error::InvalidName(e.to_string()))
    }
}
