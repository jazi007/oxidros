//! Error types for ROS2 operations.
//!
//! This module provides a unified error type for all oxidros operations,
//! supporting both RCL-based and Zenoh-based implementations.

use thiserror::Error;

// ============================================================================
// Unified Error Type
// ============================================================================

/// Unified error type for oxidros operations.
///
/// This enum covers errors from all backends (RCL, Zenoh) and common operations.
#[derive(Debug, Error)]
pub enum Error {
    /// RCL/RMW layer error with specific error code.
    #[error("RCL error: {0}")]
    Rcl(#[from] RclError),

    /// Action-specific error.
    #[error("Action error: {0}")]
    Action(#[from] ActionError),

    /// CDR serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] ros2_types::Error),

    /// Zenoh middleware error.
    #[error("Zenoh error: {0}")]
    Zenoh(String),

    /// Invalid name (topic, node, namespace, service).
    #[error("Invalid name: {0}")]
    InvalidName(String),

    /// Operation timed out.
    #[error("Operation timed out")]
    Timeout,

    /// Channel or communication closed.
    #[error("Channel closed")]
    ChannelClosed,

    /// Context not initialized.
    #[error("Context not initialized")]
    NotInitialized,

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Resource not found.
    #[error("{kind} not found: {name}")]
    NotFound {
        /// Kind of resource (e.g., "Node", "Service", "Topic")
        kind: &'static str,
        /// Name of the resource
        name: String,
    },

    /// Operation was interrupted by signal.
    #[error("Operation interrupted by signal")]
    Interrupted,

    /// Feature not implemented in this backend.
    #[error("{feature} not implemented: {reason}")]
    NotImplemented {
        /// The feature that is not implemented.
        feature: String,
        /// Reason or additional context.
        reason: String,
    },

    /// Null byte not found
    #[error("Nul byte not found {0}")]
    NullError(#[from] std::ffi::NulError),

    /// Generic error with message.
    #[error("{0}")]
    Other(String),
}

/// Result type using the unified Error.
pub type Result<T> = std::result::Result<T, Error>;

// ============================================================================
// Convenience constructors
// ============================================================================

impl Error {
    /// Create a NotFound error for a node.
    pub fn node_not_found(name: impl Into<String>) -> Self {
        Error::NotFound {
            kind: "Node",
            name: name.into(),
        }
    }

    /// Create a NotFound error for a service.
    pub fn service_not_found(name: impl Into<String>) -> Self {
        Error::NotFound {
            kind: "Service",
            name: name.into(),
        }
    }

    /// Create a NotFound error for a topic.
    pub fn topic_not_found(name: impl Into<String>) -> Self {
        Error::NotFound {
            kind: "Topic",
            name: name.into(),
        }
    }

    /// Create a Zenoh error from any error type.
    pub fn zenoh(err: impl std::fmt::Display) -> Self {
        Error::Zenoh(err.to_string())
    }
}

// ============================================================================
// RCL Error (low-level RCL/RMW error codes)
// ============================================================================

/// Errors that can occur in RCL operations.
///
/// These correspond to error codes returned by the RCL C library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RclError {
    /// Generic error.
    #[error("Generic RCL error")]
    Error,

    /// Operation timed out.
    #[error("Operation timed out")]
    Timeout,

    /// Memory allocation failed.
    #[error("Memory allocation failed")]
    BadAlloc,

    /// Invalid argument provided.
    #[error("Invalid argument")]
    InvalidArgument,

    /// Operation not supported.
    #[error("Operation not supported")]
    Unsupported,

    /// Already initialized.
    #[error("Already initialized")]
    AlreadyInit,

    /// Not initialized.
    #[error("Not initialized")]
    NotInit,

    /// RMW implementation ID mismatch.
    #[error("RMW implementation ID mismatch")]
    MismatchedRmwId,

    /// Topic name is invalid.
    #[error("Topic name is invalid")]
    TopicNameInvalid,

    /// Service name is invalid.
    #[error("Service name is invalid")]
    ServiceNameInvalid,

    /// Unknown substitution in name.
    #[error("Unknown substitution in name")]
    UnknownSubstitution,

    /// Already shutdown.
    #[error("Already shutdown")]
    AlreadyShutdown,

    /// Node is invalid.
    #[error("Node is invalid")]
    NodeInvalid,

    /// Node name is invalid.
    #[error("Node name is invalid")]
    NodeInvalidName,

    /// Node namespace is invalid.
    #[error("Node namespace is invalid")]
    NodeInvalidNamespace,

    /// Node name does not exist.
    #[error("Node name does not exist")]
    NodeNameNonExistent,

    /// Publisher is invalid.
    #[error("Publisher is invalid")]
    PublisherInvalid,

    /// Subscription is invalid.
    #[error("Subscription is invalid")]
    SubscriptionInvalid,

    /// Failed to take from subscription.
    #[error("Failed to take from subscription")]
    SubscriptionTakeFailed,

    /// Client is invalid.
    #[error("Client is invalid")]
    ClientInvalid,

    /// Failed to take from client.
    #[error("Failed to take from client")]
    ClientTakeFailed,

    /// Service is invalid.
    #[error("Service is invalid")]
    ServiceInvalid,

    /// Failed to take from service.
    #[error("Failed to take from service")]
    ServiceTakeFailed,

    /// Timer is invalid.
    #[error("Timer is invalid")]
    TimerInvalid,

    /// Timer was canceled.
    #[error("Timer was canceled")]
    TimerCanceled,

    /// Wait set is invalid.
    #[error("Wait set is invalid")]
    WaitSetInvalid,

    /// Wait set is empty.
    #[error("Wait set is empty")]
    WaitSetEmpty,

    /// Wait set is full.
    #[error("Wait set is full")]
    WaitSetFull,

    /// Invalid remap rule.
    #[error("Invalid remap rule")]
    InvalidRemapRule,

    /// Wrong lexeme.
    #[error("Wrong lexeme")]
    WrongLexeme,

    /// Invalid ROS arguments.
    #[error("Invalid ROS arguments")]
    InvalidRosArgs,

    /// Invalid parameter rule.
    #[error("Invalid parameter rule")]
    InvalidParamRule,

    /// Invalid log level rule.
    #[error("Invalid log level rule")]
    InvalidLogLevelRule,

    /// Event is invalid.
    #[error("Event is invalid")]
    EventInvalid,

    /// Failed to take event.
    #[error("Failed to take event")]
    EventTakeFailed,

    /// Lifecycle state registered.
    #[error("Lifecycle state already registered")]
    LifecycleStateRegistered,

    /// Lifecycle state not registered.
    #[error("Lifecycle state not registered")]
    LifecycleStateNotRegistered,

    /// Invalid return value (unknown error code).
    #[error("Invalid return value (unknown error code)")]
    InvalidRetVal,
}

// ============================================================================
// Action Error
// ============================================================================

/// Errors specific to ROS2 action operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ActionError {
    /// Action name is invalid.
    #[error("Action name is invalid")]
    NameInvalid,

    /// Goal was accepted.
    #[error("Goal was accepted")]
    GoalAccepted,

    /// Goal was rejected.
    #[error("Goal was rejected")]
    GoalRejected,

    /// Action client is invalid.
    #[error("Action client is invalid")]
    ClientInvalid,

    /// Failed to take from action client.
    #[error("Failed to take from action client")]
    ClientTakeFailed,

    /// Action server is invalid.
    #[error("Action server is invalid")]
    ServerInvalid,

    /// Failed to take from action server.
    #[error("Failed to take from action server")]
    ServerTakeFailed,

    /// Goal handle is invalid.
    #[error("Goal handle is invalid")]
    GoalHandleInvalid,

    /// Goal event is invalid.
    #[error("Goal event is invalid")]
    GoalEventInvalid,

    /// Wrapped RCL error.
    #[error("RCL error: {0}")]
    Rcl(#[from] RclError),

    /// Invalid return value (unknown error code).
    #[error("Invalid return value (unknown error code)")]
    InvalidRetVal,
}

// ============================================================================
// Conversions
// ============================================================================

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Other(s)
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::Other(s.to_string())
    }
}

impl From<Box<dyn std::error::Error + Send + Sync + 'static>> for Error {
    fn from(err: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        Error::Other(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::Timeout;
        assert_eq!(format!("{}", err), "Operation timed out");

        let err = Error::Rcl(RclError::NodeInvalid);
        assert_eq!(format!("{}", err), "RCL error: Node is invalid");

        let err = Error::node_not_found("my_node");
        assert_eq!(format!("{}", err), "Node not found: my_node");
    }

    #[test]
    fn test_rcl_error_conversion() {
        let rcl_err = RclError::Timeout;
        let err: Error = rcl_err.into();
        assert!(matches!(err, Error::Rcl(RclError::Timeout)));
    }

    #[test]
    fn test_action_error_conversion() {
        let action_err = ActionError::GoalRejected;
        let err: Error = action_err.into();
        assert!(matches!(err, Error::Action(ActionError::GoalRejected)));
    }
}
