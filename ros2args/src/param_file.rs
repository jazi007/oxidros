//! Parser for ROS2 parameter YAML files

use std::fs;
use std::path::Path;

use yaml_rust2::{Yaml, YamlLoader};

use crate::{
    errors::{Ros2ArgsError, Ros2ArgsResult},
    types::ParamAssignment,
};

/// Parse a ROS2 parameter YAML file
///
/// The expected structure is:
/// ```yaml
/// node_name:
///   ros__parameters:
///     param1: value1
///     param2: value2
/// another_node:
///   ros__parameters:
///     param3: value3
/// ```
///
/// Wildcards are supported for node names and namespaces:
/// - `*` matches a single token delimited by slashes (`/`)
/// - `**` matches zero or more tokens delimited by slashes
///
/// # Arguments
///
/// * `path` - Path to the YAML parameter file
///
/// # Returns
///
/// Returns a vector of `ParamAssignment` objects
///
/// # Errors
///
/// Returns an error if:
/// - The file doesn't exist
/// - The file cannot be parsed as YAML
/// - The YAML structure is invalid
///
/// # Examples
///
/// ```no_run
/// use ros2args::parse_param_file;
/// use std::path::Path;
///
/// let params = parse_param_file(Path::new("params.yaml"))?;
/// for param in params {
///     // Access typed values
///     if let Some(b) = param.as_bool() {
///         println!("Node: {:?}, Param: {} = {}", param.node_name, param.name, b);
///     }
/// }
/// # Ok::<(), ros2args::Ros2ArgsError>(())
/// ```
pub fn parse_param_file<P: AsRef<Path>>(path: P) -> Ros2ArgsResult<Vec<ParamAssignment>> {
    let path_ref = path.as_ref();

    // Read file
    let content = fs::read_to_string(path_ref)
        .map_err(|_| Ros2ArgsError::ParamFileNotFound(path_ref.to_path_buf()))?;

    // Parse YAML
    let docs = YamlLoader::load_from_str(&content)
        .map_err(|e| Ros2ArgsError::ParamFileParseError(path_ref.to_path_buf(), e.to_string()))?;

    if docs.is_empty() {
        return Ok(Vec::new());
    }

    let doc = &docs[0];
    parse_yaml_params(doc)
}

/// Parse YAML document into parameter assignments
fn parse_yaml_params(doc: &Yaml) -> Ros2ArgsResult<Vec<ParamAssignment>> {
    let mut params = Vec::new();

    let root_hash = doc.as_hash().ok_or_else(|| {
        Ros2ArgsError::InvalidParamFileStructure("Root element must be a mapping/hash".to_string())
    })?;

    for (node_key, node_value) in root_hash {
        let node_name = node_key.as_str().ok_or_else(|| {
            Ros2ArgsError::InvalidParamFileStructure("Node name must be a string".to_string())
        })?;

        // Get ros__parameters section
        let node_hash = node_value.as_hash().ok_or_else(|| {
            Ros2ArgsError::InvalidParamFileStructure(format!(
                "Node '{node_name}' must be a mapping/hash"
            ))
        })?;

        let ros_params_key = Yaml::String("ros__parameters".to_string());
        let params_value = node_hash.get(&ros_params_key).ok_or_else(|| {
            Ros2ArgsError::InvalidParamFileStructure(format!(
                "Node '{node_name}' must have 'ros__parameters' section"
            ))
        })?;

        let params_hash = params_value.as_hash().ok_or_else(|| {
            Ros2ArgsError::InvalidParamFileStructure(format!(
                "ros__parameters in node '{node_name}' must be a mapping/hash"
            ))
        })?;

        // Extract parameters
        for (param_key, param_value) in params_hash {
            let param_name = param_key.as_str().ok_or_else(|| {
                Ros2ArgsError::InvalidParamFileStructure(
                    "Parameter name must be a string".to_string(),
                )
            })?;

            // Check if node_name is a wildcard pattern
            let node_name_opt = if is_wildcard_pattern(node_name) {
                // For wildcards, we could either:
                // 1. Store the pattern as-is for later matching
                // 2. Treat it as applying to all nodes (None)
                // We'll use option 1 and store the pattern
                Some(node_name.to_string())
            } else {
                Some(node_name.to_string())
            };

            params.push(ParamAssignment {
                node_name: node_name_opt,
                name: param_name.to_string(),
                value: param_value.clone(), // Clone the YAML value to preserve type
            });
        }
    }

    Ok(params)
}

/// Check if a string is a wildcard pattern
fn is_wildcard_pattern(s: &str) -> bool {
    s.contains('*')
}

/// Match a node name against a wildcard pattern
///
/// Supports:
/// - `*` matches a single token delimited by `/`
/// - `**` matches zero or more tokens delimited by `/`
///
/// # Examples
///
/// ```
/// # use ros2args::match_wildcard_pattern;
/// assert!(match_wildcard_pattern("/**", "/foo/bar/baz"));
/// assert!(match_wildcard_pattern("/foo/*", "/foo/bar"));
/// assert!(!match_wildcard_pattern("/foo/*", "/foo/bar/baz"));
/// assert!(match_wildcard_pattern("/**/node", "/foo/bar/node"));
/// ```
#[must_use]
pub fn match_wildcard_pattern(pattern: &str, node_name: &str) -> bool {
    let pattern_parts: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let name_parts: Vec<&str> = node_name.split('/').filter(|s| !s.is_empty()).collect();

    match_parts(&pattern_parts, &name_parts)
}

/// Recursively match pattern parts against name parts
fn match_parts(pattern_parts: &[&str], name_parts: &[&str]) -> bool {
    if pattern_parts.is_empty() && name_parts.is_empty() {
        return true;
    }

    if pattern_parts.is_empty() {
        return false;
    }

    match pattern_parts[0] {
        "**" => {
            // ** matches zero or more tokens
            // Try matching with zero tokens
            if match_parts(&pattern_parts[1..], name_parts) {
                return true;
            }
            // Try matching with one or more tokens
            for i in 1..=name_parts.len() {
                if match_parts(&pattern_parts[1..], &name_parts[i..]) {
                    return true;
                }
            }
            false
        }
        "*" => {
            // * matches exactly one token
            if name_parts.is_empty() {
                return false;
            }
            match_parts(&pattern_parts[1..], &name_parts[1..])
        }
        pattern => {
            // Literal match
            if name_parts.is_empty() || pattern != name_parts[0] {
                return false;
            }
            match_parts(&pattern_parts[1..], &name_parts[1..])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_simple_param_file() {
        let yaml_content = r#"
some_node:
  ros__parameters:
    use_sim_time: true
    max_speed: 10.5
    robot_name: "test_robot"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let params = parse_param_file(temp_file.path()).unwrap();

        assert_eq!(params.len(), 3);
        assert_eq!(params[0].node_name, Some("some_node".to_string()));
        assert_eq!(params[0].name, "use_sim_time");
        assert_eq!(params[0].as_bool(), Some(true));

        let max_speed = params.iter().find(|p| p.name == "max_speed").unwrap();
        assert_eq!(max_speed.as_f64(), Some(10.5));

        let robot_name = params.iter().find(|p| p.name == "robot_name").unwrap();
        assert_eq!(robot_name.as_str(), Some("test_robot"));
    }

    #[test]
    fn test_parse_multiple_nodes() {
        let yaml_content = r"
node1:
  ros__parameters:
    param1: value1
node2:
  ros__parameters:
    param2: value2
";

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let params = parse_param_file(temp_file.path()).unwrap();

        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_wildcard_pattern_matching() {
        assert!(match_wildcard_pattern("/**", "/foo/bar/baz"));
        assert!(match_wildcard_pattern("/foo/*", "/foo/bar"));
        assert!(!match_wildcard_pattern("/foo/*", "/foo/bar/baz"));
        assert!(match_wildcard_pattern("/**/node", "/foo/bar/node"));
        assert!(match_wildcard_pattern("/**/node", "/node"));
        assert!(match_wildcard_pattern("/*/node", "/foo/node"));
        assert!(!match_wildcard_pattern("/*/node", "/foo/bar/node"));
    }

    #[test]
    fn test_invalid_yaml_structure() {
        let yaml_content = r"
some_node:
  wrong_key:
    param1: value1
";

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = parse_param_file(temp_file.path());
        assert!(result.is_err());
    }
}
