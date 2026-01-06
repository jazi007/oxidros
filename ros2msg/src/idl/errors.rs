//! Error types for IDL parsing

use thiserror::Error;

/// IDL parsing error
#[derive(Error, Debug)]
pub enum IdlError {
    /// Parsing error with location context
    #[error("IDL parsing error at line {line}, column {column}: {message}")]
    ParseError {
        /// Line number where error occurred
        line: usize,
        /// Column number where error occurred
        column: usize,
        /// Error message describing the parsing failure
        message: String,
    },

    /// IO error during file operations
    #[error("IDL IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Semantic validation error
    #[error("IDL validation error: {message}")]
    ValidationError {
        /// Error message describing the validation failure
        message: String,
    },

    /// Type resolution error
    #[error("IDL type resolution error: {message}")]
    TypeResolutionError {
        /// Error message describing the type resolution failure
        message: String,
    },

    /// Annotation processing error
    #[error("IDL annotation error: {message}")]
    AnnotationError {
        /// Error message describing the annotation error
        message: String,
    },
}

/// Result type for IDL operations
pub type IdlResult<T> = Result<T, IdlError>;
