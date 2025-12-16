# Oxidros

Formally Specified Rust Bindings for ROS2

**Oxidros** (from *oxide* + *ROS*) is a fork of safe_drive with enhanced features and improvements.

## Key Differences from safe_drive

- **No custom ament cargo required**: Oxidros works with standard Rust tooling and doesn't require a custom ament build tool
- **Build-time message generation**: Message code generation is handled at compilation time using `build.rs`, simplifying the workflow
