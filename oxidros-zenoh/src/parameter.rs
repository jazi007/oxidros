//! Parameter server for ROS2 nodes.
//!
//! This module provides a parameter server implementation that is compatible with
//! standard ROS2 parameter services. It allows nodes to have parameters that can
//! be get/set/listed/described through ROS2 service calls.
//!
//! # Example
//!
//! ```ignore
//! use oxidros_zenoh::{Context, Node, parameter::ParameterServer};
//! use oxidros_core::parameter::{Value, Parameters};
//!
//! let ctx = Context::new()?;
//! let node = ctx.create_node("my_node", None)?;
//!
//! // Create a parameter server
//! let mut param_server = node.create_parameter_server()?;
//!
//! // Set some parameters
//! {
//!     let mut params = param_server.params.write();
//!     params.set_parameter("my_param".to_string(), Value::I64(42), false, None)?;
//! }
//!
//! // Process parameter requests in event loop
//! param_server.process_once().await?;
//! ```

use crate::{error::Result, node::Node, service::server::Server};
use oxidros_core::parameter::{Parameters, Value};
use oxidros_core::qos::Profile;
use parking_lot::RwLock;
use std::sync::Arc;

// Import rcl_interfaces types when the feature is enabled
use oxidros_msg::interfaces::rcl_interfaces::{
    msg::{
        ParameterDescriptor, ParameterDescriptorSeq, ParameterValue, ParameterValueSeq,
        SetParametersResult, SetParametersResultSeq,
    },
    srv::{
        describe_parameters::{DescribeParameters, DescribeParameters_Response},
        get_parameter_types::{GetParameterTypes, GetParameterTypes_Response},
        get_parameters::{GetParameters, GetParameters_Response},
        list_parameters::{ListParameters, ListParameters_Response},
        set_parameters::{SetParameters, SetParameters_Response},
        set_parameters_atomically::{SetParametersAtomically, SetParametersAtomically_Response},
    },
};
use oxidros_msg::msg::{RosString, RosStringSeq};

/// Parameter server for a ROS2 node.
///
/// Provides the standard ROS2 parameter services:
/// - `~/list_parameters`
/// - `~/get_parameters`
/// - `~/set_parameters`
/// - `~/set_parameters_atomically`
/// - `~/describe_parameters`
/// - `~/get_parameter_types`
pub struct ParameterServer {
    /// Shared parameter storage.
    pub params: Arc<RwLock<Parameters>>,
    /// Parent node.
    node: Arc<Node>,
    /// Service servers.
    srv_list: Server<ListParameters>,
    srv_get: Server<GetParameters>,
    srv_set: Server<SetParameters>,
    srv_set_atomic: Server<SetParametersAtomically>,
    srv_describe: Server<DescribeParameters>,
    srv_get_types: Server<GetParameterTypes>,
}

impl ParameterServer {
    /// Create a new parameter server for the given node.
    ///
    /// This will:
    /// 1. Load initial parameters from command-line arguments and parameter files
    /// 2. Create the standard ROS2 parameter service endpoints
    pub fn new(node: Arc<Node>) -> Result<Self> {
        // Initialize parameters from ROS2 arguments
        let mut params = Parameters::new();

        // Load parameters from command-line assignments
        // Use original name for matching node-specific parameter rules
        let ros2_args = node.context().ros2_args();
        let original_name = node.original_name();
        let fqn = node.fully_qualified_name()?;

        // Get parameters that apply to this node (using original name for matching)
        if let Ok(param_assignments) = ros2_args.get_params_for_node(original_name) {
            for param in param_assignments {
                if let Some(value) = yaml_to_value(&param.value) {
                    let _ = params.set_parameter(param.name.clone(), value, false, None);
                }
            }
        }

        // Also check with FQN (some params might be specified with full path)
        if let Ok(param_assignments) = ros2_args.get_params_for_node(&fqn) {
            for param in param_assignments {
                if let Some(value) = yaml_to_value(&param.value) {
                    let _ = params.set_parameter(param.name.clone(), value, false, None);
                }
            }
        }

        let params = Arc::new(RwLock::new(params));

        let qos = Profile::services_default();

        // Create parameter services with private names (~/service_name)
        // The ~ prefix expands to /<namespace>/<node_name>, so the full path becomes
        // /<namespace>/<node_name>/list_parameters etc.
        let srv_list =
            node.create_server::<ListParameters>("~/list_parameters", Some(qos.clone()))?;

        let srv_get = node.create_server::<GetParameters>("~/get_parameters", Some(qos.clone()))?;

        let srv_set = node.create_server::<SetParameters>("~/set_parameters", Some(qos.clone()))?;

        let srv_set_atomic = node.create_server::<SetParametersAtomically>(
            "~/set_parameters_atomically",
            Some(qos.clone()),
        )?;

        let srv_describe =
            node.create_server::<DescribeParameters>("~/describe_parameters", Some(qos.clone()))?;

        let srv_get_types =
            node.create_server::<GetParameterTypes>("~/get_parameter_types", Some(qos))?;

        Ok(ParameterServer {
            params,
            node,
            srv_list,
            srv_get,
            srv_set,
            srv_set_atomic,
            srv_describe,
            srv_get_types,
        })
    }

    /// Get a reference to the parent node.
    pub fn node(&self) -> &Arc<Node> {
        &self.node
    }

    /// Process one iteration of parameter service requests.
    ///
    /// This is a non-blocking check that processes any pending requests
    /// on all parameter services.
    pub async fn process_once(&mut self) -> Result<()> {
        // Use tokio::select! to handle whichever service has a request ready
        tokio::select! {
            biased;

            result = self.srv_list.recv() => {
                tracing::info!("Received list_parameters request");
                if let Ok(req) = result {
                    let response = self.handle_list_parameters(&req.request);
                    let _ = req.send(&response);
                }
            }

            result = self.srv_get.recv() => {
                tracing::info!("Received get_parameters request");
                if let Ok(req) = result {
                    let response = self.handle_get_parameters(&req.request);
                    let _ = req.send(&response);
                }
            }

            result = self.srv_set.recv() => {
                if let Ok(req) = result {
                    let response = self.handle_set_parameters(&req.request);
                    let _ = req.send(&response);
                }
            }

            result = self.srv_set_atomic.recv() => {
                tracing::info!("Received set_parameters_atomically request");
                if let Ok(req) = result {
                    let response = self.handle_set_parameters_atomically(&req.request);
                    let _ = req.send(&response);
                }
            }

            result = self.srv_describe.recv() => {
                if let Ok(req) = result {
                    let response = self.handle_describe_parameters(&req.request);
                    let _ = req.send(&response);
                }
            }

            result = self.srv_get_types.recv() => {
                if let Ok(req) = result {
                    let response = self.handle_get_parameter_types(&req.request);
                    let _ = req.send(&response);
                }
            }
        }

        Ok(())
    }

    /// Try to process parameter service requests without blocking.
    ///
    /// This method polls all parameter services using `try_recv()` and processes
    /// any pending requests. Returns `true` if any request was processed.
    ///
    /// This is useful for integrating with a Selector's event loop.
    pub fn try_process_once(&mut self) -> bool {
        let mut processed = false;

        // Try to receive from list_parameters
        if let Ok(Some(req)) = self.srv_list.try_recv() {
            tracing::info!("Received list_parameters request");
            let response = self.handle_list_parameters(&req.request);
            let _ = req.send(&response);
            processed = true;
        }

        // Try to receive from get_parameters
        if let Ok(Some(req)) = self.srv_get.try_recv() {
            tracing::info!("Received get_parameters request");
            let response = self.handle_get_parameters(&req.request);
            let _ = req.send(&response);
            processed = true;
        }

        // Try to receive from set_parameters
        if let Ok(Some(req)) = self.srv_set.try_recv() {
            let response = self.handle_set_parameters(&req.request);
            let _ = req.send(&response);
            processed = true;
        }

        // Try to receive from set_parameters_atomically
        if let Ok(Some(req)) = self.srv_set_atomic.try_recv() {
            let response = self.handle_set_parameters_atomically(&req.request);
            let _ = req.send(&response);
            processed = true;
        }

        // Try to receive from describe_parameters
        if let Ok(Some(req)) = self.srv_describe.try_recv() {
            let response = self.handle_describe_parameters(&req.request);
            let _ = req.send(&response);
            processed = true;
        }

        // Try to receive from get_parameter_types
        if let Ok(Some(req)) = self.srv_get_types.try_recv() {
            let response = self.handle_get_parameter_types(&req.request);
            let _ = req.send(&response);
            processed = true;
        }

        processed
    }

    // --- Service handlers ---

    fn handle_list_parameters(
        &self,
        _request: &<ListParameters as oxidros_core::ServiceMsg>::Request,
    ) -> <ListParameters as oxidros_core::ServiceMsg>::Response {
        let guard = self.params.read();

        let mut response = ListParameters_Response::new().unwrap_or_default();

        // Collect all parameter names
        let names: Vec<_> = guard.params.keys().cloned().collect();
        if let Some(mut name_seq) = RosStringSeq::<0, 0>::new(names.len()) {
            for (i, name) in names.iter().enumerate() {
                if let Some(ros_str) = RosString::<0>::new(name) {
                    name_seq.as_mut_slice()[i] = ros_str;
                }
            }
            response.result.names = name_seq;
        }

        response
    }

    fn handle_get_parameters(
        &self,
        request: &<GetParameters as oxidros_core::ServiceMsg>::Request,
    ) -> <GetParameters as oxidros_core::ServiceMsg>::Response {
        let guard = self.params.read();

        let mut response = GetParameters_Response::new().unwrap_or_default();

        let names = &request.names;
        if let Some(mut values) = ParameterValueSeq::<0>::new(names.len()) {
            for (i, name) in names.iter().enumerate() {
                let name_str = name.to_string();
                if let Some(param) = guard.params.get(&name_str) {
                    values.as_mut_slice()[i] = value_to_parameter_value(&param.value);
                }
            }
            response.values = values;
        }

        response
    }

    fn handle_set_parameters(
        &self,
        request: &<SetParameters as oxidros_core::ServiceMsg>::Request,
    ) -> <SetParameters as oxidros_core::ServiceMsg>::Response {
        let mut guard = self.params.write();

        let mut response = SetParameters_Response::new().unwrap_or_default();

        if let Some(mut results) = SetParametersResultSeq::<0>::new(request.parameters.len()) {
            for (i, param) in request.parameters.iter().enumerate() {
                let name = param.name.to_string();
                let value = parameter_value_to_value(&param.value);

                let mut result = SetParametersResult::new().unwrap_or_default();

                match guard.set_parameter(name, value, false, None) {
                    Ok(()) => {
                        result.successful = true;
                    }
                    Err(e) => {
                        result.successful = false;
                        if let Some(reason) = RosString::new(&e.to_string()) {
                            result.reason = reason;
                        }
                    }
                }

                results.as_mut_slice()[i] = result;
            }
            response.results = results;
        }

        response
    }

    fn handle_set_parameters_atomically(
        &self,
        request: &<SetParametersAtomically as oxidros_core::ServiceMsg>::Request,
    ) -> <SetParametersAtomically as oxidros_core::ServiceMsg>::Response {
        let mut guard = self.params.write();

        let mut response = SetParametersAtomically_Response::new().unwrap_or_default();
        response.result = SetParametersResult::new().unwrap_or_default();

        // First, validate all parameters
        let mut validated: Vec<(String, Value)> = Vec::new();

        for param in request.parameters.iter() {
            let name = param.name.to_string();
            let value = parameter_value_to_value(&param.value);

            // Check if it's a valid update
            if let Some(existing) = guard.params.get(&name) {
                if existing.descriptor.read_only {
                    response.result.successful = false;
                    if let Some(reason) = RosString::new(&format!("{} is read only", name)) {
                        response.result.reason = reason;
                    }
                    return response;
                }
                if !existing.descriptor.dynamic_typing && !existing.value.type_check(&value) {
                    response.result.successful = false;
                    if let Some(reason) = RosString::new(&format!("Type mismatch for {}", name)) {
                        response.result.reason = reason;
                    }
                    return response;
                }
            }

            validated.push((name, value));
        }

        // All validated, now apply atomically
        for (name, value) in validated {
            let _ = guard.set_parameter(name, value, false, None);
        }

        response.result.successful = true;
        response
    }

    fn handle_describe_parameters(
        &self,
        request: &<DescribeParameters as oxidros_core::ServiceMsg>::Request,
    ) -> <DescribeParameters as oxidros_core::ServiceMsg>::Response {
        let guard = self.params.read();

        let mut response = DescribeParameters_Response::new().unwrap_or_default();

        if let Some(mut descriptors) = ParameterDescriptorSeq::<0>::new(request.names.len()) {
            for (i, name) in request.names.iter().enumerate() {
                let name_str = name.to_string();
                let mut desc = ParameterDescriptor::new().unwrap_or_default();

                if let Some(ros_name) = RosString::new(&name_str) {
                    desc.name = ros_name;
                }

                if let Some(param) = guard.params.get(&name_str) {
                    desc.r#type = value_type_id(&param.value);
                    if let Some(ros_desc) = RosString::new(&param.descriptor.description) {
                        desc.description = ros_desc;
                    }
                    desc.read_only = param.descriptor.read_only;
                    desc.dynamic_typing = param.descriptor.dynamic_typing;
                }

                descriptors.as_mut_slice()[i] = desc;
            }
            response.descriptors = descriptors;
        }

        response
    }

    fn handle_get_parameter_types(
        &self,
        request: &<GetParameterTypes as oxidros_core::ServiceMsg>::Request,
    ) -> <GetParameterTypes as oxidros_core::ServiceMsg>::Response {
        let guard = self.params.read();

        let mut response = GetParameterTypes_Response::new().unwrap_or_default();

        if let Some(mut types) = oxidros_msg::msg::U8Seq::<0>::new(request.names.len()) {
            for (i, name) in request.names.iter().enumerate() {
                let name_str = name.to_string();
                if let Some(param) = guard.params.get(&name_str) {
                    types.as_mut_slice()[i] = value_type_id(&param.value);
                }
            }
            response.types = types;
        }

        response
    }
}

// --- Helper functions ---

/// Convert YAML value to oxidros_core Value.
fn yaml_to_value(yaml: &yaml_rust2::Yaml) -> Option<Value> {
    use yaml_rust2::Yaml;
    match yaml {
        Yaml::Boolean(b) => Some(Value::Bool(*b)),
        Yaml::Integer(i) => Some(Value::I64(*i)),
        Yaml::Real(s) => s.parse::<f64>().ok().map(Value::F64),
        Yaml::String(s) => Some(Value::String(s.clone())),
        Yaml::Array(arr) => {
            // Try to determine array type from first element
            if let Some(first) = arr.first() {
                match first {
                    Yaml::Boolean(_) => {
                        let vals: Option<Vec<_>> = arr.iter().map(|v| v.as_bool()).collect();
                        vals.map(Value::VecBool)
                    }
                    Yaml::Integer(_) => {
                        let vals: Option<Vec<_>> = arr.iter().map(|v| v.as_i64()).collect();
                        vals.map(Value::VecI64)
                    }
                    Yaml::Real(_) => {
                        let vals: Option<Vec<_>> = arr.iter().map(|v| v.as_f64()).collect();
                        vals.map(Value::VecF64)
                    }
                    Yaml::String(_) => {
                        let vals: Option<Vec<_>> =
                            arr.iter().map(|v| v.as_str().map(String::from)).collect();
                        vals.map(Value::VecString)
                    }
                    _ => None,
                }
            } else {
                Some(Value::VecI64(vec![]))
            }
        }
        _ => None,
    }
}

/// Convert oxidros_core Value to ParameterValue message.
fn value_to_parameter_value(value: &Value) -> ParameterValue {
    let mut pv = ParameterValue::new().unwrap_or_default();

    match value {
        Value::NotSet => {
            pv.r#type = 0;
        }
        Value::Bool(b) => {
            pv.r#type = 1;
            pv.bool_value = *b;
        }
        Value::I64(i) => {
            pv.r#type = 2;
            pv.integer_value = *i;
        }
        Value::F64(f) => {
            pv.r#type = 3;
            pv.double_value = *f;
        }
        Value::String(s) => {
            pv.r#type = 4;
            if let Some(ros_str) = RosString::new(s) {
                pv.string_value = ros_str;
            }
        }
        Value::VecU8(bytes) => {
            pv.r#type = 5;
            if let Some(mut seq) = oxidros_msg::msg::ByteSeq::<0>::new(bytes.len()) {
                seq.as_mut_slice().copy_from_slice(bytes);
                pv.byte_array_value = seq;
            }
        }
        Value::VecBool(bools) => {
            pv.r#type = 6;
            if let Some(mut seq) = oxidros_msg::msg::BoolSeq::<0>::new(bools.len()) {
                seq.as_mut_slice().copy_from_slice(bools);
                pv.bool_array_value = seq;
            }
        }
        Value::VecI64(ints) => {
            pv.r#type = 7;
            if let Some(mut seq) = oxidros_msg::msg::I64Seq::<0>::new(ints.len()) {
                seq.as_mut_slice().copy_from_slice(ints);
                pv.integer_array_value = seq;
            }
        }
        Value::VecF64(floats) => {
            pv.r#type = 8;
            if let Some(mut seq) = oxidros_msg::msg::F64Seq::<0>::new(floats.len()) {
                seq.as_mut_slice().copy_from_slice(floats);
                pv.double_array_value = seq;
            }
        }
        Value::VecString(strings) => {
            pv.r#type = 9;
            if let Some(mut seq) = RosStringSeq::<0, 0>::new(strings.len()) {
                for (i, s) in strings.iter().enumerate() {
                    if let Some(ros_str) = RosString::new(s) {
                        seq.as_mut_slice()[i] = ros_str;
                    }
                }
                pv.string_array_value = seq;
            }
        }
    }

    pv
}

/// Convert ParameterValue message to oxidros_core Value.
fn parameter_value_to_value(pv: &ParameterValue) -> Value {
    match pv.r#type {
        1 => Value::Bool(pv.bool_value),
        2 => Value::I64(pv.integer_value),
        3 => Value::F64(pv.double_value),
        4 => Value::String(pv.string_value.to_string()),
        5 => Value::VecU8(pv.byte_array_value.as_slice().to_vec()),
        6 => Value::VecBool(pv.bool_array_value.as_slice().to_vec()),
        7 => Value::VecI64(pv.integer_array_value.as_slice().to_vec()),
        8 => Value::VecF64(pv.double_array_value.as_slice().to_vec()),
        9 => Value::VecString(
            pv.string_array_value
                .iter()
                .map(|s| s.to_string())
                .collect(),
        ),
        _ => Value::NotSet,
    }
}

/// Get the ROS2 parameter type ID for a value.
fn value_type_id(value: &Value) -> u8 {
    match value {
        Value::NotSet => 0,
        Value::Bool(_) => 1,
        Value::I64(_) => 2,
        Value::F64(_) => 3,
        Value::String(_) => 4,
        Value::VecU8(_) => 5,
        Value::VecBool(_) => 6,
        Value::VecI64(_) => 7,
        Value::VecF64(_) => 8,
        Value::VecString(_) => 9,
    }
}
