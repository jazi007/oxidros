//! ServiceTypeDescription and ActionTypeDescription derive macro implementations
//!
//! These macros generate hash computation for ROS2 services and actions.
//! They are separate from ros2_service!/ros2_action! which handle runtime FFI.

use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::DeriveInput;

/// Options parsed from #[ros2(...)] attributes for service/action type descriptions
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(ros2))]
struct ServiceActionOpts {
    ident: syn::Ident,
    /// The ROS2 package name
    package: String,
}

/// Implement the ServiceTypeDescription derive macro
pub fn derive_service_type_description_impl(input: DeriveInput) -> Result<TokenStream, syn::Error> {
    let opts = ServiceActionOpts::from_derive_input(&input)
        .map_err(|e| syn::Error::new_spanned(&input, e.to_string()))?;

    let service_ident = &opts.ident;
    let package = &opts.package;
    let service_name = service_ident.to_string();

    let request_ident = format_ident!("{}_Request", service_ident);
    let response_ident = format_ident!("{}_Response", service_ident);

    let service_type_name = format!("{}/srv/{}", package, service_name);
    let request_type_name = format!("{}/srv/{}_Request", package, service_name);
    let response_type_name = format!("{}/srv/{}_Response", package, service_name);
    let event_type_name = format!("{}/srv/{}_Event", package, service_name);

    let expanded = quote! {
        impl ros2_types::ServiceTypeDescription for #service_ident {
            fn type_description() -> ros2_types::types::TypeDescriptionMsg {
                let request_desc = <#request_ident as ros2_types::TypeDescription>::type_description();
                let response_desc = <#response_ident as ros2_types::TypeDescription>::type_description();

                // Build the event type description (standard structure for all services)
                let event_type_desc = ros2_types::types::IndividualTypeDescription::new(
                    #event_type_name,
                    vec![
                        ros2_types::types::Field::new("info", ros2_types::types::FieldType::nested("service_msgs/msg/ServiceEventInfo")),
                        ros2_types::types::Field::new("request", ros2_types::types::FieldType::nested_bounded_sequence(#request_type_name, 1)),
                        ros2_types::types::Field::new("response", ros2_types::types::FieldType::nested_bounded_sequence(#response_type_name, 1)),
                    ]
                );

                // Build the service type description
                let service_desc = ros2_types::types::IndividualTypeDescription::new(
                    #service_type_name,
                    vec![
                        ros2_types::types::Field::new("request_message", ros2_types::types::FieldType::nested(#request_type_name)),
                        ros2_types::types::Field::new("response_message", ros2_types::types::FieldType::nested(#response_type_name)),
                        ros2_types::types::Field::new("event_message", ros2_types::types::FieldType::nested(#event_type_name)),
                    ]
                );

                // Collect all referenced types
                let mut referenced: Vec<ros2_types::types::IndividualTypeDescription> = Vec::new();
                let mut seen = std::collections::HashSet::new();

                // Add ServiceEventInfo
                if seen.insert("service_msgs/msg/ServiceEventInfo".to_string()) {
                    referenced.push(ros2_types::types::IndividualTypeDescription::new(
                        "service_msgs/msg/ServiceEventInfo",
                        vec![
                            ros2_types::types::Field::new("event_type", ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_UINT8)),
                            ros2_types::types::Field::new("stamp", ros2_types::types::FieldType::nested("builtin_interfaces/msg/Time")),
                            // client_gid is char[16] in ROS2 IDL, which is represented as uint8[16] in type description
                            ros2_types::types::Field::new("client_gid", ros2_types::types::FieldType::array(ros2_types::FIELD_TYPE_UINT8, 16)),
                            ros2_types::types::Field::new("sequence_number", ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_INT64)),
                        ]
                    ));
                }

                // Add Time type
                if seen.insert("builtin_interfaces/msg/Time".to_string()) {
                    referenced.push(ros2_types::types::IndividualTypeDescription::new(
                        "builtin_interfaces/msg/Time",
                        vec![
                            ros2_types::types::Field::new("sec", ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_INT32)),
                            ros2_types::types::Field::new("nanosec", ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_UINT32)),
                        ]
                    ));
                }

                // Add request type and its references
                if seen.insert(request_desc.type_description.type_name.clone()) {
                    referenced.push(request_desc.type_description.clone());
                }
                for ref_desc in request_desc.referenced_type_descriptions {
                    if seen.insert(ref_desc.type_name.clone()) {
                        referenced.push(ref_desc);
                    }
                }

                // Add response type and its references
                if seen.insert(response_desc.type_description.type_name.clone()) {
                    referenced.push(response_desc.type_description.clone());
                }
                for ref_desc in response_desc.referenced_type_descriptions {
                    if seen.insert(ref_desc.type_name.clone()) {
                        referenced.push(ref_desc);
                    }
                }

                // Add event type description
                if seen.insert(event_type_desc.type_name.clone()) {
                    referenced.push(event_type_desc);
                }

                referenced.sort_by(|a, b| a.type_name.cmp(&b.type_name));
                ros2_types::types::TypeDescriptionMsg::new(service_desc, referenced)
            }

            fn service_type_name() -> ros2_types::MessageTypeName {
                ros2_types::MessageTypeName::new("srv", #package, #service_name)
            }
        }
    };

    Ok(expanded)
}

/// Implement the ActionTypeDescription derive macro
pub fn derive_action_type_description_impl(input: DeriveInput) -> Result<TokenStream, syn::Error> {
    let opts = ServiceActionOpts::from_derive_input(&input)
        .map_err(|e| syn::Error::new_spanned(&input, e.to_string()))?;

    let action_ident = &opts.ident;
    let package = &opts.package;
    let action_name = action_ident.to_string();

    let goal_ident = format_ident!("{}_Goal", action_ident);
    let result_ident = format_ident!("{}_Result", action_ident);
    let feedback_ident = format_ident!("{}_Feedback", action_ident);

    let action_type_name = format!("{}/action/{}", package, action_name);
    let goal_type_name = format!("{}/action/{}_Goal", package, action_name);
    let result_type_name = format!("{}/action/{}_Result", package, action_name);
    let feedback_type_name = format!("{}/action/{}_Feedback", package, action_name);
    let send_goal_type_name = format!("{}/action/{}_SendGoal", package, action_name);
    let get_result_type_name = format!("{}/action/{}_GetResult", package, action_name);
    let feedback_message_type_name = format!("{}/action/{}_FeedbackMessage", package, action_name);
    let send_goal_request_type_name =
        format!("{}/action/{}_SendGoal_Request", package, action_name);
    let send_goal_response_type_name =
        format!("{}/action/{}_SendGoal_Response", package, action_name);
    let send_goal_event_type_name = format!("{}/action/{}_SendGoal_Event", package, action_name);
    let get_result_request_type_name =
        format!("{}/action/{}_GetResult_Request", package, action_name);
    let get_result_response_type_name =
        format!("{}/action/{}_GetResult_Response", package, action_name);
    let get_result_event_type_name = format!("{}/action/{}_GetResult_Event", package, action_name);

    let expanded = quote! {
        impl ros2_types::ActionTypeDescription for #action_ident {
            fn type_description() -> ros2_types::types::TypeDescriptionMsg {
                let goal_desc = <#goal_ident as ros2_types::TypeDescription>::type_description();
                let result_desc = <#result_ident as ros2_types::TypeDescription>::type_description();
                let feedback_desc = <#feedback_ident as ros2_types::TypeDescription>::type_description();

                // Build the action type description
                let action_desc = ros2_types::types::IndividualTypeDescription::new(
                    #action_type_name,
                    vec![
                        ros2_types::types::Field::new("goal", ros2_types::types::FieldType::nested(#goal_type_name)),
                        ros2_types::types::Field::new("result", ros2_types::types::FieldType::nested(#result_type_name)),
                        ros2_types::types::Field::new("feedback", ros2_types::types::FieldType::nested(#feedback_type_name)),
                        ros2_types::types::Field::new("send_goal_service", ros2_types::types::FieldType::nested(#send_goal_type_name)),
                        ros2_types::types::Field::new("get_result_service", ros2_types::types::FieldType::nested(#get_result_type_name)),
                        ros2_types::types::Field::new("feedback_message", ros2_types::types::FieldType::nested(#feedback_message_type_name)),
                    ]
                );

                // Collect all referenced types
                let mut referenced: Vec<ros2_types::types::IndividualTypeDescription> = Vec::new();
                let mut seen = std::collections::HashSet::new();

                // Helper to add type and its references
                let mut add_type_desc = |desc: ros2_types::types::TypeDescriptionMsg| {
                    if seen.insert(desc.type_description.type_name.clone()) {
                        referenced.push(desc.type_description);
                    }
                    for ref_desc in desc.referenced_type_descriptions {
                        if seen.insert(ref_desc.type_name.clone()) {
                            referenced.push(ref_desc);
                        }
                    }
                };

                // Add Goal, Result, Feedback and their references
                add_type_desc(goal_desc);
                add_type_desc(result_desc);
                add_type_desc(feedback_desc);

                // Add UUID type
                if seen.insert("unique_identifier_msgs/msg/UUID".to_string()) {
                    referenced.push(ros2_types::types::IndividualTypeDescription::new(
                        "unique_identifier_msgs/msg/UUID",
                        vec![ros2_types::types::Field::new("uuid", ros2_types::types::FieldType::array(ros2_types::FIELD_TYPE_UINT8, 16))]
                    ));
                }

                // Add Time type
                if seen.insert("builtin_interfaces/msg/Time".to_string()) {
                    referenced.push(ros2_types::types::IndividualTypeDescription::new(
                        "builtin_interfaces/msg/Time",
                        vec![
                            ros2_types::types::Field::new("sec", ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_INT32)),
                            ros2_types::types::Field::new("nanosec", ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_UINT32)),
                        ]
                    ));
                }

                // Add ServiceEventInfo
                if seen.insert("service_msgs/msg/ServiceEventInfo".to_string()) {
                    referenced.push(ros2_types::types::IndividualTypeDescription::new(
                        "service_msgs/msg/ServiceEventInfo",
                        vec![
                            ros2_types::types::Field::new("event_type", ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_UINT8)),
                            ros2_types::types::Field::new("stamp", ros2_types::types::FieldType::nested("builtin_interfaces/msg/Time")),
                            // client_gid is char[16] in ROS2 IDL, which is represented as uint8[16] in type description
                            ros2_types::types::Field::new("client_gid", ros2_types::types::FieldType::array(ros2_types::FIELD_TYPE_UINT8, 16)),
                            ros2_types::types::Field::new("sequence_number", ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_INT64)),
                        ]
                    ));
                }

                // Add SendGoal_Request
                if seen.insert(#send_goal_request_type_name.to_string()) {
                    referenced.push(ros2_types::types::IndividualTypeDescription::new(
                        #send_goal_request_type_name,
                        vec![
                            ros2_types::types::Field::new("goal_id", ros2_types::types::FieldType::nested("unique_identifier_msgs/msg/UUID")),
                            ros2_types::types::Field::new("goal", ros2_types::types::FieldType::nested(#goal_type_name)),
                        ]
                    ));
                }

                // Add SendGoal_Response
                if seen.insert(#send_goal_response_type_name.to_string()) {
                    referenced.push(ros2_types::types::IndividualTypeDescription::new(
                        #send_goal_response_type_name,
                        vec![
                            ros2_types::types::Field::new("accepted", ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_BOOLEAN)),
                            ros2_types::types::Field::new("stamp", ros2_types::types::FieldType::nested("builtin_interfaces/msg/Time")),
                        ]
                    ));
                }

                // Add SendGoal_Event
                if seen.insert(#send_goal_event_type_name.to_string()) {
                    referenced.push(ros2_types::types::IndividualTypeDescription::new(
                        #send_goal_event_type_name,
                        vec![
                            ros2_types::types::Field::new("info", ros2_types::types::FieldType::nested("service_msgs/msg/ServiceEventInfo")),
                            ros2_types::types::Field::new("request", ros2_types::types::FieldType::nested_bounded_sequence(#send_goal_request_type_name, 1)),
                            ros2_types::types::Field::new("response", ros2_types::types::FieldType::nested_bounded_sequence(#send_goal_response_type_name, 1)),
                        ]
                    ));
                }

                // Add SendGoal service
                if seen.insert(#send_goal_type_name.to_string()) {
                    referenced.push(ros2_types::types::IndividualTypeDescription::new(
                        #send_goal_type_name,
                        vec![
                            ros2_types::types::Field::new("request_message", ros2_types::types::FieldType::nested(#send_goal_request_type_name)),
                            ros2_types::types::Field::new("response_message", ros2_types::types::FieldType::nested(#send_goal_response_type_name)),
                            ros2_types::types::Field::new("event_message", ros2_types::types::FieldType::nested(#send_goal_event_type_name)),
                        ]
                    ));
                }

                // Add GetResult_Request
                if seen.insert(#get_result_request_type_name.to_string()) {
                    referenced.push(ros2_types::types::IndividualTypeDescription::new(
                        #get_result_request_type_name,
                        vec![ros2_types::types::Field::new("goal_id", ros2_types::types::FieldType::nested("unique_identifier_msgs/msg/UUID"))]
                    ));
                }

                // Add GetResult_Response
                if seen.insert(#get_result_response_type_name.to_string()) {
                    referenced.push(ros2_types::types::IndividualTypeDescription::new(
                        #get_result_response_type_name,
                        vec![
                            ros2_types::types::Field::new("status", ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_INT8)),
                            ros2_types::types::Field::new("result", ros2_types::types::FieldType::nested(#result_type_name)),
                        ]
                    ));
                }

                // Add GetResult_Event
                if seen.insert(#get_result_event_type_name.to_string()) {
                    referenced.push(ros2_types::types::IndividualTypeDescription::new(
                        #get_result_event_type_name,
                        vec![
                            ros2_types::types::Field::new("info", ros2_types::types::FieldType::nested("service_msgs/msg/ServiceEventInfo")),
                            ros2_types::types::Field::new("request", ros2_types::types::FieldType::nested_bounded_sequence(#get_result_request_type_name, 1)),
                            ros2_types::types::Field::new("response", ros2_types::types::FieldType::nested_bounded_sequence(#get_result_response_type_name, 1)),
                        ]
                    ));
                }

                // Add GetResult service
                if seen.insert(#get_result_type_name.to_string()) {
                    referenced.push(ros2_types::types::IndividualTypeDescription::new(
                        #get_result_type_name,
                        vec![
                            ros2_types::types::Field::new("request_message", ros2_types::types::FieldType::nested(#get_result_request_type_name)),
                            ros2_types::types::Field::new("response_message", ros2_types::types::FieldType::nested(#get_result_response_type_name)),
                            ros2_types::types::Field::new("event_message", ros2_types::types::FieldType::nested(#get_result_event_type_name)),
                        ]
                    ));
                }

                // Add FeedbackMessage
                if seen.insert(#feedback_message_type_name.to_string()) {
                    referenced.push(ros2_types::types::IndividualTypeDescription::new(
                        #feedback_message_type_name,
                        vec![
                            ros2_types::types::Field::new("goal_id", ros2_types::types::FieldType::nested("unique_identifier_msgs/msg/UUID")),
                            ros2_types::types::Field::new("feedback", ros2_types::types::FieldType::nested(#feedback_type_name)),
                        ]
                    ));
                }

                // Sort referenced types alphabetically for canonical ordering
                referenced.sort_by(|a, b| a.type_name.cmp(&b.type_name));

                ros2_types::types::TypeDescriptionMsg::new(action_desc, referenced)
            }

            fn action_type_name() -> ros2_types::MessageTypeName {
                ros2_types::MessageTypeName::new("action", #package, #action_name)
            }
        }
    };

    Ok(expanded)
}
