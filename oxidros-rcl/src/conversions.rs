//! Type conversions for RCL types

use std::error::Error;
use std::ffi::CStr;
use std::fmt;
use std::slice::from_raw_parts;
use std::time::Duration;

use oxidros_core::RCLActionError;

use crate::RclRetErr;

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

impl From<crate::rcl_action_goal_event_t> for oxidros_core::action::GoalEvent {
    fn from(value: crate::rcl_action_goal_event_t) -> Self {
        use crate::rcl_action_goal_event_t::*;
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
impl From<oxidros_core::action::GoalEvent> for crate::rcl_action_goal_event_t {
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

impl From<&crate::rcl_variant_t> for oxidros_core::parameter::Value {
    #[allow(clippy::useless_conversion)]
    fn from(var: &crate::rcl_variant_t) -> Self {
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

impl From<RclRetErr> for oxidros_core::RCLError {
    fn from(value: RclRetErr) -> Self {
        let value = value.0 as u32;
        match value {
            crate::RCL_RET_ERROR => Self::Error,
            crate::RCL_RET_TIMEOUT => Self::Timeout,
            crate::RCL_RET_BAD_ALLOC => Self::BadAlloc,
            crate::RCL_RET_INVALID_ARGUMENT => Self::InvalidArgument,
            crate::RCL_RET_UNSUPPORTED => Self::Unsupported,
            crate::RCL_RET_ALREADY_INIT => Self::AlreadyInit,
            crate::RCL_RET_NOT_INIT => Self::NotInit,
            crate::RCL_RET_MISMATCHED_RMW_ID => Self::MismatchedRmwId,
            crate::RCL_RET_TOPIC_NAME_INVALID => Self::TopicNameInvalid,
            crate::RCL_RET_SERVICE_NAME_INVALID => Self::ServiceNameInvalid,
            crate::RCL_RET_UNKNOWN_SUBSTITUTION => Self::UnknownSubstitution,
            crate::RCL_RET_ALREADY_SHUTDOWN => Self::AlreadyShutdown,
            crate::RCL_RET_NODE_INVALID => Self::NodeInvalid,
            crate::RCL_RET_NODE_INVALID_NAME => Self::NodeInvalidName,
            crate::RCL_RET_NODE_INVALID_NAMESPACE => Self::NodeInvalidNamespace,
            crate::RCL_RET_NODE_NAME_NON_EXISTENT => Self::NodeNameNonExistent,
            crate::RCL_RET_PUBLISHER_INVALID => Self::PublisherInvalid,
            crate::RCL_RET_SUBSCRIPTION_INVALID => Self::SubscriptionInvalid,
            crate::RCL_RET_SUBSCRIPTION_TAKE_FAILED => Self::SubscriptionTakeFailed,
            crate::RCL_RET_CLIENT_INVALID => Self::ClientInvalid,
            crate::RCL_RET_CLIENT_TAKE_FAILED => Self::ClientTakeFailed,
            crate::RCL_RET_SERVICE_INVALID => Self::ServiceInvalid,
            crate::RCL_RET_SERVICE_TAKE_FAILED => Self::ServiceTakeFailed,
            crate::RCL_RET_TIMER_INVALID => Self::TimerInvalid,
            crate::RCL_RET_TIMER_CANCELED => Self::TimerCanceled,
            crate::RCL_RET_WAIT_SET_INVALID => Self::WaitSetInvalid,
            crate::RCL_RET_WAIT_SET_EMPTY => Self::WaitSetEmpty,
            crate::RCL_RET_WAIT_SET_FULL => Self::WaitSetFull,
            crate::RCL_RET_INVALID_REMAP_RULE => Self::InvalidRemapRule,
            crate::RCL_RET_WRONG_LEXEME => Self::WrongLexeme,
            crate::RCL_RET_INVALID_ROS_ARGS => Self::InvalidRosArgs,
            crate::RCL_RET_INVALID_PARAM_RULE => Self::InvalidParamRule,
            crate::RCL_RET_INVALID_LOG_LEVEL_RULE => Self::InvalidLogLevelRule,
            crate::RCL_RET_EVENT_INVALID => Self::EventInvalid,
            crate::RCL_RET_EVENT_TAKE_FAILED => Self::EventTakeFailed,
            crate::RCL_RET_LIFECYCLE_STATE_REGISTERED => Self::LifecycleStateRegistered,
            crate::RCL_RET_LIFECYCLE_STATE_NOT_REGISTERED => Self::LifecycleStateNotRegistered,
            _ => Self::InvalidRetVal,
        }
    }
}

impl From<oxidros_core::RCLError> for RclRetErr {
    fn from(value: oxidros_core::RCLError) -> Self {
        use oxidros_core::RCLError::*;
        let err = match value {
            Error => crate::RCL_RET_ERROR,
            Timeout => crate::RCL_RET_TIMEOUT,
            BadAlloc => crate::RCL_RET_BAD_ALLOC,
            InvalidArgument => crate::RCL_RET_INVALID_ARGUMENT,
            Unsupported => crate::RCL_RET_UNSUPPORTED,
            AlreadyInit => crate::RCL_RET_ALREADY_INIT,
            NotInit => crate::RCL_RET_NOT_INIT,
            MismatchedRmwId => crate::RCL_RET_MISMATCHED_RMW_ID,
            TopicNameInvalid => crate::RCL_RET_TOPIC_NAME_INVALID,
            ServiceNameInvalid => crate::RCL_RET_SERVICE_NAME_INVALID,
            UnknownSubstitution => crate::RCL_RET_UNKNOWN_SUBSTITUTION,
            AlreadyShutdown => crate::RCL_RET_ALREADY_SHUTDOWN,
            NodeInvalid => crate::RCL_RET_NODE_INVALID,
            NodeInvalidName => crate::RCL_RET_NODE_INVALID_NAME,
            NodeInvalidNamespace => crate::RCL_RET_NODE_INVALID_NAMESPACE,
            NodeNameNonExistent => crate::RCL_RET_NODE_NAME_NON_EXISTENT,
            PublisherInvalid => crate::RCL_RET_PUBLISHER_INVALID,
            SubscriptionInvalid => crate::RCL_RET_SUBSCRIPTION_INVALID,
            SubscriptionTakeFailed => crate::RCL_RET_SUBSCRIPTION_TAKE_FAILED,
            ClientInvalid => crate::RCL_RET_CLIENT_INVALID,
            ClientTakeFailed => crate::RCL_RET_CLIENT_TAKE_FAILED,
            ServiceInvalid => crate::RCL_RET_SERVICE_INVALID,
            ServiceTakeFailed => crate::RCL_RET_SERVICE_TAKE_FAILED,
            TimerInvalid => crate::RCL_RET_TIMER_INVALID,
            TimerCanceled => crate::RCL_RET_TIMER_CANCELED,
            WaitSetInvalid => crate::RCL_RET_WAIT_SET_INVALID,
            WaitSetEmpty => crate::RCL_RET_WAIT_SET_EMPTY,
            WaitSetFull => crate::RCL_RET_WAIT_SET_FULL,
            InvalidRemapRule => crate::RCL_RET_INVALID_REMAP_RULE,
            WrongLexeme => crate::RCL_RET_WRONG_LEXEME,
            InvalidRosArgs => crate::RCL_RET_INVALID_ROS_ARGS,
            InvalidParamRule => crate::RCL_RET_INVALID_PARAM_RULE,
            InvalidLogLevelRule => crate::RCL_RET_INVALID_LOG_LEVEL_RULE,
            EventInvalid => crate::RCL_RET_EVENT_INVALID,
            EventTakeFailed => crate::RCL_RET_EVENT_TAKE_FAILED,
            LifecycleStateRegistered => crate::RCL_RET_LIFECYCLE_STATE_REGISTERED,
            LifecycleStateNotRegistered => crate::RCL_RET_LIFECYCLE_STATE_NOT_REGISTERED,
            InvalidRetVal => !0,
        };
        Self(err as i32)
    }
}

impl From<RCLActionError> for RclRetErr {
    fn from(val: RCLActionError) -> Self {
        match val {
            RCLActionError::NameInvalid => crate::RCL_RET_ACTION_NAME_INVALID.into(),
            RCLActionError::GoalAccepted => crate::RCL_RET_ACTION_GOAL_ACCEPTED.into(),
            RCLActionError::GoalRejected => crate::RCL_RET_ACTION_GOAL_REJECTED.into(),
            RCLActionError::ClientInvalid => crate::RCL_RET_ACTION_CLIENT_INVALID.into(),
            RCLActionError::ClientTakeFailed => crate::RCL_RET_ACTION_CLIENT_TAKE_FAILED.into(),
            RCLActionError::ServerInvalid => crate::RCL_RET_ACTION_SERVER_INVALID.into(),
            RCLActionError::ServerTakeFailed => crate::RCL_RET_ACTION_SERVER_TAKE_FAILED.into(),
            RCLActionError::GoalHandleInvalid => crate::RCL_RET_ACTION_GOAL_HANDLE_INVALID.into(),
            RCLActionError::GoalEventInvalid => crate::RCL_RET_ACTION_GOAL_EVENT_INVALID.into(),
            RCLActionError::RCLError(err) => err.into(),
            RCLActionError::InvalidRetVal => (!0).into(),
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
