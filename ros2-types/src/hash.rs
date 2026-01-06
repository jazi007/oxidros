//! RIHS01 hash calculation
//!
//! Implements the ROS Interface Hashing Standard version 1

use crate::{Result, types::TypeDescriptionMsg};
use sha2::{Digest, Sha256};

/// RIHS01 version prefix
const RIHS01_PREFIX: &str = "RIHS01_";

/// Calculate the RIHS01 type hash for a type description
///
/// This function implements the ROS Interface Hashing Standard version 1,
/// which creates a canonical JSON representation of the type and hashes it
/// with SHA256.
///
/// The hash format is: `RIHS01_<64_character_hex_sha256>`
///
/// Implementation details matching rosidl_generator_type_description:
/// 1. default_value fields are removed from all field descriptions
/// 2. JSON uses Python-style separators: ', ' and ': ' (space after comma and colon)
/// 3. Keys are not alphabetically sorted (preserves insertion order)
/// 4. Top-level key order: "type_description" first, then "referenced_type_descriptions"
/// 5. Referenced type descriptions are sorted alphabetically by type_name
///
/// # Arguments
///
/// * `type_description` - The complete type description including referenced types
///
/// # Errors
///
/// Returns an error if JSON serialization fails
pub fn calculate_type_hash(type_description: &TypeDescriptionMsg) -> Result<String> {
    // Create canonical JSON representation matching rosidl format
    // Per rosidl_generator_type_description/__init__.py:calculate_type_hash():
    // 1. Remove default_value fields from all fields
    // 2. Use separators=(', ', ': ') - note the space after comma and colon
    // 3. sort_keys=False - Python 3.7+ dicts preserve insertion order
    // 4. Key order: "type_description" FIRST, then "referenced_type_descriptions"

    // Build JSON manually to control exact key ordering
    fn escape_json_string(s: &str) -> String {
        // Simple JSON string escaping
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    fn field_to_json(field: &crate::types::Field) -> String {
        format!(
            r#"{{"name": "{}", "type": {{"type_id": {}, "capacity": {}, "string_capacity": {}, "nested_type_name": "{}"}}}}"#,
            escape_json_string(&field.name),
            field.field_type.type_id,
            field.field_type.capacity,
            field.field_type.string_capacity,
            escape_json_string(&field.field_type.nested_type_name)
        )
    }

    fn type_desc_to_json(td: &crate::types::IndividualTypeDescription) -> String {
        let fields_json: Vec<String> = td.fields.iter().map(field_to_json).collect();
        format!(
            r#"{{"type_name": "{}", "fields": [{}]}}"#,
            escape_json_string(&td.type_name),
            fields_json.join(", ")
        )
    }

    // Build the complete JSON structure with exact key ordering
    let type_desc_json = type_desc_to_json(&type_description.type_description);

    // Sort referenced type descriptions by type_name (as rosidl does)
    let mut sorted_refs = type_description.referenced_type_descriptions.clone();
    sorted_refs.sort_by(|a, b| a.type_name.cmp(&b.type_name));

    let ref_types_json: Vec<String> = sorted_refs.iter().map(type_desc_to_json).collect();

    let hashable_repr = format!(
        r#"{{"type_description": {}, "referenced_type_descriptions": [{}]}}"#,
        type_desc_json,
        ref_types_json.join(", ")
    );

    // Calculate SHA256 hash
    let mut hasher = Sha256::new();
    hasher.update(hashable_repr.as_bytes());
    let hash_result = hasher.finalize();

    // Format as RIHS01 hash string
    let hash_hex = format!("{:x}", hash_result);
    Ok(format!("{}{}", RIHS01_PREFIX, hash_hex))
}

/// Parse a RIHS hash string and extract version and hash value
///
/// # Arguments
///
/// * `rihs_str` - RIHS formatted string (e.g., "RIHS01_abc123...")
///
/// # Returns
///
/// Returns `(version, hash_value)` tuple
///
/// # Errors
///
/// Returns an error if the string is not in valid RIHS format
pub fn parse_rihs_string(rihs_str: &str) -> Result<(u32, String)> {
    if !rihs_str.starts_with("RIHS") {
        return Err(crate::error::InvalidRihsFormat::MissingPrefix.into());
    }

    let parts: Vec<&str> = rihs_str.split('_').collect();
    if parts.len() != 2 {
        return Err(crate::error::InvalidRihsFormat::InvalidStructure.into());
    }

    let version_str = parts[0]
        .strip_prefix("RIHS")
        .ok_or(crate::error::InvalidRihsFormat::VersionExtractionFailed)?;

    let version = version_str.parse::<u32>().map_err(|_| {
        crate::error::InvalidRihsFormat::InvalidVersionNumber {
            version_str: version_str.to_string(),
        }
    })?;

    let hash_value = parts[1].to_string();

    Ok((version, hash_value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Field, FieldType, IndividualTypeDescription};

    #[test]
    fn test_parse_rihs_string() {
        let rihs = "RIHS01_abc123def456";
        let (version, hash) = parse_rihs_string(rihs).unwrap();
        assert_eq!(version, 1);
        assert_eq!(hash, "abc123def456");
    }

    #[test]
    fn test_parse_rihs_invalid() {
        assert!(parse_rihs_string("invalid").is_err());
        assert!(parse_rihs_string("RIHS_nope").is_err());
        assert!(parse_rihs_string("RIHS01").is_err());
    }

    #[test]
    fn test_calculate_hash_format() {
        let type_desc = IndividualTypeDescription::new(
            "test_pkg/msg/TestMsg",
            vec![Field::new(
                "field1",
                FieldType::primitive(crate::types::FIELD_TYPE_INT32),
            )],
        );

        let msg = TypeDescriptionMsg::new(type_desc, vec![]);
        let hash = calculate_type_hash(&msg).unwrap();

        assert!(hash.starts_with(RIHS01_PREFIX));
        assert_eq!(hash.len(), RIHS01_PREFIX.len() + 64); // SHA256 = 64 hex chars
    }
}
