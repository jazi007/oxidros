//! Definition of ROS2 primitive types
//!

use std::fmt::Display;

// ============================================================================
// rcl feature - FFI implementations
// ============================================================================
#[cfg(feature = "rcl")]
mod rcl_impl {
    use crate::rcl::*;

    macro_rules! def_sequence {
        ($ty: ident, $ty_orig:ty, $ty_seq:ty, $init:ident, $fini:ident, $eq:ident, $copy:ident) => {
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

                    let mut msg: $ty_seq = unsafe { std::mem::zeroed() };
                    if unsafe { $init(&mut msg, size as _) } {
                        Some($ty(msg))
                    } else {
                        None
                    }
                }

                pub fn null() -> Self {
                    let msg: $ty_seq = unsafe { std::mem::zeroed() };
                    $ty(msg)
                }

                pub fn as_slice(&self) -> &[$ty_orig] {
                    if self.0.data.is_null() {
                        &[]
                    } else {
                        let s = unsafe {
                            std::slice::from_raw_parts(self.0.data, self.0.size as usize)
                        };
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

            impl<const N: usize> ::oxidros_core::msg::TryClone for $ty<N> {
                fn try_clone(&self) -> Option<Self> {
                    let mut result = Self::new(self.0.size)?;
                    if unsafe { $copy(&self.0, &mut result.0) } {
                        Some(result)
                    } else {
                        None
                    }
                }
            }

            impl<const N: usize> Clone for $ty<N> {
                fn clone(&self) -> Self {
                    ::oxidros_core::msg::TryClone::try_clone(self).unwrap()
                }
            }

            impl<const N: usize> Default for $ty<N> {
                fn default() -> Self {
                    Self::null()
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
        rosidl_runtime_c__boolean__Sequence__are_equal,
        rosidl_runtime_c__boolean__Sequence__copy
    );

    def_sequence!(
        F32Seq,
        f32,
        rosidl_runtime_c__float__Sequence,
        rosidl_runtime_c__float__Sequence__init,
        rosidl_runtime_c__float__Sequence__fini,
        rosidl_runtime_c__float__Sequence__are_equal,
        rosidl_runtime_c__float__Sequence__copy
    );

    def_sequence!(
        F64Seq,
        f64,
        rosidl_runtime_c__double__Sequence,
        rosidl_runtime_c__double__Sequence__init,
        rosidl_runtime_c__double__Sequence__fini,
        rosidl_runtime_c__double__Sequence__are_equal,
        rosidl_runtime_c__double__Sequence__copy
    );

    def_sequence!(
        U8Seq,
        u8,
        rosidl_runtime_c__uint8__Sequence,
        rosidl_runtime_c__uint8__Sequence__init,
        rosidl_runtime_c__uint8__Sequence__fini,
        rosidl_runtime_c__uint8__Sequence__are_equal,
        rosidl_runtime_c__uint8__Sequence__copy
    );

    def_sequence!(
        I8Seq,
        i8,
        rosidl_runtime_c__int8__Sequence,
        rosidl_runtime_c__int8__Sequence__init,
        rosidl_runtime_c__int8__Sequence__fini,
        rosidl_runtime_c__int8__Sequence__are_equal,
        rosidl_runtime_c__int8__Sequence__copy
    );

    def_sequence!(
        U16Seq,
        u16,
        rosidl_runtime_c__uint16__Sequence,
        rosidl_runtime_c__uint16__Sequence__init,
        rosidl_runtime_c__uint16__Sequence__fini,
        rosidl_runtime_c__uint16__Sequence__are_equal,
        rosidl_runtime_c__uint16__Sequence__copy
    );

    def_sequence!(
        I16Seq,
        i16,
        rosidl_runtime_c__int16__Sequence,
        rosidl_runtime_c__int16__Sequence__init,
        rosidl_runtime_c__int16__Sequence__fini,
        rosidl_runtime_c__int16__Sequence__are_equal,
        rosidl_runtime_c__int16__Sequence__copy
    );

    def_sequence!(
        U32Seq,
        u32,
        rosidl_runtime_c__uint32__Sequence,
        rosidl_runtime_c__uint32__Sequence__init,
        rosidl_runtime_c__uint32__Sequence__fini,
        rosidl_runtime_c__uint32__Sequence__are_equal,
        rosidl_runtime_c__uint32__Sequence__copy
    );

    def_sequence!(
        I32Seq,
        i32,
        rosidl_runtime_c__int32__Sequence,
        rosidl_runtime_c__int32__Sequence__init,
        rosidl_runtime_c__int32__Sequence__fini,
        rosidl_runtime_c__int32__Sequence__are_equal,
        rosidl_runtime_c__int32__Sequence__copy
    );

    def_sequence!(
        U64Seq,
        u64,
        rosidl_runtime_c__uint64__Sequence,
        rosidl_runtime_c__uint64__Sequence__init,
        rosidl_runtime_c__uint64__Sequence__fini,
        rosidl_runtime_c__uint64__Sequence__are_equal,
        rosidl_runtime_c__uint64__Sequence__copy
    );

    def_sequence!(
        I64Seq,
        i64,
        rosidl_runtime_c__int64__Sequence,
        rosidl_runtime_c__int64__Sequence__init,
        rosidl_runtime_c__int64__Sequence__fini,
        rosidl_runtime_c__int64__Sequence__are_equal,
        rosidl_runtime_c__int64__Sequence__copy
    );
}

// ============================================================================
// non-rcl feature - Pure Rust implementations
// ============================================================================
#[cfg(not(feature = "rcl"))]
mod non_rcl_impl {
    macro_rules! def_sequence {
        ($ty: ident, $ty_orig:ty) => {
            /// A sequence of elements.
            /// `N` represents the maximum number of elements.
            /// If `N` is `0`, the sequence is unlimited.
            #[derive(Debug)]
            pub struct $ty<const N: usize>(Vec<$ty_orig>);

            impl<const N: usize> $ty<N> {
                pub fn new(size: usize) -> Option<Self> {
                    if N != 0 && size > N {
                        // the size exceeds the maximum number
                        return None;
                    }
                    Some($ty(vec![Default::default(); size]))
                }

                pub fn null() -> Self {
                    $ty(Vec::new())
                }

                pub fn as_slice(&self) -> &[$ty_orig] {
                    &self.0
                }

                pub fn as_slice_mut(&mut self) -> &mut [$ty_orig] {
                    &mut self.0
                }

                pub fn iter(&self) -> std::slice::Iter<'_, $ty_orig> {
                    self.0.iter()
                }

                pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, $ty_orig> {
                    self.0.iter_mut()
                }

                pub fn len(&self) -> usize {
                    self.0.len()
                }

                pub fn is_empty(&self) -> bool {
                    self.0.is_empty()
                }
            }

            impl<const N: usize> PartialEq for $ty<N> {
                fn eq(&self, other: &Self) -> bool {
                    self.0 == other.0
                }
            }

            impl<const N: usize> ::oxidros_core::msg::TryClone for $ty<N> {
                fn try_clone(&self) -> Option<Self> {
                    Some($ty(self.0.clone()))
                }
            }

            impl<const N: usize> Clone for $ty<N> {
                fn clone(&self) -> Self {
                    $ty(self.0.clone())
                }
            }

            impl<const N: usize> Default for $ty<N> {
                fn default() -> Self {
                    Self::null()
                }
            }

            unsafe impl<const N: usize> Sync for $ty<N> {}
            unsafe impl<const N: usize> Send for $ty<N> {}
        };
    }

    def_sequence!(BoolSeq, bool);
    def_sequence!(F32Seq, f32);
    def_sequence!(F64Seq, f64);
    def_sequence!(U8Seq, u8);
    def_sequence!(I8Seq, i8);
    def_sequence!(U16Seq, u16);
    def_sequence!(I16Seq, i16);
    def_sequence!(U32Seq, u32);
    def_sequence!(I32Seq, i32);
    def_sequence!(U64Seq, u64);
    def_sequence!(I64Seq, i64);
}

// ============================================================================
// Re-exports
// ============================================================================
#[cfg(feature = "rcl")]
pub use rcl_impl::*;

#[cfg(not(feature = "rcl"))]
pub use non_rcl_impl::*;

// ============================================================================
// Common utilities
// ============================================================================

/// The error type returned when a conversion from a slice to an array fails.
#[derive(Debug, Copy, Clone)]
pub struct TryFromSliceError(());

impl std::fmt::Display for TryFromSliceError {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        "could not convert slice to array".fmt(f)
    }
}

impl std::error::Error for TryFromSliceError {}

macro_rules! def_try_from {
    ($from:ty, $to:ty) => {
        impl TryFrom<&[$from]> for $to {
            type Error = TryFromSliceError;
            fn try_from(value: &[$from]) -> Result<Self, Self::Error> {
                let mut seq = Self::new(value.len()).ok_or(TryFromSliceError(()))?;
                seq.iter_mut().zip(value).for_each(|(dst, &src)| *dst = src);
                Ok(seq)
            }
        }
    };
}

def_try_from!(bool, BoolSeq<0>);
def_try_from!(i8, I8Seq<0>);
def_try_from!(i16, I16Seq<0>);
def_try_from!(i32, I32Seq<0>);
def_try_from!(i64, I64Seq<0>);
def_try_from!(u8, U8Seq<0>);
def_try_from!(u16, U16Seq<0>);
def_try_from!(u32, U32Seq<0>);
def_try_from!(u64, U64Seq<0>);
def_try_from!(f32, F32Seq<0>);
def_try_from!(f64, F64Seq<0>);

#[cfg(test)]
mod tests {
    use oxidros_core::msg::TryClone;

    use super::*;

    #[test]
    fn test_clone() {
        let v1: BoolSeq<0> = [true; 10].as_slice().try_into().unwrap();
        let v2 = v1.try_clone().unwrap();
        assert_eq!(v1, v2);
        let v1: U32Seq<0> = [2; 10].as_slice().try_into().unwrap();
        let v2 = v1.try_clone().unwrap();
        assert_eq!(v1, v2);
    }
}
