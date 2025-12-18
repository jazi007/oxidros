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

//! RCL (ROS Client Library) bindings for oxidros.
//!
//! This crate provides low-level Rust bindings to ROS2's RCL C library.
//! Select the appropriate ROS2 distribution using features: `jazzy`, `iron`, `humble`, or `galactic`.
//!
//! Bindings are generated at compile time using bindgen.

// Include the generated RCL bindings
include!(concat!(env!("OUT_DIR"), "/rcl.rs"));

#[cfg(any(feature = "humble", feature = "galactic"))]
pub type size_t = self::size_t;

#[cfg(any(feature = "iron", feature = "jazzy"))]
pub type size_t = usize;

// Type conversions
mod conversions;
