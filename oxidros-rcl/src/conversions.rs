//! Type conversions for RCL types

use std::error::Error;
use std::fmt;
use std::time::Duration;

// Duration <-> rmw_time_t conversions
impl From<crate::rmw_time_t> for Duration {
    fn from(t: crate::rmw_time_t) -> Self {
        Duration::new(t.sec, t.nsec as u32)
    }
}

impl From<Duration> for crate::rmw_time_t {
    fn from(t: Duration) -> Self {
        crate::rmw_time_t {
            sec: t.as_secs(),
            nsec: t.subsec_nanos() as _,
        }
    }
}

// rcutils_error_string_t Display and Error implementations
impl fmt::Display for crate::rcutils_error_string_t {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.str_;
        let inner: &[u8] = unsafe { std::slice::from_raw_parts(s.as_ptr() as *const u8, s.len()) };
        let s = String::from_utf8(inner.to_vec()).unwrap();
        write!(f, "{}", s)
    }
}

impl Error for crate::rcutils_error_string_t {}

impl From<crate::rmw_qos_history_policy_t> for oxidros_core::qos::HistoryPolicy {
    fn from(value: crate::rmw_qos_history_policy_t) -> Self {
        use crate::rmw_qos_history_policy_t::*;
        match value {
            RMW_QOS_POLICY_HISTORY_SYSTEM_DEFAULT => Self::SystemDefault,
            RMW_QOS_POLICY_HISTORY_KEEP_LAST => Self::KeepLast,
            RMW_QOS_POLICY_HISTORY_KEEP_ALL => Self::KeepAll,
            RMW_QOS_POLICY_HISTORY_UNKNOWN => Self::Unknown,
        }
    }
}

impl From<crate::rmw_qos_reliability_policy_t> for oxidros_core::ReliabilityPolicy {
    fn from(value: crate::rmw_qos_reliability_policy_t) -> Self {
        use crate::rmw_qos_reliability_policy_t::*;
        match value {
            RMW_QOS_POLICY_RELIABILITY_SYSTEM_DEFAULT => Self::SystemDefault,
            RMW_QOS_POLICY_RELIABILITY_RELIABLE => Self::Reliable,
            RMW_QOS_POLICY_RELIABILITY_BEST_EFFORT => Self::BestEffort,
            RMW_QOS_POLICY_RELIABILITY_UNKNOWN => Self::Unknown,
            RMW_QOS_POLICY_RELIABILITY_BEST_AVAILABLE => Self::BestAvailable,
        }
    }
}

impl From<crate::rmw_qos_durability_policy_t> for oxidros_core::DurabilityPolicy {
    fn from(value: crate::rmw_qos_durability_policy_t) -> Self {
        use crate::rmw_qos_durability_policy_t::*;
        match value {
            RMW_QOS_POLICY_DURABILITY_SYSTEM_DEFAULT => Self::SystemDefault,
            RMW_QOS_POLICY_DURABILITY_TRANSIENT_LOCAL => Self::TransientLocal,
            RMW_QOS_POLICY_DURABILITY_VOLATILE => Self::Volatile,
            RMW_QOS_POLICY_DURABILITY_UNKNOWN => Self::Unknown,
            RMW_QOS_POLICY_DURABILITY_BEST_AVAILABLE => Self::Unknown,
        }
    }
}

impl From<crate::rmw_qos_liveliness_policy_t> for oxidros_core::LivelinessPolicy {
    fn from(value: crate::rmw_qos_liveliness_policy_t) -> Self {
        use crate::rmw_qos_liveliness_policy_t::*;
        match value {
            RMW_QOS_POLICY_LIVELINESS_SYSTEM_DEFAULT => Self::SystemDefault,
            RMW_QOS_POLICY_LIVELINESS_AUTOMATIC => Self::Automatic,
            RMW_QOS_POLICY_LIVELINESS_MANUAL_BY_NODE
            | RMW_QOS_POLICY_LIVELINESS_MANUAL_BY_TOPIC => Self::ManualByTopic,
            RMW_QOS_POLICY_LIVELINESS_UNKNOWN => Self::Unknown,
            RMW_QOS_POLICY_LIVELINESS_BEST_AVAILABLE => Self::Unknown,
        }
    }
}

// Reverse conversions: oxidros_core policy types to RCL types

impl From<oxidros_core::qos::HistoryPolicy> for crate::rmw_qos_history_policy_t {
    fn from(value: oxidros_core::qos::HistoryPolicy) -> Self {
        use oxidros_core::qos::HistoryPolicy::*;
        match value {
            SystemDefault => Self::RMW_QOS_POLICY_HISTORY_SYSTEM_DEFAULT,
            KeepLast => Self::RMW_QOS_POLICY_HISTORY_KEEP_LAST,
            KeepAll => Self::RMW_QOS_POLICY_HISTORY_KEEP_ALL,
            Unknown => Self::RMW_QOS_POLICY_HISTORY_UNKNOWN,
        }
    }
}

impl From<oxidros_core::ReliabilityPolicy> for crate::rmw_qos_reliability_policy_t {
    fn from(value: oxidros_core::ReliabilityPolicy) -> Self {
        use oxidros_core::ReliabilityPolicy::*;
        match value {
            SystemDefault => Self::RMW_QOS_POLICY_RELIABILITY_SYSTEM_DEFAULT,
            Reliable => Self::RMW_QOS_POLICY_RELIABILITY_RELIABLE,
            BestEffort => Self::RMW_QOS_POLICY_RELIABILITY_BEST_EFFORT,
            BestAvailable => Self::RMW_QOS_POLICY_RELIABILITY_BEST_AVAILABLE,
            Unknown => Self::RMW_QOS_POLICY_RELIABILITY_UNKNOWN,
        }
    }
}

impl From<oxidros_core::DurabilityPolicy> for crate::rmw_qos_durability_policy_t {
    fn from(value: oxidros_core::DurabilityPolicy) -> Self {
        use oxidros_core::DurabilityPolicy::*;
        match value {
            SystemDefault => Self::RMW_QOS_POLICY_DURABILITY_SYSTEM_DEFAULT,
            TransientLocal => Self::RMW_QOS_POLICY_DURABILITY_TRANSIENT_LOCAL,
            Volatile => Self::RMW_QOS_POLICY_DURABILITY_VOLATILE,
            BestAvailable => Self::RMW_QOS_POLICY_DURABILITY_BEST_AVAILABLE,
            Unknown => Self::RMW_QOS_POLICY_DURABILITY_UNKNOWN,
        }
    }
}

impl From<oxidros_core::LivelinessPolicy> for crate::rmw_qos_liveliness_policy_t {
    fn from(value: oxidros_core::LivelinessPolicy) -> Self {
        use oxidros_core::LivelinessPolicy::*;
        match value {
            SystemDefault => Self::RMW_QOS_POLICY_LIVELINESS_SYSTEM_DEFAULT,
            Automatic => Self::RMW_QOS_POLICY_LIVELINESS_AUTOMATIC,
            ManualByTopic => Self::RMW_QOS_POLICY_LIVELINESS_MANUAL_BY_TOPIC,
            BestAvailable => Self::RMW_QOS_POLICY_LIVELINESS_BEST_AVAILABLE,
            Unknown => Self::RMW_QOS_POLICY_LIVELINESS_UNKNOWN,
        }
    }
}

impl From<&crate::rmw_qos_profile_t> for oxidros_core::qos::Profile {
    fn from(qos: &crate::rmw_qos_profile_t) -> Self {
        Self {
            history: qos.history.into(),
            depth: qos.depth.try_into().unwrap(),
            reliability: qos.reliability.into(),
            durability: qos.durability.into(),
            liveliness: qos.liveliness.into(),
            deadline: qos.deadline.into(),
            lifespan: qos.lifespan.into(),
            liveliness_lease_duration: qos.liveliness_lease_duration.into(),
            avoid_ros_namespace_conventions: qos.avoid_ros_namespace_conventions,
        }
    }
}

impl From<&oxidros_core::Profile> for crate::rmw_qos_profile_t {
    fn from(qos: &oxidros_core::Profile) -> Self {
        crate::rmw_qos_profile_t {
            history: qos.history.into(),
            depth: qos.depth as _,
            reliability: qos.reliability.into(),
            durability: qos.durability.into(),
            liveliness: qos.liveliness.into(),
            deadline: qos.deadline.into(),
            lifespan: qos.lifespan.into(),
            liveliness_lease_duration: qos.liveliness_lease_duration.into(),
            avoid_ros_namespace_conventions: qos.avoid_ros_namespace_conventions,
        }
    }
}
