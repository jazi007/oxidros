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
#[cfg(any(
    feature = "galactic",
    feature = "humble",
    feature = "iron",
    feature = "jazzy"
))]
pub use oxidros_msg::*;

// Re-export submodules explicitly (pub use * doesn't re-export modules)
#[cfg(any(
    feature = "galactic",
    feature = "humble",
    feature = "iron",
    feature = "jazzy"
))]
pub use oxidros_msg::{common_interfaces, interfaces, ros2msg};
