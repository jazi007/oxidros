//! Time-related types for ROS2 messages.

/// Represents a timestamp that may not be safe across all platforms.
///
/// The "Unsafe" prefix indicates this is subject to the year-2038 problem
/// on 32-bit systems since `sec` is an `i32`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnsafeTime {
    /// Seconds since UNIX epoch.
    pub sec: i32,

    /// Nanoseconds component.
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

/// Represents a duration that may not be safe across all platforms.
///
/// The "Unsafe" prefix indicates this is subject to the year-2038 problem
/// on 32-bit systems since `sec` is an `i32`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

// Conversions to/from std types
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
