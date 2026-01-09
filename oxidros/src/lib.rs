//! Unified ROS2 library for Rust.
//!
//! This crate provides a unified API for ROS2 communication with multiple backend options:
//!
//! - **RCL backend** (`rcl` feature): Uses the official ROS2 C library via FFI.
//!   Requires a ROS2 installation and a distribution feature (`humble`, `jazzy`, or `kilted`).
//!
//! - **Zenoh backend** (`zenoh` feature): Pure Rust implementation using Zenoh middleware.
//!   Compatible with `rmw_zenoh_cpp`. No ROS2 installation required.
//!
//! # Feature Flags
//!
//! Choose exactly one backend:
//!
//! - `humble` - RCL backend for ROS2 Humble
//! - `jazzy` - RCL backend for ROS2 Jazzy
//! - `kilted` - RCL backend for ROS2 Kilted
//! - `zenoh` - Zenoh backend (pure Rust)
//!
//! # Example
//!
//! ```ignore
//! use oxidros::prelude::*;
//!
//! let ctx = Context::new()?;
//! let node = ctx.create_node("my_node", None)?;
//! ```

// Compile-time check: ensure exactly one backend is selected
#[cfg(all(feature = "rcl", feature = "zenoh"))]
compile_error!("Features `rcl` and `zenoh` are mutually exclusive. Choose one backend.");

#[cfg(not(any(feature = "rcl", feature = "zenoh")))]
compile_error!("No backend selected. Enable one of: `humble`, `jazzy`, `kilted`, or `zenoh`.");

// Prelude module for convenient imports
pub mod prelude;

// Re-export the selected backend
#[cfg(all(feature = "rcl", not(feature = "zenoh")))]
pub use oxidros_rcl::{self, action, clock, logger, service, topic};

#[cfg(all(feature = "zenoh", not(feature = "rcl")))]
pub use oxidros_zenoh;

// Always re-export core types and traits
pub use oxidros_core::{self, error};

// Re-export message types
pub use oxidros_msg::{self, msg};
pub mod qos {
    pub use oxidros_core::qos::*;
}
