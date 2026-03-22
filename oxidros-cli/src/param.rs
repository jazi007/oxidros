use clap::Subcommand;
use oxidros_zenoh::Context;
use serde_json::{Value, json};

#[derive(Subcommand)]
pub enum ParamCommand {
    /// List parameters of a node
    List {
        /// Node name (e.g. /my_node)
        node_name: String,
        /// Filter by parameter name prefix
        #[arg(long)]
        prefix: Option<String>,
    },
    /// Get a parameter value
    Get {
        /// Node name (e.g. /my_node)
        node_name: String,
        /// Parameter name
        param_name: String,
    },
    /// Set a parameter value
    Set {
        /// Node name (e.g. /my_node)
        node_name: String,
        /// Parameter name
        param_name: String,
        /// Parameter value (YAML format)
        value: String,
    },
    /// Describe a parameter
    Describe {
        /// Node name (e.g. /my_node)
        node_name: String,
        /// Parameter name
        param_name: String,
    },
    /// Dump all parameters of a node as YAML
    Dump {
        /// Node name (e.g. /my_node)
        node_name: String,
    },
}

pub async fn run(cmd: ParamCommand, ctx: &Context) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        ParamCommand::List { node_name, prefix } => list(ctx, &node_name, prefix.as_deref()).await,
        ParamCommand::Get {
            node_name,
            param_name,
        } => get(ctx, &node_name, &param_name).await,
        ParamCommand::Set {
            node_name,
            param_name,
            value,
        } => set(ctx, &node_name, &param_name, &value).await,
        ParamCommand::Describe {
            node_name,
            param_name,
        } => describe(ctx, &node_name, &param_name).await,
        ParamCommand::Dump { node_name } => dump(ctx, &node_name).await,
    }
}

// ============================================================================
// ROS2 parameter type constants
// ============================================================================

const PARAMETER_NOT_SET: u64 = 0;
const PARAMETER_BOOL: u64 = 1;
const PARAMETER_INTEGER: u64 = 2;
const PARAMETER_DOUBLE: u64 = 3;
const PARAMETER_STRING: u64 = 4;
const PARAMETER_BYTE_ARRAY: u64 = 5;
const PARAMETER_BOOL_ARRAY: u64 = 6;
const PARAMETER_INTEGER_ARRAY: u64 = 7;
const PARAMETER_DOUBLE_ARRAY: u64 = 8;
const PARAMETER_STRING_ARRAY: u64 = 9;

fn type_name(type_id: u64) -> &'static str {
    match type_id {
        PARAMETER_NOT_SET => "not set",
        PARAMETER_BOOL => "bool",
        PARAMETER_INTEGER => "integer",
        PARAMETER_DOUBLE => "double",
        PARAMETER_STRING => "string",
        PARAMETER_BYTE_ARRAY => "byte_array",
        PARAMETER_BOOL_ARRAY => "bool_array",
        PARAMETER_INTEGER_ARRAY => "integer_array",
        PARAMETER_DOUBLE_ARRAY => "double_array",
        PARAMETER_STRING_ARRAY => "string_array",
        _ => "unknown",
    }
}

// ============================================================================
// param list
// ============================================================================

async fn list(
    ctx: &Context,
    node: &str,
    prefix: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let graph = ctx.graph_cache();
    let svc = format!("{node}/list_parameters");

    let request = match prefix {
        Some(p) => json!({ "prefixes": [p], "depth": 0 }),
        None => json!({ "prefixes": [], "depth": 0 }),
    };

    let response = crate::service::raw_call(ctx, &graph, &svc, &request).await?;

    let names = response.pointer("/result/names").and_then(|v| v.as_array());

    match names {
        Some(arr) if !arr.is_empty() => {
            for name in arr {
                if let Some(s) = name.as_str() {
                    println!("  {s}");
                }
            }
        }
        _ => {
            eprintln!("No parameters found for node '{node}'.");
        }
    }
    Ok(())
}

// ============================================================================
// param get
// ============================================================================

async fn get(ctx: &Context, node: &str, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let graph = ctx.graph_cache();
    let svc = format!("{node}/get_parameters");
    let request = json!({ "names": [name] });
    let response = crate::service::raw_call(ctx, &graph, &svc, &request).await?;

    let values = response.pointer("/values").and_then(|v| v.as_array());

    match values {
        Some(arr) if !arr.is_empty() => {
            let pv = &arr[0];
            let type_id = pv.get("type").and_then(|v| v.as_u64()).unwrap_or(0);
            let display = format_parameter_value(pv, type_id);
            println!("{type}: {display}", type = type_name(type_id));
        }
        _ => {
            eprintln!("Parameter '{name}' not found on node '{node}'.");
        }
    }
    Ok(())
}

// ============================================================================
// param set
// ============================================================================

async fn set(
    ctx: &Context,
    node: &str,
    name: &str,
    value_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let graph = ctx.graph_cache();
    let svc = format!("{node}/set_parameters");

    let param_value = parse_to_parameter_value(value_str)?;
    let request = json!({
        "parameters": [{
            "name": name,
            "value": param_value,
        }]
    });

    let response = crate::service::raw_call(ctx, &graph, &svc, &request).await?;

    let results = response.pointer("/results").and_then(|v| v.as_array());

    match results {
        Some(arr) if !arr.is_empty() => {
            let result = &arr[0];
            let ok = result
                .get("successful")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if ok {
                println!("Set parameter successful");
            } else {
                let reason = result
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                eprintln!("Set parameter failed: {reason}");
            }
        }
        _ => {
            eprintln!("Unexpected response from set_parameters service.");
        }
    }
    Ok(())
}

// ============================================================================
// param describe
// ============================================================================

async fn describe(ctx: &Context, node: &str, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let graph = ctx.graph_cache();
    let svc = format!("{node}/describe_parameters");
    let request = json!({ "names": [name] });
    let response = crate::service::raw_call(ctx, &graph, &svc, &request).await?;

    let descriptors = response.pointer("/descriptors").and_then(|v| v.as_array());

    match descriptors {
        Some(arr) if !arr.is_empty() => {
            let desc = &arr[0];
            let pname = desc.get("name").and_then(|v| v.as_str()).unwrap_or(name);
            let type_id = desc.get("type").and_then(|v| v.as_u64()).unwrap_or(0);
            let description = desc
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let constraints = desc
                .get("additional_constraints")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let read_only = desc
                .get("read_only")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            println!("Parameter name: {pname}");
            println!("  Type: {}", type_name(type_id));
            if !description.is_empty() {
                println!("  Description: {description}");
            }
            if !constraints.is_empty() {
                println!("  Constraints: {constraints}");
            }
            println!("  Read only: {read_only}");

            // Print ranges if present
            if let Some(ranges) = desc.get("floating_point_range").and_then(|v| v.as_array()) {
                for r in ranges {
                    let from = r.get("from_value").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let to = r.get("to_value").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let step = r.get("step").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    println!("  Floating point range: [{from}, {to}], step: {step}");
                }
            }
            if let Some(ranges) = desc.get("integer_range").and_then(|v| v.as_array()) {
                for r in ranges {
                    let from = r.get("from_value").and_then(|v| v.as_i64()).unwrap_or(0);
                    let to = r.get("to_value").and_then(|v| v.as_i64()).unwrap_or(0);
                    let step = r.get("step").and_then(|v| v.as_i64()).unwrap_or(0);
                    println!("  Integer range: [{from}, {to}], step: {step}");
                }
            }
        }
        _ => {
            eprintln!("Parameter '{name}' not found on node '{node}'.");
        }
    }
    Ok(())
}

// ============================================================================
// param dump
// ============================================================================

async fn dump(ctx: &Context, node: &str) -> Result<(), Box<dyn std::error::Error>> {
    let graph = ctx.graph_cache();

    // Step 1: List all parameters
    let list_svc = format!("{node}/list_parameters");
    let list_request = json!({ "prefixes": [], "depth": 0 });
    let list_response = crate::service::raw_call(ctx, &graph, &list_svc, &list_request).await?;

    let names: Vec<&str> = list_response
        .pointer("/result/names")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    if names.is_empty() {
        eprintln!("No parameters found for node '{node}'.");
        return Ok(());
    }

    // Step 2: Get all parameter values
    let get_svc = format!("{node}/get_parameters");
    let name_values: Vec<Value> = names.iter().map(|n| json!(n)).collect();
    let get_request = json!({ "names": name_values });
    let get_response = crate::service::raw_call(ctx, &graph, &get_svc, &get_request).await?;

    let values = get_response.pointer("/values").and_then(|v| v.as_array());

    // Step 3: Print as YAML
    let node_short = node.strip_prefix('/').unwrap_or(node);
    println!("{node_short}:");
    println!("  ros__parameters:");

    if let Some(vals) = values {
        for (i, name) in names.iter().enumerate() {
            let pv = vals.get(i);
            let type_id = pv
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let display = pv
                .map(|v| format_parameter_value(v, type_id))
                .unwrap_or_else(|| "not set".to_string());
            println!("    {name}: {display}");
        }
    }

    Ok(())
}

// ============================================================================
// Helpers
// ============================================================================

/// Extract the human-readable value from a ParameterValue decoded JSON object.
fn format_parameter_value(pv: &Value, type_id: u64) -> String {
    match type_id {
        PARAMETER_NOT_SET => "not set".to_string(),
        PARAMETER_BOOL => pv
            .get("bool_value")
            .and_then(|v| v.as_bool())
            .map(|b| b.to_string())
            .unwrap_or_else(|| "false".to_string()),
        PARAMETER_INTEGER => pv
            .get("integer_value")
            .and_then(|v| v.as_i64())
            .map(|i| i.to_string())
            .unwrap_or_else(|| "0".to_string()),
        PARAMETER_DOUBLE => pv
            .get("double_value")
            .and_then(|v| v.as_f64())
            .map(format_float)
            .unwrap_or_else(|| "0.0".to_string()),
        PARAMETER_STRING => pv
            .get("string_value")
            .and_then(|v| v.as_str())
            .map(|s| format!("'{s}'"))
            .unwrap_or_else(|| "''".to_string()),
        PARAMETER_BYTE_ARRAY => format_array(pv, "byte_array_value"),
        PARAMETER_BOOL_ARRAY => format_array(pv, "bool_array_value"),
        PARAMETER_INTEGER_ARRAY => format_array(pv, "integer_array_value"),
        PARAMETER_DOUBLE_ARRAY => format_array(pv, "double_array_value"),
        PARAMETER_STRING_ARRAY => format_array(pv, "string_array_value"),
        _ => "unknown".to_string(),
    }
}

fn format_array(pv: &Value, key: &str) -> String {
    match pv.get(key).and_then(|v| v.as_array()) {
        Some(arr) => {
            let items: Vec<String> = arr
                .iter()
                .map(|v| match v {
                    Value::String(s) => format!("'{s}'"),
                    Value::Number(n) => {
                        if let Some(f) = n.as_f64()
                            && n.is_f64()
                        {
                            return format_float(f);
                        }
                        n.to_string()
                    }
                    Value::Bool(b) => b.to_string(),
                    other => other.to_string(),
                })
                .collect();
            format!("[{}]", items.join(", "))
        }
        None => "[]".to_string(),
    }
}

fn format_float(f: f64) -> String {
    if f == f.floor() && f.abs() < 1e15 {
        format!("{f:.1}")
    } else {
        format!("{f}")
    }
}

/// Parse a user-provided value string into a ParameterValue JSON object.
///
/// Infers the type from the YAML value:
/// - `true`/`false` → PARAMETER_BOOL
/// - integer literal → PARAMETER_INTEGER
/// - float literal → PARAMETER_DOUBLE
/// - quoted or other string → PARAMETER_STRING
/// - `[1, 2, 3]` → array types
fn parse_to_parameter_value(value_str: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let yaml_val: Value = crate::yaml::parse_yaml_to_json(value_str)?;

    let (type_id, pv) = match &yaml_val {
        Value::Bool(b) => (
            PARAMETER_BOOL,
            json!({
                "type": PARAMETER_BOOL,
                "bool_value": b,
                "integer_value": 0,
                "double_value": 0.0,
                "string_value": "",
                "byte_array_value": [],
                "bool_array_value": [],
                "integer_array_value": [],
                "double_array_value": [],
                "string_array_value": []
            }),
        ),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                (
                    PARAMETER_INTEGER,
                    json!({
                        "type": PARAMETER_INTEGER,
                        "bool_value": false,
                        "integer_value": i,
                        "double_value": 0.0,
                        "string_value": "",
                        "byte_array_value": [],
                        "bool_array_value": [],
                        "integer_array_value": [],
                        "double_array_value": [],
                        "string_array_value": []
                    }),
                )
            } else if let Some(f) = n.as_f64() {
                (
                    PARAMETER_DOUBLE,
                    json!({
                        "type": PARAMETER_DOUBLE,
                        "bool_value": false,
                        "integer_value": 0,
                        "double_value": f,
                        "string_value": "",
                        "byte_array_value": [],
                        "bool_array_value": [],
                        "integer_array_value": [],
                        "double_array_value": [],
                        "string_array_value": []
                    }),
                )
            } else {
                return Err("Cannot parse number".into());
            }
        }
        Value::String(s) => (
            PARAMETER_STRING,
            json!({
                "type": PARAMETER_STRING,
                "bool_value": false,
                "integer_value": 0,
                "double_value": 0.0,
                "string_value": s,
                "byte_array_value": [],
                "bool_array_value": [],
                "integer_array_value": [],
                "double_array_value": [],
                "string_array_value": []
            }),
        ),
        Value::Array(arr) => parse_array_parameter(arr)?,
        _ => {
            return Err(format!("Unsupported value type: {yaml_val}").into());
        }
    };

    let _ = type_id; // used for clarity in match arms
    Ok(pv)
}

/// Determine the array parameter type and build the ParameterValue JSON.
fn parse_array_parameter(arr: &[Value]) -> Result<(u64, Value), Box<dyn std::error::Error>> {
    if arr.is_empty() {
        // Default to byte array for empty arrays
        return Ok((
            PARAMETER_BYTE_ARRAY,
            json!({
                "type": PARAMETER_BYTE_ARRAY,
                "bool_value": false,
                "integer_value": 0,
                "double_value": 0.0,
                "string_value": "",
                "byte_array_value": [],
                "bool_array_value": [],
                "integer_array_value": [],
                "double_array_value": [],
                "string_array_value": []
            }),
        ));
    }

    // Infer type from first element
    let base = make_parameter_value_base();
    match &arr[0] {
        Value::Bool(_) => {
            let mut pv = base;
            pv["type"] = json!(PARAMETER_BOOL_ARRAY);
            pv["bool_array_value"] = json!(arr);
            Ok((PARAMETER_BOOL_ARRAY, pv))
        }
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                let mut pv = base;
                pv["type"] = json!(PARAMETER_INTEGER_ARRAY);
                pv["integer_array_value"] = json!(arr);
                Ok((PARAMETER_INTEGER_ARRAY, pv))
            } else {
                let mut pv = base;
                pv["type"] = json!(PARAMETER_DOUBLE_ARRAY);
                pv["double_array_value"] = json!(arr);
                Ok((PARAMETER_DOUBLE_ARRAY, pv))
            }
        }
        Value::String(_) => {
            let mut pv = base;
            pv["type"] = json!(PARAMETER_STRING_ARRAY);
            pv["string_array_value"] = json!(arr);
            Ok((PARAMETER_STRING_ARRAY, pv))
        }
        _ => Err("Unsupported array element type".into()),
    }
}

fn make_parameter_value_base() -> Value {
    json!({
        "type": PARAMETER_NOT_SET,
        "bool_value": false,
        "integer_value": 0,
        "double_value": 0.0,
        "string_value": "",
        "byte_array_value": [],
        "bool_array_value": [],
        "integer_array_value": [],
        "double_array_value": [],
        "string_array_value": []
    })
}
