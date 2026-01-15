# OxidROS Roadmap

This document outlines planned features and improvements for the OxidROS project.

## Planned Features

### 1. Actions for oxidros-zenoh

Add action support to the Zenoh backend, enabling goal-based long-running operations over Zenoh transport.

### 2. Managed Node (Lifecycle Node)

Implement ROS2 Managed Node support for both RCL and Zenoh backends.

A managed node follows a state machine with well-defined transitions:
- **Unconfigured** → **Inactive** → **Active** → **Finalized**

Reference: [ROS2 Node Lifecycle Design](https://design.ros2.org/articles/node_lifecycle.html)

Key states:
- `Unconfigured` - Node is created but not configured
- `Inactive` - Node is configured but not processing
- `Active` - Node is fully operational
- `Finalized` - Node is being destroyed

Transitions:
- `configure()` - Unconfigured → Inactive
- `activate()` - Inactive → Active  
- `deactivate()` - Active → Inactive
- `cleanup()` - Inactive → Unconfigured
- `shutdown()` - Any → Finalized

### 3. Parameter CLI Tool for Zenoh

Create a Rust-based CLI tool (`ros2-zenoh-param` or similar) for parameter management over Zenoh:

```bash
# List all parameters on a node
ros2-zenoh param list /my_node

# Get a parameter value
ros2-zenoh param get /my_node my_parameter

# Set a parameter value
ros2-zenoh param set /my_node my_parameter 42

# Dump all parameters to YAML
ros2-zenoh param dump /my_node
```

This enables parameter introspection and configuration for nodes running on the Zenoh backend without requiring the ROS2 RCL layer.
