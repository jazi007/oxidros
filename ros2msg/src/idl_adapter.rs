//! MSG/SRV/Action to IDL converter
//!
//! This module converts ROS2 message, service, and action definitions to IDL format,
//! matching the behavior of rosidl_adapter.

use crate::msg::types::{AnnotationValue, Field, Value};
use crate::msg::validation::PrimitiveValue;
use crate::{ActionSpecification, MessageSpecification, ServiceSpecification};
use std::collections::BTreeSet;
use std::fmt::Write;

/// Convert a MSG type to its IDL equivalent
fn msg_type_to_idl(msg_type: &str) -> &'static str {
    match msg_type {
        "bool" => "boolean",
        "byte" => "octet",
        // Note: ROS2 MSG "char" is an alias for uint8 (unsigned 8-bit integer)
        // This is different from IDL "char" which is a signed 8-bit character
        "char" | "uint8" => "uint8",
        "int8" => "int8",
        "int16" => "int16",
        "uint16" => "uint16",
        "int32" => "int32",
        "uint32" => "uint32",
        "int64" => "int64",
        "uint64" => "uint64",
        "float32" => "float",
        "float64" => "double",
        "string" => "string",
        "wstring" => "wstring",
        _ => panic!("Unknown primitive type: {msg_type}"),
    }
}

/// Get the IDL type string for a field type
/// For fixed-size arrays, returns the raw array syntax (e.g., "double[36]")
/// which will be replaced with typedef names later
fn get_idl_type(field: &Field) -> String {
    let base_type = if field.field_type.is_primitive_type() {
        msg_type_to_idl(&field.field_type.base_type.type_name)
    } else {
        // Nested type: just use the type name, includes handle the module structure
        &field.field_type.base_type.type_name
    };

    let mut result = String::new();

    // Handle arrays and sequences
    if field.field_type.is_array {
        if let Some(size) = field.field_type.array_size {
            if field.field_type.is_upper_bound {
                // Bounded sequence: sequence<type, N>
                write!(result, "sequence<{base_type}, {size}>").unwrap();
            } else {
                // Fixed-size array: type[size]
                write!(result, "{base_type}[{size}]").unwrap();
            }
        } else {
            // Unbounded sequence: sequence<type>
            write!(result, "sequence<{base_type}>").unwrap();
        }
    } else if let Some(bound) = field.field_type.base_type.string_upper_bound {
        // Bounded string: string<N>
        write!(result, "{base_type}<{bound}>").unwrap();
    } else {
        // Simple type or unbounded string
        result = base_type.to_string();
    }

    result
}

/// Get the typedef name for a fixed-size array type
/// e.g., `"double[36]"` -> `"double__36"`
/// For nested types, converts `"pkg::msg::Type[36]"` -> `"pkg_msg_Type__36"`
fn get_typedef_name(array_type: &str) -> String {
    if let Some(bracket_pos) = array_type.find('[') {
        let base = &array_type[..bracket_pos];
        let size = &array_type[bracket_pos + 1..array_type.len() - 1];
        // Replace colons with underscores for nested types
        let sanitized_base = base.replace("::", "_");
        format!("{sanitized_base}__{size}")
    } else {
        array_type.replace("::", "_")
    }
}

/// Get the typedef declaration for a fixed-size array type
/// e.g., `"double[36]"` -> `"typedef double double__36[36];"`
fn get_typedef_declaration(array_type: &str) -> String {
    if let Some(bracket_pos) = array_type.find('[') {
        let base = &array_type[..bracket_pos];
        let size_part = &array_type[bracket_pos..]; // includes "[36]"
        let typedef_name = get_typedef_name(array_type);
        format!("typedef {base} {typedef_name}{size_part};")
    } else {
        // Shouldn't happen, but fallback
        format!("typedef {array_type};")
    }
}

/// Collect all fixed-size array typedefs from fields
fn collect_array_typedefs(fields: &[Field]) -> BTreeSet<String> {
    let mut typedefs = BTreeSet::new();

    for field in fields {
        let idl_type = get_idl_type(field);
        // Check if it's a fixed-size array (contains "[")
        if idl_type.contains('[') && !idl_type.starts_with("sequence<") {
            typedefs.insert(idl_type);
        }
    }

    typedefs
}

/// Convert a `PrimitiveValue` to an IDL literal string
fn primitive_value_to_idl(_idl_type: &str, value: &PrimitiveValue) -> String {
    match value {
        PrimitiveValue::Bool(b) => {
            if *b {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        PrimitiveValue::String(s) => {
            format!("\"{}\"", s.replace('\\', r"\\").replace('"', r#"\""#))
        }
        _ => value.to_string(),
    }
}

/// Convert a Value to an IDL literal string
fn value_to_idl(idl_type: &str, value: &Value) -> String {
    match value {
        Value::Primitive(pv) => primitive_value_to_idl(idl_type, pv),
        Value::Array(values) => {
            // Array literals are represented as strings in IDL
            let mut result = String::from("[");
            for (i, v) in values.iter().enumerate() {
                if i > 0 {
                    result.push_str(", ");
                }
                result.push_str(&v.to_string());
            }
            result.push(']');
            format!("\"{}\"", result.replace('\\', r"\\").replace('"', r#"\""#))
        }
    }
}

/// Get the include file path for a type
fn get_include_file(field: &Field) -> Option<String> {
    if field.field_type.is_primitive_type() {
        None
    } else {
        Some(format!(
            "{}/msg/{}.idl",
            field.field_type.base_type.pkg_name.as_deref().unwrap_or(""),
            field.field_type.base_type.type_name
        ))
    }
}

/// Helper to get comment annotation as a vector of strings
fn get_comment_lines(
    annotations: &std::collections::HashMap<String, AnnotationValue>,
) -> Vec<String> {
    if let Some(comment) = annotations.get("comment") {
        match comment {
            AnnotationValue::StringList(lines) => lines.clone(),
            AnnotationValue::String(s) => vec![s.clone()],
            AnnotationValue::Bool(_) => Vec::new(),
        }
    } else {
        Vec::new()
    }
}

/// Helper to get unit annotation as a string
fn get_unit_str(
    annotations: &std::collections::HashMap<String, AnnotationValue>,
) -> Option<String> {
    if let Some(unit) = annotations.get("unit") {
        match unit {
            AnnotationValue::String(s) => Some(s.clone()),
            AnnotationValue::StringList(list) if !list.is_empty() => Some(list[0].clone()),
            _ => None,
        }
    } else {
        None
    }
}

/// Convert a message to IDL format
///
/// # Panics
/// Panics if writing to the output string fails (which should never happen in practice).
#[must_use]
pub fn message_to_idl(msg: &MessageSpecification, package_name: &str, input_file: &str) -> String {
    let mut output = String::new();

    // Header comment
    writeln!(
        output,
        "// generated from rosidl_adapter/resource/msg.idl.em"
    )
    .unwrap();
    writeln!(output, "// with input from {package_name}/{input_file}").unwrap();
    writeln!(
        output,
        "// generated code does not contain a copyright notice"
    )
    .unwrap();
    writeln!(output).unwrap();

    // Collect include files
    let mut includes = BTreeSet::new();
    for field in &msg.fields {
        if let Some(include) = get_include_file(field) {
            includes.insert(include);
        }
    }

    // Write includes
    for include in &includes {
        writeln!(output, "#include \"{include}\"").unwrap();
    }
    if !includes.is_empty() {
        writeln!(output).unwrap();
    }

    // Module declaration
    writeln!(output, "module {package_name} {{").unwrap();
    writeln!(output, "  module msg {{").unwrap();

    // Collect and write array typedefs
    let typedefs = collect_array_typedefs(&msg.fields);
    for typedef_type in &typedefs {
        writeln!(output, "    {}", get_typedef_declaration(typedef_type)).unwrap();
    }

    // Write struct
    write_struct_idl(&mut output, msg, "    ", &typedefs).unwrap();

    writeln!(output, "  }};").unwrap();
    writeln!(output, "}};").unwrap();

    output
}

/// Write a struct definition in IDL format
fn write_struct_idl(
    output: &mut String,
    msg: &MessageSpecification,
    indent: &str,
    typedefs: &BTreeSet<String>,
) -> std::fmt::Result {
    // Constants module (if any)
    if !msg.constants.is_empty() {
        writeln!(output, "{indent}module {}_Constants {{", msg.msg_name)?;
        for constant in &msg.constants {
            // Comments
            let comments = get_comment_lines(&constant.annotations);
            if !comments.is_empty() {
                writeln!(output, "{indent}  @verbatim (language=\"comment\", text=")?;
                for (i, line) in comments.iter().enumerate() {
                    write!(
                        output,
                        "{indent}    \"{}\"",
                        line.replace('\\', r"\\").replace('"', r#"\""#)
                    )?;
                    if i < comments.len() - 1 {
                        write!(output, " \"\\n\"")?;
                    }
                    writeln!(output)?;
                }
                writeln!(output, "{indent}  )")?;
            }

            let idl_type = msg_type_to_idl(&constant.type_name);
            let idl_value = primitive_value_to_idl(idl_type, &constant.value);
            writeln!(
                output,
                "{indent}  const {idl_type} {} = {idl_value};",
                constant.name
            )?;
        }
        writeln!(output, "{indent}}}; ")?;
    }

    // Struct comment
    let comments = get_comment_lines(&msg.annotations);
    if !comments.is_empty() {
        writeln!(output, "{indent}@verbatim (language=\"comment\", text=")?;
        for (i, line) in comments.iter().enumerate() {
            write!(
                output,
                "{indent}  \"{}\"",
                line.replace('\\', r"\\").replace('"', r#"\""#)
            )?;
            if i < comments.len() - 1 {
                write!(output, " \"\\n\"")?;
            }
            writeln!(output)?;
        }
        writeln!(output, "{indent})")?;
    }

    writeln!(output, "{indent}struct {} {{", msg.msg_name)?;

    // Fields
    if msg.fields.is_empty() {
        writeln!(
            output,
            "{indent}  uint8 structure_needs_at_least_one_member;"
        )?;
    } else {
        for (i, field) in msg.fields.iter().enumerate() {
            if i > 0 {
                writeln!(output)?;
            }

            // Field comment
            let comments = get_comment_lines(&field.annotations);
            if !comments.is_empty() {
                writeln!(output, "{indent}  @verbatim (language=\"comment\", text=")?;
                for (j, line) in comments.iter().enumerate() {
                    write!(
                        output,
                        "{indent}    \"{}\"",
                        line.replace('\\', r"\\").replace('"', r#"\""#)
                    )?;
                    if j < comments.len() - 1 {
                        write!(output, " \"\\n\"")?;
                    }
                    writeln!(output)?;
                }
                writeln!(output, "{indent}  )")?;
            }

            // Default value annotation
            if let Some(ref default_value) = field.default_value {
                let idl_type = get_idl_type(field);
                let idl_value = value_to_idl(&idl_type, default_value);
                writeln!(output, "{indent}  @default (value={idl_value})")?;
            }

            // Unit annotation
            if let Some(unit_str) = get_unit_str(&field.annotations) {
                writeln!(
                    output,
                    "{indent}  @unit (value=\"{}\")",
                    unit_str.replace('\\', r"\\").replace('"', r#"\""#)
                )?;
            }

            let mut idl_type = get_idl_type(field);
            // Replace fixed-size array with typedef name if applicable
            if typedefs.contains(&idl_type) {
                idl_type = get_typedef_name(&idl_type);
            }
            writeln!(output, "{indent}  {idl_type} {};", field.name)?;
        }
    }

    writeln!(output, "{indent}  }};")?;

    Ok(())
}

/// Convert a service to IDL format
///
/// # Panics
/// Panics if writing to the output string fails (which should never happen in practice).
#[must_use]
pub fn service_to_idl(srv: &ServiceSpecification, package_name: &str, input_file: &str) -> String {
    let mut output = String::new();

    // Header comment
    writeln!(
        output,
        "// generated from rosidl_adapter/resource/srv.idl.em"
    )
    .unwrap();
    writeln!(output, "// with input from {package_name}/{input_file}").unwrap();
    writeln!(
        output,
        "// generated code does not contain a copyright notice"
    )
    .unwrap();
    writeln!(output).unwrap();

    // Collect includes from both request and response
    let mut includes = BTreeSet::new();
    for field in srv.request.fields.iter().chain(srv.response.fields.iter()) {
        if let Some(include) = get_include_file(field) {
            includes.insert(include);
        }
    }

    // Write includes
    for include in &includes {
        writeln!(output, "#include \"{include}\"").unwrap();
    }
    if !includes.is_empty() {
        writeln!(output).unwrap();
    }

    // Module declaration
    writeln!(output, "module {package_name} {{").unwrap();
    writeln!(output, "  module srv {{").unwrap();

    // Collect typedefs from both request and response
    let mut typedefs = collect_array_typedefs(&srv.request.fields);
    typedefs.extend(collect_array_typedefs(&srv.response.fields));

    // Write typedefs
    for typedef_type in &typedefs {
        writeln!(output, "    {}", get_typedef_declaration(typedef_type)).unwrap();
    }

    // Request struct
    write_struct_idl(&mut output, &srv.request, "    ", &typedefs).unwrap();
    writeln!(output).unwrap();

    // Response struct
    write_struct_idl(&mut output, &srv.response, "    ", &typedefs).unwrap();

    writeln!(output, "  }};").unwrap();
    writeln!(output, "}};").unwrap();

    output
}

/// Convert an action to IDL format
///
/// # Panics
/// Panics if writing to the output string fails (which should never happen in practice).
#[must_use]
pub fn action_to_idl(action: &ActionSpecification, package_name: &str, input_file: &str) -> String {
    let mut output = String::new();

    // Header comment
    writeln!(
        output,
        "// generated from rosidl_adapter/resource/action.idl.em"
    )
    .unwrap();
    writeln!(output, "// with input from {package_name}/{input_file}").unwrap();
    writeln!(
        output,
        "// generated code does not contain a copyright notice"
    )
    .unwrap();
    writeln!(output).unwrap();

    // Collect includes from goal, result, and feedback
    let mut includes = BTreeSet::new();
    for field in action
        .goal
        .fields
        .iter()
        .chain(action.result.fields.iter())
        .chain(action.feedback.fields.iter())
    {
        if let Some(include) = get_include_file(field) {
            includes.insert(include);
        }
    }

    // Write includes
    for include in &includes {
        writeln!(output, "#include \"{include}\"").unwrap();
    }
    if !includes.is_empty() {
        writeln!(output).unwrap();
    }

    // Module declaration
    writeln!(output, "module {package_name} {{").unwrap();
    writeln!(output, "  module action {{").unwrap();

    // Collect typedefs from goal, result, and feedback
    let mut typedefs = collect_array_typedefs(&action.goal.fields);
    typedefs.extend(collect_array_typedefs(&action.result.fields));
    typedefs.extend(collect_array_typedefs(&action.feedback.fields));

    // Write typedefs
    for typedef_type in &typedefs {
        writeln!(output, "    {}", get_typedef_declaration(typedef_type)).unwrap();
    }

    // Goal struct
    write_struct_idl(&mut output, &action.goal, "    ", &typedefs).unwrap();
    writeln!(output).unwrap();

    // Result struct
    write_struct_idl(&mut output, &action.result, "    ", &typedefs).unwrap();
    writeln!(output).unwrap();

    // Feedback struct
    write_struct_idl(&mut output, &action.feedback, "    ", &typedefs).unwrap();

    writeln!(output, "  }};").unwrap();
    writeln!(output, "}};").unwrap();

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_action_string, parse_message_string, parse_service_string};

    #[test]
    fn test_simple_message_to_idl() {
        let msg_content = r"
int32 x
int32 y
string name
";
        let msg = parse_message_string("test_msgs", "Point", msg_content).unwrap();
        let idl = message_to_idl(&msg, "test_msgs", "msg/Point.msg");

        assert!(idl.contains("module test_msgs {"));
        assert!(idl.contains("module msg {"));
        assert!(idl.contains("struct Point {"));
        assert!(idl.contains("int32 x;"));
        assert!(idl.contains("int32 y;"));
        assert!(idl.contains("string name;"));
    }

    #[test]
    fn test_empty_message_to_idl() {
        let msg_content = "";
        let msg = parse_message_string("std_msgs", "Empty", msg_content).unwrap();
        let idl = message_to_idl(&msg, "std_msgs", "msg/Empty.msg");

        assert!(idl.contains("struct Empty {"));
        assert!(idl.contains("uint8 structure_needs_at_least_one_member;"));
    }

    #[test]
    fn test_array_types_to_idl() {
        let msg_content = r"
int32[5] fixed_array
int32[] unbounded_array
int32[<=10] bounded_array
";
        let msg = parse_message_string("test_msgs", "Arrays", msg_content).unwrap();
        let idl = message_to_idl(&msg, "test_msgs", "msg/Arrays.msg");

        // Fixed-size arrays now use typedefs
        assert!(idl.contains("typedef int32 int32__5[5];"));
        assert!(idl.contains("int32__5 fixed_array;"));
        assert!(idl.contains("sequence<int32> unbounded_array;"));
        assert!(idl.contains("sequence<int32, 10> bounded_array;"));
    }

    #[test]
    fn test_byte_type_conversion() {
        let msg_content = r"
byte data
byte[] array_data
";
        let msg = parse_message_string("test_msgs", "ByteTest", msg_content).unwrap();
        let idl = message_to_idl(&msg, "test_msgs", "msg/ByteTest.msg");

        // byte should convert to octet in IDL
        assert!(idl.contains("octet data;"));
        assert!(idl.contains("sequence<octet> array_data;"));
    }

    #[test]
    fn test_service_to_idl() {
        let srv_content = r"
int32 a
int32 b
---
int32 sum
";
        let srv = parse_service_string("test_srvs", "AddTwoInts", srv_content).unwrap();
        let idl = service_to_idl(&srv, "test_srvs", "srv/AddTwoInts.srv");

        assert!(idl.contains("module test_srvs {"));
        assert!(idl.contains("module srv {"));
        assert!(idl.contains("struct AddTwoInts_Request {"));
        assert!(idl.contains("struct AddTwoInts_Response {"));
        assert!(idl.contains("int32 a;"));
        assert!(idl.contains("int32 b;"));
        assert!(idl.contains("int32 sum;"));
    }

    #[test]
    fn test_action_to_idl() {
        let action_content = r"
int32 order
---
int32[] sequence
---
int32[] partial_sequence
";
        let action = parse_action_string("test_actions", "Fibonacci", action_content).unwrap();
        let idl = action_to_idl(&action, "test_actions", "action/Fibonacci.action");

        assert!(idl.contains("module test_actions {"));
        assert!(idl.contains("module action {"));
        assert!(idl.contains("struct Fibonacci_Goal {"));
        assert!(idl.contains("struct Fibonacci_Result {"));
        assert!(idl.contains("struct Fibonacci_Feedback {"));
        assert!(idl.contains("int32 order;"));
        assert!(idl.contains("sequence<int32> sequence;"));
        assert!(idl.contains("sequence<int32> partial_sequence;"));
    }

    #[test]
    fn test_default_values_to_idl() {
        let msg_content = r#"
int32 x 10
string name "default"
bool flag true
"#;
        let msg = parse_message_string("test_msgs", "Defaults", msg_content).unwrap();
        let idl = message_to_idl(&msg, "test_msgs", "msg/Defaults.msg");

        assert!(idl.contains("@default (value=10)"));
        assert!(idl.contains("@default (value=\"default\")"));
        assert!(idl.contains("@default (value=TRUE)"));
    }

    #[test]
    fn test_constants_to_idl() {
        let msg_content = r#"
int32 CONSTANT_VALUE=42
string CONSTANT_STRING="hello"
bool CONSTANT_BOOL=true

int32 data
"#;
        let msg = parse_message_string("test_msgs", "Constants", msg_content).unwrap();
        let idl = message_to_idl(&msg, "test_msgs", "msg/Constants.msg");

        assert!(idl.contains("Constants_Constants {"));
        assert!(idl.contains("const int32 CONSTANT_VALUE = 42;"));
        assert!(idl.contains("const string CONSTANT_STRING = \"hello\";"));
        assert!(idl.contains("const boolean CONSTANT_BOOL = TRUE;"));
    }
}
