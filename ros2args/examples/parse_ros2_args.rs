//! Example demonstrating ROS2 command-line arguments parsing
//!
//! This example shows how to parse ROS2 arguments from the command line
//! and work with parameter files.

use ros2args::{LogLevel, parse_param_file, parse_ros2_args};
use std::env;
use std::io::Write;
use tempfile::NamedTempFile;
use yaml_rust2::Yaml;

/// Helper function to print YAML values with type information
fn print_yaml_value(value: &Yaml) {
    match value {
        Yaml::Boolean(b) => println!("{} (bool)", b),
        Yaml::Integer(i) => println!("{} (int)", i),
        Yaml::Real(f) => println!("{} (float)", f),
        Yaml::String(s) => println!("\"{}\" (string)", s),
        Yaml::Array(arr) => println!("{:?} (array)", arr),
        Yaml::Hash(hash) => println!("{:?} (hash)", hash),
        Yaml::Null => println!("null"),
        _ => println!("{:?}", value),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== ROS2 Command-Line Arguments Parser Example ===\n");

    // Example 1: Parse command-line arguments
    println!("Example 1: Parsing command-line arguments");
    println!("------------------------------------------");

    let example_args = vec![
        "my_node".to_string(),
        "--user-defined-arg".to_string(),
        "value1".to_string(),
        "--ros-args".to_string(),
        "-r".to_string(),
        "old_topic:=/new_topic".to_string(),
        "-r".to_string(),
        "my_node:another_topic:=/remapped_topic".to_string(),
        "-p".to_string(),
        "use_sim_time:=true".to_string(),
        "-p".to_string(),
        "my_node:rate:=10".to_string(),
        "--log-level".to_string(),
        "INFO".to_string(),
        "--log-level".to_string(),
        "rclcpp:=DEBUG".to_string(),
        "--enable-rosout-logs".to_string(),
        "--disable-stdout-logs".to_string(),
        "-e".to_string(),
        "/my/enclave".to_string(),
        "--".to_string(),
        "--more-user-args".to_string(),
    ];

    let (ros_args, user_args) = parse_ros2_args(&example_args)?;

    println!("User-defined arguments: {:?}", user_args);
    println!("\nParsed ROS2 arguments:");

    println!("\n  Remapping rules:");
    for rule in &ros_args.remap_rules {
        match &rule.node_name {
            Some(node) => println!("    [Node: {}] {} -> {}", node, rule.from, rule.to),
            None => println!("    [Global] {} -> {}", rule.from, rule.to),
        }
    }

    println!("\n  Parameter assignments:");
    for param in &ros_args.param_assignments {
        match &param.node_name {
            Some(node) => {
                print!("    [Node: {}] {} = ", node, param.name);
                print_yaml_value(param.value());
            }
            None => {
                print!("    [Global] {} = ", param.name);
                print_yaml_value(param.value());
            }
        }
    }

    println!("\n  Log levels:");
    for log_level in &ros_args.log_levels {
        match &log_level.logger_name {
            Some(logger) => println!("    [Logger: {}] {}", logger, log_level.level.as_str()),
            None => println!("    [Global] {}", log_level.level.as_str()),
        }
    }

    println!("\n  Logging output configuration:");
    println!("    Rosout: {:?}", ros_args.logging_output.rosout);
    println!("    Stdout: {:?}", ros_args.logging_output.stdout);
    println!(
        "    External lib: {:?}",
        ros_args.logging_output.external_lib
    );

    if let Some(enclave) = &ros_args.enclave {
        println!("\n  Enclave: {}", enclave);
    }

    // Example 2: Working with node-specific arguments
    println!("\n\nExample 2: Getting node-specific arguments");
    println!("-------------------------------------------");

    let node_name = "my_node";
    println!("Arguments for node '{}':", node_name);

    let node_remaps = ros_args.get_remap_rules_for_node(node_name);
    println!("\n  Remapping rules:");
    for rule in node_remaps {
        println!("    {} -> {}", rule.from, rule.to);
    }

    let node_params = ros_args.get_params_for_node(node_name)?;
    println!("\n  Parameters:");
    for param in &node_params {
        print!("    {} = ", param.name);
        print_yaml_value(param.value());
    }

    // Example 3: Parse a parameter file
    println!("\n\nExample 3: Parsing a parameter file");
    println!("------------------------------------");

    // Create a temporary parameter file
    let yaml_content = r#"
# ROS2 parameter file example
robot_controller:
  ros__parameters:
    use_sim_time: true
    max_speed: 1.5
    min_speed: 0.1
    control_frequency: 50

sensor_node:
  ros__parameters:
    topic_name: "/sensors/camera"
    frame_rate: 30
    enable_compression: false

# Wildcard example - applies to all nodes
/**:
  ros__parameters:
    global_timeout: 5.0

# Wildcard example - applies to all nodes in /navigation namespace
/navigation/*:
  ros__parameters:
    planner_frequency: 10.0
"#;

    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(yaml_content.as_bytes())?;
    temp_file.flush()?;

    println!("Parameter file content:");
    println!("{}", yaml_content);

    let params = parse_param_file(temp_file.path())?;

    println!("\nParsed parameters:");
    for param in &params {
        match &param.node_name {
            Some(node) => {
                print!("  [{}] {} = ", node, param.name);
                print_yaml_value(param.value());
            }
            None => {
                print!("  [Global] {} = ", param.name);
                print_yaml_value(param.value());
            }
        }
    }

    // Example 4: LogLevel usage
    println!("\n\nExample 4: Working with log levels");
    println!("-----------------------------------");

    let log_levels = vec!["DEBUG", "INFO", "WARN", "ERROR", "FATAL"];
    for level_str in log_levels {
        let level = level_str.parse::<LogLevel>()?;
        println!("  {} -> {:?} -> {}", level_str, level, level.as_str());
    }

    // Example 5: Multiple ROS args sections
    println!("\n\nExample 5: Multiple --ros-args sections");
    println!("----------------------------------------");

    let multi_section_args = vec![
        "program".to_string(),
        "--ros-args".to_string(),
        "-r".to_string(),
        "topic1:=/remapped1".to_string(),
        "--".to_string(),
        "--user-arg1".to_string(),
        "--ros-args".to_string(),
        "-p".to_string(),
        "param1:=value1".to_string(),
        "--".to_string(),
        "--user-arg2".to_string(),
    ];

    let (ros_args2, user_args2) = parse_ros2_args(&multi_section_args)?;

    println!("User arguments: {:?}", user_args2);
    println!("Remapping rules: {}", ros_args2.remap_rules.len());
    println!(
        "Parameter assignments: {}",
        ros_args2.param_assignments.len()
    );

    // Example 6: Using actual command-line args (if provided)
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        println!("\n\nExample 6: Parsing actual command-line arguments");
        println!("-------------------------------------------------");
        println!("Provided args: {:?}", args);

        match parse_ros2_args(&args) {
            Ok((ros_args, user_args)) => {
                println!("\nUser args: {:?}", user_args);
                println!("ROS2 remapping rules: {}", ros_args.remap_rules.len());
                println!("ROS2 parameters: {}", ros_args.param_assignments.len());
                println!("ROS2 log levels: {}", ros_args.log_levels.len());
            }
            Err(e) => {
                println!("\nError parsing arguments: {}", e);
            }
        }
    } else {
        println!("\n\nTip: Run this example with ROS2 arguments to see parsing in action!");
        println!("Example:");
        println!(
            "  cargo run --example parse_ros2_args -- --ros-args -r old:=new -p param:=42 --log-level DEBUG"
        );
    }

    println!("\n=== Example completed successfully! ===");
    Ok(())
}
