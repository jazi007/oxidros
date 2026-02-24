//! ROS2 message code generation utilities.
//!
//! This module provides functionality for generating Rust types from ROS2 interface
//! definition files (`.msg`, `.srv`, `.action`, `.idl`). It uses the [`ros2msg`] crate
//! for parsing and code generation, combined with [`ros2_types`] derive macros for
//! generating type support code.
//!
//! # Overview
//!
//! The module provides several ways to generate ROS2 message types:
//!
//! - [`generate_msgs`] - Simple API for generating types from specific packages
//! - [`generate_msgs_with_config`] - Full control via [`Config`] struct
//! - [`get_base_generator`] - Low-level access to the generator for customization
//! - [`detect_ros_availability`] - Check if ROS2 is available and how
//!
//! # ROS2 Detection
//!
//! The module uses [`RosAvailability`] to represent three possible states:
//!
//! - **Sourced**: `AMENT_PREFIX_PATH` is set (full functionality with linking)
//! - **CommonInstall**: ROS2 found at common paths but not sourced (generation only)
//! - **NotAvailable**: No ROS2 installation (use pre-generated files)
//!
//! # Configuration
//!
//! Use [`Config`] and [`ConfigBuilder`] to customize the generation process:
//!
//! ```rust,ignore
//! use oxidros_build::msg::{Config, generate_msgs_with_config};
//!
//! let config = Config::builder()
//!     .packages(&["std_msgs", "geometry_msgs"])
//!     .uuid_path("my_crate::unique_identifier_msgs")
//!     .primitive_path("oxidros_msg")
//!     .extra_search_path("/custom/ros2/share")
//!     .build();
//!
//! generate_msgs_with_config(&config);
//! ```
//!
//! # Path Resolution
//!
//! The module automatically finds ROS2 packages using this priority:
//!
//! 1. **AMENT_PREFIX_PATH** - If set (standard sourced ROS2 environment)
//! 2. **Common installation paths** - Falls back to standard locations:
//!    - Linux: `/opt/ros/{humble,jazzy,kilted}`
//!    - Windows: `C:\pixi_ws\ros2-windows`, `C:\dev\ros2_{distro}`, etc.
//! 3. **Extra paths** - User-provided paths via [`ConfigBuilder::extra_search_path`]
//!
//! # Example
//!
//! ```rust,ignore
//! // In build.rs
//! use oxidros_build::msg::{detect_ros_availability, Config, RosAvailability};
//!
//! fn main() {
//!     let config = Config::builder()
//!         .packages(&["std_msgs", "geometry_msgs"])
//!         .build();
//!
//!     match detect_ros_availability(&config) {
//!         RosAvailability::Sourced { .. } | RosAvailability::CommonInstall { .. } => {
//!             // Generate messages
//!             oxidros_build::msg::generate_msgs_with_config(&config);
//!         }
//!         RosAvailability::NotAvailable => {
//!             // Use pre-committed generated files
//!             println!("cargo:warning=Using pre-generated message files");
//!         }
//!     }
//! }
//! ```
//!
//! # Generated Output
//!
//! The generated Rust code includes:
//! - Struct definitions for each message/service/action type
//! - `#[ros2(...)]` attributes for FFI interop
//! - `Ros2Msg` derive macro implementations for type support
//! - Proper module hierarchy matching the ROS2 package structure
//!
//! # Interface File Priority
//!
//! When both `.idl` and native (`.msg`, `.srv`, `.action`) files exist for the
//! same interface, the native files take priority to avoid duplicate definitions.

mod callbacks;
mod config;
mod generator;

/// Returns true if a ROS2 environment is sourced (ROS_DISTRO is set).
///
/// This is a simple check for whether ROS2 environment variables are available.
/// For more detailed detection, use [`detect_ros_availability`].
///
/// # Example
///
/// ```rust,ignore
/// if oxidros_build::msg::is_ros2_sourced() {
///     // Link ROS2 libraries
/// }
/// ```
pub fn is_ros2_sourced() -> bool {
    std::env::var("ROS_DISTRO").is_ok()
}

// Re-export public API
pub use config::{Config, ConfigBuilder};
pub use generator::{
    RosAvailability, detect_ros_availability, generate_msgs, generate_msgs_with_config,
    get_base_generator,
};
