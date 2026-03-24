//! Message code generation utilities.
//!
//! This module generates Rust types from ROS2 interface definition files
//! (`.msg`, `.srv`, `.action`, `.idl`). No ROS2 installation is required —
//! you can point directly to directories containing message definitions.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! // build.rs
//! fn main() {
//!     oxidros_build::ros2_env_var_changed();
//!
//!     let config = oxidros_build::msg::Config::builder()
//!         .packages(&["my_custom_msgs"])
//!         .build();
//!
//!     oxidros_build::msg::generate_msgs_with_config(&config);
//! }
//! ```
//!
//! Then in `lib.rs`:
//!
//! ```rust,ignore
//! include!(concat!(env!("OUT_DIR"), "/generated/mod.rs"));
//! ```
//!
//! # Without a ROS2 Installation
//!
//! Clone your message repos and use `extra_search_path()`:
//!
//! ```rust,ignore
//! let config = oxidros_build::msg::Config::builder()
//!     .packages(&["my_custom_msgs", "std_msgs"])
//!     .extra_search_path("/path/to/cloned/common_interfaces")
//!     .extra_search_path("/path/to/my/custom_msgs")
//!     .build();
//!
//! oxidros_build::msg::generate_msgs_with_config(&config);
//! ```
//!
//! The directory layout should follow the standard ROS2 convention:
//!
//! ```text
//! my_custom_msgs/
//! ├── msg/
//! │   ├── MyMessage.msg
//! │   └── AnotherMessage.msg
//! └── srv/
//!     └── MyService.srv
//! ```
//!
//! # API Levels
//!
//! - [`generate_msgs`] — Simple: just pass package names
//! - [`generate_msgs_with_config`] — Full control via [`Config`]
//! - [`get_base_generator`] — Low-level: returns a [`ros2msg::Generator`] for customization
//! - [`detect_ros_availability`] — Check if/how ROS2 is available
//!
//! # Package Discovery
//!
//! Packages are searched for in this order:
//!
//! 1. **`AMENT_PREFIX_PATH`** — If set (sourced ROS2 environment)
//! 2. **Common paths** — `/opt/ros/{humble,jazzy,kilted}` (Linux), `C:\dev\ros2_*` (Windows)
//! 3. **Extra paths** — Via [`ConfigBuilder::extra_search_path`]
//!
//! For Zenoh-based or non-RCL projects, just use `extra_search_path()` to
//! point directly to your message definitions — no ROS2 install needed.
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
