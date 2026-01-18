//! Core traits for ROS2 message types
//!
//! This module provides traits for ROS2 messages, services, and actions
//! that are used by the derive macros.

use crate::Result;
use std::ffi::c_void;

/// Trait for types that have type support information.
///
/// This allows the runtime to understand the structure of messages
/// for serialization and deserialization.
///
/// # Serialization
///
/// The `to_bytes` and `from_bytes` methods provide CDR serialization:
/// - For RCL (DDS-based): Uses `rmw_serialize`/`rmw_deserialize` internally
/// - For native Zenoh: Uses serde with `cdr-encoding` crate
pub trait TypeSupport: 'static + Send + Sync {
    /// Returns an opaque pointer to the type support structure.
    ///
    /// The actual type of this pointer depends on the implementation
    /// (e.g., `rosidl_message_type_support_t` in RCL).
    fn type_support() -> *const c_void {
        std::ptr::null()
    }

    /// Serialize this message to CDR-encoded bytes.
    ///
    /// # Implementation
    /// - `rcl` feature: Uses RMW serialization functions
    /// - `zenoh` feature: Uses serde + cdr-encoding crate
    ///
    /// # Errors
    /// Returns `Error::CdrError` if serialization fails.
    fn to_bytes(&self) -> Result<Vec<u8>>;

    /// Deserialize a message from CDR-encoded bytes.
    ///
    /// # Implementation
    /// - `rcl` feature: Uses RMW deserialization functions
    /// - `zenoh` feature: Uses serde + cdr-encoding crate
    ///
    /// # Errors
    /// Returns `Error::CdrError` if deserialization fails.
    fn from_bytes(bytes: &[u8]) -> Result<Self>
    where
        Self: Sized;

    /// Returns the type name in DDS format.
    ///
    /// Example: `"std_msgs::msg::dds_::String_"`
    ///
    /// This is used for Zenoh key expressions and type matching.
    fn type_name() -> &'static str;

    /// Returns the RIHS01 type hash for this message type.
    ///
    /// # Implementation
    /// - For RCL: Returns empty string (hash is handled by rosidl typesupport)
    /// - For Zenoh: Computes hash from TypeDescription
    ///
    /// The hash format is: `RIHS01_<64_character_hex_sha256>`
    fn type_hash() -> Result<::std::string::String> {
        Ok("RIHS01_00".to_string())
    }
}

/// Trait for type that can fail cloning
///
/// Used for FFI types where cloning may fail due to memory allocation.
pub trait TryClone: Sized {
    /// Returns Some(Self) if clone succeeds else None
    fn try_clone(&self) -> Option<Self>;
}

/// Trait for ROS2 service message types.
///
/// Services consist of a request and response message pair.
pub trait ServiceMsg: 'static + Send + Sync {
    /// The request message type.
    type Request: TypeSupport;

    /// The response message type.
    type Response: TypeSupport;

    /// Returns an opaque pointer to the service type support structure.
    fn type_support() -> *const c_void {
        std::ptr::null()
    }

    /// Returns the type name in DDS format.
    ///
    /// Example: `"example_interfaces::srv::dds_::AddTwoInts_"`
    ///
    /// This is used for Zenoh key expressions and type matching.
    fn type_name() -> &'static str;

    /// Returns the RIHS01 type hash for this message type.
    ///
    /// # Implementation
    /// - For RCL: Returns empty string (hash is handled by rosidl typesupport)
    /// - For Zenoh: Computes hash from TypeDescription
    ///
    /// The hash format is: `RIHS01_<64_character_hex_sha256>`
    fn type_hash() -> Result<::std::string::String> {
        Ok("RIHS01_00".to_string())
    }
}

/// Trait for ROS2 action message types.
///
/// Actions are more complex than services and include goals, results,
/// and feedback messages.
pub trait ActionMsg: 'static + Send + Sync {
    /// The goal service type.
    type Goal: ActionGoal;

    /// The result service type.
    type Result: ActionResult;

    /// The feedback message type.
    type Feedback: TypeSupport + GetUUID;

    /// Returns an opaque pointer to the action type support structure.
    fn type_support() -> *const c_void {
        std::ptr::null()
    }

    /// Returns the type name in DDS format.
    ///
    /// Example: `"example_interfaces::srv::dds_::AddTwoInts_"`
    ///
    /// This is used for Zenoh key expressions and type matching.
    fn type_name() -> &'static str;

    /// The goal content type (the actual goal data).
    type GoalContent: TypeSupport;

    /// Create a new goal request with the given goal and UUID.
    fn new_goal_request(
        goal: Self::GoalContent,
        uuid: [u8; 16],
    ) -> <Self::Goal as ActionGoal>::Request;

    /// The result content type (the actual result data).
    type ResultContent: TypeSupport + TryClone;

    /// Create a new result response with the given status and result.
    fn new_result_response(
        status: u8,
        result: Self::ResultContent,
    ) -> <Self::Result as ActionResult>::Response;

    /// The feedback content type (the actual feedback data).
    type FeedbackContent: TypeSupport;

    /// Create a new feedback message with the given feedback and UUID.
    fn new_feedback_message(feedback: Self::FeedbackContent, uuid: [u8; 16]) -> Self::Feedback;

    /// Returns the RIHS01 type hash for this message type.
    ///
    /// # Implementation
    /// - For RCL: Returns empty string (hash is handled by rosidl typesupport)
    /// - For Zenoh: Computes hash from TypeDescription
    ///
    /// The hash format is: `RIHS01_<64_character_hex_sha256>`
    fn type_hash() -> Result<::std::string::String> {
        Ok("RIHS01_00".to_string())
    }
}

/// Trait for action goal types.
pub trait ActionGoal: 'static + Send + Sync {
    /// The request message type for sending a goal.
    type Request: TypeSupport + GetUUID;

    /// The response message type for goal acceptance/rejection.
    type Response: TypeSupport + GoalResponse;

    /// Returns an opaque pointer to the goal service type support structure.
    fn type_support() -> *const c_void {
        std::ptr::null()
    }
}

/// Trait for types that contain a UUID.
///
/// Used for tracking goals and feedback in actions.
pub trait GetUUID: 'static + Send + Sync {
    /// Returns a reference to the UUID.
    fn get_uuid(&self) -> &[u8; 16];
}

/// Trait for action goal response types.
pub trait GoalResponse: 'static + Send + Sync {
    /// Returns whether the goal was accepted.
    fn is_accepted(&self) -> bool;

    /// Returns the timestamp of the response.
    fn get_time_stamp(&self) -> UnsafeTime;

    /// Creates a new goal response with the given acceptance status and timestamp.
    fn new(accepted: bool, stamp: UnsafeTime) -> Self;
}

/// Trait for action result types.
pub trait ActionResult: 'static + Send + Sync {
    /// The request message type for getting a result.
    type Request: TypeSupport + GetUUID;

    /// The response message type containing the result.
    type Response: TypeSupport + ResultResponse;

    /// Returns an opaque pointer to the result service type support structure.
    fn type_support() -> *const c_void {
        std::ptr::null()
    }
}

/// Trait for action result response types.
pub trait ResultResponse: 'static + Send + Sync {
    /// Returns the status code of the result.
    fn get_status(&self) -> u8;
}

/// Represents a timestamp that may not be safe across all platforms.
///
/// The "Unsafe" prefix indicates this is subject to the year-2038 problem
/// on 32-bit systems since `sec` is an `i32`.
///
/// This is compatible with ROS2's builtin_interfaces/Time.
#[repr(C)]
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct UnsafeTime {
    /// Seconds since UNIX epoch.
    pub sec: i32,
    /// Nanoseconds component (0-999999999).
    pub nanosec: u32,
}

impl UnsafeTime {
    /// Creates a new UnsafeTime instance.
    pub const fn new(sec: i32, nanosec: u32) -> Self {
        Self { sec, nanosec }
    }

    /// Creates an UnsafeTime representing the UNIX epoch (0 seconds).
    pub const fn zero() -> Self {
        Self { sec: 0, nanosec: 0 }
    }
}

// Conversions to/from std types for UnsafeTime
use std::time::{Duration, SystemTime};

impl From<&SystemTime> for UnsafeTime {
    fn from(t: &SystemTime) -> Self {
        let dur = t.duration_since(SystemTime::UNIX_EPOCH).unwrap();

        let sec = dur.as_secs();
        if sec > i32::MAX as u64 {
            panic!("SystemTime too far in future (year-2038 problem)");
        }

        UnsafeTime {
            sec: sec as i32,
            nanosec: dur.subsec_nanos(),
        }
    }
}

impl From<SystemTime> for UnsafeTime {
    fn from(t: SystemTime) -> Self {
        (&t).into()
    }
}

impl From<&UnsafeTime> for SystemTime {
    fn from(t: &UnsafeTime) -> Self {
        let nanos = Duration::from_nanos(t.nanosec as u64);
        let secs = Duration::from_secs(t.sec as u64);
        let dur = nanos + secs;
        SystemTime::UNIX_EPOCH + dur
    }
}

impl From<UnsafeTime> for SystemTime {
    fn from(t: UnsafeTime) -> Self {
        (&t).into()
    }
}

/// Represents a duration that may not be safe across all platforms.
///
/// The "Unsafe" prefix indicates this is subject to the year-2038 problem
/// on 32-bit systems since `sec` is an `i32`.
#[repr(C)]
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct UnsafeDuration {
    /// Seconds component.
    pub sec: i32,
    /// Nanoseconds component.
    pub nanosec: u32,
}

impl UnsafeDuration {
    /// Creates a new UnsafeDuration instance.
    pub const fn new(sec: i32, nanosec: u32) -> Self {
        Self { sec, nanosec }
    }

    /// Creates a zero duration.
    pub const fn zero() -> Self {
        Self { sec: 0, nanosec: 0 }
    }
}

impl From<&Duration> for UnsafeDuration {
    fn from(t: &Duration) -> Self {
        let sec = t.as_secs();

        if sec > i32::MAX as u64 {
            panic!("Duration too long (year-2038 problem)");
        }

        let nanosec = t.subsec_nanos();

        UnsafeDuration {
            sec: sec as i32,
            nanosec,
        }
    }
}

impl From<Duration> for UnsafeDuration {
    fn from(t: Duration) -> Self {
        (&t).into()
    }
}

impl From<&UnsafeDuration> for Duration {
    fn from(t: &UnsafeDuration) -> Self {
        Duration::from_secs(t.sec as u64) + Duration::from_nanos(t.nanosec as u64)
    }
}

impl From<UnsafeDuration> for Duration {
    fn from(t: UnsafeDuration) -> Self {
        (&t).into()
    }
}

/// Raw sequence type for FFI compatibility
///
/// This represents a dynamically-sized sequence of elements as used in ROS2 C API.
///
/// # Memory Management
///
/// - **With `rcl` feature**: Memory is managed by ROS2 C libraries through FFI.
///   Use `TryClone` for proper copying via FFI functions.
/// - **Without `rcl` feature**: Memory can be managed by Rust using `from_vec()`.
///   The sequence takes ownership and will free memory on drop.
#[repr(C)]
#[derive(Debug)]
pub struct SequenceRaw<T> {
    /// Pointer to the data array
    pub data: *mut T,
    /// Current number of elements
    pub size: usize,
    /// Allocated capacity
    pub capacity: usize,
}

impl<T> SequenceRaw<T> {
    /// Create a null/empty sequence
    pub const fn null() -> Self {
        Self {
            data: std::ptr::null_mut(),
            size: 0,
            capacity: 0,
        }
    }

    /// Check if the sequence is empty
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Get the length of the sequence
    pub fn len(&self) -> usize {
        self.size
    }

    /// Get a slice of the sequence data
    ///
    pub fn as_slice(&self) -> &[T] {
        if self.data.is_null() || self.size == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.data, self.size) }
        }
    }
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.as_slice().iter()
    }

    /// Get a mutable slice of the sequence data
    ///
    #[deprecated(note = "use as_mut_slice instead")]
    pub fn as_slice_mut(&mut self) -> &mut [T] {
        if self.data.is_null() || self.size == 0 {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(self.data, self.size) }
        }
    }

    /// Get a mutable slice of the sequence data
    ///
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        if self.data.is_null() || self.size == 0 {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(self.data, self.size) }
        }
    }
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.as_mut_slice().iter_mut()
    }
}

// Non-rcl implementations: Rust-managed memory using Vec
#[cfg(not(feature = "rcl"))]
impl<T> SequenceRaw<T> {
    /// Create a sequence from a Vec (takes ownership)
    ///
    /// The Vec's memory is transferred to the sequence. The sequence will
    /// free the memory when dropped.
    pub fn from_vec(mut vec: Vec<T>) -> Self {
        let data = vec.as_mut_ptr();
        let size = vec.len();
        let capacity = vec.capacity();
        std::mem::forget(vec); // Don't drop the Vec, we own the memory now
        Self {
            data,
            size,
            capacity,
        }
    }

    /// Convert the sequence back to a Vec (takes ownership)
    ///
    /// # Safety
    /// Only call this on sequences created with `from_vec()` or that you know
    /// were allocated by Rust.
    pub unsafe fn into_vec(self) -> Vec<T> {
        if self.data.is_null() {
            Vec::new()
        } else {
            let vec = unsafe { Vec::from_raw_parts(self.data, self.size, self.capacity) };
            std::mem::forget(self); // Don't run our Drop
            vec
        }
    }
}

// Clone implementation for non-rcl: actually clone the data
#[cfg(not(feature = "rcl"))]
impl<T: Clone> Clone for SequenceRaw<T> {
    fn clone(&self) -> Self {
        if self.data.is_null() || self.size == 0 {
            Self::null()
        } else {
            // Clone the data into a new Vec
            let slice = unsafe { std::slice::from_raw_parts(self.data, self.size) };
            let vec: Vec<T> = slice.to_vec();
            Self::from_vec(vec)
        }
    }
}

// Clone implementation for rcl: shallow copy (use TryClone for proper cloning)
#[cfg(feature = "rcl")]
impl<T> Clone for SequenceRaw<T> {
    fn clone(&self) -> Self {
        // For rcl, we do a shallow copy. The actual data cloning should be done
        // through TryClone which uses FFI copy functions.
        Self {
            data: self.data,
            size: self.size,
            capacity: self.capacity,
        }
    }
}

// PartialEq implementation: compare actual elements
impl<T: PartialEq> PartialEq for SequenceRaw<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.size != other.size {
            return false;
        }
        if self.data.is_null() && other.data.is_null() {
            return true;
        }
        if self.data.is_null() || other.data.is_null() {
            return false;
        }
        // Compare actual elements
        let self_slice = unsafe { std::slice::from_raw_parts(self.data, self.size) };
        let other_slice = unsafe { std::slice::from_raw_parts(other.data, other.size) };
        self_slice == other_slice
    }
}

// Default implementation for SequenceRaw
impl<T> Default for SequenceRaw<T> {
    fn default() -> Self {
        Self::null()
    }
}

// Drop implementation for non-rcl: free Rust-managed memory
#[cfg(not(feature = "rcl"))]
impl<T> Drop for SequenceRaw<T> {
    fn drop(&mut self) {
        if !self.data.is_null() && self.capacity > 0 {
            // Reconstruct the Vec and let it drop, freeing the memory
            unsafe {
                let _ = Vec::from_raw_parts(self.data, self.size, self.capacity);
            }
        }
    }
}

// Serde implementations for non-rcl: serialize as a sequence
#[cfg(not(feature = "rcl"))]
impl<T: serde::Serialize> serde::Serialize for SequenceRaw<T> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;
        let slice = self.as_slice();
        let mut seq = serializer.serialize_seq(Some(slice.len()))?;
        for element in slice {
            seq.serialize_element(element)?;
        }
        seq.end()
    }
}

#[cfg(not(feature = "rcl"))]
impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for SequenceRaw<T> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let vec = Vec::<T>::deserialize(deserializer)?;
        Ok(Self::from_vec(vec))
    }
}

unsafe impl<T: Send> Send for SequenceRaw<T> {}
unsafe impl<T: Sync> Sync for SequenceRaw<T> {}

pub trait CdrSerde: Sized {
    fn serialize(&self) -> crate::error::Result<Vec<u8>>;
    fn deserialize(bytes: &[u8]) -> crate::error::Result<Self>;
}

impl<T: serde::Serialize + serde::de::DeserializeOwned> CdrSerde for T {
    fn serialize(&self) -> crate::error::Result<Vec<u8>> {
        // Prepend CDR encapsulation header (0x00 0x01 0x00 0x00 = CDR LE)
        let mut result = vec![0x00, 0x01, 0x00, 0x00];
        let mut buffer = cdr_encoding::to_vec::<T, byteorder::LittleEndian>(self)
            .map_err(|e| crate::Error::CdrError(e.to_string()))?;
        result.append(&mut buffer);
        Ok(result)
    }
    fn deserialize(bytes: &[u8]) -> crate::error::Result<Self> {
        if bytes.len() < 4 {
            return Err(crate::Error::CdrError(format!(
                "Bad encoding {} is less than 4",
                bytes.len()
            )));
        }
        let v = cdr_encoding::from_bytes::<T, byteorder::LittleEndian>(&bytes[4..])
            .map_err(|e| crate::Error::CdrError(e.to_string()))?;
        Ok(v.0)
    }
}
