//! Type definitions for ROS2 command-line arguments

use std::path::PathBuf;
use yaml_rust2::Yaml;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Represents a name remapping rule
///
/// Remapping rules can be either global (applying to all nodes) or node-specific.
///
/// # Examples
///
/// - Global: `foo:=bar` remaps `foo` to `bar` for all nodes
/// - Node-specific: `my_node:foo:=bar` remaps `foo` to `bar` only for `my_node`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RemapRule {
    /// Optional node name to target (None means applies to all nodes)
    pub node_name: Option<String>,
    /// The original name to remap from
    pub from: String,
    /// The new name to remap to
    pub to: String,
}

impl RemapRule {
    /// Create a new global remapping rule
    #[must_use]
    pub fn new_global(from: String, to: String) -> Self {
        Self {
            node_name: None,
            from,
            to,
        }
    }

    /// Create a new node-specific remapping rule
    #[must_use]
    pub fn new_node_specific(node_name: String, from: String, to: String) -> Self {
        Self {
            node_name: Some(node_name),
            from,
            to,
        }
    }

    /// Check if this rule applies to a specific node
    #[must_use]
    pub fn applies_to_node(&self, node_name: &str) -> bool {
        self.node_name.as_ref().is_none_or(|n| n == node_name)
    }
}

/// Represents a parameter assignment
///
/// Parameters can be either global (applying to all nodes) or node-specific.
/// Values are stored as YAML types to preserve type information.
///
/// # Examples
///
/// - Global: `use_sim_time:=true`
/// - Node-specific: `my_node:use_sim_time:=true`
#[derive(Debug, Clone, PartialEq)]
pub struct ParamAssignment {
    /// Optional node name to target (None means applies to all nodes)
    pub node_name: Option<String>,
    /// Parameter name
    pub name: String,
    /// Parameter value (stored as YAML value to preserve type information)
    pub value: Yaml,
}

impl ParamAssignment {
    /// Create a new global parameter assignment
    #[must_use]
    pub fn new_global(name: String, value: Yaml) -> Self {
        Self {
            node_name: None,
            name,
            value,
        }
    }

    /// Create a new node-specific parameter assignment
    #[must_use]
    pub fn new_node_specific(node_name: String, name: String, value: Yaml) -> Self {
        Self {
            node_name: Some(node_name),
            name,
            value,
        }
    }

    /// Check if this parameter applies to a specific node
    #[must_use]
    pub fn applies_to_node(&self, node_name: &str) -> bool {
        self.node_name.as_ref().is_none_or(|n| n == node_name)
    }

    /// Get the value as a boolean, if it is one
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        self.value.as_bool()
    }

    /// Get the value as an integer, if it is one
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        self.value.as_i64()
    }

    /// Get the value as a float, if it is one
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        self.value.as_f64()
    }

    /// Get the value as a string, if it is one
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        self.value.as_str()
    }

    /// Get the value as a YAML array, if it is one
    #[must_use]
    pub fn as_vec(&self) -> Option<&Vec<Yaml>> {
        self.value.as_vec()
    }

    /// Get the value as a YAML hash/map, if it is one
    #[must_use]
    pub fn as_hash(&self) -> Option<&yaml_rust2::yaml::Hash> {
        self.value.as_hash()
    }

    /// Check if the value is null
    #[must_use]
    pub fn is_null(&self) -> bool {
        self.value.is_null()
    }

    /// Get a reference to the underlying YAML value
    #[must_use]
    pub fn value(&self) -> &Yaml {
        &self.value
    }
}

/// Log levels supported by ROS2
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LogLevel {
    /// Debug level logging
    Debug,
    /// Info level logging
    Info,
    /// Warning level logging
    Warn,
    /// Error level logging
    Error,
    /// Fatal level logging
    Fatal,
}

impl LogLevel {
    /// Convert the log level to a string
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Fatal => "FATAL",
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "DEBUG" => Ok(Self::Debug),
            "INFO" => Ok(Self::Info),
            "WARN" | "WARNING" => Ok(Self::Warn),
            "ERROR" => Ok(Self::Error),
            "FATAL" => Ok(Self::Fatal),
            _ => Err(format!("Invalid log level: {s}")),
        }
    }
}

/// Represents a log level assignment
///
/// Can be either a global log level or logger-specific.
///
/// # Examples
///
/// - Global: `--log-level DEBUG`
/// - Logger-specific: `--log-level rclcpp:=DEBUG`
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LogLevelAssignment {
    /// Optional logger name (None means global)
    pub logger_name: Option<String>,
    /// Log level
    pub level: LogLevel,
}

impl LogLevelAssignment {
    /// Create a new global log level assignment
    #[must_use]
    pub fn new_global(level: LogLevel) -> Self {
        Self {
            logger_name: None,
            level,
        }
    }

    /// Create a new logger-specific log level assignment
    #[must_use]
    pub fn new_logger_specific(logger_name: String, level: LogLevel) -> Self {
        Self {
            logger_name: Some(logger_name),
            level,
        }
    }
}

/// Represents logging output configuration flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LoggingOutputConfig {
    /// Enable/disable rosout logging (None means not specified)
    pub rosout: Option<bool>,
    /// Enable/disable stdout logging (None means not specified)
    pub stdout: Option<bool>,
    /// Enable/disable external library logging (None means not specified)
    pub external_lib: Option<bool>,
}

/// Complete set of parsed ROS2 command-line arguments
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ros2Args {
    /// Name remapping rules
    pub remap_rules: Vec<RemapRule>,
    /// Parameter assignments
    pub param_assignments: Vec<ParamAssignment>,
    /// Parameter files to load
    pub param_files: Vec<PathBuf>,
    /// Log level assignments
    pub log_levels: Vec<LogLevelAssignment>,
    /// Log configuration file
    pub log_config_file: Option<PathBuf>,
    /// Logging output configuration
    pub logging_output: LoggingOutputConfig,
    /// Enclave path for security
    pub enclave: Option<String>,
}

impl Ros2Args {
    /// Create a new empty `Ros2Args`
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse ROS2 arguments from command-line arguments (typically from `std::env::args()`)
    ///
    /// This is a convenience method that calls the parser directly with the provided arguments.
    /// Multiple `--ros-args` sections are supported and will be merged into a single `Ros2Args` structure.
    ///
    /// # Arguments
    ///
    /// * `args` - Command-line arguments as an iterator of strings
    ///
    /// # Returns
    ///
    /// Returns a tuple of `(Ros2Args, Vec<String>)` where:
    /// - `Ros2Args` contains all parsed ROS2 arguments
    /// - `Vec<String>` contains remaining user-defined arguments
    ///
    /// # Errors
    ///
    /// Returns an error if any ROS2 argument is malformed or invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use ros2args::Ros2Args;
    ///
    /// let args = vec![
    ///     "my_program".to_string(),
    ///     "--ros-args".to_string(),
    ///     "-r".to_string(),
    ///     "foo:=bar".to_string(),
    ///     "-p".to_string(),
    ///     "use_sim_time:=true".to_string(),
    /// ];
    ///
    /// let (ros_args, user_args) = Ros2Args::from_args(&args)?;
    /// assert_eq!(ros_args.remap_rules.len(), 1);
    /// assert_eq!(ros_args.param_assignments.len(), 1);
    /// # Ok::<(), ros2args::Ros2ArgsError>(())
    /// ```
    pub fn from_args<I, S>(args: I) -> crate::Ros2ArgsResult<(Self, Vec<String>)>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let args_vec: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
        crate::parse_ros2_args(&args_vec)
    }

    /// Parse ROS2 arguments from the current process's command-line arguments
    ///
    /// This is a convenience method that reads from `std::env::args()` and parses ROS2 arguments.
    /// Only the parsed ROS2 arguments are returned; user arguments are discarded.
    ///
    /// # Returns
    ///
    /// Returns the parsed `Ros2Args` structure.
    ///
    /// # Errors
    ///
    /// Returns an error if any ROS2 argument is malformed or invalid.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ros2args::Ros2Args;
    ///
    /// // Parse arguments from std::env::args()
    /// let ros_args = Ros2Args::from_env()?;
    ///
    /// println!("ROS2 remapping rules: {:?}", ros_args.remap_rules);
    /// # Ok::<(), ros2args::Ros2ArgsError>(())
    /// ```
    pub fn from_env() -> crate::Ros2ArgsResult<Self> {
        let args: Vec<String> = std::env::args().collect();
        let (ros_args, _user_args) = crate::parse_ros2_args(&args)?;
        Ok(ros_args)
    }

    /// Get all remapping rules that apply to a specific node
    #[must_use]
    pub fn get_remap_rules_for_node(&self, node_name: &str) -> Vec<&RemapRule> {
        self.remap_rules
            .iter()
            .filter(|r| r.applies_to_node(node_name))
            .collect()
    }

    /// Get all parameter assignments that apply to a specific node
    ///
    /// This includes both command-line parameter assignments and parameters from YAML files.
    /// Returns an error if any parameter file cannot be parsed.
    ///
    /// # Errors
    ///
    /// Returns an error if any parameter file cannot be read or parsed.
    pub fn get_params_for_node(
        &self,
        node_name: &str,
    ) -> crate::Ros2ArgsResult<Vec<ParamAssignment>> {
        let mut params = Vec::new();

        // Add command-line parameter assignments
        params.extend(
            self.param_assignments
                .iter()
                .filter(|p| p.applies_to_node(node_name))
                .cloned(),
        );

        // Parse and add parameters from YAML files
        for param_file in &self.param_files {
            let file_params = crate::param_file::parse_param_file(param_file)?;
            params.extend(
                file_params
                    .into_iter()
                    .filter(|p| p.applies_to_node(node_name)),
            );
        }

        Ok(params)
    }

    /// Merge another `Ros2Args` into this one
    pub fn merge(&mut self, other: Ros2Args) {
        self.remap_rules.extend(other.remap_rules);
        self.param_assignments.extend(other.param_assignments);
        self.param_files.extend(other.param_files);
        self.log_levels.extend(other.log_levels);
        if other.log_config_file.is_some() {
            self.log_config_file = other.log_config_file;
        }
        if other.logging_output.rosout.is_some() {
            self.logging_output.rosout = other.logging_output.rosout;
        }
        if other.logging_output.stdout.is_some() {
            self.logging_output.stdout = other.logging_output.stdout;
        }
        if other.logging_output.external_lib.is_some() {
            self.logging_output.external_lib = other.logging_output.external_lib;
        }
        if other.enclave.is_some() {
            self.enclave = other.enclave;
        }
    }
}
