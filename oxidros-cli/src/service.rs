use clap::Subcommand;
use oxidros_zenoh::{Context, EntityKind, GraphCache};
use std::time::Duration;
use zenoh::bytes::ZBytes;
use zenoh::query::QueryTarget;

#[derive(Subcommand)]
pub enum ServiceCommand {
    /// List all services
    List {
        /// Show service types
        #[arg(short = 't', long)]
        show_types: bool,
    },
    /// Call a service
    Call {
        /// Service name (e.g. /add_two_ints)
        service_name: String,
        /// Service type (e.g. example_interfaces/srv/AddTwoInts)
        service_type: String,
        /// Request data in YAML format (e.g. "{a: 1, b: 2}")
        #[arg(default_value = "{}")]
        data: String,
    },
}

pub async fn run(cmd: ServiceCommand, ctx: &Context) -> Result<(), Box<dyn std::error::Error>> {
    let graph = ctx.graph_cache();
    match cmd {
        ServiceCommand::List { show_types } => list(&graph, show_types),
        ServiceCommand::Call {
            service_name,
            service_type,
            data,
        } => call(ctx, &graph, &service_name, &service_type, &data).await,
    }
}

fn list(graph: &GraphCache, show_types: bool) -> Result<(), Box<dyn std::error::Error>> {
    let services = graph.get_service_names_and_types();
    if services.is_empty() {
        eprintln!("No services found.");
        return Ok(());
    }
    for (name, type_name) in &services {
        if show_types {
            println!("{name} [type: {type_name}]");
        } else {
            println!("{name}");
        }
    }
    Ok(())
}

// ============================================================================
// service call
// ============================================================================

async fn call(
    ctx: &Context,
    graph: &GraphCache,
    service_name: &str,
    service_type: &str,
    yaml_data: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse YAML input → serde_json::Value
    let yaml_value = crate::yaml::parse_yaml_to_json(yaml_data)?;

    eprintln!("waiting for service to become available...");

    let response_value = raw_call(ctx, graph, service_name, &yaml_value).await?;

    // Print response
    println!("requester: making request: {service_type}_{{}}");
    println!();
    println!("response:");
    print_yaml(&response_value, 0);
    println!();

    Ok(())
}

// ============================================================================
// Raw service call — reusable from param.rs and other modules
// ============================================================================

/// Perform a raw service call: encode request JSON → CDR, send Zenoh query, decode response.
///
/// `service_name` is the fully-qualified service name (e.g. `"/my_node/list_parameters"`).
/// `request` is the request body as a `serde_json::Value`.
pub(crate) async fn raw_call(
    ctx: &Context,
    graph: &GraphCache,
    service_name: &str,
    request: &serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let (dds_type, type_hash) = find_service_type(graph, service_name)
        .ok_or_else(|| format!("Service '{service_name}' not found in the graph"))?;

    let request_dds = service_dds_subtype(&dds_type, "Request");
    let response_dds = service_dds_subtype(&dds_type, "Response");

    let request_desc = crate::type_resolve::resolve(&request_dds, &type_hash, ctx, graph)
        .await
        .ok_or_else(|| format!("Cannot resolve request type for '{request_dds}'"))?;
    let response_desc = crate::type_resolve::resolve(&response_dds, &type_hash, ctx, graph)
        .await
        .ok_or_else(|| format!("Cannot resolve response type for '{response_dds}'"))?;

    let cdr_bytes = oxidros_dynamic::encode_cdr(request, &request_desc)
        .map_err(|e| format!("Encode error: {e}"))?;

    let svc_name_stripped = service_name.strip_prefix('/').unwrap_or(service_name);
    let key_expr = format!(
        "{}/{}/{}/{}",
        ctx.domain_id(),
        svc_name_stripped,
        dds_type,
        type_hash,
    );

    let attachment = crate::type_resolve::build_attachment();

    let replies = ctx
        .session()
        .get(&key_expr)
        .payload(ZBytes::from(cdr_bytes))
        .attachment(ZBytes::from(attachment.to_vec()))
        .target(QueryTarget::All)
        .timeout(Duration::from_secs(10))
        .await
        .map_err(|e| format!("Service call failed: {e}"))?;

    let reply = replies
        .recv_async()
        .await
        .map_err(|e| format!("No reply received: {e}"))?;
    let sample = reply
        .result()
        .map_err(|e| format!("Service returned error: {:?}", e.payload()))?;
    let response_bytes = sample.payload().to_bytes();

    oxidros_dynamic::decode_cdr(&response_bytes, &response_desc)
        .map_err(|e| format!("Decode error: {e}").into())
}

/// Find the DDS service type and hash from the graph cache.
pub(crate) fn find_service_type(
    graph: &GraphCache,
    service_name: &str,
) -> Option<(String, String)> {
    let entities = graph.get_all_entities();
    let entity = entities.into_iter().find(|e| {
        e.kind == EntityKind::ServiceServer && e.topic_name.as_deref() == Some(service_name)
    })?;
    Some((entity.type_name.clone()?, entity.type_hash.clone()?))
}

/// Derive a service request/response DDS type name from the base service DDS type.
///
/// `"pkg::srv::dds_::TypeName_"` + `"Request"` → `"pkg::srv::dds_::TypeName_Request_"`
pub(crate) fn service_dds_subtype(service_dds: &str, suffix: &str) -> String {
    if let Some(base) = service_dds.strip_suffix('_') {
        format!("{base}_{suffix}_")
    } else {
        format!("{service_dds}_{suffix}_")
    }
}

/// Print a serde_json::Value in YAML-like format.
pub(crate) fn print_yaml(value: &serde_json::Value, indent: usize) {
    let prefix = "  ".repeat(indent);
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                match val {
                    serde_json::Value::Object(_) => {
                        println!("{prefix}{key}:");
                        print_yaml(val, indent + 1);
                    }
                    serde_json::Value::Array(arr) => {
                        if arr.is_empty() {
                            println!("{prefix}{key}: []");
                        } else {
                            println!("{prefix}{key}:");
                            for item in arr {
                                if item.is_object() {
                                    println!("{prefix}- ");
                                    print_yaml(item, indent + 1);
                                } else {
                                    println!("{prefix}- {}", format_scalar(item));
                                }
                            }
                        }
                    }
                    _ => {
                        println!("{prefix}{key}: {}", format_scalar(val));
                    }
                }
            }
        }
        _ => {
            println!("{prefix}{}", format_scalar(value));
        }
    }
}

fn format_scalar(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => format!("'{s}'"),
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => format!("{v}"),
    }
}
