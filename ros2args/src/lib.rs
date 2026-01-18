#![deny(
    unsafe_code,
    unused_must_use,
    unreachable_pub,
    rust_2018_idioms,
    missing_docs,
    clippy::pedantic
)]

//! ROS2 Command-Line Arguments Parser
//!
//! This module provides a comprehensive parser for ROS2 command-line arguments
//! based on the [ROS2 Command Line Arguments specification](https://design.ros2.org/articles/ros_command_line_arguments.html).
//!
//! # Features
//!
//! - **Name remapping**: Parse `--remap` / `-r` arguments for topic/service/node remapping
//! - **Parameter assignment**: Parse `--param` / `-p` arguments for single parameter assignments
//! - **Parameter files**: Parse `--params-file` arguments and load YAML parameter files
//! - **Logging configuration**: Parse log levels, log config files, and logging output flags
//! - **Enclave assignment**: Parse `--enclave` / `-e` arguments for security enclaves
//! - **Wildcard support**: Support wildcard patterns in parameter files (`*`, `**`)
//! - **Multiple ROS args sections**: Handle multiple `--ros-args` sections in the same command line
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```
//! use ros2args::parse_ros2_args;
//!
//! let args = vec![
//!     "my_program".to_string(),
//!     "--ros-args".to_string(),
//!     "-r".to_string(),
//!     "old_topic:=new_topic".to_string(),
//!     "-p".to_string(),
//!     "use_sim_time:=true".to_string(),
//!     "--log-level".to_string(),
//!     "DEBUG".to_string(),
//! ];
//!
//! let (ros_args, user_args) = parse_ros2_args(&args)?;
//!
//! assert_eq!(ros_args.remap_rules.len(), 1);
//! assert_eq!(ros_args.param_assignments.len(), 1);
//! assert_eq!(ros_args.log_levels.len(), 1);
//! # Ok::<(), ros2args::Ros2ArgsError>(())
//! ```
//!
//! ## Node-Specific Arguments
//!
//! ```
//! use ros2args::parse_ros2_args;
//!
//! let args = vec![
//!     "my_program".to_string(),
//!     "--ros-args".to_string(),
//!     "-r".to_string(),
//!     "my_node:old_topic:=new_topic".to_string(),
//!     "-p".to_string(),
//!     "my_node:param_name:=42".to_string(),
//! ];
//!
//! let (ros_args, _) = parse_ros2_args(&args)?;
//!
//! // Get remapping rules for a specific node
//! let node_remaps = ros_args.get_remap_rules_for_node("my_node");
//! assert_eq!(node_remaps.len(), 1);
//!
//! // Get parameters for a specific node
//! let node_params = ros_args.get_params_for_node("my_node")?;
//! assert_eq!(node_params.len(), 1);
//! # Ok::<(), ros2args::Ros2ArgsError>(())
//! ```
//!
//! ## Parameter Files
//!
//! ```no_run
//! use ros2args::parse_param_file;
//! use std::path::Path;
//!
//! // Parse a YAML parameter file
//! let params = parse_param_file(Path::new("config/params.yaml"))?;
//!
//! for param in params {
//!     // Access typed values
//!     if let Some(b) = param.as_bool() {
//!         println!("Node: {:?}, Param: {} (bool) = {}", param.node_name, param.name, b);
//!     } else if let Some(i) = param.as_i64() {
//!         println!("Node: {:?}, Param: {} (int) = {}", param.node_name, param.name, i);
//!     }
//! }
//! # Ok::<(), ros2args::Ros2ArgsError>(())
//! ```
//!
//! ## Multiple ROS Args Sections
//!
//! ```
//! use ros2args::parse_ros2_args;
//!
//! let args = vec![
//!     "program".to_string(),
//!     "--ros-args".to_string(),
//!     "-r".to_string(),
//!     "foo:=bar".to_string(),
//!     "--".to_string(),
//!     "--user-arg".to_string(),
//!     "--ros-args".to_string(),
//!     "-p".to_string(),
//!     "param:=value".to_string(),
//! ];
//!
//! let (ros_args, user_args) = parse_ros2_args(&args)?;
//!
//! // Both sections are merged
//! assert_eq!(ros_args.remap_rules.len(), 1);
//! assert_eq!(ros_args.param_assignments.len(), 1);
//! assert_eq!(user_args, vec!["program", "--user-arg"]);
//! # Ok::<(), ros2args::Ros2ArgsError>(())
//! ```

mod errors;
pub mod names;
mod param_file;
mod parser;
mod types;

pub use errors::{Ros2ArgsError, Ros2ArgsResult};
pub use names::{
    NameKind, build_node_fqn, expand_topic_name, expand_topic_name_with_fqn, extract_base_name,
    extract_namespace, is_absolute_name, is_hidden_name, is_private_name, is_relative_name,
    is_valid_name_char, is_valid_topic_char, validate_fully_qualified_name, validate_namespace,
    validate_node_name, validate_substitution, validate_topic_name,
};
pub use param_file::{match_wildcard_pattern, parse_param_file};
pub use parser::parse_ros2_args;
pub use types::{
    LogLevel, LogLevelAssignment, LoggingOutputConfig, ParamAssignment, RemapRule, Ros2Args,
};
