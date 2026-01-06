/// Service specification parsing
use std::fs;
use std::path::Path;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::errors::{ParseError, ParseResult};
use super::message::{MessageSpecification, parse_message_string};
use crate::msg::validation::{
    SERVICE_REQUEST_MESSAGE_SUFFIX, SERVICE_REQUEST_RESPONSE_SEPARATOR,
    SERVICE_RESPONSE_MESSAGE_SUFFIX, is_valid_message_name, is_valid_package_name,
};

/// Service specification containing request and response messages
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ServiceSpecification {
    /// Package name
    pub pkg_name: String,
    /// Service name
    pub srv_name: String,
    /// Request message specification
    pub request: MessageSpecification,
    /// Response message specification
    pub response: MessageSpecification,
}

impl ServiceSpecification {
    /// Create a new service specification
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::InvalidResourceName`] if the package name or service name are invalid.
    pub fn new(
        pkg_name: String,
        srv_name: String,
        request: MessageSpecification,
        response: MessageSpecification,
    ) -> ParseResult<Self> {
        if !is_valid_package_name(&pkg_name) {
            return Err(ParseError::InvalidResourceName {
                name: pkg_name,
                reason: "invalid package name pattern".to_string(),
            });
        }

        if !is_valid_message_name(&srv_name) {
            return Err(ParseError::InvalidResourceName {
                name: srv_name,
                reason: "invalid service name pattern".to_string(),
            });
        }

        Ok(ServiceSpecification {
            pkg_name,
            srv_name,
            request,
            response,
        })
    }

    /// Get the full service name (package/service)
    #[must_use]
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.pkg_name, self.srv_name)
    }

    /// Check if the service has any request fields
    #[must_use]
    pub fn has_request_fields(&self) -> bool {
        self.request.has_fields()
    }

    /// Check if the service has any response fields
    #[must_use]
    pub fn has_response_fields(&self) -> bool {
        self.response.has_fields()
    }

    /// Check if the service has any request constants
    #[must_use]
    pub fn has_request_constants(&self) -> bool {
        self.request.has_constants()
    }

    /// Check if the service has any response constants
    #[must_use]
    pub fn has_response_constants(&self) -> bool {
        self.response.has_constants()
    }

    /// Get request field by name
    #[must_use]
    pub fn get_request_field(&self, name: &str) -> Option<&super::types::Field> {
        self.request.get_field(name)
    }

    /// Get response field by name
    #[must_use]
    pub fn get_response_field(&self, name: &str) -> Option<&super::types::Field> {
        self.response.get_field(name)
    }

    /// Get request constant by name
    #[must_use]
    pub fn get_request_constant(&self, name: &str) -> Option<&super::types::Constant> {
        self.request.get_constant(name)
    }

    /// Get response constant by name
    #[must_use]
    pub fn get_response_constant(&self, name: &str) -> Option<&super::types::Constant> {
        self.response.get_constant(name)
    }
}

impl std::fmt::Display for ServiceSpecification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "# {}/{}", self.pkg_name, self.srv_name)?;
        write!(f, "{}", self.request)?;
        writeln!(f, "{SERVICE_REQUEST_RESPONSE_SEPARATOR}")?;
        write!(f, "{}", self.response)?;
        Ok(())
    }
}

/// Parse a service file
///
/// # Errors
///
/// Returns [`ParseError`] if the file cannot be read or the service format is invalid.
pub fn parse_service_file<P: AsRef<Path>>(
    pkg_name: &str,
    interface_filename: P,
) -> ParseResult<ServiceSpecification> {
    let path = interface_filename.as_ref();
    let basename = path.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
        ParseError::InvalidServiceSpecification {
            reason: "invalid filename".to_string(),
        }
    })?;

    let srv_name = basename
        .strip_suffix(".srv")
        .unwrap_or(basename)
        .to_string();

    let content = fs::read_to_string(path)?;
    parse_service_string(pkg_name, &srv_name, &content)
}

/// Parse a service from string content
///
/// # Errors
///
/// Returns [`ParseError`] if the service format is invalid.
pub fn parse_service_string(
    pkg_name: &str,
    srv_name: &str,
    service_string: &str,
) -> ParseResult<ServiceSpecification> {
    let lines: Vec<&str> = service_string.lines().collect();

    // Find separator indices
    let separator_indices: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter_map(|(i, line)| {
            if line.trim() == SERVICE_REQUEST_RESPONSE_SEPARATOR {
                Some(i)
            } else {
                None
            }
        })
        .collect();

    // Validate separator count
    if separator_indices.is_empty() {
        return Err(ParseError::InvalidServiceSpecification {
            reason: format!(
                "Could not find separator '{SERVICE_REQUEST_RESPONSE_SEPARATOR}' between request and response"
            ),
        });
    }

    if separator_indices.len() != 1 {
        return Err(ParseError::InvalidServiceSpecification {
            reason: format!(
                "Found {} separators '{}', expected exactly 1",
                separator_indices.len(),
                SERVICE_REQUEST_RESPONSE_SEPARATOR
            ),
        });
    }

    let separator_index = separator_indices[0];

    // Split into request and response parts
    let request_lines = &lines[..separator_index];
    let response_lines = &lines[separator_index + 1..];

    let request_string = request_lines.join("\n");
    let response_string = response_lines.join("\n");

    // Parse request and response messages
    let request_msg_name = format!("{srv_name}{SERVICE_REQUEST_MESSAGE_SUFFIX}");
    let response_msg_name = format!("{srv_name}{SERVICE_RESPONSE_MESSAGE_SUFFIX}");

    let request = parse_message_string(pkg_name, &request_msg_name, &request_string)?;
    let response = parse_message_string(pkg_name, &response_msg_name, &response_string)?;

    ServiceSpecification::new(
        pkg_name.to_string(),
        srv_name.to_string(),
        request,
        response,
    )
}

/// Create service-related message specifications for event handling
///
/// # Errors
///
/// Returns [`ParseError`] if the event message cannot be created.
pub fn create_service_event_message(
    pkg_name: &str,
    srv_name: &str,
    request: &MessageSpecification,
    response: &MessageSpecification,
) -> ParseResult<MessageSpecification> {
    use crate::msg::types::{BaseType, Field, Type};
    use crate::msg::validation::SERVICE_EVENT_MESSAGE_SUFFIX;

    let event_msg_name = format!("{srv_name}{SERVICE_EVENT_MESSAGE_SUFFIX}");
    let mut event_msg = MessageSpecification::new(pkg_name.to_string(), event_msg_name)?;

    // Add standard event fields
    // service_msgs/ServiceEventInfo info
    let info_type = Type {
        base_type: BaseType {
            pkg_name: Some("service_msgs".to_string()),
            type_name: "ServiceEventInfo".to_string(),
            string_upper_bound: None,
        },
        is_array: false,
        array_size: None,
        is_upper_bound: false,
    };
    let info_field = Field::new(info_type, "info", None)?;
    event_msg.add_field(info_field);

    // Request array field
    let request_type = Type {
        base_type: BaseType {
            pkg_name: Some(pkg_name.to_string()),
            type_name: request.msg_name.clone(),
            string_upper_bound: None,
        },
        is_array: true,
        array_size: Some(1),
        is_upper_bound: true,
    };
    let request_field = Field::new(request_type, "request", None)?;
    event_msg.add_field(request_field);

    // Response array field
    let response_type = Type {
        base_type: BaseType {
            pkg_name: Some(pkg_name.to_string()),
            type_name: response.msg_name.clone(),
            string_upper_bound: None,
        },
        is_array: true,
        array_size: Some(1),
        is_upper_bound: true,
    };
    let response_field = Field::new(response_type, "response", None)?;
    event_msg.add_field(response_field);

    Ok(event_msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_service() {
        let content = r"
# Request
int32 a
int32 b
---
# Response
int32 sum
";

        let spec = parse_service_string("test_msgs", "AddTwoInts", content).unwrap();
        assert_eq!(spec.pkg_name, "test_msgs");
        assert_eq!(spec.srv_name, "AddTwoInts");
        assert_eq!(spec.request.fields.len(), 2);
        assert_eq!(spec.response.fields.len(), 1);

        assert_eq!(spec.request.fields[0].name, "a");
        assert_eq!(spec.request.fields[1].name, "b");
        assert_eq!(spec.response.fields[0].name, "sum");
    }

    #[test]
    fn test_parse_empty_service() {
        let content = "---";

        let spec = parse_service_string("test_msgs", "Empty", content).unwrap();
        assert_eq!(spec.request.fields.len(), 0);
        assert_eq!(spec.response.fields.len(), 0);
    }

    #[test]
    fn test_parse_service_with_constants() {
        let content = r#"
# Request constants
int32 MAX_VALUE=100

# Request fields
int32 value
---
# Response constants
string STATUS_OK="ok"
string STATUS_ERROR="error"

# Response fields
string status
int32 result
"#;

        let spec = parse_service_string("test_msgs", "TestService", content).unwrap();
        assert_eq!(spec.request.constants.len(), 1);
        assert_eq!(spec.request.fields.len(), 1);
        assert_eq!(spec.response.constants.len(), 2);
        assert_eq!(spec.response.fields.len(), 2);
    }

    #[test]
    fn test_service_missing_separator() {
        let content = r"
int32 a
int32 b
int32 sum
";

        let result = parse_service_string("test_msgs", "BadService", content);
        assert!(result.is_err());

        if let Err(ParseError::InvalidServiceSpecification { reason }) = result {
            assert!(reason.contains("Could not find separator"));
        }
    }

    #[test]
    fn test_service_multiple_separators() {
        let content = r"
int32 a
---
int32 b
---
int32 sum
";

        let result = parse_service_string("test_msgs", "BadService", content);
        assert!(result.is_err());

        if let Err(ParseError::InvalidServiceSpecification { reason }) = result {
            assert!(reason.contains("Found 2 separators"));
        }
    }

    #[test]
    fn test_service_display() {
        let content = r"
int32 a
int32 b
---
int32 sum
";

        let spec = parse_service_string("test_msgs", "AddTwoInts", content).unwrap();
        let display_string = spec.to_string();

        assert!(display_string.contains("test_msgs/AddTwoInts"));
        assert!(display_string.contains("---"));
        assert!(display_string.contains("int32 a"));
        assert!(display_string.contains("int32 b"));
        assert!(display_string.contains("int32 sum"));
    }

    #[test]
    fn test_create_service_event_message() {
        let content = r"
int32 a
---
int32 result
";

        let spec = parse_service_string("test_msgs", "TestService", content).unwrap();
        let event_msg =
            create_service_event_message("test_msgs", "TestService", &spec.request, &spec.response)
                .unwrap();

        assert_eq!(event_msg.msg_name, "TestService_Event");
        assert_eq!(event_msg.fields.len(), 3); // info, request[], response[]

        assert_eq!(event_msg.fields[0].name, "info");
        assert_eq!(event_msg.fields[1].name, "request");
        assert_eq!(event_msg.fields[2].name, "response");

        // Check that request and response are arrays
        assert!(event_msg.fields[1].field_type.is_array);
        assert!(event_msg.fields[2].field_type.is_array);
    }
}
