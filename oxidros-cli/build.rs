//! Build script for oxidros-cli
//!
//! Generates a compile-time `phf::Map<&str, &str>` mapping DDS type names
//! to JSON-serialized `TypeDescriptionMsg` values. This enables the CLI to
//! decode any known ROS2 message type without depending on `oxidros-msg` at runtime.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

use oxidros_build::msg::{Config, RosAvailability, detect_ros_availability};
use ros2msg::msg::{MessageSpecification, parse_message_file, parse_service_file};

fn main() {
    oxidros_build::ros2_env_var_changed();

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("type_registry.rs");

    let config = Config::builder().build();
    let availability = detect_ros_availability(&config);

    let share_paths = match &availability {
        RosAvailability::Sourced { share_paths }
        | RosAvailability::CommonInstall { share_paths } => share_paths.clone(),
        RosAvailability::NotAvailable => {
            // Generate an empty registry — can still compile without ROS2
            write_empty_registry(&dest_path);
            return;
        }
    };

    // Collect all .msg files from all packages, parse them into MessageSpecification
    // Collect all .msg and .srv files from all packages
    let mut all_specs: HashMap<String, MessageSpecification> = HashMap::new();
    // Track which keys are srv request/response (for DDS name generation)
    let mut srv_keys: Vec<String> = Vec::new();

    for share_path in &share_paths {
        let Ok(entries) = fs::read_dir(share_path) else {
            continue;
        };
        for entry in entries.flatten() {
            let pkg_path = entry.path();
            if !pkg_path.is_dir() {
                continue;
            }
            let pkg_name = match pkg_path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };

            // Scan .msg files
            let msg_dir = pkg_path.join("msg");
            if msg_dir.is_dir()
                && let Ok(msg_entries) = fs::read_dir(&msg_dir)
            {
                for msg_entry in msg_entries.flatten() {
                    let path = msg_entry.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("msg") {
                        continue;
                    }
                    if let Ok(spec) = parse_message_file(&pkg_name, &path) {
                        let key = format!("{}/msg/{}", spec.pkg_name, spec.msg_name);
                        all_specs.insert(key, spec);
                    }
                }
            }

            // Scan .srv files
            let srv_dir = pkg_path.join("srv");
            if srv_dir.is_dir()
                && let Ok(srv_entries) = fs::read_dir(&srv_dir)
            {
                for srv_entry in srv_entries.flatten() {
                    let path = srv_entry.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("srv") {
                        continue;
                    }
                    if let Ok(srv_spec) = parse_service_file(&pkg_name, &path) {
                        // Request sub-message
                        let req_key =
                            format!("{}/srv/{}", srv_spec.pkg_name, srv_spec.request.msg_name);
                        srv_keys.push(req_key.clone());
                        all_specs.insert(req_key, srv_spec.request);

                        // Response sub-message
                        let resp_key =
                            format!("{}/srv/{}", srv_spec.pkg_name, srv_spec.response.msg_name);
                        srv_keys.push(resp_key.clone());
                        all_specs.insert(resp_key, srv_spec.response);
                    }
                }
            }
        }
    }

    // Convert each MessageSpecification → TypeDescriptionMsg JSON
    // Collect entries first so strings live long enough for phf_codegen
    let mut entries: Vec<(String, String)> = Vec::new();

    for (key, spec) in &all_specs {
        let iface_type = if srv_keys.contains(key) { "srv" } else { "msg" };
        let json = match spec_to_type_description_json(spec, iface_type, &all_specs) {
            Some(j) => j,
            None => continue,
        };
        let dds_name = format!(
            "{}::{}::dds_::{}_",
            spec.pkg_name, iface_type, spec.msg_name
        );
        entries.push((dds_name, json));
    }

    // Sort for deterministic output
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut map = phf_codegen::Map::new();
    for (dds_name, json) in &entries {
        // Escape backslashes in JSON for embedding in raw string literal
        let escaped = json.replace('\\', "\\\\").replace('"', "\\\"");
        map.entry(dds_name.as_str(), format!("\"{}\"", escaped));
    }

    let mut out = String::new();
    out.push_str(&format!(
        "/// Auto-generated type registry ({} types: {} msg, {} srv)\n",
        entries.len(),
        entries.len() - srv_keys.len(),
        srv_keys.len()
    ));
    out.push_str("static REGISTRY: phf::Map<&'static str, &'static str> = ");
    out.push_str(&map.build().to_string());
    out.push_str(";\n");

    fs::write(&dest_path, out).expect("Failed to write type_registry.rs");

    println!(
        "cargo:info=Type registry generated with {} types ({} msg, {} srv)",
        entries.len(),
        entries.len() - srv_keys.len(),
        srv_keys.len()
    );
}

fn write_empty_registry(dest: &Path) {
    let out = "\
/// Auto-generated type registry (0 types — no ROS2 detected)\n\
static REGISTRY: phf::Map<&'static str, &'static str> = phf::phf_map! {};\n";
    fs::write(dest, out).expect("Failed to write empty type_registry.rs");
}

// ============================================================================
// MessageSpecification → TypeDescriptionMsg JSON conversion
// ============================================================================

use serde::Serialize;

/// Serializable mirror of ros2_types::types::TypeDescriptionMsg for build.rs use.
#[derive(Serialize)]
struct TypeDescriptionMsg {
    type_description: IndividualTypeDescription,
    referenced_type_descriptions: Vec<IndividualTypeDescription>,
}

#[derive(Serialize)]
struct IndividualTypeDescription {
    type_name: String,
    fields: Vec<FieldDesc>,
}

#[derive(Serialize)]
struct FieldDesc {
    name: String,
    #[serde(rename = "type")]
    field_type: FieldTypeDesc,
    default_value: String,
}

#[derive(Serialize)]
struct FieldTypeDesc {
    type_id: u8,
    capacity: u64,
    string_capacity: u64,
    nested_type_name: String,
}

// Field type constants (must match ros2_types::types)
const FIELD_TYPE_NESTED_TYPE: u8 = 1;
const FIELD_TYPE_INT8: u8 = 2;
const FIELD_TYPE_UINT8: u8 = 3;
const FIELD_TYPE_INT16: u8 = 4;
const FIELD_TYPE_UINT16: u8 = 5;
const FIELD_TYPE_INT32: u8 = 6;
const FIELD_TYPE_UINT32: u8 = 7;
const FIELD_TYPE_INT64: u8 = 8;
const FIELD_TYPE_UINT64: u8 = 9;
const FIELD_TYPE_FLOAT: u8 = 10;
const FIELD_TYPE_DOUBLE: u8 = 11;
const FIELD_TYPE_CHAR: u8 = 13;
const FIELD_TYPE_BOOLEAN: u8 = 15;
const FIELD_TYPE_BYTE: u8 = 16;
const FIELD_TYPE_STRING: u8 = 17;
const FIELD_TYPE_WSTRING: u8 = 18;
const FIELD_TYPE_BOUNDED_STRING: u8 = 21;
const FIELD_TYPE_BOUNDED_WSTRING: u8 = 22;

// Array offsets
const ARRAY_OFFSET: u8 = 48; // base + 48 = fixed-size array
const BOUNDED_SEQ_OFFSET: u8 = 96; // base + 96 = bounded sequence
const UNBOUNDED_SEQ_OFFSET: u8 = 144; // base + 144 = unbounded sequence

/// Map a ros2msg primitive type name to a base type_id.
fn primitive_type_id(type_name: &str) -> Option<u8> {
    match type_name {
        "bool" => Some(FIELD_TYPE_BOOLEAN),
        "byte" => Some(FIELD_TYPE_BYTE),
        "char" => Some(FIELD_TYPE_CHAR),
        "float32" => Some(FIELD_TYPE_FLOAT),
        "float64" => Some(FIELD_TYPE_DOUBLE),
        "int8" => Some(FIELD_TYPE_INT8),
        "uint8" => Some(FIELD_TYPE_UINT8),
        "int16" => Some(FIELD_TYPE_INT16),
        "uint16" => Some(FIELD_TYPE_UINT16),
        "int32" => Some(FIELD_TYPE_INT32),
        "uint32" => Some(FIELD_TYPE_UINT32),
        "int64" => Some(FIELD_TYPE_INT64),
        "uint64" => Some(FIELD_TYPE_UINT64),
        "string" => Some(FIELD_TYPE_STRING),
        "wstring" => Some(FIELD_TYPE_WSTRING),
        _ => None,
    }
}

/// Convert a ros2msg Field → our FieldDesc + optional nested type FQN.
/// Returns (FieldDesc, Option<nested_fqn>) where nested_fqn is the
/// fully-qualified name like "geometry_msgs/msg/Point" if this field
/// references another message type.
fn convert_field(field: &ros2msg::msg::Field) -> (FieldDesc, Option<String>) {
    let bt = &field.field_type.base_type;
    let type_name = bt.type_name.as_str();

    // Handle "time" and "duration" pseudo-primitives — they are nested types
    if type_name == "time" || type_name == "duration" {
        let nested_fqn = if type_name == "time" {
            "builtin_interfaces/msg/Time".to_string()
        } else {
            "builtin_interfaces/msg/Duration".to_string()
        };
        let ft = if field.field_type.is_array {
            make_container_type(FIELD_TYPE_NESTED_TYPE, &field.field_type, &nested_fqn)
        } else {
            FieldTypeDesc {
                type_id: FIELD_TYPE_NESTED_TYPE,
                capacity: 0,
                string_capacity: 0,
                nested_type_name: nested_fqn.clone(),
            }
        };
        return (
            FieldDesc {
                name: field.name.clone(),
                field_type: ft,
                default_value: String::new(),
            },
            Some(nested_fqn),
        );
    }

    // Try primitive type
    if let Some(base_id) = primitive_type_id(type_name) {
        let ft = if field.field_type.is_array {
            make_container_type(base_id, &field.field_type, "")
        } else if (type_name == "string" || type_name == "wstring")
            && bt.string_upper_bound.is_some()
        {
            // Bounded string (not in an array)
            let bound = bt.string_upper_bound.unwrap() as u64;
            let tid = if type_name == "string" {
                FIELD_TYPE_BOUNDED_STRING
            } else {
                FIELD_TYPE_BOUNDED_WSTRING
            };
            FieldTypeDesc {
                type_id: tid,
                capacity: 0,
                string_capacity: bound,
                nested_type_name: String::new(),
            }
        } else {
            FieldTypeDesc {
                type_id: base_id,
                capacity: 0,
                string_capacity: bt.string_upper_bound.map(|b| b as u64).unwrap_or(0),
                nested_type_name: String::new(),
            }
        };
        return (
            FieldDesc {
                name: field.name.clone(),
                field_type: ft,
                default_value: String::new(),
            },
            None,
        );
    }

    // Nested type
    let pkg = bt.pkg_name.as_deref().unwrap_or("UNKNOWN");
    let nested_fqn = format!("{}/msg/{}", pkg, type_name);
    let ft = if field.field_type.is_array {
        make_container_type(FIELD_TYPE_NESTED_TYPE, &field.field_type, &nested_fqn)
    } else {
        FieldTypeDesc {
            type_id: FIELD_TYPE_NESTED_TYPE,
            capacity: 0,
            string_capacity: 0,
            nested_type_name: nested_fqn.clone(),
        }
    };

    (
        FieldDesc {
            name: field.name.clone(),
            field_type: ft,
            default_value: String::new(),
        },
        Some(nested_fqn),
    )
}

/// Build the FieldTypeDesc for an array/sequence container.
fn make_container_type(
    base_id: u8,
    ros_type: &ros2msg::msg::Type,
    nested_name: &str,
) -> FieldTypeDesc {
    let is_nested = base_id == FIELD_TYPE_NESTED_TYPE;
    if let Some(size) = ros_type.array_size {
        if ros_type.is_upper_bound {
            // Bounded sequence: type[<=N]
            FieldTypeDesc {
                type_id: base_id + BOUNDED_SEQ_OFFSET,
                capacity: size as u64,
                string_capacity: 0,
                nested_type_name: if is_nested {
                    nested_name.to_string()
                } else {
                    String::new()
                },
            }
        } else {
            // Fixed-size array: type[N]
            FieldTypeDesc {
                type_id: base_id + ARRAY_OFFSET,
                capacity: size as u64,
                string_capacity: 0,
                nested_type_name: if is_nested {
                    nested_name.to_string()
                } else {
                    String::new()
                },
            }
        }
    } else {
        // Unbounded dynamic array: type[]
        FieldTypeDesc {
            type_id: base_id + UNBOUNDED_SEQ_OFFSET,
            capacity: 0,
            string_capacity: 0,
            nested_type_name: if is_nested {
                nested_name.to_string()
            } else {
                String::new()
            },
        }
    }
}

/// Build the full TypeDescriptionMsg JSON for a message spec.
/// Recursively resolves nested types from `all_specs`.
fn spec_to_type_description_json(
    spec: &MessageSpecification,
    iface_type: &str,
    all_specs: &HashMap<String, MessageSpecification>,
) -> Option<String> {
    let mut referenced: Vec<IndividualTypeDescription> = Vec::new();
    let mut visited: Vec<String> = Vec::new();

    let fqn = format!("{}/{}/{}", spec.pkg_name, iface_type, spec.msg_name);
    let main_desc =
        build_individual_description(&fqn, spec, all_specs, &mut referenced, &mut visited);

    let msg = TypeDescriptionMsg {
        type_description: main_desc,
        referenced_type_descriptions: referenced,
    };

    serde_json::to_string(&msg).ok()
}

/// Build an IndividualTypeDescription for a single message, and collect
/// all referenced (nested) types into `referenced`.
fn build_individual_description(
    fqn: &str,
    spec: &MessageSpecification,
    all_specs: &HashMap<String, MessageSpecification>,
    referenced: &mut Vec<IndividualTypeDescription>,
    visited: &mut Vec<String>,
) -> IndividualTypeDescription {
    let mut fields = Vec::new();

    for field in &spec.fields {
        let (fd, nested_fqn) = convert_field(field);
        fields.push(fd);

        // If nested, recursively add the referenced type
        if let Some(ref nfqn) = nested_fqn
            && !visited.contains(nfqn)
        {
            visited.push(nfqn.clone());
            if let Some(nested_spec) = all_specs.get(nfqn) {
                let nested_desc =
                    build_individual_description(nfqn, nested_spec, all_specs, referenced, visited);
                referenced.push(nested_desc);
            }
        }
    }

    IndividualTypeDescription {
        type_name: fqn.to_string(),
        fields,
    }
}
