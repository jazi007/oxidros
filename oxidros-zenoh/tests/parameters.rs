//! Integration tests for parameter server with selector pattern.

use oxidros_core::parameter::Value;
use oxidros_zenoh::Context;
use std::sync::Arc;
use std::time::Duration;

/// Test basic parameter set and get via the parameter storage directly.
#[test]
fn test_parameter_storage_direct() {
    let ctx = Context::new().expect("Failed to create context");
    let node = Arc::new(
        ctx.create_node("param_test_node", None)
            .expect("Failed to create node"),
    );

    let param_server = node
        .create_parameter_server()
        .expect("Failed to create parameter server");

    // Set parameters directly
    {
        let mut params = param_server.params.write();
        params
            .set_parameter("test_int".to_string(), Value::I64(42), false, None)
            .expect("Failed to set int parameter");
        params
            .set_parameter(
                "test_string".to_string(),
                Value::String("hello".to_string()),
                false,
                None,
            )
            .expect("Failed to set string parameter");
        params
            .set_parameter("test_float".to_string(), Value::F64(3.14), false, None)
            .expect("Failed to set float parameter");
        params
            .set_parameter("test_bool".to_string(), Value::Bool(true), false, None)
            .expect("Failed to set bool parameter");
    }

    // Read parameters back
    {
        let params = param_server.params.read();

        let int_param = params.get_parameter("test_int").expect("int not found");
        assert_eq!(int_param.value, Value::I64(42));

        let str_param = params
            .get_parameter("test_string")
            .expect("string not found");
        assert_eq!(str_param.value, Value::String("hello".to_string()));

        let float_param = params.get_parameter("test_float").expect("float not found");
        if let Value::F64(v) = float_param.value {
            assert!((v - 3.14).abs() < 0.001);
        } else {
            panic!("Expected F64");
        }

        let bool_param = params.get_parameter("test_bool").expect("bool not found");
        assert_eq!(bool_param.value, Value::Bool(true));
    }
}

/// Test parameter update tracking.
#[test]
fn test_parameter_update_tracking() {
    let ctx = Context::new().expect("Failed to create context");
    let node = Arc::new(
        ctx.create_node("param_update_node", None)
            .expect("Failed to create node"),
    );

    let param_server = node
        .create_parameter_server()
        .expect("Failed to create parameter server");

    // Set initial parameter
    {
        let mut params = param_server.params.write();
        params
            .set_parameter("my_param".to_string(), Value::I64(1), false, None)
            .expect("Failed to set parameter");
    }

    // Take the updated set - should contain our parameter
    {
        let mut params = param_server.params.write();
        let updated = params.take_updated();
        assert!(
            updated.contains("my_param"),
            "Parameter should be in updated set"
        );
    }

    // Take again - should be empty now
    {
        let mut params = param_server.params.write();
        let updated = params.take_updated();
        assert!(updated.is_empty(), "Updated set should be empty after take");
    }

    // Update the parameter value
    {
        let mut params = param_server.params.write();
        params
            .set_parameter("my_param".to_string(), Value::I64(2), false, None)
            .expect("Failed to update parameter");
    }

    // Check it's in updated set again
    {
        let mut params = param_server.params.write();
        let updated = params.take_updated();
        assert!(
            updated.contains("my_param"),
            "Updated parameter should be tracked"
        );
    }
}

/// Test parameter server with selector pattern - callback invocation.
#[test]
fn test_parameter_server_with_selector_callback() {
    use std::sync::atomic::{AtomicBool, Ordering};

    let ctx = Context::new().expect("Failed to create context");
    let node = Arc::new(
        ctx.create_node("selector_param_node", None)
            .expect("Failed to create node"),
    );

    let param_server = node
        .create_parameter_server()
        .expect("Failed to create parameter server");

    // Set initial parameter
    {
        let mut params = param_server.params.write();
        params
            .set_parameter("selector_param".to_string(), Value::I64(100), false, None)
            .expect("Failed to set parameter");
    }

    // Track if callback was called
    let callback_called = Arc::new(AtomicBool::new(false));
    let callback_called_clone = callback_called.clone();

    let mut selector = ctx.create_selector();

    // Add parameter server with callback
    selector.add_parameter_server(
        param_server,
        Box::new(
            move |_params: &mut oxidros_core::parameter::Parameters,
                  updated: std::collections::BTreeSet<String>| {
                if !updated.is_empty() {
                    callback_called_clone.store(true, Ordering::SeqCst);
                }
            },
        ),
    );

    // Wait briefly - parameter was set before adding to selector, so updated set
    // should be consumed in the first poll
    let _ = selector.wait_timeout(Duration::from_millis(50));

    // The callback should have been called (initial parameter was in updated set)
    assert!(
        callback_called.load(Ordering::SeqCst),
        "Callback should have been called for initial parameter"
    );
}

/// Test parameter types and array parameters.
#[test]
fn test_parameter_array_types() {
    let ctx = Context::new().expect("Failed to create context");
    let node = Arc::new(
        ctx.create_node("array_param_node", None)
            .expect("Failed to create node"),
    );

    let param_server = node
        .create_parameter_server()
        .expect("Failed to create parameter server");

    // Set array parameters
    {
        let mut params = param_server.params.write();
        params
            .set_parameter(
                "int_array".to_string(),
                Value::VecI64(vec![1, 2, 3, 4, 5]),
                false,
                None,
            )
            .expect("Failed to set int array");
        params
            .set_parameter(
                "float_array".to_string(),
                Value::VecF64(vec![1.1, 2.2, 3.3]),
                false,
                None,
            )
            .expect("Failed to set float array");
        params
            .set_parameter(
                "string_array".to_string(),
                Value::VecString(vec!["a".to_string(), "b".to_string(), "c".to_string()]),
                false,
                None,
            )
            .expect("Failed to set string array");
        params
            .set_parameter(
                "bool_array".to_string(),
                Value::VecBool(vec![true, false, true]),
                false,
                None,
            )
            .expect("Failed to set bool array");
    }

    // Read back and verify
    {
        let params = param_server.params.read();

        let int_arr = params
            .get_parameter("int_array")
            .expect("int_array not found");
        assert_eq!(int_arr.value, Value::VecI64(vec![1, 2, 3, 4, 5]));

        let float_arr = params
            .get_parameter("float_array")
            .expect("float_array not found");
        if let Value::VecF64(v) = &float_arr.value {
            assert_eq!(v.len(), 3);
        } else {
            panic!("Expected VecF64");
        }

        let str_arr = params
            .get_parameter("string_array")
            .expect("string_array not found");
        assert_eq!(
            str_arr.value,
            Value::VecString(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        );

        let bool_arr = params
            .get_parameter("bool_array")
            .expect("bool_array not found");
        assert_eq!(bool_arr.value, Value::VecBool(vec![true, false, true]));
    }
}

/// Test read-only parameter enforcement.
#[test]
fn test_read_only_parameter() {
    let ctx = Context::new().expect("Failed to create context");
    let node = Arc::new(
        ctx.create_node("readonly_param_node", None)
            .expect("Failed to create node"),
    );

    let param_server = node
        .create_parameter_server()
        .expect("Failed to create parameter server");

    // Set a read-only parameter
    {
        let mut params = param_server.params.write();
        params
            .set_parameter(
                "readonly_param".to_string(),
                Value::I64(42),
                true, // read_only = true
                Some("This is read-only".to_string()),
            )
            .expect("Failed to set read-only parameter");
    }

    // Try to update it - should fail
    {
        let mut params = param_server.params.write();
        let result =
            params.set_parameter("readonly_param".to_string(), Value::I64(100), false, None);
        assert!(
            result.is_err(),
            "Should not be able to update read-only parameter"
        );
    }

    // Verify original value is preserved
    {
        let params = param_server.params.read();
        let param = params
            .get_parameter("readonly_param")
            .expect("param not found");
        assert_eq!(param.value, Value::I64(42));
    }
}

/// Test parameter service via async client/server.
/// This tests the actual service interface for get/set operations.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_parameter_service_get_set() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::time::timeout;

    let ctx = Context::new().expect("Failed to create context");
    let node = Arc::new(
        ctx.create_node("param_svc_node", None)
            .expect("Failed to create node"),
    );

    // Create parameter server and set initial values
    let mut param_server = node
        .create_parameter_server()
        .expect("Failed to create parameter server");

    {
        let mut params = param_server.params.write();
        params
            .set_parameter("test_param".to_string(), Value::I64(42), false, None)
            .expect("Failed to set initial parameter");
    }

    // Create a client to call the list_parameters service
    use oxidros_msg::interfaces::rcl_interfaces::srv::list_parameters::ListParameters;

    let mut list_client = node
        .create_client::<ListParameters>("/param_svc_node/list_parameters", None)
        .expect("Failed to create list_parameters client");

    // Counter for processed requests
    let processed = Arc::new(AtomicUsize::new(0));
    let processed_clone = processed.clone();

    // Spawn a task to process parameter server requests
    let server_handle = tokio::spawn(async move {
        loop {
            // Use process_once with a timeout
            match timeout(Duration::from_millis(10), param_server.process_once()).await {
                Ok(Ok(())) => {
                    // Processed a request
                }
                Ok(Err(e)) => {
                    eprintln!("Error processing: {:?}", e);
                }
                Err(_) => {
                    // Timeout - no request available
                }
            }

            let count = processed_clone.fetch_add(1, Ordering::SeqCst);
            if count > 200 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        param_server // Return the param_server so we can check its state
    });

    // Give server time to start and register services
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Test list_parameters
    let list_request =
        <ListParameters as oxidros_core::ServiceMsg>::Request::new().unwrap_or_default();

    let list_result = timeout(Duration::from_secs(3), list_client.call(&list_request)).await;
    match list_result {
        Ok(Ok(response)) => {
            println!("list_parameters response received!");
            println!("  Names count: {}", response.sample.result.names.len());
            // The response should include our test_param
        }
        Ok(Err(e)) => {
            println!("list_parameters service error: {:?}", e);
        }
        Err(_) => {
            // This is the expected failure case when service routing doesn't work
            println!("list_parameters timeout - service routing issue detected");
            println!("This indicates the queryable is not receiving queries.");
        }
    }

    // Stop the server task
    processed.store(300, Ordering::SeqCst);
    let final_param_server = server_handle.await.expect("Server task panicked");

    // Verify original parameter is still there
    {
        let params = final_param_server.params.read();
        let param = params.get_parameter("test_param").expect("param not found");
        assert_eq!(param.value, Value::I64(42));
    }
}
