use crate::rcl;
use std::time::{Duration, SystemTime};

// Duration <-> rmw_time_t conversions are now in oxidros-rcl

// Note: From implementations for SystemTime <-> UnsafeTime and Duration <-> UnsafeDuration
// are now in oxidros-core to avoid orphan rule violations

pub(crate) fn rcl_time_to_system_time(t: rcl::rcutils_time_point_value_t) -> SystemTime {
    let from_epoch = Duration::from_nanos(t as u64);
    SystemTime::UNIX_EPOCH + from_epoch
}
