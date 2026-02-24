#![allow(dead_code)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(deref_nullptr)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]
#![allow(unused_imports)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::manual_c_str_literals)]
#![allow(clippy::useless_conversion)]

// Re-export all messages from oxidros-msg crate (generated at compile time)
#[cfg(any(ros_distro_humble, ros_distro_jazzy, ros_distro_kilted))]
pub use oxidros_msg::*;

// Re-export submodules explicitly (pub use * doesn't re-export modules)
#[cfg(any(ros_distro_humble, ros_distro_jazzy, ros_distro_kilted))]
pub use oxidros_msg::{common_interfaces, interfaces, primitives::*, ros2msg, strings::*};
