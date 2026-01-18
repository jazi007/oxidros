//! Error types for ROS2 argument parsing

use std::path::PathBuf;
use thiserror::Error;

use crate::names::NameKind;

/// Errors that can occur during ROS2 argument parsing
#[derive(Debug, Error)]
pub enum Ros2ArgsError {
    /// Invalid remapping rule format
    #[error("Invalid remapping rule '{0}': expected format 'from:=to' or 'node:from:=to'")]
    InvalidRemapRule(String),

    /// Invalid ROS2 name (node, topic, namespace, or substitution)
    #[error("Invalid {kind} name '{name}': {reason}")]
    InvalidName {
        /// The kind of name that failed validation
        kind: NameKind,
        /// The invalid name
        name: String,
        /// The reason the name is invalid
        reason: String,
    },

    /// Invalid parameter assignment format
    #[error(
        "Invalid parameter assignment '{0}': expected format 'name:=value' or 'node:name:=value'"
    )]
    InvalidParamAssignment(String),

    /// Invalid YAML value in parameter
    #[error("Invalid YAML value in parameter '{0}': {1}")]
    InvalidYamlValue(String, String),

    /// Invalid log level
    #[error("Invalid log level '{0}': expected DEBUG, INFO, WARN, ERROR, or FATAL")]
    InvalidLogLevel(String),

    /// Invalid log level assignment format
    #[error("Invalid log level assignment '{0}': expected 'LEVEL' or 'logger:=LEVEL'")]
    InvalidLogLevelAssignment(String),

    /// Parameter file not found
    #[error("Parameter file not found: {0}")]
    ParamFileNotFound(PathBuf),

    /// Parameter file parsing error
    #[error("Failed to parse parameter file '{0}': {1}")]
    ParamFileParseError(PathBuf, String),

    /// Invalid parameter file structure
    #[error("Invalid parameter file structure: {0}")]
    InvalidParamFileStructure(String),

    /// Log configuration file not found
    #[error("Log configuration file not found: {0}")]
    LogConfigFileNotFound(PathBuf),

    /// Invalid enclave path
    #[error("Invalid enclave path '{0}': must be a fully qualified path")]
    InvalidEnclavePath(String),

    /// Missing required argument value
    #[error("Missing value for argument '{0}'")]
    MissingArgumentValue(String),

    /// Unexpected argument
    #[error("Unexpected argument '{0}' in ROS args section")]
    UnexpectedArgument(String),

    /// IO error
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

/// Result type for ROS2 argument parsing operations
pub type Ros2ArgsResult<T> = Result<T, Ros2ArgsError>;
