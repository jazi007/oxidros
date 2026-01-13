//! Type conversions for RCL types

use std::error::Error;
use std::ffi::CStr;
use std::fmt;
use std::slice::from_raw_parts;
use std::time::Duration;

use crate::error::ActionError;
use crate::rcl::{rmw_message_info_t, rmw_service_info_t};

use super::RclRetErr;

// Duration <-> rmw_time_t conversions
impl From<super::rmw_time_t> for Duration {
    fn from(t: super::rmw_time_t) -> Self {
        Duration::new(t.sec, t.nsec as u32)
    }
}

impl From<Duration> for super::rmw_time_t {
    fn from(t: Duration) -> Self {
        super::rmw_time_t {
            sec: t.as_secs(),
            nsec: t.subsec_nanos() as _,
        }
    }
}

// rcutils_error_string_t Display and Error implementations
impl fmt::Display for super::rcutils_error_string_t {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.str_;
        let inner: &[u8] = unsafe { std::slice::from_raw_parts(s.as_ptr() as *const u8, s.len()) };
        let s = String::from_utf8(inner.to_vec()).unwrap();
        write!(f, "{}", s)
    }
}

impl Error for super::rcutils_error_string_t {}

impl From<super::rmw_qos_history_policy_t> for oxidros_core::qos::HistoryPolicy {
    fn from(value: super::rmw_qos_history_policy_t) -> Self {
        use super::rmw_qos_history_policy_t::*;
        match value {
            RMW_QOS_POLICY_HISTORY_SYSTEM_DEFAULT => Self::SystemDefault,
            RMW_QOS_POLICY_HISTORY_KEEP_LAST => Self::KeepLast,
            RMW_QOS_POLICY_HISTORY_KEEP_ALL => Self::KeepAll,
            RMW_QOS_POLICY_HISTORY_UNKNOWN => Self::Unknown,
        }
    }
}

impl From<super::rmw_qos_reliability_policy_t> for oxidros_core::ReliabilityPolicy {
    fn from(value: super::rmw_qos_reliability_policy_t) -> Self {
        use super::rmw_qos_reliability_policy_t::*;
        match value {
            RMW_QOS_POLICY_RELIABILITY_SYSTEM_DEFAULT => Self::SystemDefault,
            RMW_QOS_POLICY_RELIABILITY_RELIABLE => Self::Reliable,
            RMW_QOS_POLICY_RELIABILITY_BEST_EFFORT => Self::BestEffort,
            RMW_QOS_POLICY_RELIABILITY_UNKNOWN => Self::Unknown,
            #[cfg(any(feature = "jazzy", feature = "kilted"))]
            RMW_QOS_POLICY_RELIABILITY_BEST_AVAILABLE => Self::BestAvailable,
        }
    }
}

impl From<super::rmw_qos_durability_policy_t> for oxidros_core::DurabilityPolicy {
    fn from(value: super::rmw_qos_durability_policy_t) -> Self {
        use super::rmw_qos_durability_policy_t::*;
        match value {
            RMW_QOS_POLICY_DURABILITY_SYSTEM_DEFAULT => Self::SystemDefault,
            RMW_QOS_POLICY_DURABILITY_TRANSIENT_LOCAL => Self::TransientLocal,
            RMW_QOS_POLICY_DURABILITY_VOLATILE => Self::Volatile,
            RMW_QOS_POLICY_DURABILITY_UNKNOWN => Self::Unknown,
            #[cfg(any(feature = "jazzy", feature = "kilted"))]
            RMW_QOS_POLICY_DURABILITY_BEST_AVAILABLE => Self::Unknown,
        }
    }
}

impl From<super::rmw_qos_liveliness_policy_t> for oxidros_core::LivelinessPolicy {
    fn from(value: super::rmw_qos_liveliness_policy_t) -> Self {
        use super::rmw_qos_liveliness_policy_t::*;
        match value {
            RMW_QOS_POLICY_LIVELINESS_SYSTEM_DEFAULT => Self::SystemDefault,
            RMW_QOS_POLICY_LIVELINESS_AUTOMATIC => Self::Automatic,
            RMW_QOS_POLICY_LIVELINESS_MANUAL_BY_NODE
            | RMW_QOS_POLICY_LIVELINESS_MANUAL_BY_TOPIC => Self::ManualByTopic,
            RMW_QOS_POLICY_LIVELINESS_UNKNOWN => Self::Unknown,
            #[cfg(any(feature = "jazzy", feature = "kilted"))]
            RMW_QOS_POLICY_LIVELINESS_BEST_AVAILABLE => Self::Unknown,
        }
    }
}

// Reverse conversions: oxidros_core policy types to RCL types

impl From<oxidros_core::qos::HistoryPolicy> for super::rmw_qos_history_policy_t {
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

impl From<oxidros_core::ReliabilityPolicy> for super::rmw_qos_reliability_policy_t {
    fn from(value: oxidros_core::ReliabilityPolicy) -> Self {
        use oxidros_core::ReliabilityPolicy::*;
        match value {
            SystemDefault => Self::RMW_QOS_POLICY_RELIABILITY_SYSTEM_DEFAULT,
            Reliable => Self::RMW_QOS_POLICY_RELIABILITY_RELIABLE,
            BestEffort => Self::RMW_QOS_POLICY_RELIABILITY_BEST_EFFORT,
            #[cfg(any(feature = "jazzy", feature = "kilted"))]
            BestAvailable => Self::RMW_QOS_POLICY_RELIABILITY_BEST_AVAILABLE,
            #[cfg(not(any(feature = "jazzy", feature = "kilted")))]
            BestAvailable => Self::RMW_QOS_POLICY_RELIABILITY_UNKNOWN,
            Unknown => Self::RMW_QOS_POLICY_RELIABILITY_UNKNOWN,
        }
    }
}

impl From<oxidros_core::DurabilityPolicy> for super::rmw_qos_durability_policy_t {
    fn from(value: oxidros_core::DurabilityPolicy) -> Self {
        use oxidros_core::DurabilityPolicy::*;
        match value {
            SystemDefault => Self::RMW_QOS_POLICY_DURABILITY_SYSTEM_DEFAULT,
            TransientLocal => Self::RMW_QOS_POLICY_DURABILITY_TRANSIENT_LOCAL,
            Volatile => Self::RMW_QOS_POLICY_DURABILITY_VOLATILE,
            #[cfg(any(feature = "jazzy", feature = "kilted"))]
            BestAvailable => Self::RMW_QOS_POLICY_DURABILITY_BEST_AVAILABLE,
            #[cfg(not(any(feature = "jazzy", feature = "kilted")))]
            BestAvailable => Self::RMW_QOS_POLICY_DURABILITY_UNKNOWN,
            Unknown => Self::RMW_QOS_POLICY_DURABILITY_UNKNOWN,
        }
    }
}

impl From<oxidros_core::LivelinessPolicy> for super::rmw_qos_liveliness_policy_t {
    fn from(value: oxidros_core::LivelinessPolicy) -> Self {
        use oxidros_core::LivelinessPolicy::*;
        match value {
            SystemDefault => Self::RMW_QOS_POLICY_LIVELINESS_SYSTEM_DEFAULT,
            Automatic => Self::RMW_QOS_POLICY_LIVELINESS_AUTOMATIC,
            ManualByTopic => Self::RMW_QOS_POLICY_LIVELINESS_MANUAL_BY_TOPIC,
            #[cfg(any(feature = "jazzy", feature = "kilted"))]
            BestAvailable => Self::RMW_QOS_POLICY_LIVELINESS_BEST_AVAILABLE,
            #[cfg(not(any(feature = "jazzy", feature = "kilted")))]
            BestAvailable => Self::RMW_QOS_POLICY_LIVELINESS_UNKNOWN,
            Unknown => Self::RMW_QOS_POLICY_LIVELINESS_UNKNOWN,
        }
    }
}

impl From<&super::rmw_qos_profile_t> for oxidros_core::qos::Profile {
    fn from(qos: &super::rmw_qos_profile_t) -> Self {
        Self {
            history: qos.history.into(),
            depth: qos.depth,
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

impl From<&oxidros_core::Profile> for super::rmw_qos_profile_t {
    fn from(qos: &oxidros_core::Profile) -> Self {
        super::rmw_qos_profile_t {
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

impl From<super::rcl_action_goal_event_t> for oxidros_core::action::GoalEvent {
    fn from(value: super::rcl_action_goal_event_t) -> Self {
        use super::rcl_action_goal_event_t::*;
        match value {
            GOAL_EVENT_EXECUTE => Self::Execute,
            GOAL_EVENT_CANCEL_GOAL => Self::CancelGoal,
            GOAL_EVENT_SUCCEED => Self::Succeed,
            GOAL_EVENT_ABORT => Self::Abort,
            GOAL_EVENT_CANCELED => Self::Canceled,
            GOAL_EVENT_NUM_EVENTS => Self::NumEvents,
        }
    }
}
impl From<oxidros_core::action::GoalEvent> for super::rcl_action_goal_event_t {
    fn from(value: oxidros_core::action::GoalEvent) -> Self {
        match value {
            oxidros_core::action::GoalEvent::Execute => Self::GOAL_EVENT_EXECUTE,
            oxidros_core::action::GoalEvent::CancelGoal => Self::GOAL_EVENT_CANCEL_GOAL,
            oxidros_core::action::GoalEvent::Succeed => Self::GOAL_EVENT_SUCCEED,
            oxidros_core::action::GoalEvent::Abort => Self::GOAL_EVENT_ABORT,
            oxidros_core::action::GoalEvent::Canceled => Self::GOAL_EVENT_CANCELED,
            oxidros_core::action::GoalEvent::NumEvents => Self::GOAL_EVENT_NUM_EVENTS,
        }
    }
}

impl From<&super::rcl_variant_t> for oxidros_core::parameter::Value {
    #[allow(clippy::useless_conversion)]
    fn from(var: &super::rcl_variant_t) -> Self {
        use oxidros_core::Value;
        if !var.bool_value.is_null() {
            Value::Bool(unsafe { *var.bool_value })
        } else if !var.integer_value.is_null() {
            Value::I64(unsafe { *var.integer_value })
        } else if !var.double_value.is_null() {
            Value::F64(unsafe { *var.double_value })
        } else if !var.string_value.is_null() {
            let s = unsafe { CStr::from_ptr(var.string_value) };
            Value::String(s.to_str().unwrap_or("").into())
        } else if !var.bool_array_value.is_null() {
            let v = &unsafe { *var.bool_array_value };
            let s = unsafe { from_raw_parts(v.values, v.size.try_into().unwrap()) };
            Value::VecBool(s.into())
        } else if !var.integer_array_value.is_null() {
            let v = &unsafe { *var.integer_array_value };
            let s = unsafe { from_raw_parts(v.values, v.size.try_into().unwrap()) };
            Value::VecI64(s.into())
        } else if !var.byte_array_value.is_null() {
            let v = &unsafe { *var.byte_array_value };
            let s = unsafe { from_raw_parts(v.values, v.size.try_into().unwrap()) };
            Value::VecU8(s.into())
        } else if !var.double_array_value.is_null() {
            let v = &unsafe { *var.double_array_value };
            let s = unsafe { from_raw_parts(v.values, v.size.try_into().unwrap()) };
            Value::VecF64(s.into())
        } else if !var.string_array_value.is_null() {
            let v = &unsafe { *var.string_array_value };
            let s = unsafe { from_raw_parts(v.data, v.size.try_into().unwrap()) };
            let s = s
                .iter()
                .map(|p| unsafe { CStr::from_ptr(*p).to_str().unwrap_or("").into() })
                .collect();
            Value::VecString(s)
        } else {
            Value::NotSet
        }
    }
}

impl From<RclRetErr> for crate::error::RclError {
    fn from(value: RclRetErr) -> Self {
        let value = value.0 as u32;
        match value {
            super::RCL_RET_ERROR => Self::Error,
            super::RCL_RET_TIMEOUT => Self::Timeout,
            super::RCL_RET_BAD_ALLOC => Self::BadAlloc,
            super::RCL_RET_INVALID_ARGUMENT => Self::InvalidArgument,
            super::RCL_RET_UNSUPPORTED => Self::Unsupported,
            super::RCL_RET_ALREADY_INIT => Self::AlreadyInit,
            super::RCL_RET_NOT_INIT => Self::NotInit,
            super::RCL_RET_MISMATCHED_RMW_ID => Self::MismatchedRmwId,
            super::RCL_RET_TOPIC_NAME_INVALID => Self::TopicNameInvalid,
            super::RCL_RET_SERVICE_NAME_INVALID => Self::ServiceNameInvalid,
            super::RCL_RET_UNKNOWN_SUBSTITUTION => Self::UnknownSubstitution,
            super::RCL_RET_ALREADY_SHUTDOWN => Self::AlreadyShutdown,
            super::RCL_RET_NODE_INVALID => Self::NodeInvalid,
            super::RCL_RET_NODE_INVALID_NAME => Self::NodeInvalidName,
            super::RCL_RET_NODE_INVALID_NAMESPACE => Self::NodeInvalidNamespace,
            super::RCL_RET_NODE_NAME_NON_EXISTENT => Self::NodeNameNonExistent,
            super::RCL_RET_PUBLISHER_INVALID => Self::PublisherInvalid,
            super::RCL_RET_SUBSCRIPTION_INVALID => Self::SubscriptionInvalid,
            super::RCL_RET_SUBSCRIPTION_TAKE_FAILED => Self::SubscriptionTakeFailed,
            super::RCL_RET_CLIENT_INVALID => Self::ClientInvalid,
            super::RCL_RET_CLIENT_TAKE_FAILED => Self::ClientTakeFailed,
            super::RCL_RET_SERVICE_INVALID => Self::ServiceInvalid,
            super::RCL_RET_SERVICE_TAKE_FAILED => Self::ServiceTakeFailed,
            super::RCL_RET_TIMER_INVALID => Self::TimerInvalid,
            super::RCL_RET_TIMER_CANCELED => Self::TimerCanceled,
            super::RCL_RET_WAIT_SET_INVALID => Self::WaitSetInvalid,
            super::RCL_RET_WAIT_SET_EMPTY => Self::WaitSetEmpty,
            super::RCL_RET_WAIT_SET_FULL => Self::WaitSetFull,
            super::RCL_RET_INVALID_REMAP_RULE => Self::InvalidRemapRule,
            super::RCL_RET_WRONG_LEXEME => Self::WrongLexeme,
            super::RCL_RET_INVALID_ROS_ARGS => Self::InvalidRosArgs,
            super::RCL_RET_INVALID_PARAM_RULE => Self::InvalidParamRule,
            super::RCL_RET_INVALID_LOG_LEVEL_RULE => Self::InvalidLogLevelRule,
            super::RCL_RET_EVENT_INVALID => Self::EventInvalid,
            super::RCL_RET_EVENT_TAKE_FAILED => Self::EventTakeFailed,
            super::RCL_RET_LIFECYCLE_STATE_REGISTERED => Self::LifecycleStateRegistered,
            super::RCL_RET_LIFECYCLE_STATE_NOT_REGISTERED => Self::LifecycleStateNotRegistered,
            _ => Self::InvalidRetVal,
        }
    }
}

impl From<oxidros_core::RclError> for RclRetErr {
    fn from(value: oxidros_core::RclError) -> Self {
        use oxidros_core::RclError::*;
        let err = match value {
            Error => super::RCL_RET_ERROR,
            Timeout => super::RCL_RET_TIMEOUT,
            BadAlloc => super::RCL_RET_BAD_ALLOC,
            InvalidArgument => super::RCL_RET_INVALID_ARGUMENT,
            Unsupported => super::RCL_RET_UNSUPPORTED,
            AlreadyInit => super::RCL_RET_ALREADY_INIT,
            NotInit => super::RCL_RET_NOT_INIT,
            MismatchedRmwId => super::RCL_RET_MISMATCHED_RMW_ID,
            TopicNameInvalid => super::RCL_RET_TOPIC_NAME_INVALID,
            ServiceNameInvalid => super::RCL_RET_SERVICE_NAME_INVALID,
            UnknownSubstitution => super::RCL_RET_UNKNOWN_SUBSTITUTION,
            AlreadyShutdown => super::RCL_RET_ALREADY_SHUTDOWN,
            NodeInvalid => super::RCL_RET_NODE_INVALID,
            NodeInvalidName => super::RCL_RET_NODE_INVALID_NAME,
            NodeInvalidNamespace => super::RCL_RET_NODE_INVALID_NAMESPACE,
            NodeNameNonExistent => super::RCL_RET_NODE_NAME_NON_EXISTENT,
            PublisherInvalid => super::RCL_RET_PUBLISHER_INVALID,
            SubscriptionInvalid => super::RCL_RET_SUBSCRIPTION_INVALID,
            SubscriptionTakeFailed => super::RCL_RET_SUBSCRIPTION_TAKE_FAILED,
            ClientInvalid => super::RCL_RET_CLIENT_INVALID,
            ClientTakeFailed => super::RCL_RET_CLIENT_TAKE_FAILED,
            ServiceInvalid => super::RCL_RET_SERVICE_INVALID,
            ServiceTakeFailed => super::RCL_RET_SERVICE_TAKE_FAILED,
            TimerInvalid => super::RCL_RET_TIMER_INVALID,
            TimerCanceled => super::RCL_RET_TIMER_CANCELED,
            WaitSetInvalid => super::RCL_RET_WAIT_SET_INVALID,
            WaitSetEmpty => super::RCL_RET_WAIT_SET_EMPTY,
            WaitSetFull => super::RCL_RET_WAIT_SET_FULL,
            InvalidRemapRule => super::RCL_RET_INVALID_REMAP_RULE,
            WrongLexeme => super::RCL_RET_WRONG_LEXEME,
            InvalidRosArgs => super::RCL_RET_INVALID_ROS_ARGS,
            InvalidParamRule => super::RCL_RET_INVALID_PARAM_RULE,
            InvalidLogLevelRule => super::RCL_RET_INVALID_LOG_LEVEL_RULE,
            EventInvalid => super::RCL_RET_EVENT_INVALID,
            EventTakeFailed => super::RCL_RET_EVENT_TAKE_FAILED,
            LifecycleStateRegistered => super::RCL_RET_LIFECYCLE_STATE_REGISTERED,
            LifecycleStateNotRegistered => super::RCL_RET_LIFECYCLE_STATE_NOT_REGISTERED,
            InvalidRetVal => !0,
        };
        Self(err as i32)
    }
}

impl From<ActionError> for RclRetErr {
    fn from(val: ActionError) -> Self {
        match val {
            ActionError::NameInvalid => super::RCL_RET_ACTION_NAME_INVALID.into(),
            ActionError::GoalAccepted => super::RCL_RET_ACTION_GOAL_ACCEPTED.into(),
            ActionError::GoalRejected => super::RCL_RET_ACTION_GOAL_REJECTED.into(),
            ActionError::ClientInvalid => super::RCL_RET_ACTION_CLIENT_INVALID.into(),
            ActionError::ClientTakeFailed => super::RCL_RET_ACTION_CLIENT_TAKE_FAILED.into(),
            ActionError::ServerInvalid => super::RCL_RET_ACTION_SERVER_INVALID.into(),
            ActionError::ServerTakeFailed => super::RCL_RET_ACTION_SERVER_TAKE_FAILED.into(),
            ActionError::GoalHandleInvalid => super::RCL_RET_ACTION_GOAL_HANDLE_INVALID.into(),
            ActionError::GoalEventInvalid => super::RCL_RET_ACTION_GOAL_EVENT_INVALID.into(),
            ActionError::Rcl(err) => err.into(),
            ActionError::InvalidRetVal => (!0).into(),
        }
    }
}

impl From<u32> for RclRetErr {
    fn from(value: u32) -> Self {
        Self(value as i32)
    }
}

impl From<i32> for RclRetErr {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

impl From<rmw_message_info_t> for oxidros_core::message::MessageInfo {
    #[cfg(feature = "humble")]
    fn from(value: rmw_message_info_t) -> Self {
        let mut publisher_gid = [0; 16];
        for (o, i) in publisher_gid.iter_mut().zip(value.publisher_gid.data) {
            *o = i;
        }
        Self {
            sequence_number: value.publication_sequence_number as i64,
            source_timestamp_ns: value.source_timestamp,
            publisher_gid,
        }
    }
    #[cfg(not(feature = "humble"))]
    fn from(value: rmw_message_info_t) -> Self {
        Self {
            sequence_number: value.publication_sequence_number as i64,
            source_timestamp_ns: value.source_timestamp,
            publisher_gid: to_gid(value.publisher_gid.data),
        }
    }
}

impl From<rmw_service_info_t> for oxidros_core::message::MessageInfo {
    #[cfg(feature = "humble")]
    fn from(value: rmw_service_info_t) -> Self {
        let mut publisher_gid = [0; 16];
        for (o, i) in publisher_gid.iter_mut().zip(value.request_id.writer_guid) {
            *o = i.try_into().unwrap_or_default();
        }
        Self {
            sequence_number: value.request_id.sequence_number,
            source_timestamp_ns: value.source_timestamp,
            publisher_gid,
        }
    }
    #[cfg(not(feature = "humble"))]
    fn from(value: rmw_service_info_t) -> Self {
        Self {
            sequence_number: value.request_id.sequence_number,
            source_timestamp_ns: value.source_timestamp,
            publisher_gid: to_gid(value.request_id.writer_guid),
        }
    }
}
