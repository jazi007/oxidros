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
//! use oxidros_build::msg::{emit_ros_idl, generate_msgs};
//!
//! fn main() {
//!     // Generate types from specific packages
//!     generate_msgs(&["std_msgs", "geometry_msgs", "sensor_msgs"]);
//!
//!     // Or generate types from ALL packages
//!     generate_msgs(&[]);
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

pub(crate) fn is_ros2_env() -> bool {
    std::env::var("ROS_DISTRO").is_ok()
}

// Re-export public API
pub use config::{Config, ConfigBuilder};
pub use generator::{generate_msgs, generate_msgs_with_config, get_base_generator};
