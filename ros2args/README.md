# ROS2 Command-Line Arguments Parser

A standalone Rust crate providing a comprehensive implementation of the ROS2 command-line arguments specification as defined in the [ROS2 Command Line Arguments design document](https://design.ros2.org/articles/ros_command_line_arguments.html).

This crate is completely independent and can be used in any Rust project that needs to parse ROS2-style command-line arguments, regardless of whether you're building a full ROS2 node or just need compatible argument parsing.

## Features

- ✅ **Name remapping** - Parse `--remap` / `-r` arguments for topic/service/node remapping
- ✅ **Parameter assignment** - Parse `--param` / `-p` arguments for single parameter assignments
- ✅ **Parameter files** - Load and parse YAML parameter files with `--params-file`
- ✅ **Wildcard support** - Full wildcard pattern matching (`*`, `**`) in parameter files

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
ros2args = "0.1"
```

### Basic Example

```rust
use ros2args::parse_ros2_args;

let args = vec![
    "my_program".to_string(),
    "--ros-args".to_string(),
    "-r".to_string(),
    "old_topic:=/new_topic".to_string(),
    "-p".to_string(),
    "use_sim_time:=true".to_string(),
    "--log-level".to_string(),
    "DEBUG".to_string(),
];

let (ros_args, user_args) = parse_ros2_args(&args)?;

// Access parsed arguments
for rule in &ros_args.remap_rules {
    println!("Remap: {} -> {}", rule.from, rule.to);
}

for param in &ros_args.param_assignments {
    println!("Param: {} = {}", param.name, param.value);
}
```

### Node-Specific Arguments

```rust
use ros2args::parse_ros2_args;

let args = vec![
    "program".to_string(),
    "--ros-args".to_string(),
    "-r".to_string(),
    "my_node:old_topic:=/new_topic".to_string(),
    "-p".to_string(),
    "my_node:param:=42".to_string(),
];

let (ros_args, _) = parse_ros2_args(&args)?;

// Get arguments for a specific node
let node_remaps = ros_args.get_remap_rules_for_node("my_node");
let node_params = ros_args.get_params_for_node("my_node");
```

### Parameter Files

```rust
use ros2args::parse_param_file;
use std::path::Path;

// Parse a YAML parameter file
let params = parse_param_file(Path::new("config/params.yaml"))?;

for param in params {
    if let Some(node) = &param.node_name {
        println!("[{}] {} = {}", node, param.name, param.value);
    }
}
```

Example YAML parameter file:

```yaml
robot_controller:
  ros__parameters:
    use_sim_time: true
    max_speed: 1.5
    control_frequency: 50

# Wildcard - applies to all nodes
/**:
  ros__parameters:
    global_timeout: 5.0

# Wildcard - applies to all nodes in /navigation namespace
/navigation/*:
  ros__parameters:
    planner_frequency: 10.0
```

### Multiple ROS Args Sections

```rust
use ros2args::parse_ros2_args;

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
];

let (ros_args, user_args) = parse_ros2_args(&args)?;
// Both ROS args sections are merged
```

## Supported Arguments

### Name Remapping

```bash
--ros-args -r old_topic:=/new_topic              # Global remapping
--ros-args --remap my_node:old:=/new             # Node-specific remapping
```

### Parameter Assignment

```bash
--ros-args -p use_sim_time:=true                 # Global parameter
--ros-args --param my_node:rate:=10              # Node-specific parameter
```

### Parameter Files

```bash
--ros-args --params-file config/params.yaml
```

### Logging Configuration

```bash
--ros-args --log-level DEBUG                     # Global log level
--ros-args --log-level rclcpp:=WARN              # Logger-specific log level
--ros-args --log-config-file log.config          # External log config
--ros-args --enable-rosout-logs                  # Enable rosout logging
--ros-args --disable-stdout-logs                 # Disable stdout logging
--ros-args --enable-external-lib-logs            # Enable external lib logging
```

## Wildcard Patterns

Parameter files support wildcard patterns for node names:

- `*` - Matches a single token delimited by `/`
- `**` - Matches zero or more tokens delimited by `/`

Examples:

```yaml
# Matches all nodes
/**:
  ros__parameters:
    global_param: value

# Matches any node named 'controller' at any depth
/**/controller:
  ros__parameters:
    specific_param: value

# Matches any node directly under /robot namespace
/robot/*:
  ros__parameters:
    robot_param: value
```

## References

- [ROS2 Command Line Arguments Design](https://design.ros2.org/articles/ros_command_line_arguments.html)
- [ROS2 Parameter YAML File Format](https://docs.ros.org/en/rolling/Concepts/About-ROS-2-Parameters.html)
