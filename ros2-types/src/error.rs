//! Error types for type hash calculation

use thiserror::Error;

/// Result type for type hash operations
pub type Result<T> = std::result::Result<T, Error>;

/// Invalid RIHS format error details
#[derive(Debug, Error)]
pub enum InvalidRihsFormat {
    /// String does not start with 'RIHS'
    #[error("String does not start with 'RIHS'")]
    MissingPrefix,

    /// Invalid format structure
    #[error("Expected format RIHS<version>_<hash>")]
    InvalidStructure,

    /// Could not extract version
    #[error("Could not extract version from RIHS string")]
    VersionExtractionFailed,

    /// Invalid version number
    #[error("Invalid version number: {version_str}")]
    InvalidVersionNumber {
        /// The invalid version string
        version_str: String,
    },
}

/// Type description error details
#[derive(Debug, Error)]
pub enum TypeDescriptionError {
    /// Missing required field
    #[error("Missing required field: {field_name}")]
    MissingField {
        /// Name of the missing field
        field_name: String,
    },

    /// Invalid field value
    #[error("Invalid value for field '{field_name}': {reason}")]
    InvalidFieldValue {
        /// Field name
        field_name: String,
        /// Reason for invalidity
        reason: String,
    },
}

/// Errors that can occur during type hash calculation
#[derive(Debug, Error)]
pub enum Error {
    /// JSON serialization error
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),

    /// Invalid RIHS format
    #[error(transparent)]
    InvalidRihsFormat(#[from] InvalidRihsFormat),

    /// Type description error
    #[error(transparent)]
    TypeDescriptionError(#[from] TypeDescriptionError),

    /// CDR serialization error
    #[error("CDR serialization error: {0}")]
    CdrError(String),
}
