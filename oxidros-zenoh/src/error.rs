//! Error types for oxidros-zenoh.

use thiserror::Error;

/// Result type for oxidros-zenoh operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in oxidros-zenoh.
#[derive(Debug, Error)]
pub enum Error {
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
