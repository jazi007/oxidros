/// Action specification parsing
use std::fs;
use std::path::Path;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::errors::{ParseError, ParseResult};
use super::message::{MessageSpecification, parse_message_string};
use super::service::ServiceSpecification;
use crate::msg::validation::{
    ACTION_FEEDBACK_SUFFIX, ACTION_GOAL_SUFFIX, ACTION_REQUEST_RESPONSE_SEPARATOR,
    ACTION_RESULT_SUFFIX, is_valid_message_name, is_valid_package_name,
};

/// Action specification containing goal, result, and feedback messages
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ActionSpecification {
    /// Package name
    pub pkg_name: String,
    /// Action name
    pub action_name: String,
    /// Goal message specification
    pub goal: MessageSpecification,
    /// Result message specification
    pub result: MessageSpecification,
    /// Feedback message specification
    pub feedback: MessageSpecification,
    /// Goal service specification (derived)
    pub goal_service: ServiceSpecification,
    /// Result service specification (derived)
    pub result_service: ServiceSpecification,
}

impl ActionSpecification {
    /// Create a new action specification
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::InvalidResourceName`] if the package name or action name are invalid.
    pub fn new(
        pkg_name: String,
        action_name: String,
        goal: MessageSpecification,
        result: MessageSpecification,
        feedback: MessageSpecification,
    ) -> ParseResult<Self> {
        if !is_valid_package_name(&pkg_name) {
            return Err(ParseError::InvalidResourceName {
                name: pkg_name,
                reason: "invalid package name pattern".to_string(),
            });
        }

        if !is_valid_message_name(&action_name) {
            return Err(ParseError::InvalidResourceName {
                name: action_name,
                reason: "invalid action name pattern".to_string(),
            });
        }

        // Create derived services
        let goal_service = create_goal_service(&pkg_name, &action_name, &goal)?;
        let result_service = create_result_service(&pkg_name, &action_name, &result)?;

        Ok(ActionSpecification {
            pkg_name,
            action_name,
            goal,
            result,
            feedback,
            goal_service,
            result_service,
        })
    }

    /// Get the full action name (package/action)
    #[must_use]
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.pkg_name, self.action_name)
    }

    /// Check if the action has any goal fields
    #[must_use]
    pub fn has_goal_fields(&self) -> bool {
        self.goal.has_fields()
    }

    /// Check if the action has any result fields
    #[must_use]
    pub fn has_result_fields(&self) -> bool {
        self.result.has_fields()
    }

    /// Check if the action has any feedback fields
    #[must_use]
    pub fn has_feedback_fields(&self) -> bool {
        self.feedback.has_fields()
    }

    /// Check if the action has any goal constants
    #[must_use]
    pub fn has_goal_constants(&self) -> bool {
        self.goal.has_constants()
    }

    /// Check if the action has any result constants
    #[must_use]
    pub fn has_result_constants(&self) -> bool {
        self.result.has_constants()
    }

    /// Check if the action has any feedback constants
    #[must_use]
    pub fn has_feedback_constants(&self) -> bool {
        self.feedback.has_constants()
    }

    /// Get goal field by name
    #[must_use]
    pub fn get_goal_field(&self, name: &str) -> Option<&super::types::Field> {
        self.goal.get_field(name)
    }

    /// Get result field by name
    #[must_use]
    pub fn get_result_field(&self, name: &str) -> Option<&super::types::Field> {
        self.result.get_field(name)
    }

    /// Get feedback field by name
    #[must_use]
    pub fn get_feedback_field(&self, name: &str) -> Option<&super::types::Field> {
        self.feedback.get_field(name)
    }

    /// Get goal constant by name
    #[must_use]
    pub fn get_goal_constant(&self, name: &str) -> Option<&super::types::Constant> {
        self.goal.get_constant(name)
    }

    /// Get result constant by name
    #[must_use]
    pub fn get_result_constant(&self, name: &str) -> Option<&super::types::Constant> {
        self.result.get_constant(name)
    }

    /// Get feedback constant by name
    #[must_use]
    pub fn get_feedback_constant(&self, name: &str) -> Option<&super::types::Constant> {
        self.feedback.get_constant(name)
    }

    /// Get all message specifications (goal, result, feedback)
    #[must_use]
    pub fn all_messages(&self) -> Vec<&MessageSpecification> {
        vec![&self.goal, &self.result, &self.feedback]
    }

    /// Get all service specifications (goal service, result service)
    #[must_use]
    pub fn all_services(&self) -> Vec<&ServiceSpecification> {
        vec![&self.goal_service, &self.result_service]
    }
}

impl std::fmt::Display for ActionSpecification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "# {}/{}", self.pkg_name, self.action_name)?;

        // Write goal
        write!(f, "{}", self.goal)?;
        writeln!(f, "{ACTION_REQUEST_RESPONSE_SEPARATOR}")?;

        // Write result
        write!(f, "{}", self.result)?;
        writeln!(f, "{ACTION_REQUEST_RESPONSE_SEPARATOR}")?;

        // Write feedback
        write!(f, "{}", self.feedback)?;

        Ok(())
    }
}

/// Parse an action file
///
/// # Errors
///
/// Returns [`ParseError`] if the file cannot be read or the action format is invalid.
pub fn parse_action_file<P: AsRef<Path>>(
    pkg_name: &str,
    interface_filename: P,
) -> ParseResult<ActionSpecification> {
    let path = interface_filename.as_ref();
    let basename = path.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
        ParseError::InvalidActionSpecification {
            reason: "invalid filename".to_string(),
        }
    })?;

    let action_name = basename
        .strip_suffix(".action")
        .unwrap_or(basename)
        .to_string();

    let content = fs::read_to_string(path)?;
    parse_action_string(pkg_name, &action_name, &content)
}

/// Parse an action from string content
///
/// # Errors
///
/// Returns [`ParseError`] if the action format is invalid.
pub fn parse_action_string(
    pkg_name: &str,
    action_name: &str,
    action_string: &str,
) -> ParseResult<ActionSpecification> {
    let lines: Vec<&str> = action_string.lines().collect();

    // Find separator indices
    let separator_indices: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter_map(|(i, line)| {
            if line.trim() == ACTION_REQUEST_RESPONSE_SEPARATOR {
                Some(i)
            } else {
                None
            }
        })
        .collect();

    // Validate separator count (must be exactly 2)
    if separator_indices.len() != 2 {
        return Err(ParseError::InvalidActionSpecification {
            reason: format!(
                "Found {} separators '{}', expected exactly 2 for goal/result/feedback",
                separator_indices.len(),
                ACTION_REQUEST_RESPONSE_SEPARATOR
            ),
        });
    }

    let first_separator = separator_indices[0];
    let second_separator = separator_indices[1];

    // Split into goal, result, and feedback parts
    let goal_lines = &lines[..first_separator];
    let result_lines = &lines[first_separator + 1..second_separator];
    let feedback_lines = &lines[second_separator + 1..];

    let goal_string = goal_lines.join("\n");
    let result_string = result_lines.join("\n");
    let feedback_string = feedback_lines.join("\n");

    // Parse goal, result, and feedback messages
    let goal_msg_name = format!("{action_name}{ACTION_GOAL_SUFFIX}");
    let result_msg_name = format!("{action_name}{ACTION_RESULT_SUFFIX}");
    let feedback_msg_name = format!("{action_name}{ACTION_FEEDBACK_SUFFIX}");

    let goal = parse_message_string(pkg_name, &goal_msg_name, &goal_string)?;
    let result = parse_message_string(pkg_name, &result_msg_name, &result_string)?;
    let feedback = parse_message_string(pkg_name, &feedback_msg_name, &feedback_string)?;

    ActionSpecification::new(
        pkg_name.to_string(),
        action_name.to_string(),
        goal,
        result,
        feedback,
    )
}

/// Create the goal service specification for an action
fn create_goal_service(
    pkg_name: &str,
    action_name: &str,
    goal: &MessageSpecification,
) -> ParseResult<ServiceSpecification> {
    use crate::msg::types::{BaseType, Field, Type};

    // Create request message (contains goal + goal_id)
    let request_msg_name = format!("{action_name}_SendGoal_Request");
    let mut request = MessageSpecification::new(pkg_name.to_string(), request_msg_name)?;

    // Add goal_id field
    let goal_id_type = Type {
        base_type: BaseType {
            pkg_name: Some("unique_identifier_msgs".to_string()),
            type_name: "UUID".to_string(),
            string_upper_bound: None,
        },
        is_array: false,
        array_size: None,
        is_upper_bound: false,
    };
    let goal_id_field = Field::new(goal_id_type, "goal_id", None)?;
    request.add_field(goal_id_field);

    // Add goal field
    let goal_type = Type {
        base_type: BaseType {
            pkg_name: Some(pkg_name.to_string()),
            type_name: goal.msg_name.clone(),
            string_upper_bound: None,
        },
        is_array: false,
        array_size: None,
        is_upper_bound: false,
    };
    let goal_field = Field::new(goal_type, "goal", None)?;
    request.add_field(goal_field);

    // Create response message (contains accepted + stamp)
    let response_msg_name = format!("{action_name}_SendGoal_Response");
    let mut response = MessageSpecification::new(pkg_name.to_string(), response_msg_name)?;

    // Add accepted field
    let accepted_type = Type {
        base_type: BaseType {
            pkg_name: None,
            type_name: "bool".to_string(),
            string_upper_bound: None,
        },
        is_array: false,
        array_size: None,
        is_upper_bound: false,
    };
    let accepted_field = Field::new(accepted_type, "accepted", None)?;
    response.add_field(accepted_field);

    // Add stamp field
    let stamp_type = Type {
        base_type: BaseType {
            pkg_name: Some("builtin_interfaces".to_string()),
            type_name: "Time".to_string(),
            string_upper_bound: None,
        },
        is_array: false,
        array_size: None,
        is_upper_bound: false,
    };
    let stamp_field = Field::new(stamp_type, "stamp", None)?;
    response.add_field(stamp_field);

    ServiceSpecification::new(
        pkg_name.to_string(),
        format!("{action_name}_SendGoal"),
        request,
        response,
    )
}

/// Create the result service specification for an action
fn create_result_service(
    pkg_name: &str,
    action_name: &str,
    result: &MessageSpecification,
) -> ParseResult<ServiceSpecification> {
    use crate::msg::types::{BaseType, Field, Type};

    // Create request message (contains goal_id)
    let request_msg_name = format!("{action_name}_GetResult_Request");
    let mut request = MessageSpecification::new(pkg_name.to_string(), request_msg_name)?;

    // Add goal_id field
    let goal_id_type = Type {
        base_type: BaseType {
            pkg_name: Some("unique_identifier_msgs".to_string()),
            type_name: "UUID".to_string(),
            string_upper_bound: None,
        },
        is_array: false,
        array_size: None,
        is_upper_bound: false,
    };
    let goal_id_field = Field::new(goal_id_type, "goal_id", None)?;
    request.add_field(goal_id_field);

    // Create response message (contains status + result)
    let response_msg_name = format!("{action_name}_GetResult_Response");
    let mut response = MessageSpecification::new(pkg_name.to_string(), response_msg_name)?;

    // Add status field
    let status_type = Type {
        base_type: BaseType {
            pkg_name: None,
            type_name: "int8".to_string(),
            string_upper_bound: None,
        },
        is_array: false,
        array_size: None,
        is_upper_bound: false,
    };
    let status_field = Field::new(status_type, "status", None)?;
    response.add_field(status_field);

    // Add result field
    let result_type = Type {
        base_type: BaseType {
            pkg_name: Some(pkg_name.to_string()),
            type_name: result.msg_name.clone(),
            string_upper_bound: None,
        },
        is_array: false,
        array_size: None,
        is_upper_bound: false,
    };
    let result_field = Field::new(result_type, "result", None)?;
    response.add_field(result_field);

    ServiceSpecification::new(
        pkg_name.to_string(),
        format!("{action_name}_GetResult"),
        request,
        response,
    )
}

/// Create feedback message specification for publishing action feedback
///
/// # Errors
///
/// Returns [`ParseError`] if the feedback message cannot be created.
pub fn create_feedback_message(
    pkg_name: &str,
    action_name: &str,
    feedback: &MessageSpecification,
) -> ParseResult<MessageSpecification> {
    use crate::msg::types::{BaseType, Field, Type};

    let feedback_msg_name = format!("{action_name}_FeedbackMessage");
    let mut feedback_msg = MessageSpecification::new(pkg_name.to_string(), feedback_msg_name)?;

    // Add goal_id field
    let goal_id_type = Type {
        base_type: BaseType {
            pkg_name: Some("unique_identifier_msgs".to_string()),
            type_name: "UUID".to_string(),
            string_upper_bound: None,
        },
        is_array: false,
        array_size: None,
        is_upper_bound: false,
    };
    let goal_id_field = Field::new(goal_id_type, "goal_id", None)?;
    feedback_msg.add_field(goal_id_field);

    // Add feedback field
    let feedback_type = Type {
        base_type: BaseType {
            pkg_name: Some(pkg_name.to_string()),
            type_name: feedback.msg_name.clone(),
            string_upper_bound: None,
        },
        is_array: false,
        array_size: None,
        is_upper_bound: false,
    };
    let feedback_field = Field::new(feedback_type, "feedback", None)?;
    feedback_msg.add_field(feedback_field);

    Ok(feedback_msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_action() {
        let content = r"
# Goal
int32 order
---
# Result
int32[] sequence
---
# Feedback
int32[] partial_sequence
";

        let spec = parse_action_string("test_msgs", "Fibonacci", content).unwrap();
        assert_eq!(spec.pkg_name, "test_msgs");
        assert_eq!(spec.action_name, "Fibonacci");

        // Check goal
        assert_eq!(spec.goal.fields.len(), 1);
        assert_eq!(spec.goal.fields[0].name, "order");

        // Check result
        assert_eq!(spec.result.fields.len(), 1);
        assert_eq!(spec.result.fields[0].name, "sequence");
        assert!(spec.result.fields[0].field_type.is_array);

        // Check feedback
        assert_eq!(spec.feedback.fields.len(), 1);
        assert_eq!(spec.feedback.fields[0].name, "partial_sequence");
        assert!(spec.feedback.fields[0].field_type.is_array);
    }

    #[test]
    fn test_parse_empty_action() {
        let content = "---\n---";

        let spec = parse_action_string("test_msgs", "Empty", content).unwrap();
        assert_eq!(spec.goal.fields.len(), 0);
        assert_eq!(spec.result.fields.len(), 0);
        assert_eq!(spec.feedback.fields.len(), 0);
    }

    #[test]
    fn test_parse_action_with_constants() {
        let content = r#"
# Goal constants
int32 MIN_ORDER=1
int32 MAX_ORDER=100

# Goal fields
int32 order
---
# Result constants
string STATUS_SUCCESS="success"

# Result fields
int32[] sequence
string status
---
# Feedback
int32[] partial_sequence
"#;

        let spec = parse_action_string("test_msgs", "TestAction", content).unwrap();

        // Check goal constants and fields
        assert_eq!(spec.goal.constants.len(), 2);
        assert_eq!(spec.goal.fields.len(), 1);

        // Check result constants and fields
        assert_eq!(spec.result.constants.len(), 1);
        assert_eq!(spec.result.fields.len(), 2);

        // Check feedback
        assert_eq!(spec.feedback.fields.len(), 1);
    }

    #[test]
    fn test_action_wrong_separator_count() {
        // Only one separator
        let content = r"
int32 order
---
int32[] sequence
";

        let result = parse_action_string("test_msgs", "BadAction", content);
        assert!(result.is_err());

        if let Err(ParseError::InvalidActionSpecification { reason }) = result {
            assert!(reason.contains("Found 1 separators"));
            assert!(reason.contains("expected exactly 2"));
        }

        // Three separators
        let content = r"
int32 order
---
int32[] sequence
---
int32[] partial
---
extra
";

        let result = parse_action_string("test_msgs", "BadAction", content);
        assert!(result.is_err());

        if let Err(ParseError::InvalidActionSpecification { reason }) = result {
            assert!(reason.contains("Found 3 separators"));
        }
    }

    #[test]
    fn test_action_display() {
        let content = r"
int32 order
---
int32[] sequence
---
int32[] partial_sequence
";

        let spec = parse_action_string("test_msgs", "Fibonacci", content).unwrap();
        let display_string = spec.to_string();

        assert!(display_string.contains("test_msgs/Fibonacci"));
        assert!(display_string.matches("---").count() == 2);
        assert!(display_string.contains("int32 order"));
        assert!(display_string.contains("int32[] sequence"));
        assert!(display_string.contains("int32[] partial_sequence"));
    }

    #[test]
    fn test_action_derived_services() {
        let content = r"
int32 order
---
int32[] sequence
---
int32[] partial_sequence
";

        let spec = parse_action_string("test_msgs", "Fibonacci", content).unwrap();

        // Check goal service
        assert_eq!(spec.goal_service.srv_name, "Fibonacci_SendGoal");
        assert_eq!(spec.goal_service.request.fields.len(), 2); // goal_id + goal
        assert_eq!(spec.goal_service.response.fields.len(), 2); // accepted + stamp

        assert_eq!(spec.goal_service.request.fields[0].name, "goal_id");
        assert_eq!(spec.goal_service.request.fields[1].name, "goal");

        // Check result service
        assert_eq!(spec.result_service.srv_name, "Fibonacci_GetResult");
        assert_eq!(spec.result_service.request.fields.len(), 1); // goal_id
        assert_eq!(spec.result_service.response.fields.len(), 2); // status + result

        assert_eq!(spec.result_service.request.fields[0].name, "goal_id");
        assert_eq!(spec.result_service.response.fields[0].name, "status");
        assert_eq!(spec.result_service.response.fields[1].name, "result");
    }

    #[test]
    fn test_create_feedback_message() {
        let content = r"
---
---
int32[] partial_sequence
";

        let spec = parse_action_string("test_msgs", "Fibonacci", content).unwrap();
        let feedback_msg =
            create_feedback_message("test_msgs", "Fibonacci", &spec.feedback).unwrap();

        assert_eq!(feedback_msg.msg_name, "Fibonacci_FeedbackMessage");
        assert_eq!(feedback_msg.fields.len(), 2); // goal_id + feedback

        assert_eq!(feedback_msg.fields[0].name, "goal_id");
        assert_eq!(feedback_msg.fields[1].name, "feedback");
    }
}
