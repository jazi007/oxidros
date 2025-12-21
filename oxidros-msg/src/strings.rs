//! Definition of Strings
use crate::rcl::*;
use std::{ffi::CString, fmt::Display, mem::transmute};

/// String.
/// `N` represents the maximum number of characters excluding `\0`.
/// If `N` is `0`, the string is unlimited.
#[repr(C)]
#[derive(Debug, Clone)]
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

    pub fn as_slice(&self) -> &[std::os::raw::c_char] {
        if self.0.data.is_null() {
            &[]
        } else {
            let s = unsafe { std::slice::from_raw_parts(self.0.data, self.0.size) };
            s
        }
    }

    pub fn as_slice_mut(&mut self) -> &mut [std::os::raw::c_char] {
        if self.0.data.is_null() {
            &mut []
        } else {
            let s = unsafe { std::slice::from_raw_parts_mut(self.0.data, self.0.size) };
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
#[derive(Debug, Clone)]
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

    pub fn as_slice(&self) -> &[RosString<STRLEN>] {
        if self.0.data.is_null() {
            &[]
        } else {
            let s = unsafe { std::slice::from_raw_parts(self.0.data, self.0.size) };
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

/// WString.
/// `N` represents the maximum number of characters excluding `\0`.
/// If `N` is `0`, the string is unlimited.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct RosWString<const N: usize>(rosidl_runtime_c__U16String);

impl<const N: usize> RosWString<N> {
    pub fn new(s: &str) -> Option<Self> {
        let mut msg: rosidl_runtime_c__U16String =
            unsafe { std::mem::MaybeUninit::zeroed().assume_init() };

        // initialize string
        if unsafe { rosidl_runtime_c__U16String__init(&mut msg) } {
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
        let msg: rosidl_runtime_c__U16String =
            unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
        Self(msg)
    }

    fn assign_string(msg: &mut rosidl_runtime_c__U16String, s: &str) -> bool {
        let cs = widestring::U16CString::from_str(s).unwrap();

        // assign string
        if N == 0 {
            unsafe { rosidl_runtime_c__U16String__assign(msg, cs.as_ptr()) }
        } else {
            unsafe { rosidl_runtime_c__U16String__assignn(msg, cs.as_ptr(), N as _) }
        }
    }

    pub fn assign(&mut self, s: &str) -> bool {
        Self::assign_string(&mut self.0, s)
    }

    pub fn as_slice(&self) -> &[u16] {
        if self.0.data.is_null() {
            &[]
        } else {
            let s = unsafe { std::slice::from_raw_parts(self.0.data, self.0.size) };
            s
        }
    }

    pub fn as_slice_mut(&mut self) -> &mut [u16] {
        if self.0.data.is_null() {
            &mut []
        } else {
            let s = unsafe { std::slice::from_raw_parts_mut(self.0.data, self.0.size) };
            s
        }
    }

    pub fn get_string(&self) -> String {
        if let Ok(m) = String::from_utf16(self.as_slice()) {
            m
        } else {
            "".to_string()
        }
    }
}

impl<const N: usize> Drop for RosWString<N> {
    fn drop(&mut self) {
        unsafe { rosidl_runtime_c__U16String__fini(&mut self.0 as *mut _) };
    }
}

impl<const N: usize> Display for RosWString<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self.get_string();
        write!(f, "{s}")
    }
}

impl<const N: usize> PartialEq for RosWString<N> {
    fn eq(&self, other: &Self) -> bool {
        unsafe { rosidl_runtime_c__U16String__are_equal(&self.0, &other.0) }
    }
}

unsafe impl<const N: usize> Sync for RosWString<N> {}
unsafe impl<const N: usize> Send for RosWString<N> {}

/// Sequence of string.
/// `STRLEN` represents the maximum number of characters excluding `\0`.
/// If `STRLEN` is `0`, the string is unlimited.
/// `M` represents the maximum number of elements in a sequence.
/// If `SEQLEN` is `0`, the sequence is unlimited.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct RosWStringSeq<const STRLEN: usize, const SEQLEN: usize>(
    rosidl_runtime_c__U16String__Sequence,
);

impl<const STRLEN: usize, const SEQLEN: usize> RosWStringSeq<STRLEN, SEQLEN> {
    pub fn new(size: usize) -> Option<Self> {
        if SEQLEN != 0 && size > SEQLEN {
            // the size exceeds in the maximum number
            return None;
        }

        let mut msg: rosidl_runtime_c__U16String__Sequence =
            unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
        if unsafe { rosidl_runtime_c__U16String__Sequence__init(&mut msg, size as _) } {
            Some(Self(msg))
        } else {
            None
        }
    }

    pub fn null() -> Self {
        let msg: rosidl_runtime_c__U16String__Sequence =
            unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
        Self(msg)
    }

    pub fn as_slice(&self) -> &[RosWString<STRLEN>] {
        if self.0.data.is_null() {
            &[]
        } else {
            let s = unsafe { std::slice::from_raw_parts(self.0.data, self.0.size) };
            unsafe { transmute::<&[rosidl_runtime_c__U16String], &[RosWString<STRLEN>]>(s) }
        }
    }

    pub fn as_slice_mut(&mut self) -> &mut [RosWString<STRLEN>] {
        if self.0.data.is_null() {
            &mut []
        } else {
            let s = unsafe {
                std::slice::from_raw_parts_mut(self.0.data, self.0.size.try_into().unwrap())
            };
            unsafe { transmute::<&mut [rosidl_runtime_c__U16String], &mut [RosWString<STRLEN>]>(s) }
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, RosWString<STRLEN>> {
        self.as_slice().iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, RosWString<STRLEN>> {
        self.as_slice_mut().iter_mut()
    }

    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<const STRLEN: usize, const SEQLEN: usize> Drop for RosWStringSeq<STRLEN, SEQLEN> {
    fn drop(&mut self) {
        unsafe { rosidl_runtime_c__U16String__Sequence__fini(&mut self.0 as *mut _) };
    }
}

impl<const STRLEN: usize, const SEQLEN: usize> PartialEq for RosWStringSeq<STRLEN, SEQLEN> {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            rosidl_runtime_c__U16String__Sequence__are_equal(
                &self.0 as *const _,
                &other.0 as *const _,
            )
        }
    }
}

unsafe impl<const STRLEN: usize, const SEQLEN: usize> Sync for RosWStringSeq<STRLEN, SEQLEN> {}
unsafe impl<const STRLEN: usize, const SEQLEN: usize> Send for RosWStringSeq<STRLEN, SEQLEN> {}
