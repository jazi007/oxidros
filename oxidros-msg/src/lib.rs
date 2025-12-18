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

//! Generated ROS2 message types for oxidros.
//!
//! This crate provides Rust bindings for ROS2 messages, services, and actions.
//! Select the appropriate ROS2 distribution using features: `jazzy`, `iron`, `humble`, or `galactic`.
//!
//! Messages are generated at compile time using ros2-msg-gen and bindgen.

use std::{ffi::CString, fmt::Display, mem::transmute};

// Include runtime C bindings first (provides rosidl_runtime_c types)
mod runtime_c {
    include!(concat!(env!("OUT_DIR"), "/runtime_c.rs"));
}

// Re-export runtime_c types
pub use runtime_c::*;

// Definition of Sequence types -------------------------------------------------------------------

macro_rules! def_sequence {
    ($ty: ident, $ty_orig:ty, $ty_seq:ty, $init:ident, $fini:ident, $eq:ident) => {
        /// A sequence of elements.
        /// `N` represents the maximum number of elements.
        /// If `N` is `0`, the sequence is unlimited.
        #[repr(C)]
        #[derive(Debug)]
        pub struct $ty<const N: usize>($ty_seq);

        impl<const N: usize> $ty<N> {
            pub fn new(size: usize) -> Option<Self> {
                if N != 0 && size > N {
                    // the size exceeds in the maximum number
                    return None;
                }

                let mut msg: $ty_seq = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
                if unsafe { $init(&mut msg, size as _) } {
                    Some($ty(msg))
                } else {
                    None
                }
            }

            pub fn null() -> Self {
                let msg: $ty_seq = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
                $ty(msg)
            }

            pub fn as_slice(&self) -> &[$ty_orig] {
                if self.0.data.is_null() {
                    &[]
                } else {
                    let s =
                        unsafe { std::slice::from_raw_parts(self.0.data, self.0.size as usize) };
                    s
                }
            }

            pub fn as_slice_mut(&mut self) -> &mut [$ty_orig] {
                if self.0.data.is_null() {
                    &mut []
                } else {
                    let s = unsafe {
                        std::slice::from_raw_parts_mut(self.0.data, self.0.size as usize)
                    };
                    s
                }
            }

            pub fn iter(&self) -> std::slice::Iter<'_, $ty_orig> {
                self.as_slice().iter()
            }

            pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, $ty_orig> {
                self.as_slice_mut().iter_mut()
            }

            pub fn len(&self) -> usize {
                self.as_slice().len()
            }

            pub fn is_empty(&self) -> bool {
                self.len() == 0
            }
        }

        impl<const N: usize> Drop for $ty<N> {
            fn drop(&mut self) {
                unsafe { $fini(&mut self.0 as *mut _) };
            }
        }

        impl<const N: usize> PartialEq for $ty<N> {
            fn eq(&self, other: &Self) -> bool {
                unsafe { $eq(&self.0, &other.0) }
            }
        }

        unsafe impl<const N: usize> Sync for $ty<N> {}
        unsafe impl<const N: usize> Send for $ty<N> {}
    };
}

def_sequence!(
    BoolSeq,
    bool,
    rosidl_runtime_c__boolean__Sequence,
    rosidl_runtime_c__boolean__Sequence__init,
    rosidl_runtime_c__boolean__Sequence__fini,
    rosidl_runtime_c__boolean__Sequence__are_equal
);

def_sequence!(
    F32Seq,
    f32,
    rosidl_runtime_c__float__Sequence,
    rosidl_runtime_c__float__Sequence__init,
    rosidl_runtime_c__float__Sequence__fini,
    rosidl_runtime_c__float__Sequence__are_equal
);

def_sequence!(
    F64Seq,
    f64,
    rosidl_runtime_c__double__Sequence,
    rosidl_runtime_c__double__Sequence__init,
    rosidl_runtime_c__double__Sequence__fini,
    rosidl_runtime_c__double__Sequence__are_equal
);

def_sequence!(
    U8Seq,
    u8,
    rosidl_runtime_c__uint8__Sequence,
    rosidl_runtime_c__uint8__Sequence__init,
    rosidl_runtime_c__uint8__Sequence__fini,
    rosidl_runtime_c__uint8__Sequence__are_equal
);

def_sequence!(
    I8Seq,
    i8,
    rosidl_runtime_c__int8__Sequence,
    rosidl_runtime_c__int8__Sequence__init,
    rosidl_runtime_c__int8__Sequence__fini,
    rosidl_runtime_c__int8__Sequence__are_equal
);

def_sequence!(
    U16Seq,
    u16,
    rosidl_runtime_c__uint16__Sequence,
    rosidl_runtime_c__uint16__Sequence__init,
    rosidl_runtime_c__uint16__Sequence__fini,
    rosidl_runtime_c__uint16__Sequence__are_equal
);

def_sequence!(
    I16Seq,
    i16,
    rosidl_runtime_c__int16__Sequence,
    rosidl_runtime_c__int16__Sequence__init,
    rosidl_runtime_c__int16__Sequence__fini,
    rosidl_runtime_c__int16__Sequence__are_equal
);

def_sequence!(
    U32Seq,
    u32,
    rosidl_runtime_c__uint32__Sequence,
    rosidl_runtime_c__uint32__Sequence__init,
    rosidl_runtime_c__uint32__Sequence__fini,
    rosidl_runtime_c__uint32__Sequence__are_equal
);

def_sequence!(
    I32Seq,
    i32,
    rosidl_runtime_c__int32__Sequence,
    rosidl_runtime_c__int32__Sequence__init,
    rosidl_runtime_c__int32__Sequence__fini,
    rosidl_runtime_c__int32__Sequence__are_equal
);

def_sequence!(
    U64Seq,
    u64,
    rosidl_runtime_c__uint64__Sequence,
    rosidl_runtime_c__uint64__Sequence__init,
    rosidl_runtime_c__uint64__Sequence__fini,
    rosidl_runtime_c__uint64__Sequence__are_equal
);

def_sequence!(
    I64Seq,
    i64,
    rosidl_runtime_c__int64__Sequence,
    rosidl_runtime_c__int64__Sequence__init,
    rosidl_runtime_c__int64__Sequence__fini,
    rosidl_runtime_c__int64__Sequence__are_equal
);

// Definition of String ---------------------------------------------------------------------------

/// String.
/// `N` represents the maximum number of characters excluding `\0`.
/// If `N` is `0`, the string is unlimited.
#[repr(C)]
#[derive(Debug)]
pub struct RosString<const N: usize>(rosidl_runtime_c__String);

impl<const N: usize> RosString<N> {
    pub fn new(s: &str) -> Option<Self> {
        let mut msg: rosidl_runtime_c__String =
            unsafe { std::mem::MaybeUninit::zeroed().assume_init() };

        // initialize string
        if unsafe { rosidl_runtime_c__String__init(&mut msg) } {
            if Self::assign_string(&mut msg, s) {
                Some(Self(msg))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn null() -> Self {
        let msg: rosidl_runtime_c__String =
            unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
        Self(msg)
    }

    fn assign_string(msg: &mut rosidl_runtime_c__String, s: &str) -> bool {
        let cs = CString::new(s).unwrap();

        // assign string
        if N == 0 {
            unsafe { rosidl_runtime_c__String__assign(msg, cs.as_ptr()) }
        } else {
            unsafe { rosidl_runtime_c__String__assignn(msg, cs.as_ptr(), N as _) }
        }
    }

    pub fn assign(&mut self, s: &str) -> bool {
        Self::assign_string(&mut self.0, s)
    }

    #[allow(clippy::unnecessary_cast)]
    pub fn as_slice(&self) -> &[std::os::raw::c_char] {
        if self.0.data.is_null() {
            &[]
        } else {
            let s = unsafe { std::slice::from_raw_parts(self.0.data, self.0.size as usize) };
            s
        }
    }

    #[allow(clippy::unnecessary_cast)]
    pub fn as_slice_mut(&mut self) -> &mut [std::os::raw::c_char] {
        if self.0.data.is_null() {
            &mut []
        } else {
            let s = unsafe { std::slice::from_raw_parts_mut(self.0.data, self.0.size as usize) };
            s
        }
    }

    pub fn get_string(&self) -> String {
        if let Ok(m) = String::from_utf8(self.as_slice().iter().map(|&c| c as u8).collect()) {
            m
        } else {
            "".to_string()
        }
    }
}

impl<const N: usize> Drop for RosString<N> {
    fn drop(&mut self) {
        unsafe { rosidl_runtime_c__String__fini(&mut self.0 as *mut _) };
    }
}

impl<const N: usize> Display for RosString<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self.get_string();
        write!(f, "{s}")
    }
}

impl<const N: usize> PartialEq for RosString<N> {
    fn eq(&self, other: &Self) -> bool {
        unsafe { rosidl_runtime_c__String__are_equal(&self.0, &other.0) }
    }
}

unsafe impl<const N: usize> Sync for RosString<N> {}
unsafe impl<const N: usize> Send for RosString<N> {}

/// Sequence of string.
/// `STRLEN` represents the maximum number of characters excluding `\0`.
/// If `STRLEN` is `0`, the string is unlimited.
/// `M` represents the maximum number of elements in a sequence.
/// If `SEQLEN` is `0`, the sequence is unlimited.
#[repr(C)]
#[derive(Debug)]
pub struct RosStringSeq<const STRLEN: usize, const SEQLEN: usize>(
    rosidl_runtime_c__String__Sequence,
);

impl<const STRLEN: usize, const SEQLEN: usize> RosStringSeq<STRLEN, SEQLEN> {
    pub fn new(size: usize) -> Option<Self> {
        if SEQLEN != 0 && size > SEQLEN {
            // the size exceeds in the maximum number
            return None;
        }

        let mut msg: rosidl_runtime_c__String__Sequence =
            unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
        if unsafe { rosidl_runtime_c__String__Sequence__init(&mut msg, size as _) } {
            Some(Self(msg))
        } else {
            None
        }
    }

    pub fn null() -> Self {
        let msg: rosidl_runtime_c__String__Sequence =
            unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
        Self(msg)
    }

    #[allow(clippy::unnecessary_cast)]
    pub fn as_slice(&self) -> &[RosString<STRLEN>] {
        if self.0.data.is_null() {
            &[]
        } else {
            let s = unsafe { std::slice::from_raw_parts(self.0.data, self.0.size as usize) };
            unsafe { transmute::<&[rosidl_runtime_c__String], &[RosString<STRLEN>]>(s) }
        }
    }

    pub fn as_slice_mut(&mut self) -> &mut [RosString<STRLEN>] {
        if self.0.data.is_null() {
            &mut []
        } else {
            let s = unsafe {
                std::slice::from_raw_parts_mut(self.0.data, self.0.size.try_into().unwrap())
            };
            unsafe { transmute::<&mut [rosidl_runtime_c__String], &mut [RosString<STRLEN>]>(s) }
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, RosString<STRLEN>> {
        self.as_slice().iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, RosString<STRLEN>> {
        self.as_slice_mut().iter_mut()
    }

    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<const STRLEN: usize, const SEQLEN: usize> Drop for RosStringSeq<STRLEN, SEQLEN> {
    fn drop(&mut self) {
        unsafe { rosidl_runtime_c__String__Sequence__fini(&mut self.0 as *mut _) };
    }
}

impl<const STRLEN: usize, const SEQLEN: usize> PartialEq for RosStringSeq<STRLEN, SEQLEN> {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            rosidl_runtime_c__String__Sequence__are_equal(&self.0 as *const _, &other.0 as *const _)
        }
    }
}

unsafe impl<const STRLEN: usize, const SEQLEN: usize> Sync for RosStringSeq<STRLEN, SEQLEN> {}
unsafe impl<const STRLEN: usize, const SEQLEN: usize> Send for RosStringSeq<STRLEN, SEQLEN> {}

// Re-export msg module utilities for generated code
pub mod msg {
    pub use crate::{
        BoolSeq, F32Seq, F64Seq, I16Seq, I32Seq, I64Seq, I8Seq, RosString, RosStringSeq, U16Seq,
        U32Seq, U64Seq, U8Seq,
    };
    pub use oxidros_core::TypeSupport;
}

// Re-export rcl types for generated code
pub mod rcl {
    // Re-export C types from runtime_c
    pub use crate::{
        rosidl_action_type_support_t, rosidl_message_type_support_t, rosidl_service_type_support_t,
    };
}

// Re-export builtin_interfaces types
pub mod builtin_interfaces {
    pub use oxidros_core::{UnsafeDuration, UnsafeTime};
}

// Include generated message modules
pub mod common_interfaces {
    //! Common ROS2 interface messages (geometry_msgs, sensor_msgs, etc.)
    include!(concat!(env!("OUT_DIR"), "/common_interfaces/mod.rs"));
}

pub mod interfaces {
    //! ROS2 core interfaces (rcl_interfaces, action_msgs, etc.)
    include!(concat!(env!("OUT_DIR"), "/interfaces/mod.rs"));
}

pub mod ros2msg {
    //! Additional ROS2 messages (unique_identifier_msgs, etc.)
    include!(concat!(env!("OUT_DIR"), "/ros2msg/mod.rs"));
}

// Re-export commonly used items
pub use ros2msg::*;

// Re-export oxidros_core module so generated code can use crate::oxidros_core::TypeSupport
pub use oxidros_core;

// Re-export traits from oxidros-core at the top level for convenience
pub use oxidros_core::{
    ActionGoal, ActionMsg, ActionResult, GetUUID, GoalResponse, ResultResponse, ServiceMsg,
    TypeSupport,
};

// Re-export UnsafeTime and UnsafeDuration from oxidros-core
pub use oxidros_core::{UnsafeDuration, UnsafeTime};
