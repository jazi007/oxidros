/// Message specification parsing
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::errors::{ParseError, ParseResult};
use crate::msg::types::{AnnotationValue, Annotations, Constant, Field, Type};
use crate::msg::validation::{
    COMMENT_DELIMITER, CONSTANT_SEPARATOR, OPTIONAL_ANNOTATION, is_valid_message_name,
    is_valid_package_name,
};

/// Message specification
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MessageSpecification {
    /// Package name
    pub pkg_name: String,
    /// Message name
    pub msg_name: String,
    /// List of fields
    pub fields: Vec<Field>,
    /// List of constants
    pub constants: Vec<Constant>,
    /// Annotations for the message
    pub annotations: Annotations,
}

impl MessageSpecification {
    /// Create a new empty message specification
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::InvalidResourceName`] if the package name or message name are invalid.
    pub fn new(pkg_name: String, msg_name: String) -> ParseResult<Self> {
        if !is_valid_package_name(&pkg_name) {
            return Err(ParseError::InvalidResourceName {
                name: pkg_name,
                reason: "invalid package name pattern".to_string(),
            });
        }

        if !is_valid_message_name(&msg_name) {
            return Err(ParseError::InvalidResourceName {
                name: msg_name,
                reason: "invalid message name pattern".to_string(),
            });
        }

        Ok(MessageSpecification {
            pkg_name,
            msg_name,
            fields: Vec::new(),
            constants: Vec::new(),
            annotations: HashMap::new(),
        })
    }

    /// Add a field to the message
    pub fn add_field(&mut self, field: Field) {
        self.fields.push(field);
    }

    /// Add a constant to the message
    pub fn add_constant(&mut self, constant: Constant) {
        self.constants.push(constant);
    }

    /// Get field by name
    #[must_use]
    pub fn get_field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Get constant by name
    #[must_use]
    pub fn get_constant(&self, name: &str) -> Option<&Constant> {
        self.constants.iter().find(|c| c.name == name)
    }

    /// Check if message has any fields
    #[must_use]
    pub fn has_fields(&self) -> bool {
        !self.fields.is_empty()
    }

    /// Check if message has any constants
    #[must_use]
    pub fn has_constants(&self) -> bool {
        !self.constants.is_empty()
    }
}

impl std::fmt::Display for MessageSpecification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "# {}/{}", self.pkg_name, self.msg_name)?;

        // Write constants first
        for constant in &self.constants {
            writeln!(f, "{constant}")?;
        }

        if !self.constants.is_empty() && !self.fields.is_empty() {
            writeln!(f)?; // Empty line between constants and fields
        }

        // Write fields
        for field in &self.fields {
            writeln!(f, "{field}")?;
        }

        Ok(())
    }
}

/// Parse a message file
///
/// # Errors
///
/// Returns [`ParseError`] if the file cannot be read or the message format is invalid.
pub fn parse_message_file<P: AsRef<Path>>(
    pkg_name: &str,
    interface_filename: P,
) -> ParseResult<MessageSpecification> {
    let path = interface_filename.as_ref();
    let basename =
        path.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| ParseError::InvalidField {
                reason: "invalid filename".to_string(),
            })?;

    let msg_name = basename
        .strip_suffix(".msg")
        .unwrap_or(basename)
        .to_string();

    let content = fs::read_to_string(path)?;
    parse_message_string(pkg_name, &msg_name, &content)
}

/// Parse a message from string content
///
/// # Errors
///
/// Returns [`ParseError`] if the message format is invalid.
pub fn parse_message_string(
    pkg_name: &str,
    msg_name: &str,
    message_string: &str,
) -> ParseResult<MessageSpecification> {
    let mut spec = MessageSpecification::new(pkg_name.to_string(), msg_name.to_string())?;

    // Replace tabs with spaces for consistent parsing
    let normalized_content = message_string.replace('\t', " ");

    // Extract file-level comments and content
    let (file_level_comments, content_lines) = extract_file_level_comments(&normalized_content);

    // Set file-level comments as message annotations
    if !file_level_comments.is_empty() {
        spec.annotations.insert(
            "comment".to_string(),
            AnnotationValue::StringList(file_level_comments),
        );
    }

    // Parse content lines
    let mut current_comments = Vec::<String>::new();
    let mut is_optional = false;

    for (line_num, line) in content_lines.iter().enumerate() {
        let line = line.trim_end();

        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }

        // Handle comments
        let (line_content, comment) = extract_line_comment(line);

        if let Some(comment_text) = comment {
            if line_content.trim().is_empty() {
                // This is a comment-only line - collect for next element
                current_comments.push(comment_text);
                continue;
            }
            // Line has both content and comment, collect the comment
            current_comments.push(comment_text);
        }

        let line_content = line_content.trim();
        if line_content.is_empty() {
            continue;
        }

        // Check for optional annotation
        if line_content == OPTIONAL_ANNOTATION {
            is_optional = true;
            continue;
        }

        // Parse the line as field or constant
        match parse_line_content(line_content, pkg_name, line_num + 1) {
            Ok(LineContent::Field(mut field)) => {
                // Add collected comments
                if !current_comments.is_empty() {
                    field.annotations.insert(
                        "comment".to_string(),
                        AnnotationValue::StringList(current_comments.clone()),
                    );
                    current_comments.clear();
                }

                // Add optional annotation if present
                if is_optional {
                    field
                        .annotations
                        .insert("optional".to_string(), AnnotationValue::Bool(true));
                    is_optional = false;
                }

                spec.add_field(field);
            }
            Ok(LineContent::Constant(mut constant)) => {
                // Add collected comments
                if !current_comments.is_empty() {
                    constant.annotations.insert(
                        "comment".to_string(),
                        AnnotationValue::StringList(current_comments.clone()),
                    );
                    current_comments.clear();
                }

                spec.add_constant(constant);
            }
            Err(e) => {
                return Err(ParseError::LineParseError {
                    line: line_num + 1,
                    message: format!("Error parsing line '{line_content}': {e}"),
                });
            }
        }
    }

    // Process comments for all elements
    process_comments(&mut spec);

    Ok(spec)
}

/// Content parsed from a line
enum LineContent {
    Field(Field),
    Constant(Constant),
}

/// Extract file-level comments from the beginning of the message
fn extract_file_level_comments(message_string: &str) -> (Vec<String>, Vec<String>) {
    let lines: Vec<String> = message_string
        .lines()
        .map(std::string::ToString::to_string)
        .collect();

    let mut file_level_comments = Vec::new();
    let mut first_content_index = 0;

    // Extract comments at the very top, until we hit a blank line or non-comment content
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            // Blank line marks end of file-level comments
            first_content_index = i + 1;
            break;
        } else if trimmed.starts_with(COMMENT_DELIMITER) {
            // This is a file-level comment
            if let Some(comment_text) = trimmed.strip_prefix(COMMENT_DELIMITER) {
                file_level_comments.push(comment_text.trim_start().to_string());
            }
        } else {
            // First non-comment, non-blank line - no file-level comments if we haven't seen a blank
            first_content_index = i;
            break;
        }
    }

    let content_lines = lines[first_content_index..].to_vec();

    (file_level_comments, content_lines)
}

/// Extract comment from a line, returning (content, comment)
fn extract_line_comment(line: &str) -> (String, Option<String>) {
    if let Some(comment_index) = line.find(COMMENT_DELIMITER) {
        let content = line[..comment_index].to_string();
        let comment = line[comment_index + 1..].trim_start().to_string();
        (content, Some(comment))
    } else {
        (line.to_string(), None)
    }
}

/// Parse a single line of content (field or constant definition)
fn parse_line_content(line: &str, pkg_name: &str, _line_num: usize) -> ParseResult<LineContent> {
    // Check if this is a constant (contains '=' but not as part of '<=' array bounds)
    if line.contains(CONSTANT_SEPARATOR) && !is_array_bound_syntax(line) {
        parse_constant_line(line)
    } else {
        parse_field_line(line, pkg_name)
    }
}

/// Check if line contains array bound syntax (<=) which should not be confused with constants
fn is_array_bound_syntax(line: &str) -> bool {
    // Check for array bounds in brackets
    if line.contains("<=") && (line.contains('[') || line.contains(']')) {
        return true;
    }

    // Check for string bounds (e.g., "string<=50")
    if line.contains("<=") && (line.contains("string") || line.contains("wstring")) {
        return true;
    }

    false
}

/// Parse a constant definition line
fn parse_constant_line(line: &str) -> ParseResult<LineContent> {
    let parts: Vec<&str> = line.splitn(2, CONSTANT_SEPARATOR).collect();
    if parts.len() != 2 {
        return Err(ParseError::InvalidConstant {
            reason: "constant must have format: TYPE NAME=VALUE".to_string(),
        });
    }

    let left_part = parts[0].trim();
    let value_part = parts[1].trim();

    // Parse type and name from left part
    let type_name_parts: Vec<&str> = left_part.split_whitespace().collect();
    if type_name_parts.len() != 2 {
        return Err(ParseError::InvalidConstant {
            reason: "constant must have format: TYPE NAME=VALUE".to_string(),
        });
    }

    let type_name = type_name_parts[0];
    let const_name = type_name_parts[1];

    let constant = Constant::new(type_name, const_name, value_part)?;
    Ok(LineContent::Constant(constant))
}

/// Parse a field definition line
fn parse_field_line(line: &str, pkg_name: &str) -> ParseResult<LineContent> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(ParseError::InvalidField {
            reason: "field must have at least type and name".to_string(),
        });
    }

    let type_string = parts[0];
    let field_name = parts[1];

    // Check for default value
    let default_value = if parts.len() > 2 {
        Some(parts[2..].join(" "))
    } else {
        None
    };

    let field_type = Type::new(type_string, Some(pkg_name))?;
    let field = Field::new(field_type, field_name, default_value.as_deref())?;

    Ok(LineContent::Field(field))
}

/// Process comments to extract special annotations like units
fn process_comments(spec: &mut MessageSpecification) {
    // Process message-level comments
    process_element_comments(&mut spec.annotations);

    // Process field comments
    for field in &mut spec.fields {
        process_element_comments(&mut field.annotations);
    }

    // Process constant comments
    for constant in &mut spec.constants {
        process_element_comments(&mut constant.annotations);
    }
}

/// Process comments for a single element to extract special annotations
fn process_element_comments(annotations: &mut Annotations) {
    if let Some(AnnotationValue::StringList(comments)) = annotations.get("comment").cloned() {
        // Look for unit annotations in brackets
        let comment_text = comments.join("\n");

        let mut processed_comments = if let Some(unit) = extract_unit_from_comment(&comment_text) {
            annotations.insert("unit".to_string(), AnnotationValue::String(unit.clone()));

            // Remove unit from comments
            comments
                .into_iter()
                .map(|line| remove_unit_from_line(&line, &unit))
                .collect()
        } else {
            comments
        };

        // Remove empty lines and update comments
        processed_comments.retain(|line| !line.trim().is_empty());

        if processed_comments.is_empty() {
            annotations.remove("comment");
        } else {
            annotations.insert(
                "comment".to_string(),
                AnnotationValue::StringList(processed_comments),
            );
        }
    }
}

/// Extract unit annotation from comment text
fn extract_unit_from_comment(comment: &str) -> Option<String> {
    // Look for [unit] pattern that doesn't contain commas
    let re = regex::Regex::new(r"\[([^,\]]+)\]").ok()?;
    let captures = re.captures(comment)?;
    captures.get(1).map(|m| m.as_str().trim().to_string())
}

/// Remove unit annotation from a comment line
fn remove_unit_from_line(line: &str, unit: &str) -> String {
    let pattern = format!("[{unit}]");
    line.replace(&pattern, "").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::validation::PrimitiveValue;

    #[test]
    fn test_parse_simple_message() {
        let content = r"
# This is a test message
int32 x
int32 y
string name
";

        let spec = parse_message_string("test_msgs", "TestMessage", content).unwrap();
        assert_eq!(spec.pkg_name, "test_msgs");
        assert_eq!(spec.msg_name, "TestMessage");
        assert_eq!(spec.fields.len(), 3);
        assert_eq!(spec.fields[0].name, "x");
        assert_eq!(spec.fields[1].name, "y");
        assert_eq!(spec.fields[2].name, "name");
    }

    #[test]
    fn test_parse_message_with_constants() {
        let content = r#"
# Constants
int32 MAX_VALUE=100
string DEFAULT_NAME="test"

# Fields
int32 value
string name
"#;

        let spec = parse_message_string("test_msgs", "TestMessage", content).unwrap();
        assert_eq!(spec.constants.len(), 2);
        assert_eq!(spec.fields.len(), 2);

        let max_const = spec.get_constant("MAX_VALUE").unwrap();
        assert_eq!(max_const.value, PrimitiveValue::Int32(100));
    }

    #[test]
    fn test_parse_message_with_arrays() {
        let content = r"
int32[] dynamic_array
int32[5] fixed_array
int32[<=10] bounded_array
";

        let spec = parse_message_string("test_msgs", "TestMessage", content).unwrap();
        assert_eq!(spec.fields.len(), 3);

        assert!(spec.fields[0].field_type.is_dynamic_array());
        assert_eq!(spec.fields[1].field_type.array_size, Some(5));
        assert!(spec.fields[2].field_type.is_bounded_array());
    }

    #[test]
    fn test_parse_message_with_comments() {
        let content = r"
# File level comment
# Second line

int32 x  # X coordinate
int32 y  # Y coordinate
";

        let spec = parse_message_string("test_msgs", "TestMessage", content).unwrap();

        // Should have file-level comments
        if let Some(AnnotationValue::StringList(comments)) = spec.annotations.get("comment") {
            assert!(comments.contains(&"File level comment".to_string()));
        }

        // Fields should have comments
        assert!(spec.fields[0].annotations.contains_key("comment"));
        assert!(spec.fields[1].annotations.contains_key("comment"));
    }

    #[test]
    fn test_parse_message_with_optional_fields() {
        let content = r"
int32 required_field
@optional
int32 optional_field
";

        let spec = parse_message_string("test_msgs", "TestMessage", content).unwrap();
        assert_eq!(spec.fields.len(), 2);

        // First field should not be optional
        assert!(!spec.fields[0].annotations.contains_key("optional"));

        // Second field should be optional
        if let Some(AnnotationValue::Bool(is_optional)) = spec.fields[1].annotations.get("optional")
        {
            assert!(is_optional);
        }
    }
}
