//! # Oxidros - Rust bindings for ROS2
//!
//! `oxidros` is a unified ROS2 library for Rust, providing a consistent API
//! with multiple backend implementations:
//!
//! - **RCL Backend** (`rcl` feature): FFI bindings to the official ROS2 C library.
//!   Requires a ROS2 installation (Humble, Jazzy, or Kilted).
//!
//! - **Zenoh Backend** (`zenoh` feature): Pure Rust implementation using Zenoh middleware.
//!   Compatible with `rmw_zenoh_cpp`. No ROS2 installation required at runtime.
//!
//! # Feature Flags
//!
//! Choose exactly one backend by enabling one of these features:
//!
//! | Feature | Backend | Requirements |
//! |---------|---------|--------------|
//! | `humble` | RCL | ROS2 Humble installation |
//! | `jazzy` | RCL | ROS2 Jazzy installation |
//! | `kilted` | RCL | ROS2 Kilted installation |
//! | `zenoh` | Zenoh | None (pure Rust) |
//!
//! # Quick Start
//!
//! Add to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! oxidros = { version = "0.1", features = ["jazzy"] }  # or "zenoh"
//! tokio = { version = "1", features = ["full"] }
//! ```
//!
//! # Usage Patterns
//!
//! Oxidros supports two execution patterns:
//!
//! ## Async/Await Pattern (Recommended)
//!
//! The async pattern uses tokio for concurrent message handling:
//!
//! ```ignore
//! use oxidros::prelude::*;
//! use oxidros::msg::common_interfaces::std_msgs;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), DynError> {
//!     init_ros_logging("my_node");
//!
//!     let ctx = Context::new()?;
//!     let node = ctx.new_node("my_node", None)?;
//!
//!     // Create publisher
//!     let publisher = node.create_publisher::<std_msgs::msg::String>("chatter", None)?;
//!
//!     // Create subscriber
//!     let mut subscriber = node.create_subscriber::<std_msgs::msg::String>("chatter", None)?;
//!
//!     // Publish a message
//!     let mut msg = std_msgs::msg::String::new();
//!     msg.data.set_string("Hello, World!");
//!     publisher.publish(&msg)?;
//!
//!     // Receive messages asynchronously
//!     let received = subscriber.recv().await?;
//!     tracing::info!("Received: {}", received.sample.data.get_string());
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Selector Pattern (Callback-based)
//!
//! The selector pattern uses callbacks for event-driven programming:
//!
//! ```ignore
//! use oxidros::prelude::*;
//! use oxidros::msg::common_interfaces::std_msgs;
//! use std::time::Duration;
//!
//! fn main() -> Result<(), DynError> {
//!     init_ros_logging("my_node");
//!
//!     let ctx = Context::new()?;
//!     let node = ctx.new_node("my_node", None)?;
//!     let mut selector = ctx.new_selector()?;
//!
//!     // Add subscriber with callback
//!     let subscriber = node.create_subscriber::<std_msgs::msg::String>("chatter", None)?;
//!     selector.add_subscriber(subscriber, Box::new(|msg| {
//!         tracing::info!("Received: {}", msg.sample.data.get_string());
//!     }));
//!
//!     // Add timer
//!     selector.add_wall_timer("timer", Duration::from_secs(1), Box::new(|| {
//!         tracing::info!("Timer fired!");
//!     }));
//!
//!     // Event loop
//!     loop {
//!         selector.wait()?;
//!     }
//! }
//! ```
//!
//! # Services
//!
//! Service clients and servers use async/await:
//!
//! ```ignore
//! use oxidros::prelude::*;
//! use oxidros::msg::common_interfaces::example_interfaces;
//!
//! type AddTwoInts = example_interfaces::srv::AddTwoInts;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), DynError> {
//!     let ctx = Context::new()?;
//!     let node = ctx.new_node("my_node", None)?;
//!
//!     // Client
//!     let mut client = node.create_client::<AddTwoInts>("add_two_ints", None)?;
//!     let mut request = example_interfaces::srv::AddTwoInts_Request::new();
//!     request.a = 1;
//!     request.b = 2;
//!     let response = client.call(&request).await?;
//!     println!("Sum: {}", response.sample.sum);
//!
//!     Ok(())
//! }
//! ```
//!
//! # Parameters
//!
//! Parameter servers can be used with the selector pattern:
//!
//! ```ignore
//! use oxidros::prelude::*;
//!
//! fn main() -> Result<(), DynError> {
//!     let ctx = Context::new()?;
//!     let node = ctx.new_node("my_node", None)?;
//!     let param_server = node.create_parameter_server()?;
//!
//!     // Set initial parameters (name, value, read_only, description)
//!     {
//!         let mut params = param_server.params.write();
//!         params.set_parameter(
//!             "rate".to_string(),
//!             ParameterValue::F64(1.0),
//!             false,
//!             Some("Update rate".to_string()),
//!         )?;
//!     }
//!
//!     // Add to selector with update callback
//!     let mut selector = ctx.new_selector()?;
//!     selector.add_parameter_server(param_server, Box::new(|_params, updated| {
//!         for name in updated {
//!             tracing::info!("Parameter '{}' updated", name);
//!         }
//!     }));
//!
//!     loop { selector.wait()?; }
//! }
//! ```
//!
//! # Logging
//!
//! Oxidros uses the `tracing` ecosystem for logging:
//!
//! ```ignore
//! use oxidros::prelude::*;
//!
//! fn main() {
//!     init_ros_logging("my_node");
//!
//!     tracing::info!("Node started");
//!     tracing::debug!("Debug message");
//!     tracing::warn!("Warning!");
//!     tracing::error!("Error occurred");
//! }
//! ```
//!
//! # Crate Structure
//!
//! - [`oxidros`](crate) - This unified API crate (use in applications)
//! - `oxidros-rcl` - RCL backend implementation
//! - `oxidros-zenoh` - Zenoh backend implementation
//! - `oxidros-core` - Shared types and traits
//! - `oxidros-msg` - ROS2 message type generation
//! - `ros2-types` - CDR serialization and type traits

// Compile-time check: ensure exactly one backend is selected
#[cfg(all(feature = "rcl", feature = "zenoh"))]
compile_error!("Features `rcl` and `zenoh` are mutually exclusive. Choose one backend.");

#[cfg(not(any(feature = "rcl", feature = "zenoh")))]
compile_error!("No backend selected. Enable one of: `humble`, `jazzy`, `kilted`, or `zenoh`.");

// Prelude module for convenient imports
pub mod prelude;

// Re-export the selected backend
#[cfg(all(feature = "rcl", not(feature = "zenoh")))]
pub use oxidros_rcl::{self, action, clock, logger, service, topic};

#[cfg(all(feature = "zenoh", not(feature = "rcl")))]
pub use oxidros_zenoh;

// Always re-export core types and traits
pub use oxidros_core::{self, error};

// Re-export message types
pub use oxidros_msg::{self, msg};
pub mod qos {
    pub use oxidros_core::qos::*;
}
