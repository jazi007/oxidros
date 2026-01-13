//! Parameter server example
//!
//! Demonstrates using the parameter server with the selector pattern.
//!
//! Run with:
//! ```bash
//! cargo run -p simple --bin parameters --features jazzy
//! ```
//!
//! Interact with parameters:
//! ```bash
//! ros2 param list /param_demo
//! ros2 param get /param_demo rate
//! ros2 param set /param_demo rate 2.0
//! ros2 param describe /param_demo rate
//! ```

use oxidros::error::Result;
use oxidros::prelude::*;
use std::time::Duration;

fn main() -> Result<()> {
    // Initialize logging
    init_ros_logging("parameters");

    let ctx = Context::new()?;
    let node = ctx.create_node("param_demo", None)?;

    // Create parameter server
    let param_server = node.create_parameter_server()?;

    // Set initial parameters
    {
        let mut params = param_server.params.write();

        // Add string parameter (name, value, read_only, description)
        params.set_parameter(
            "name".to_string(),
            Value::String("demo_node".to_string()),
            false,
            Some("Node name".to_string()),
        )?;

        // Add numeric parameters
        params.set_parameter(
            "rate".to_string(),
            Value::F64(1.0),
            false,
            Some("Update rate in Hz".to_string()),
        )?;

        params.set_parameter(
            "max_count".to_string(),
            Value::I64(100),
            false,
            Some("Maximum count".to_string()),
        )?;

        // Add boolean parameter
        params.set_parameter(
            "enabled".to_string(),
            Value::Bool(true),
            false,
            Some("Whether the node is enabled".to_string()),
        )?;

        // Add array parameter
        params.set_parameter(
            "gains".to_string(),
            Value::VecF64(vec![1.0, 0.5, 0.1]),
            false,
            Some("PID gains".to_string()),
        )?;
    }

    // Create selector for event handling
    let mut selector = ctx.create_selector()?;
    let params_clone = param_server.params.clone();

    // Add parameter server to selector for handling get/set requests
    // The callback is invoked when parameters are updated
    selector.add_parameter_server(
        param_server,
        Box::new(|_params, updated| {
            for name in updated {
                tracing::info!("Parameter '{}' was updated", name);
            }
        }),
    );

    // Add timer to periodically check and use parameters
    /*
    selector.add_wall_timer(
        "param_check",
        Duration::from_secs(5),
        Box::new(move || {
            let params = params_clone.read();

            // Read parameters
            if let Some(rate) = params.get_parameter("rate")
                && let Value::F64(value) = rate.value
            {
                tracing::info!("Current rate: {}", value);
            }

            if let Some(enabled) = params.get_parameter("enabled")
                && let Value::Bool(value) = enabled.value
            {
                tracing::info!("Enabled: {}", value);
            }

            // List all parameters
            let names: Vec<_> = params.params.keys().cloned().collect();
            tracing::info!("Parameters: {:?}", names);
        }),
    );
    */

    tracing::info!("Parameter server demo started");
    tracing::info!("Node: /param_demo");
    tracing::info!("Try: ros2 param list /param_demo");
    tracing::info!("     ros2 param get /param_demo rate");
    tracing::info!("     ros2 param set /param_demo rate 2.0");

    // Main event loop
    loop {
        if let Err(e) = selector.wait() {
            tracing::error!("Selector error: {}", e);
            break;
        }
    }

    Ok(())
}
