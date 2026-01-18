//! Parser for ROS2 command-line arguments

use std::path::PathBuf;
use yaml_rust2::YamlLoader;

use crate::{
    errors::{Ros2ArgsError, Ros2ArgsResult},
    types::{LogLevel, LogLevelAssignment, ParamAssignment, RemapRule, Ros2Args},
};

/// Parse ROS2 arguments from command-line arguments
///
/// This function extracts and parses all `--ros-args` sections from the provided
/// command-line arguments. Multiple `--ros-args` sections are supported and will
/// be merged into a single `Ros2Args` structure.
///
/// # Arguments
///
/// * `args` - Command-line arguments (typically from `std::env::args()`)
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
/// use ros2args::parse_ros2_args;
///
/// let args = vec![
///     "my_program".to_string(),
///     "--user-arg".to_string(),
///     "--ros-args".to_string(),
///     "-r".to_string(),
///     "foo:=bar".to_string(),
///     "-p".to_string(),
///     "use_sim_time:=true".to_string(),
///     "--".to_string(),
///     "--another-user-arg".to_string(),
/// ];
///
/// let (ros_args, user_args) = parse_ros2_args(&args)?;
/// assert_eq!(ros_args.remap_rules.len(), 1);
/// assert_eq!(ros_args.param_assignments.len(), 1);
/// assert_eq!(user_args, vec!["my_program", "--user-arg", "--another-user-arg"]);
/// # Ok::<(), ros2args::Ros2ArgsError>(())
/// ```
pub fn parse_ros2_args(args: &[String]) -> Ros2ArgsResult<(Ros2Args, Vec<String>)> {
    let mut ros_args = Ros2Args::new();
    let mut user_args = Vec::new();
    let mut i = 0;

    while i < args.len() {
        if args[i] == "--ros-args" {
            // Parse this ROS args section
            let (section_args, next_idx) = extract_ros_args_section(args, i + 1);
            let section_parsed = parse_ros_args_section(&section_args)?;
            ros_args.merge(section_parsed);
            i = next_idx;
        } else {
            // User-defined argument
            user_args.push(args[i].clone());
            i += 1;
        }
    }

    Ok((ros_args, user_args))
}

/// Extract a single `--ros-args` section
///
/// Returns the extracted arguments and the index of the next argument to process
fn extract_ros_args_section(args: &[String], start_idx: usize) -> (Vec<String>, usize) {
    let mut section_args = Vec::new();
    let mut i = start_idx;

    while i < args.len() {
        if args[i] == "--" {
            // End of ROS args section
            return (section_args, i + 1);
        }
        if args[i] == "--ros-args" {
            // Start of another ROS args section
            return (section_args, i);
        }
        section_args.push(args[i].clone());
        i += 1;
    }

    (section_args, i)
}

/// Parse a single ROS args section
fn parse_ros_args_section(args: &[String]) -> Ros2ArgsResult<Ros2Args> {
    let mut ros_args = Ros2Args::new();
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--remap" | "-r" => {
                i += 1;
                if i >= args.len() {
                    return Err(Ros2ArgsError::MissingArgumentValue("--remap".to_string()));
                }
                let rule = parse_remap_rule(&args[i])?;
                ros_args.remap_rules.push(rule);
            }
            "--param" | "-p" => {
                i += 1;
                if i >= args.len() {
                    return Err(Ros2ArgsError::MissingArgumentValue("--param".to_string()));
                }
                let param = parse_param_assignment(&args[i])?;
                ros_args.param_assignments.push(param);
            }
            "--params-file" => {
                i += 1;
                if i >= args.len() {
                    return Err(Ros2ArgsError::MissingArgumentValue(
                        "--params-file".to_string(),
                    ));
                }
                ros_args.param_files.push(PathBuf::from(&args[i]));
            }
            "--log-level" => {
                i += 1;
                if i >= args.len() {
                    return Err(Ros2ArgsError::MissingArgumentValue(
                        "--log-level".to_string(),
                    ));
                }
                let log_level = parse_log_level_assignment(&args[i])?;
                ros_args.log_levels.push(log_level);
            }
            "--log-config-file" => {
                i += 1;
                if i >= args.len() {
                    return Err(Ros2ArgsError::MissingArgumentValue(
                        "--log-config-file".to_string(),
                    ));
                }
                ros_args.log_config_file = Some(PathBuf::from(&args[i]));
            }
            "--enable-rosout-logs" => {
                ros_args.logging_output.rosout = Some(true);
            }
            "--disable-rosout-logs" => {
                ros_args.logging_output.rosout = Some(false);
            }
            "--enable-stdout-logs" => {
                ros_args.logging_output.stdout = Some(true);
            }
            "--disable-stdout-logs" => {
                ros_args.logging_output.stdout = Some(false);
            }
            "--enable-external-lib-logs" => {
                ros_args.logging_output.external_lib = Some(true);
            }
            "--disable-external-lib-logs" => {
                ros_args.logging_output.external_lib = Some(false);
            }
            "--enclave" | "-e" => {
                i += 1;
                if i >= args.len() {
                    return Err(Ros2ArgsError::MissingArgumentValue("--enclave".to_string()));
                }
                ros_args.enclave = Some(args[i].clone());
            }
            arg => {
                return Err(Ros2ArgsError::UnexpectedArgument(arg.to_string()));
            }
        }
        i += 1;
    }

    Ok(ros_args)
}

/// Parse a remapping rule from a string
///
/// Supports both formats:
/// - `from:=to` (global)
/// - `node:from:=to` (node-specific)
fn parse_remap_rule(s: &str) -> Ros2ArgsResult<RemapRule> {
    let parts: Vec<&str> = s.split(":=").collect();
    if parts.len() != 2 {
        return Err(Ros2ArgsError::InvalidRemapRule(s.to_string()));
    }

    let to = parts[1].to_string();
    let from_parts: Vec<&str> = parts[0].split(':').collect();

    if from_parts.len() == 2 {
        // Node-specific: node:from:=to
        Ok(RemapRule::new_node_specific(
            from_parts[0].to_string(),
            from_parts[1].to_string(),
            to,
        ))
    } else if from_parts.len() == 1 {
        // Global: from:=to
        Ok(RemapRule::new_global(from_parts[0].to_string(), to))
    } else {
        Err(Ros2ArgsError::InvalidRemapRule(s.to_string()))
    }
}

/// Parse a parameter assignment from a string
///
/// Supports both formats:
/// - `name:=value` (global)
/// - `node:name:=value` (node-specific)
///
/// The value is parsed as YAML to preserve type information.
fn parse_param_assignment(s: &str) -> Ros2ArgsResult<ParamAssignment> {
    let parts: Vec<&str> = s.split(":=").collect();
    if parts.len() != 2 {
        return Err(Ros2ArgsError::InvalidParamAssignment(s.to_string()));
    }

    // Parse the value as YAML to preserve type information
    let yaml_value = YamlLoader::load_from_str(parts[1])
        .map_err(|e| {
            Ros2ArgsError::InvalidYamlValue(parts[1].to_string(), format!("YAML parse error: {e}"))
        })?
        .into_iter()
        .next()
        .ok_or_else(|| {
            Ros2ArgsError::InvalidYamlValue(parts[1].to_string(), "Empty YAML value".to_string())
        })?;

    let name_parts: Vec<&str> = parts[0].split(':').collect();

    if name_parts.len() == 2 {
        // Node-specific: node:name:=value
        Ok(ParamAssignment::new_node_specific(
            name_parts[0].to_string(),
            name_parts[1].to_string(),
            yaml_value,
        ))
    } else if name_parts.len() == 1 {
        // Global: name:=value
        Ok(ParamAssignment::new_global(
            name_parts[0].to_string(),
            yaml_value,
        ))
    } else {
        Err(Ros2ArgsError::InvalidParamAssignment(s.to_string()))
    }
}

/// Parse a log level assignment from a string
///
/// Supports both formats:
/// - `LEVEL` (global)
/// - `logger:=LEVEL` (logger-specific)
fn parse_log_level_assignment(s: &str) -> Ros2ArgsResult<LogLevelAssignment> {
    if let Some((logger, level_str)) = s.split_once(":=") {
        // Logger-specific
        let level = level_str
            .parse::<LogLevel>()
            .map_err(|_| Ros2ArgsError::InvalidLogLevel(level_str.to_string()))?;
        Ok(LogLevelAssignment::new_logger_specific(
            logger.to_string(),
            level,
        ))
    } else {
        // Global
        let level = s
            .parse::<LogLevel>()
            .map_err(|_| Ros2ArgsError::InvalidLogLevel(s.to_string()))?;
        Ok(LogLevelAssignment::new_global(level))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_global_remap_rule() {
        let rule = parse_remap_rule("foo:=bar").unwrap();
        assert_eq!(rule.node_name, None);
        assert_eq!(rule.from, "foo");
        assert_eq!(rule.to, "bar");
    }

    #[test]
    fn test_parse_node_specific_remap_rule() {
        let rule = parse_remap_rule("my_node:foo:=bar").unwrap();
        assert_eq!(rule.node_name, Some("my_node".to_string()));
        assert_eq!(rule.from, "foo");
        assert_eq!(rule.to, "bar");
    }

    #[test]
    fn test_parse_global_param() {
        let param = parse_param_assignment("use_sim_time:=true").unwrap();
        assert_eq!(param.node_name, None);
        assert_eq!(param.name, "use_sim_time");
        assert_eq!(param.as_bool(), Some(true));
    }

    #[test]
    fn test_parse_node_specific_param() {
        let param = parse_param_assignment("my_node:param_name:=42").unwrap();
        assert_eq!(param.node_name, Some("my_node".to_string()));
        assert_eq!(param.name, "param_name");
        assert_eq!(param.as_i64(), Some(42));
    }

    #[test]
    fn test_parse_global_log_level() {
        let log = parse_log_level_assignment("DEBUG").unwrap();
        assert_eq!(log.logger_name, None);
        assert_eq!(log.level, LogLevel::Debug);
    }

    #[test]
    fn test_parse_logger_specific_log_level() {
        let log = parse_log_level_assignment("rclcpp:=WARN").unwrap();
        assert_eq!(log.logger_name, Some("rclcpp".to_string()));
        assert_eq!(log.level, LogLevel::Warn);
    }

    #[test]
    fn test_parse_complete_ros_args() {
        let args = vec![
            "my_program".to_string(),
            "--user-arg".to_string(),
            "--ros-args".to_string(),
            "-r".to_string(),
            "foo:=bar".to_string(),
            "-p".to_string(),
            "use_sim_time:=true".to_string(),
            "--log-level".to_string(),
            "DEBUG".to_string(),
            "--enable-rosout-logs".to_string(),
            "--".to_string(),
            "--another-user-arg".to_string(),
        ];

        let (ros_args, user_args) = parse_ros2_args(&args).unwrap();

        assert_eq!(ros_args.remap_rules.len(), 1);
        assert_eq!(ros_args.remap_rules[0].from, "foo");
        assert_eq!(ros_args.remap_rules[0].to, "bar");

        assert_eq!(ros_args.param_assignments.len(), 1);
        assert_eq!(ros_args.param_assignments[0].name, "use_sim_time");
        assert_eq!(ros_args.param_assignments[0].as_bool(), Some(true));

        assert_eq!(ros_args.log_levels.len(), 1);
        assert_eq!(ros_args.log_levels[0].level, LogLevel::Debug);

        assert_eq!(ros_args.logging_output.rosout, Some(true));

        assert_eq!(
            user_args,
            vec!["my_program", "--user-arg", "--another-user-arg"]
        );
    }

    #[test]
    fn test_multiple_ros_args_sections() {
        let args = vec![
            "program".to_string(),
            "--ros-args".to_string(),
            "-r".to_string(),
            "foo:=bar".to_string(),
            "--".to_string(),
            "--ros-args".to_string(),
            "-p".to_string(),
            "param:=value".to_string(),
        ];

        let (ros_args, _) = parse_ros2_args(&args).unwrap();

        assert_eq!(ros_args.remap_rules.len(), 1);
        assert_eq!(ros_args.param_assignments.len(), 1);
    }
}
