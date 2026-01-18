//! Integration tests for ROS2 command-line arguments parser

use ros2args::{
    LogLevel, Ros2ArgsError, match_wildcard_pattern, parse_param_file, parse_ros2_args,
};
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_simple_remap() {
    let args = vec![
        "program".to_string(),
        "--ros-args".to_string(),
        "-r".to_string(),
        "old:=new".to_string(),
    ];

    let (ros_args, user_args) = parse_ros2_args(&args).unwrap();

    assert_eq!(ros_args.remap_rules.len(), 1);
    assert_eq!(ros_args.remap_rules[0].from, "old");
    assert_eq!(ros_args.remap_rules[0].to, "new");
    assert_eq!(ros_args.remap_rules[0].node_name, None);
    assert_eq!(user_args, vec!["program"]);
}

#[test]
fn test_node_specific_remap() {
    let args = vec![
        "program".to_string(),
        "--ros-args".to_string(),
        "--remap".to_string(),
        "my_node:old_topic:=/new_topic".to_string(),
    ];

    let (ros_args, _) = parse_ros2_args(&args).unwrap();

    assert_eq!(ros_args.remap_rules.len(), 1);
    assert_eq!(
        ros_args.remap_rules[0].node_name,
        Some("my_node".to_string())
    );
    assert_eq!(ros_args.remap_rules[0].from, "old_topic");
    assert_eq!(ros_args.remap_rules[0].to, "/new_topic");
}

#[test]
fn test_simple_param() {
    let args = vec![
        "program".to_string(),
        "--ros-args".to_string(),
        "-p".to_string(),
        "use_sim_time:=true".to_string(),
    ];

    let (ros_args, _) = parse_ros2_args(&args).unwrap();

    assert_eq!(ros_args.param_assignments.len(), 1);
    assert_eq!(ros_args.param_assignments[0].name, "use_sim_time");
    assert_eq!(ros_args.param_assignments[0].as_bool(), Some(true));
    assert_eq!(ros_args.param_assignments[0].node_name, None);
}

#[test]
fn test_node_specific_param() {
    let args = vec![
        "program".to_string(),
        "--ros-args".to_string(),
        "--param".to_string(),
        "my_node:max_speed:=10.5".to_string(),
    ];

    let (ros_args, _) = parse_ros2_args(&args).unwrap();

    assert_eq!(ros_args.param_assignments.len(), 1);
    assert_eq!(
        ros_args.param_assignments[0].node_name,
        Some("my_node".to_string())
    );
    assert_eq!(ros_args.param_assignments[0].name, "max_speed");
    assert_eq!(ros_args.param_assignments[0].as_f64(), Some(10.5));
}

#[test]
fn test_params_file() {
    let args = vec![
        "program".to_string(),
        "--ros-args".to_string(),
        "--params-file".to_string(),
        "/path/to/params.yaml".to_string(),
    ];

    let (ros_args, _) = parse_ros2_args(&args).unwrap();

    assert_eq!(ros_args.param_files.len(), 1);
    assert_eq!(
        ros_args.param_files[0].to_str().unwrap(),
        "/path/to/params.yaml"
    );
}

#[test]
fn test_global_log_level() {
    let args = vec![
        "program".to_string(),
        "--ros-args".to_string(),
        "--log-level".to_string(),
        "DEBUG".to_string(),
    ];

    let (ros_args, _) = parse_ros2_args(&args).unwrap();

    assert_eq!(ros_args.log_levels.len(), 1);
    assert_eq!(ros_args.log_levels[0].logger_name, None);
    assert_eq!(ros_args.log_levels[0].level, LogLevel::Debug);
}

#[test]
fn test_logger_specific_log_level() {
    let args = vec![
        "program".to_string(),
        "--ros-args".to_string(),
        "--log-level".to_string(),
        "rclcpp:=WARN".to_string(),
        "--log-level".to_string(),
        "my_logger:=ERROR".to_string(),
    ];

    let (ros_args, _) = parse_ros2_args(&args).unwrap();

    assert_eq!(ros_args.log_levels.len(), 2);
    assert_eq!(
        ros_args.log_levels[0].logger_name,
        Some("rclcpp".to_string())
    );
    assert_eq!(ros_args.log_levels[0].level, LogLevel::Warn);
    assert_eq!(
        ros_args.log_levels[1].logger_name,
        Some("my_logger".to_string())
    );
    assert_eq!(ros_args.log_levels[1].level, LogLevel::Error);
}

#[test]
fn test_log_config_file() {
    let args = vec![
        "program".to_string(),
        "--ros-args".to_string(),
        "--log-config-file".to_string(),
        "log.config".to_string(),
    ];

    let (ros_args, _) = parse_ros2_args(&args).unwrap();

    assert!(ros_args.log_config_file.is_some());
    assert_eq!(
        ros_args.log_config_file.unwrap().to_str().unwrap(),
        "log.config"
    );
}

#[test]
fn test_logging_output_flags() {
    let args = vec![
        "program".to_string(),
        "--ros-args".to_string(),
        "--enable-rosout-logs".to_string(),
        "--disable-stdout-logs".to_string(),
        "--enable-external-lib-logs".to_string(),
    ];

    let (ros_args, _) = parse_ros2_args(&args).unwrap();

    assert_eq!(ros_args.logging_output.rosout, Some(true));
    assert_eq!(ros_args.logging_output.stdout, Some(false));
    assert_eq!(ros_args.logging_output.external_lib, Some(true));
}

#[test]
fn test_enclave() {
    let args = vec![
        "program".to_string(),
        "--ros-args".to_string(),
        "-e".to_string(),
        "/foo/bar".to_string(),
    ];

    let (ros_args, _) = parse_ros2_args(&args).unwrap();

    assert_eq!(ros_args.enclave, Some("/foo/bar".to_string()));
}

#[test]
fn test_multiple_ros_args_sections() {
    let args = vec![
        "program".to_string(),
        "--ros-args".to_string(),
        "-r".to_string(),
        "foo:=bar".to_string(),
        "--".to_string(),
        "--user-arg".to_string(),
        "--ros-args".to_string(),
        "-p".to_string(),
        "param:=value".to_string(),
        "--".to_string(),
        "--another-user-arg".to_string(),
    ];

    let (ros_args, user_args) = parse_ros2_args(&args).unwrap();

    assert_eq!(ros_args.remap_rules.len(), 1);
    assert_eq!(ros_args.param_assignments.len(), 1);
    assert_eq!(
        user_args,
        vec!["program", "--user-arg", "--another-user-arg"]
    );
}

#[test]
fn test_complex_command_line() {
    let args = vec![
        "my_node".to_string(),
        "--user-defined".to_string(),
        "--ros-args".to_string(),
        "-r".to_string(),
        "old_topic:=new_topic".to_string(),
        "-r".to_string(),
        "my_node:another:=/remapped".to_string(),
        "-p".to_string(),
        "use_sim_time:=true".to_string(),
        "-p".to_string(),
        "my_node:rate:=10".to_string(),
        "--params-file".to_string(),
        "config.yaml".to_string(),
        "--log-level".to_string(),
        "INFO".to_string(),
        "--log-level".to_string(),
        "rclcpp:=DEBUG".to_string(),
        "--enable-rosout-logs".to_string(),
        "-e".to_string(),
        "/my/enclave".to_string(),
        "--".to_string(),
        "--more-user-args".to_string(),
    ];

    let (ros_args, user_args) = parse_ros2_args(&args).unwrap();

    assert_eq!(ros_args.remap_rules.len(), 2);
    assert_eq!(ros_args.param_assignments.len(), 2);
    assert_eq!(ros_args.param_files.len(), 1);
    assert_eq!(ros_args.log_levels.len(), 2);
    assert_eq!(ros_args.logging_output.rosout, Some(true));
    assert_eq!(ros_args.enclave, Some("/my/enclave".to_string()));
    assert_eq!(
        user_args,
        vec!["my_node", "--user-defined", "--more-user-args"]
    );
}

#[test]
fn test_empty_ros_args_section() {
    let args = vec![
        "program".to_string(),
        "--ros-args".to_string(),
        "--".to_string(),
    ];

    let (ros_args, user_args) = parse_ros2_args(&args).unwrap();

    assert_eq!(ros_args.remap_rules.len(), 0);
    assert_eq!(ros_args.param_assignments.len(), 0);
    assert_eq!(user_args, vec!["program"]);
}

#[test]
fn test_ros_args_without_double_dash() {
    let args = vec![
        "program".to_string(),
        "--ros-args".to_string(),
        "-r".to_string(),
        "foo:=bar".to_string(),
    ];

    let (ros_args, _) = parse_ros2_args(&args).unwrap();

    assert_eq!(ros_args.remap_rules.len(), 1);
}

#[test]
fn test_invalid_remap_rule() {
    let args = vec![
        "program".to_string(),
        "--ros-args".to_string(),
        "-r".to_string(),
        "invalid_format".to_string(),
    ];

    let result = parse_ros2_args(&args);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        Ros2ArgsError::InvalidRemapRule(_)
    ));
}

#[test]
fn test_invalid_param_assignment() {
    let args = vec![
        "program".to_string(),
        "--ros-args".to_string(),
        "-p".to_string(),
        "invalid".to_string(),
    ];

    let result = parse_ros2_args(&args);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        Ros2ArgsError::InvalidParamAssignment(_)
    ));
}

#[test]
fn test_invalid_log_level() {
    let args = vec![
        "program".to_string(),
        "--ros-args".to_string(),
        "--log-level".to_string(),
        "INVALID".to_string(),
    ];

    let result = parse_ros2_args(&args);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        Ros2ArgsError::InvalidLogLevel(_)
    ));
}

#[test]
fn test_missing_argument_value() {
    let args = vec![
        "program".to_string(),
        "--ros-args".to_string(),
        "-r".to_string(),
    ];

    let result = parse_ros2_args(&args);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        Ros2ArgsError::MissingArgumentValue(_)
    ));
}

#[test]
fn test_get_remap_rules_for_node() {
    let args = vec![
        "program".to_string(),
        "--ros-args".to_string(),
        "-r".to_string(),
        "global:=remapped".to_string(),
        "-r".to_string(),
        "node1:specific:=value".to_string(),
        "-r".to_string(),
        "node2:other:=value2".to_string(),
    ];

    let (ros_args, _) = parse_ros2_args(&args).unwrap();

    let node1_rules = ros_args.get_remap_rules_for_node("node1");
    assert_eq!(node1_rules.len(), 2); // global + node1-specific

    let node2_rules = ros_args.get_remap_rules_for_node("node2");
    assert_eq!(node2_rules.len(), 2); // global + node2-specific

    let node3_rules = ros_args.get_remap_rules_for_node("node3");
    assert_eq!(node3_rules.len(), 1); // only global
}

#[test]
fn test_get_params_for_node() {
    let args = vec![
        "program".to_string(),
        "--ros-args".to_string(),
        "-p".to_string(),
        "global_param:=1".to_string(),
        "-p".to_string(),
        "node1:specific_param:=2".to_string(),
    ];

    let (ros_args, _) = parse_ros2_args(&args).unwrap();

    let node1_params = ros_args.get_params_for_node("node1").unwrap();
    assert_eq!(node1_params.len(), 2); // global + node1-specific

    let node2_params = ros_args.get_params_for_node("node2").unwrap();
    assert_eq!(node2_params.len(), 1); // only global
}

#[test]
fn test_get_params_for_node_with_yaml_file() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a temporary YAML file
    let yaml_content = r#"
node1:
  ros__parameters:
    yaml_param1: 100
    yaml_param2: "from_yaml"
node2:
  ros__parameters:
    yaml_param3: true
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    // Parse command-line args with YAML file reference
    let args = vec![
        "program".to_string(),
        "--ros-args".to_string(),
        "-p".to_string(),
        "global_param:=1".to_string(),
        "-p".to_string(),
        "node1:cmd_param:=2".to_string(),
        "--params-file".to_string(),
        temp_file.path().to_str().unwrap().to_string(),
    ];

    let (ros_args, _) = parse_ros2_args(&args).unwrap();

    // Test node1 - should get global + node-specific from cmd + YAML params
    let node1_params = ros_args.get_params_for_node("node1").unwrap();
    assert_eq!(node1_params.len(), 4); // global_param, cmd_param, yaml_param1, yaml_param2

    let param_names: Vec<_> = node1_params.iter().map(|p| p.name.as_str()).collect();
    assert!(param_names.contains(&"global_param"));
    assert!(param_names.contains(&"cmd_param"));
    assert!(param_names.contains(&"yaml_param1"));
    assert!(param_names.contains(&"yaml_param2"));

    // Test node2 - should get global from cmd + YAML params
    let node2_params = ros_args.get_params_for_node("node2").unwrap();
    assert_eq!(node2_params.len(), 2); // global_param, yaml_param3

    let param_names: Vec<_> = node2_params.iter().map(|p| p.name.as_str()).collect();
    assert!(param_names.contains(&"global_param"));
    assert!(param_names.contains(&"yaml_param3"));
}

// Parameter file tests

#[test]
fn test_parse_simple_param_file() {
    let yaml_content = r#"
some_node:
  ros__parameters:
    use_sim_time: true
    max_speed: 10.5
    robot_name: "test_robot"
    count: 42
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let params = parse_param_file(temp_file.path()).unwrap();

    assert_eq!(params.len(), 4);
    assert!(
        params
            .iter()
            .all(|p| p.node_name == Some("some_node".to_string()))
    );

    let use_sim_time = params.iter().find(|p| p.name == "use_sim_time").unwrap();
    assert_eq!(use_sim_time.as_bool(), Some(true));

    let max_speed = params.iter().find(|p| p.name == "max_speed").unwrap();
    assert_eq!(max_speed.as_f64(), Some(10.5));

    let robot_name = params.iter().find(|p| p.name == "robot_name").unwrap();
    assert_eq!(robot_name.as_str(), Some("test_robot"));

    let count = params.iter().find(|p| p.name == "count").unwrap();
    assert_eq!(count.as_i64(), Some(42));
}

#[test]
fn test_parse_multiple_nodes_param_file() {
    let yaml_content = r#"
node1:
  ros__parameters:
    param1: value1
    param2: 123
node2:
  ros__parameters:
    param3: true
    param4: 45.67
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let params = parse_param_file(temp_file.path()).unwrap();

    assert_eq!(params.len(), 4);

    let node1_params: Vec<_> = params
        .iter()
        .filter(|p| p.node_name == Some("node1".to_string()))
        .collect();
    assert_eq!(node1_params.len(), 2);

    let node2_params: Vec<_> = params
        .iter()
        .filter(|p| p.node_name == Some("node2".to_string()))
        .collect();
    assert_eq!(node2_params.len(), 2);
}

#[test]
fn test_parse_wildcard_param_file() {
    let yaml_content = r#"
/**:
  ros__parameters:
    global_param: global_value
/foo/*:
  ros__parameters:
    namespace_param: namespace_value
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let params = parse_param_file(temp_file.path()).unwrap();

    assert_eq!(params.len(), 2);
    assert!(
        params
            .iter()
            .any(|p| p.node_name == Some("/**".to_string()))
    );
    assert!(
        params
            .iter()
            .any(|p| p.node_name == Some("/foo/*".to_string()))
    );
}

#[test]
fn test_wildcard_pattern_matching() {
    // Test /** (matches everything)
    assert!(match_wildcard_pattern("/**", "/foo"));
    assert!(match_wildcard_pattern("/**", "/foo/bar"));
    assert!(match_wildcard_pattern("/**", "/foo/bar/baz"));

    // Test /* (matches single level)
    assert!(match_wildcard_pattern("/*", "/foo"));
    assert!(!match_wildcard_pattern("/*", "/foo/bar"));

    // Test /foo/* (matches single level under /foo)
    assert!(match_wildcard_pattern("/foo/*", "/foo/bar"));
    assert!(!match_wildcard_pattern("/foo/*", "/foo/bar/baz"));
    assert!(!match_wildcard_pattern("/foo/*", "/bar/baz"));

    // Test /**/node (matches node at any depth)
    assert!(match_wildcard_pattern("/**/node", "/node"));
    assert!(match_wildcard_pattern("/**/node", "/foo/node"));
    assert!(match_wildcard_pattern("/**/node", "/foo/bar/node"));
    assert!(!match_wildcard_pattern("/**/node", "/foo/bar/other"));

    // Test /foo/**/bar (matches bar at any depth under /foo)
    assert!(match_wildcard_pattern("/foo/**/bar", "/foo/bar"));
    assert!(match_wildcard_pattern("/foo/**/bar", "/foo/baz/bar"));
    assert!(match_wildcard_pattern("/foo/**/bar", "/foo/baz/qux/bar"));
    assert!(!match_wildcard_pattern("/foo/**/bar", "/other/bar"));
}

#[test]
fn test_invalid_param_file_structure() {
    let yaml_content = r#"
some_node:
  wrong_key:
    param1: value1
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(yaml_content.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let result = parse_param_file(temp_file.path());
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        Ros2ArgsError::InvalidParamFileStructure(_)
    ));
}

#[test]
fn test_param_file_not_found() {
    let result = parse_param_file("/nonexistent/path/to/file.yaml");
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        Ros2ArgsError::ParamFileNotFound(_)
    ));
}

#[test]
fn test_log_level_string_conversion() {
    assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
    assert_eq!(LogLevel::Info.as_str(), "INFO");
    assert_eq!(LogLevel::Warn.as_str(), "WARN");
    assert_eq!(LogLevel::Error.as_str(), "ERROR");
    assert_eq!(LogLevel::Fatal.as_str(), "FATAL");

    assert_eq!("DEBUG".parse::<LogLevel>().unwrap(), LogLevel::Debug);
    assert_eq!("debug".parse::<LogLevel>().unwrap(), LogLevel::Debug);
    assert_eq!("INFO".parse::<LogLevel>().unwrap(), LogLevel::Info);
    assert_eq!("WARN".parse::<LogLevel>().unwrap(), LogLevel::Warn);
    assert_eq!("WARNING".parse::<LogLevel>().unwrap(), LogLevel::Warn);
    assert_eq!("ERROR".parse::<LogLevel>().unwrap(), LogLevel::Error);
    assert_eq!("FATAL".parse::<LogLevel>().unwrap(), LogLevel::Fatal);

    assert!("INVALID".parse::<LogLevel>().is_err());
}

#[test]
fn test_merge_ros_args() {
    let mut args1 = parse_ros2_args(&[
        "p".to_string(),
        "--ros-args".to_string(),
        "-r".to_string(),
        "a:=b".to_string(),
    ])
    .unwrap()
    .0;

    let args2 = parse_ros2_args(&[
        "p".to_string(),
        "--ros-args".to_string(),
        "-p".to_string(),
        "x:=1".to_string(),
    ])
    .unwrap()
    .0;

    args1.merge(args2);

    assert_eq!(args1.remap_rules.len(), 1);
    assert_eq!(args1.param_assignments.len(), 1);
}

// ==================== Name Validation Integration Tests ====================

use ros2args::{
    NameKind, is_absolute_name, is_hidden_name, is_private_name, is_relative_name,
    validate_fully_qualified_name, validate_namespace, validate_node_name, validate_topic_name,
};

#[test]
fn test_validate_topic_name_valid_cases() {
    // Basic valid names
    assert!(validate_topic_name("foo").is_ok());
    assert!(validate_topic_name("bar_baz").is_ok());
    assert!(validate_topic_name("CamelCase").is_ok());
    assert!(validate_topic_name("_private").is_ok());

    // Absolute paths
    assert!(validate_topic_name("/foo").is_ok());
    assert!(validate_topic_name("/foo/bar/baz").is_ok());
    assert!(validate_topic_name("/robot1/camera/image").is_ok());

    // Private namespace
    assert!(validate_topic_name("~").is_ok());
    assert!(validate_topic_name("~/foo").is_ok());
    assert!(validate_topic_name("~/foo/bar").is_ok());

    // Substitutions
    assert!(validate_topic_name("{node}").is_ok());
    assert!(validate_topic_name("{node}/topic").is_ok());
    assert!(validate_topic_name("topic/{ns}/data").is_ok());
}

#[test]
fn test_validate_topic_name_invalid_cases() {
    // Empty
    assert!(validate_topic_name("").is_err());

    // Starts with number
    assert!(validate_topic_name("123topic").is_err());
    assert!(validate_topic_name("/123topic").is_err());

    // Double slashes
    assert!(validate_topic_name("foo//bar").is_err());
    assert!(validate_topic_name("//foo").is_err());

    // Double underscores
    assert!(validate_topic_name("foo__bar").is_err());

    // Trailing slash
    assert!(validate_topic_name("foo/").is_err());
    assert!(validate_topic_name("/foo/bar/").is_err());

    // Invalid tilde usage
    assert!(validate_topic_name("~foo").is_err());
    assert!(validate_topic_name("foo~").is_err());
    assert!(validate_topic_name("/~").is_err());
    assert!(validate_topic_name("foo/~/bar").is_err());

    // Unbalanced braces
    assert!(validate_topic_name("{foo").is_err());
    assert!(validate_topic_name("foo}").is_err());

    // Invalid characters
    assert!(validate_topic_name("foo bar").is_err());
    assert!(validate_topic_name("foo-bar").is_err());
    assert!(validate_topic_name("foo.bar").is_err());
}

#[test]
fn test_validate_node_name_valid_cases() {
    assert!(validate_node_name("my_node").is_ok());
    assert!(validate_node_name("node123").is_ok());
    assert!(validate_node_name("MyNode").is_ok());
    assert!(validate_node_name("NODE").is_ok());
    assert!(validate_node_name("_private").is_ok());
    assert!(validate_node_name("a").is_ok());
}

#[test]
fn test_validate_node_name_invalid_cases() {
    // Empty
    assert!(validate_node_name("").is_err());

    // Starts with number
    assert!(validate_node_name("123node").is_err());

    // Contains slash
    assert!(validate_node_name("my/node").is_err());

    // Contains tilde
    assert!(validate_node_name("~node").is_err());

    // Contains braces
    assert!(validate_node_name("{node}").is_err());

    // Double underscores
    assert!(validate_node_name("my__node").is_err());

    // Invalid characters
    assert!(validate_node_name("my-node").is_err());
    assert!(validate_node_name("my.node").is_err());
}

#[test]
fn test_validate_namespace_valid_cases() {
    assert!(validate_namespace("/").is_ok());
    assert!(validate_namespace("/foo").is_ok());
    assert!(validate_namespace("/foo/bar").is_ok());
    assert!(validate_namespace("/my_namespace").is_ok());
    assert!(validate_namespace("/_private").is_ok());
}

#[test]
fn test_validate_namespace_invalid_cases() {
    // Empty
    assert!(validate_namespace("").is_err());

    // Not starting with slash
    assert!(validate_namespace("foo").is_err());

    // Trailing slash
    assert!(validate_namespace("/foo/").is_err());

    // Double slash
    assert!(validate_namespace("//foo").is_err());
    assert!(validate_namespace("/foo//bar").is_err());

    // Token starts with number
    assert!(validate_namespace("/123").is_err());
    assert!(validate_namespace("/foo/123bar").is_err());

    // Double underscores
    assert!(validate_namespace("/foo__bar").is_err());
}

#[test]
fn test_validate_fully_qualified_name_valid() {
    assert!(validate_fully_qualified_name("/foo").is_ok());
    assert!(validate_fully_qualified_name("/bar/baz").is_ok());
    assert!(validate_fully_qualified_name("/_private/thing").is_ok());
}

#[test]
fn test_validate_fully_qualified_name_invalid() {
    // Not absolute
    assert!(validate_fully_qualified_name("foo").is_err());

    // Contains tilde
    assert!(validate_fully_qualified_name("/~/foo").is_err());

    // Contains substitution
    assert!(validate_fully_qualified_name("/{sub}").is_err());
}

#[test]
fn test_name_classification_helpers() {
    // Relative names
    assert!(is_relative_name("foo"));
    assert!(is_relative_name("foo/bar"));
    assert!(!is_relative_name("/foo"));
    assert!(!is_relative_name("~"));
    assert!(!is_relative_name("~/foo"));

    // Absolute names
    assert!(is_absolute_name("/foo"));
    assert!(is_absolute_name("/foo/bar"));
    assert!(!is_absolute_name("foo"));
    assert!(!is_absolute_name("~"));

    // Private names
    assert!(is_private_name("~"));
    assert!(is_private_name("~/foo"));
    assert!(!is_private_name("/foo"));
    assert!(!is_private_name("foo"));

    // Hidden names
    assert!(is_hidden_name("_hidden"));
    assert!(is_hidden_name("/foo/_bar"));
    assert!(is_hidden_name("/_private/thing"));
    assert!(!is_hidden_name("foo"));
    assert!(!is_hidden_name("/foo/bar"));
}

#[test]
fn test_name_kind_display() {
    assert_eq!(format!("{}", NameKind::Topic), "topic");
    assert_eq!(format!("{}", NameKind::Node), "node");
    assert_eq!(format!("{}", NameKind::Namespace), "namespace");
    assert_eq!(format!("{}", NameKind::Substitution), "substitution");
}

#[test]
fn test_error_messages_contain_name() {
    let err = validate_topic_name("foo//bar").unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("foo//bar"));
    assert!(msg.contains("topic"));
    assert!(msg.contains("repeated forward slashes"));
}
