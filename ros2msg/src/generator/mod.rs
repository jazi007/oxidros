//! Code generator for converting ROS2 messages/services/actions/IDL to Rust types
//!
//! This module provides a bindgen-style API for generating Rust code from ROS2 interface files.
//! Each source file (.msg, .srv, .action, .idl) generates a corresponding .rs file.
//!
//! # Example
//!
//! ```no_run
//! use ros2msg::generator::Generator;
//!
//! Generator::new()
//!     .header("// Auto-generated code - do not edit")
//!     .derive_debug(true)
//!     .derive_clone(true)
//!     .derive_eq(true)
//!     .raw_line("use serde::{Serialize, Deserialize};")
//!     .ctypes_prefix("std::os::raw")
//!     .include("/opt/ros/jazzy/share/std_msgs/msg/Header.msg")
//!     .include("/opt/ros/jazzy/share/std_msgs/msg/String.msg")
//!     .output_dir("generated")
//!     .emit_rerun_if_changed(true)
//!     .generate()
//!     .expect("Failed to generate bindings");
//! ```

mod builder;
mod callbacks;
mod codegen;
mod config;
mod token_gen;
mod types;

pub use builder::Generator;
pub use callbacks::{FieldInfo, ItemInfo, ModuleInfo, ModuleLevel, ParseCallbacks};
pub use codegen::CodeGenerator;
pub use config::{GeneratorConfig, sanitize_rust_identifier};
pub use types::TypeMapper;

use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::idl::IdlError;
use crate::msg::ParseError;

/// Configuration error details
#[derive(Debug, Error)]
pub enum ConfigError {
    /// File has no extension
    #[error("File has no extension")]
    NoFileExtension,

    /// Unsupported file extension
    #[error("Unsupported file extension: {extension}")]
    UnsupportedFileExtension {
        /// The unsupported extension
        extension: String,
    },

    /// Cannot extract package name from path
    #[error("Cannot extract package name from path: {path}")]
    PackageNameExtractionFailed {
        /// The path that failed
        path: PathBuf,
    },

    /// Cannot extract module name from path
    #[error("Cannot extract module name from path")]
    ModuleNameExtractionFailed,

    /// Output directory is required
    #[error("Output directory is required but not set")]
    OutputDirectoryRequired,

    /// No input files provided
    #[error("No input files provided to generator")]
    NoInputFiles,
}

/// Code generation error details
#[derive(Debug, Error)]
pub enum GenerationError {
    /// No message found in generated IDL
    #[error("No message found in generated IDL")]
    NoMessageInIdl,

    /// Invalid service IDL structure
    #[error("Expected 2 messages (Request/Response) in service IDL, found {found}")]
    InvalidServiceIdl {
        /// Number of messages found
        found: usize,
    },

    /// Invalid action IDL structure
    #[error("Expected 3 messages (Goal/Result/Feedback) in action IDL, found {found}")]
    InvalidActionIdl {
        /// Number of messages found
        found: usize,
    },
}

/// Errors that can occur during code generation
#[derive(Debug, Error)]
pub enum GeneratorError {
    /// MSG/SRV/Action parse error
    #[error(transparent)]
    MsgParseError(#[from] ParseError),

    /// IDL parse error
    #[error(transparent)]
    IdlParseError(#[from] IdlError),

    /// I/O error
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    /// Configuration error
    #[error(transparent)]
    ConfigError(#[from] ConfigError),

    /// Code generation error
    #[error(transparent)]
    GenerationError(#[from] GenerationError),
}

/// Result type for generator operations
pub type GeneratorResult<T> = Result<T, GeneratorError>;

/// ROS2 interface file type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileType {
    /// Message file (.msg)
    Message,
    /// Service file (.srv)
    Service,
    /// Action file (.action)
    Action,
    /// IDL file (.idl)
    Idl,
}

impl FileType {
    /// Get the file extension for this file type
    #[must_use]
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Message => "msg",
            Self::Service => "srv",
            Self::Action => "action",
            Self::Idl => "idl",
        }
    }

    /// Parse file type from extension string
    #[must_use]
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "msg" => Some(Self::Message),
            "srv" => Some(Self::Service),
            "action" => Some(Self::Action),
            "idl" => Some(Self::Idl),
            _ => None,
        }
    }

    /// Get the import depth for this file type
    ///
    /// Returns the number of `super::` needed to reach the generated/ root
    #[must_use]
    pub const fn import_depth(&self) -> usize {
        // msg/srv/action/idl files are organized as: generated/package/type/file.rs
        // So depth is: file -> type -> package -> generated = 3 levels
        3
    }
}

impl std::fmt::Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.extension())
    }
}

/// ROS2 interface kind - distinguishes the semantic type of interface
///
/// While `FileType` represents the source file extension, `InterfaceKind`
/// represents the semantic type of the interface content. An IDL file
/// may contain any of msg/srv/action interfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterfaceKind {
    /// Message interface
    Message,
    /// Service interface (request/response pair)
    Service,
    /// Action interface (goal/result/feedback)
    Action,
}

impl InterfaceKind {
    /// Get the directory name for this interface kind
    #[must_use]
    pub fn dir_name(&self) -> &'static str {
        match self {
            Self::Message => "msg",
            Self::Service => "srv",
            Self::Action => "action",
        }
    }

    /// Get the import depth for this interface kind
    #[must_use]
    pub const fn import_depth(&self) -> usize {
        // All interface kinds are organized as: generated/package/type/file.rs
        // So depth is: file -> type -> package -> generated = 3 levels
        3
    }
}

impl std::fmt::Display for InterfaceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.dir_name())
    }
}

/// Generated code output
#[derive(Debug, Clone)]
pub struct GeneratedCode {
    /// The generated Rust code
    pub code: String,

    /// Original source file path
    pub source_file: PathBuf,

    /// Package name
    pub package_name: String,

    /// Module name (file basename without extension)
    pub module_name: String,

    /// File type (source file extension)
    pub file_type: FileType,

    /// Interface kind (semantic type: msg/srv/action)
    /// This is determined from the content, not the file extension.
    /// For .idl files, this reflects whether the IDL contains a message, service, or action.
    pub interface_kind: InterfaceKind,

    /// Dependencies: packages that this type references
    pub dependencies: Vec<String>,
}

impl GeneratedCode {
    /// Write the generated code to a file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written
    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> GeneratorResult<()> {
        let path = path.as_ref();

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, &self.code)?;
        Ok(())
    }

    /// Get the suggested output filename based on the source file
    #[must_use]
    pub fn suggested_filename(&self) -> String {
        format!("{}.rs", self.module_name)
    }
}
