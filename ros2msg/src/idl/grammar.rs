//! IDL Grammar Parser
//!
//! Complete implementation of the ROS2 IDL parser using pest.
//! This module provides full IDL parsing functionality based on the grammar.lark specification.
//!
//! The parser supports:
//! - Comments (7.2.2)
//! - Identifiers (7.2.3)
//! - Literals (7.2.6) - integers, floats, chars, strings, booleans
//! - Preprocessing (#include directives) (7.3)
//! - Core Data Types (7.4.1)
//! - Annotations (7.4.15.4.2)
//! - Modules, constants, type declarations
//! - Structures, enums
//! - Sequences, arrays, strings
//! - Expression evaluation with operators
//! - Complete grammar rules from ROS2 IDL specification

use std::collections::HashMap;
use std::path::PathBuf;

use super::errors::{IdlError, IdlResult};
use super::parser_pest;
use super::types::{
    ACTION_FEEDBACK_MESSAGE_SUFFIX, ACTION_FEEDBACK_SUFFIX, ACTION_GOAL_SERVICE_SUFFIX,
    ACTION_GOAL_SUFFIX, ACTION_RESULT_SERVICE_SUFFIX, ACTION_RESULT_SUFFIX, Action, Annotatable,
    Array, Constant, IdlContent, IdlContentElement, IdlFile, IdlLocator, IdlType, Include, Member,
    Message, NamespacedType, SERVICE_EVENT_MESSAGE_SUFFIX, SERVICE_REQUEST_MESSAGE_SUFFIX,
    SERVICE_RESPONSE_MESSAGE_SUFFIX, Service, Structure,
};
use super::values::IdlValue;

/// Convert annotation parameters from parser format to `IdlValue`
fn convert_annotation_params(params: &[(String, IdlValue)]) -> IdlValue {
    if params.is_empty() {
        return IdlValue::Null;
    }
    let mut map = HashMap::new();
    for (key, value) in params {
        map.insert(key.clone(), value.clone());
    }
    IdlValue::Object(map)
}

/// Collect typedefs from definitions
fn collect_typedefs(definitions: &[parser_pest::IdlDefinition]) -> HashMap<String, IdlType> {
    let mut typedefs = HashMap::new();

    for def in definitions {
        if let parser_pest::IdlDefinition::Typedef(typedef) = def {
            // Build the resolved type
            let resolved_type = if typedef.array_sizes.is_empty() {
                // Simple typedef (no array)
                typedef.base_type.clone()
            } else if typedef.array_sizes.len() == 1 {
                // Single-dimension array typedef like `typedef double double__36[36];`
                IdlType::Array(Array::new(
                    typedef.base_type.clone(),
                    typedef.array_sizes[0],
                ))
            } else {
                // Multi-dimensional arrays - nest them
                let mut current_type = typedef.base_type.clone();
                for &size in typedef.array_sizes.iter().rev() {
                    current_type = IdlType::Array(Array::new(current_type, size));
                }
                current_type
            };

            typedefs.insert(typedef.name.clone(), resolved_type);
        }
    }

    // Resolve chained typedefs: if typedef A points to typedef B, resolve A to B's target
    // Repeat until no more changes to handle multi-level chains
    let mut changed = true;
    while changed {
        changed = false;
        let keys: Vec<_> = typedefs.keys().cloned().collect();
        for key in keys {
            let mut resolved = typedefs.get(&key).cloned().unwrap();
            let original = resolved.clone();
            resolve_typedef(&mut resolved, &typedefs);
            if resolved != original {
                typedefs.insert(key, resolved);
                changed = true;
            }
        }
    }

    typedefs
}

/// Resolve typedef references in a member's type
fn resolve_typedef(member_type: &mut IdlType, typedefs: &HashMap<String, IdlType>) {
    match member_type {
        IdlType::Named(named) => {
            if let Some(resolved) = typedefs.get(&named.name) {
                *member_type = resolved.clone();
            }
        }
        IdlType::Array(arr) => {
            resolve_typedef(&mut arr.value_type, typedefs);
        }
        IdlType::BoundedSequence(seq) => {
            resolve_typedef(&mut seq.value_type, typedefs);
        }
        IdlType::UnboundedSequence(seq) => {
            resolve_typedef(&mut seq.value_type, typedefs);
        }
        _ => {}
    }
}

/// Resolve all typedef references in a structure's members
fn resolve_typedefs_in_structure(structure: &mut Structure, typedefs: &HashMap<String, IdlType>) {
    for member in &mut structure.members {
        resolve_typedef(&mut member.member_type, typedefs);
    }
}

/// Convert a parsed struct definition to a Message
fn convert_struct_to_message(
    struct_def: &parser_pest::IdlStruct,
    namespaces: &[String],
    constants_map: &HashMap<String, Vec<Constant>>,
    typedefs: &HashMap<String, IdlType>,
) -> Message {
    let namespaced_type = NamespacedType::new(namespaces.to_vec(), struct_def.name.clone());

    let members: Vec<Member> = struct_def
        .fields
        .iter()
        .map(|f| {
            let mut member = Member::new(f.field_type.clone(), f.name.clone());
            member.annotations.annotations = f
                .annotations
                .iter()
                .map(|a| super::types::Annotation {
                    name: a.name.clone(),
                    value: convert_annotation_params(&a.params),
                })
                .collect();
            member
        })
        .collect();

    let mut structure = Structure::new(namespaced_type);
    structure.members = members;
    structure.annotations.annotations = struct_def
        .annotations
        .iter()
        .map(|a| super::types::Annotation {
            name: a.name.clone(),
            value: convert_annotation_params(&a.params),
        })
        .collect();

    // Resolve typedef references in members
    resolve_typedefs_in_structure(&mut structure, typedefs);

    let mut message = Message::new(structure);
    if let Some(constants) = constants_map.get(&struct_def.name) {
        message.constants.clone_from(constants);
    }

    message
}

/// Collect constants from _Constants modules
fn collect_constants(definitions: &[parser_pest::IdlDefinition]) -> HashMap<String, Vec<Constant>> {
    let mut constants_map = HashMap::new();

    for def in definitions {
        if let parser_pest::IdlDefinition::Module(module) = def
            && let Some(base_name) = module.name.strip_suffix("_Constants")
        {
            let mut constants = Vec::new();
            for const_def in &module.definitions {
                if let parser_pest::IdlDefinition::Constant(c) = const_def {
                    let mut constant =
                        Constant::new(c.name.clone(), c.const_type.clone(), c.value.clone());
                    constant.annotations.annotations = c
                        .annotations
                        .iter()
                        .map(|a| super::types::Annotation {
                            name: a.name.clone(),
                            value: convert_annotation_params(&a.params),
                        })
                        .collect();
                    constants.push(constant);
                }
            }
            if !constants.is_empty() {
                constants_map.insert(base_name.to_string(), constants);
            }
        }
    }
    constants_map
}
/// Build a map of struct name -> Message for a set of definitions
fn build_message_map(
    definitions: &[parser_pest::IdlDefinition],
    namespaces: &[String],
    constants_map: &HashMap<String, Vec<Constant>>,
    typedefs: &HashMap<String, IdlType>,
) -> HashMap<String, Message> {
    let mut messages = HashMap::new();

    for def in definitions {
        if let parser_pest::IdlDefinition::Struct(struct_def) = def {
            let message =
                convert_struct_to_message(struct_def, namespaces, constants_map, typedefs);
            messages.insert(struct_def.name.clone(), message);
        }
    }

    messages
}

/// Process a `srv` module and extract Service definitions
fn process_srv_module(
    module: &parser_pest::IdlModule,
    namespaces: &[String],
) -> Vec<IdlContentElement> {
    let mut elements = Vec::new();
    let constants_map = collect_constants(&module.definitions);
    let typedefs = collect_typedefs(&module.definitions);

    let mut srv_namespaces = namespaces.to_vec();
    srv_namespaces.push(module.name.clone());

    let mut messages = build_message_map(
        &module.definitions,
        &srv_namespaces,
        &constants_map,
        &typedefs,
    );

    // Find all service base names by looking for _Request suffix
    let request_names: Vec<String> = messages
        .keys()
        .filter(|name| name.ends_with(SERVICE_REQUEST_MESSAGE_SUFFIX))
        .cloned()
        .collect();

    for request_name in request_names {
        let Some(base) = request_name.strip_suffix(SERVICE_REQUEST_MESSAGE_SUFFIX) else {
            continue;
        };

        let response_name = format!("{base}{SERVICE_RESPONSE_MESSAGE_SUFFIX}");
        let event_name = format!("{base}{SERVICE_EVENT_MESSAGE_SUFFIX}");

        // Both request and response must exist
        let (Some(request), Some(response)) = (
            messages.remove(&request_name),
            messages.remove(&response_name),
        ) else {
            // Put request back if response doesn't exist
            if let Some(req) = messages.remove(&request_name) {
                messages.insert(request_name, req);
            }
            continue;
        };

        // Clear structure annotations for request/response to match Python behavior
        // Python's rosidl_parser doesn't include structure annotations for service messages
        let mut request = request;
        let mut response = response;
        request.structure.annotations = Annotatable::new();
        response.structure.annotations = Annotatable::new();

        let mut service = Service::new(
            NamespacedType::new(srv_namespaces.clone(), base),
            request,
            response,
        );

        // Attach event message if present
        if let Some(event) = messages.remove(&event_name) {
            service.event_message = event;
        }

        elements.push(IdlContentElement::Service(service));
    }

    // Any remaining messages stay as messages
    for message in messages.into_values() {
        elements.push(IdlContentElement::Message(message));
    }

    // Process nested modules (but not as srv)
    for def in &module.definitions {
        if let parser_pest::IdlDefinition::Module(nested) = def
            && !nested.name.ends_with("_Constants")
        {
            elements.extend(convert_definitions_with_namespace(
                &nested.definitions,
                &srv_namespaces,
            ));
        }
    }

    // Process constants at srv level
    for def in &module.definitions {
        if let parser_pest::IdlDefinition::Constant(c) = def {
            elements.push(IdlContentElement::Constant(Constant::new(
                c.name.clone(),
                c.const_type.clone(),
                c.value.clone(),
            )));
        }
    }

    elements
}

/// Process an `action` module and extract Action and Service definitions
#[allow(clippy::too_many_lines)]
fn process_action_module(
    module: &parser_pest::IdlModule,
    namespaces: &[String],
) -> Vec<IdlContentElement> {
    let mut elements = Vec::new();
    let constants_map = collect_constants(&module.definitions);
    let typedefs = collect_typedefs(&module.definitions);

    let mut action_namespaces = namespaces.to_vec();
    action_namespaces.push(module.name.clone());

    let mut messages = build_message_map(
        &module.definitions,
        &action_namespaces,
        &constants_map,
        &typedefs,
    );

    // First, extract services (SendGoal, GetResult) from the action module
    let mut services: HashMap<String, Service> = HashMap::new();

    let request_names: Vec<String> = messages
        .keys()
        .filter(|name| name.ends_with(SERVICE_REQUEST_MESSAGE_SUFFIX))
        .cloned()
        .collect();

    for request_name in request_names {
        let Some(base) = request_name.strip_suffix(SERVICE_REQUEST_MESSAGE_SUFFIX) else {
            continue;
        };

        let response_name = format!("{base}{SERVICE_RESPONSE_MESSAGE_SUFFIX}");
        let event_name = format!("{base}{SERVICE_EVENT_MESSAGE_SUFFIX}");

        let (Some(request), Some(response)) = (
            messages.remove(&request_name),
            messages.remove(&response_name),
        ) else {
            continue;
        };

        // Clear structure annotations for request/response to match Python behavior
        let mut request = request;
        let mut response = response;
        request.structure.annotations = Annotatable::new();
        response.structure.annotations = Annotatable::new();

        let mut service = Service::new(
            NamespacedType::new(action_namespaces.clone(), base),
            request,
            response,
        );

        if let Some(event) = messages.remove(&event_name) {
            service.event_message = event;
        }

        services.insert(base.to_string(), service);
    }

    // Now find actions by looking for _Goal suffix
    let goal_names: Vec<String> = messages
        .keys()
        .filter(|name| name.ends_with(ACTION_GOAL_SUFFIX))
        .cloned()
        .collect();

    for goal_name in goal_names {
        let Some(base) = goal_name.strip_suffix(ACTION_GOAL_SUFFIX) else {
            continue;
        };

        let result_name = format!("{base}{ACTION_RESULT_SUFFIX}");
        let feedback_name = format!("{base}{ACTION_FEEDBACK_SUFFIX}");
        let feedback_msg_name = format!("{base}{ACTION_FEEDBACK_MESSAGE_SUFFIX}");
        let send_goal_name = format!("{base}{ACTION_GOAL_SERVICE_SUFFIX}");
        let get_result_name = format!("{base}{ACTION_RESULT_SERVICE_SUFFIX}");

        // Goal, Result, Feedback must all exist
        let (Some(goal), Some(result), Some(feedback)) = (
            messages.remove(&goal_name),
            messages.remove(&result_name),
            messages.remove(&feedback_name),
        ) else {
            continue;
        };

        // Clear structure annotations for goal/result/feedback to match Python behavior
        let mut goal = goal;
        let mut result = result;
        let mut feedback = feedback;
        goal.structure.annotations = Annotatable::new();
        result.structure.annotations = Annotatable::new();
        feedback.structure.annotations = Annotatable::new();

        let mut action = Action::new(
            NamespacedType::new(action_namespaces.clone(), base),
            goal,
            result,
            feedback,
        );

        // Attach optional feedback message
        if let Some(feedback_msg) = messages.remove(&feedback_msg_name) {
            action.feedback_message = feedback_msg;
        }

        // Attach services
        if let Some(send_goal) = services.remove(&send_goal_name) {
            action.send_goal_service = send_goal;
        }
        if let Some(get_result) = services.remove(&get_result_name) {
            action.get_result_service = get_result;
        }

        // Add implicit includes (deduplicated at the top level in parse_idl_string)
        for include in &action.implicit_includes {
            elements.push(IdlContentElement::Include(include.clone()));
        }
        elements.push(IdlContentElement::Action(action));
    }

    // Remaining services that weren't part of an action
    for service in services.into_values() {
        elements.push(IdlContentElement::Service(service));
    }

    // Remaining messages
    for message in messages.into_values() {
        elements.push(IdlContentElement::Message(message));
    }

    // Process nested modules
    for def in &module.definitions {
        if let parser_pest::IdlDefinition::Module(nested) = def
            && !nested.name.ends_with("_Constants")
        {
            elements.extend(convert_definitions_with_namespace(
                &nested.definitions,
                &action_namespaces,
            ));
        }
    }

    // Process constants
    for def in &module.definitions {
        if let parser_pest::IdlDefinition::Constant(c) = def {
            elements.push(IdlContentElement::Constant(Constant::new(
                c.name.clone(),
                c.const_type.clone(),
                c.value.clone(),
            )));
        }
    }

    elements
}

/// Convert parser definitions to content elements, extracting structures from modules
/// and recognizing `srv`/`action` modules to produce Service/Action types.
fn convert_definitions_with_namespace(
    definitions: &[parser_pest::IdlDefinition],
    namespaces: &[String],
) -> Vec<IdlContentElement> {
    let mut elements = Vec::new();
    let constants_map = collect_constants(definitions);
    let typedefs = collect_typedefs(definitions);

    for def in definitions {
        match def {
            parser_pest::IdlDefinition::Module(module) => {
                // Skip _Constants modules (handled via constants_map)
                if module.name.ends_with("_Constants") {
                    continue;
                }

                // Check if this is a srv or action module
                match module.name.as_str() {
                    "srv" => {
                        elements.extend(process_srv_module(module, namespaces));
                    }
                    "action" => {
                        elements.extend(process_action_module(module, namespaces));
                    }
                    _ => {
                        // Regular module: recurse with updated namespace
                        let mut new_namespaces = namespaces.to_vec();
                        new_namespaces.push(module.name.clone());
                        elements.extend(convert_definitions_with_namespace(
                            &module.definitions,
                            &new_namespaces,
                        ));
                    }
                }
            }
            parser_pest::IdlDefinition::Struct(struct_def) => {
                let message =
                    convert_struct_to_message(struct_def, namespaces, &constants_map, &typedefs);
                elements.push(IdlContentElement::Message(message));
            }
            parser_pest::IdlDefinition::Constant(const_def) => {
                let constant = Constant::new(
                    const_def.name.clone(),
                    const_def.const_type.clone(),
                    const_def.value.clone(),
                );
                elements.push(IdlContentElement::Constant(constant));
            }
            _ => {
                // Typedefs are collected and used for resolution, not emitted as elements
                // Handle enums, unions, etc. when needed
            }
        }
    }

    elements
}

/// Parse IDL content from a string and return an `IdlFile`
///
/// This function provides complete IDL parsing based on the ROS2 IDL specification
/// using a pest parser that implements the full grammar.lark specification.
///
/// # Errors
///
/// Returns parsing errors if the input content is not valid IDL syntax.
pub fn parse_idl_string(
    content: &str,
    base_path: PathBuf,
    relative_path: PathBuf,
) -> IdlResult<IdlFile> {
    let locator = IdlLocator::new(base_path, relative_path);

    match parser_pest::parse_idl(content) {
        Ok(parsed_file) => {
            let mut elements = Vec::new();

            // Add file-level includes
            for include_path in &parsed_file.includes {
                elements.push(IdlContentElement::Include(Include::new(
                    include_path.clone(),
                )));
            }

            // Convert definitions with proper srv/action handling
            // This may add implicit includes from actions
            let mut converted = convert_definitions_with_namespace(&parsed_file.definitions, &[]);

            // Deduplicate includes - only add implicit includes if not already present
            // Collect existing include locators
            let existing_locators: std::collections::HashSet<String> = elements
                .iter()
                .filter_map(|el| {
                    if let IdlContentElement::Include(inc) = el {
                        Some(inc.locator.clone())
                    } else {
                        None
                    }
                })
                .collect();

            // Filter converted elements - only keep non-duplicate includes
            let filtered: Vec<_> = converted
                .drain(..)
                .filter(|el| {
                    if let IdlContentElement::Include(inc) = el {
                        !existing_locators.contains(&inc.locator)
                    } else {
                        true
                    }
                })
                .collect();

            elements.extend(filtered);

            let content = IdlContent { elements };
            Ok(IdlFile::new(locator, content))
        }
        Err(parse_error) => Err(IdlError::ParseError {
            line: 1,
            column: 1,
            message: format!("Parse error: {parse_error}"),
        }),
    }
}

/// Parse IDL content from a file path
///
/// # Errors
///
/// Returns I/O errors if the file cannot be read, or parsing errors if the content is invalid.
pub fn parse_idl_file(locator: &IdlLocator) -> IdlResult<IdlFile> {
    let path = locator.get_absolute_path();
    let content = std::fs::read_to_string(&path)?;

    parse_idl_string(
        &content,
        locator.basepath.clone(),
        locator.relative_path.clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placeholder_parse() {
        let input = "const int32 MY_CONST = 42;";
        let result = parse_idl_string(input, std::env::temp_dir(), PathBuf::from("test.idl"));
        assert!(result.is_ok());

        let idl_file = result.unwrap();
        assert_eq!(idl_file.locator.relative_path, PathBuf::from("test.idl"));
    }
}
