//! Error types for oxidros-zenoh.
//!
//! This module provides error types that are compatible with `oxidros-core`
//! while also handling Zenoh-specific errors.

use oxidros_core::error::OError;
use thiserror::Error;

/// Result type for oxidros-zenoh operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in oxidros-zenoh.
#[derive(Debug, Error)]
pub enum Error {
    /// Core ROS2 error from oxidros-core
    #[error("{0}")]
    Core(OError),

    /// Zenoh session error
    #[error("Zenoh error: {0}")]
    Zenoh(#[from] zenoh::Error),

    /// CDR serialization error
    #[error("CDR serialization error: {0}")]
    Cdr(#[from] ros2_types::Error),

    /// Invalid name (topic, node, namespace)
    #[error("Invalid name: {0}")]
    InvalidName(#[from] ros2args::Ros2ArgsError),

    /// Context not initialized
    #[error("Context not initialized")]
    ContextNotInitialized,

    /// Node not found
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    /// Service not available
    #[error("Service not available: {0}")]
    ServiceNotAvailable(String),

    /// Timeout waiting for response
    #[error("Timeout")]
    Timeout,

    /// Channel closed
    #[error("Channel closed")]
    ChannelClosed,

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

impl From<OError> for Error {
    fn from(err: OError) -> Self {
        Error::Core(err)
    }
}

impl From<Error> for OError {
    fn from(err: Error) -> Self {
        match err {
            Error::Core(e) => e,
            Error::Timeout => OError::Timeout,
            Error::InvalidName(_) => OError::TopicNameInvalid,
            Error::InvalidConfig(_) => OError::InvalidArgument,
            Error::ContextNotInitialized => OError::NotInit,
            Error::ChannelClosed => OError::Error,
            Error::NodeNotFound(_) => OError::NodeInvalid,
            Error::ServiceNotAvailable(_) => OError::ServiceInvalid,
            Error::Zenoh(_) => OError::Error,
            Error::Cdr(_) => OError::Error,
        }
    }
}
