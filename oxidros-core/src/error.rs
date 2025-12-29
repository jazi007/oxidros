//! Error types for ROS2 operations.

use std::{
    error::Error,
    fmt::{self, Debug, Display},
};

/// Dynamic error type that can be sent and shared between threads.
pub type DynError = Box<dyn Error + Send + Sync + 'static>;

/// Result type using RCLError.
pub type OResult<T> = Result<T, OError>;

/// Errors that can occur in RCL operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OError {
    /// Generic error.
    Error,

    /// Operation timed out.
    Timeout,

    /// Memory allocation failed.
    BadAlloc,

    /// Invalid argument provided.
    InvalidArgument,

    /// Operation not supported.
    Unsupported,

    /// Already initialized.
    AlreadyInit,

    /// Not initialized.
    NotInit,

    /// RMW implementation ID mismatch.
    MismatchedRmwId,

    /// Topic name is invalid.
    TopicNameInvalid,

    /// Service name is invalid.
    ServiceNameInvalid,

    /// Unknown substitution in name.
    UnknownSubstitution,

    /// Already shutdown.
    AlreadyShutdown,

    /// Node is invalid.
    NodeInvalid,

    /// Node name is invalid.
    NodeInvalidName,

    /// Node namespace is invalid.
    NodeInvalidNamespace,

    /// Node name does not exist.
    NodeNameNonExistent,

    /// Publisher is invalid.
    PublisherInvalid,

    /// Subscription is invalid.
    SubscriptionInvalid,

    /// Failed to take from subscription.
    SubscriptionTakeFailed,

    /// Client is invalid.
    ClientInvalid,

    /// Failed to take from client.
    ClientTakeFailed,

    /// Service is invalid.
    ServiceInvalid,

    /// Failed to take from service.
    ServiceTakeFailed,

    /// Timer is invalid.
    TimerInvalid,

    /// Timer was canceled.
    TimerCanceled,

    /// Wait set is invalid.
    WaitSetInvalid,

    /// Wait set is empty.
    WaitSetEmpty,

    /// Wait set is full.
    WaitSetFull,

    /// Invalid remap rule.
    InvalidRemapRule,

    /// Wrong lexeme.
    WrongLexeme,

    /// Invalid ROS arguments.
    InvalidRosArgs,

    /// Invalid parameter rule.
    InvalidParamRule,

    /// Invalid log level rule.
    InvalidLogLevelRule,

    /// Event is invalid.
    EventInvalid,

    /// Failed to take event.
    EventTakeFailed,

    /// Lifecycle state registered.
    LifecycleStateRegistered,

    /// Lifecycle state not registered.
    LifecycleStateNotRegistered,

    /// Invalid return value (unknown error code).
    InvalidRetVal,
}

impl Display for OError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for OError {}

/// Result type using RCLActionError.
pub type RCLActionResult<T> = Result<T, RCLActionError>;

/// Errors specific to RCL action operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RCLActionError {
    /// Action name is invalid.
    NameInvalid,

    /// Goal was accepted.
    GoalAccepted,

    /// Goal was rejected.
    GoalRejected,

    /// Action client is invalid.
    ClientInvalid,

    /// Failed to take from action client.
    ClientTakeFailed,

    /// Action server is invalid.
    ServerInvalid,

    /// Failed to take from action server.
    ServerTakeFailed,

    /// Goal handle is invalid.
    GoalHandleInvalid,

    /// Goal event is invalid.
    GoalEventInvalid,

    /// Generic RCL error occurred.
    RCLError(OError),

    /// Invalid return value (unknown error code).
    InvalidRetVal,
}

impl Display for RCLActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for RCLActionError {}

impl From<OError> for RCLActionError {
    fn from(err: OError) -> Self {
        RCLActionError::RCLError(err)
    }
}
