//! Configuration for the code generator

use super::callbacks::ParseCallbacks;
use heck::ToSnakeCase;
use std::path::PathBuf;
use std::sync::Arc;

/// Generator configuration
#[derive(Clone)]
pub struct GeneratorConfig {
    /// Derives to add to all generated structs
    pub derives: Vec<String>,

    /// Raw lines to add at the top of generated files
    pub raw_lines: Vec<String>,

    /// Header comment to add to generated files
    pub header: Option<String>,

    /// Prefix for C types (e.g., "`std::os::raw`")
    pub ctypes_prefix: Option<String>,

    /// Whether to emit cargo:rerun-if-changed directives
    pub emit_rerun_if_changed: bool,

    /// Parse callbacks for customization
    pub parse_callbacks: Option<Arc<dyn ParseCallbacks>>,

    /// Output directory for generated files
    pub output_dir: Option<PathBuf>,

    /// Input file for single-file generation
    pub input_file: Option<PathBuf>,

    /// Output file for single-file generation
    pub output_file: Option<PathBuf>,

    /// Allowlist of items to include (if empty, include all)
    pub allowlist: Vec<String>,

    /// Blocklist of items to exclude
    pub blocklist: Vec<String>,

    /// Whether to recursively include dependencies
    pub allowlist_recursively: bool,

    /// Package search paths for finding dependencies
    pub package_search_paths: Vec<PathBuf>,
}

impl GeneratorConfig {
    /// Create a new default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            derives: Vec::new(),
            raw_lines: Vec::new(),
            header: None,
            ctypes_prefix: None,
            emit_rerun_if_changed: false,
            parse_callbacks: None,
            output_dir: None,
            input_file: None,
            output_file: None,
            allowlist: Vec::new(),
            blocklist: Vec::new(),
            allowlist_recursively: false,
            package_search_paths: Vec::new(),
        }
    }

    /// Transform an item name using callbacks if available
    #[must_use]
    pub fn transform_item_name(
        &self,
        name: &str,
        package: &str,
        interface_kind: super::InterfaceKind,
    ) -> String {
        if let Some(cb) = &self.parse_callbacks {
            use super::callbacks::ItemInfo;
            let info = ItemInfo::new(
                name.to_string(),
                String::new(),
                package.to_string(),
                interface_kind,
            );
            if let Some(transformed) = cb.item_name(&info) {
                return sanitize_rust_identifier(&transformed);
            }
        }
        sanitize_rust_identifier(name)
    }

    /// Transform a field name using callbacks if available
    #[must_use]
    pub fn transform_field_name(
        &self,
        name: &str,
        parent_name: &str,
        package: &str,
        field_type: &str,
        ros_type_name: &str,
        array_size: Option<u32>,
    ) -> String {
        if let Some(cb) = &self.parse_callbacks {
            use super::callbacks::FieldInfo;
            let info = FieldInfo::new(
                name.to_string(),
                field_type.to_string(),
                parent_name.to_string(),
                package.to_string(),
                ros_type_name.to_string(),
                array_size,
                None, // ros2_type_override not available in this context
                None, // capacity not available in this context
                None, // string_capacity not available in this context
                None, // default_value not available in this context
            );
            if let Some(transformed) = cb.field_name(&info) {
                return sanitize_rust_identifier(&transformed);
            }
        }
        sanitize_rust_identifier(name)
    }

    /// Transform a module name using callbacks if available
    #[must_use]
    pub fn transform_module_name(
        &self,
        name: &str,
        package: Option<&str>,
        interface_kind: Option<super::InterfaceKind>,
    ) -> String {
        if let Some(cb) = &self.parse_callbacks
            && let (Some(pkg), Some(ik)) = (package, interface_kind)
        {
            use super::callbacks::ItemInfo;
            let info = ItemInfo::new(name.to_string(), String::new(), pkg.to_string(), ik);
            if let Some(transformed) = cb.module_name(&info) {
                return sanitize_rust_identifier(&transformed.to_snake_case());
            }
        }
        sanitize_rust_identifier(&name.to_snake_case())
    }

    /// Check if an item should be included based on allow/blocklist
    #[must_use]
    pub fn should_include_item(&self, name: &str) -> bool {
        // Check blocklist first
        if self.blocklist.iter().any(|pattern| name.contains(pattern)) {
            return false;
        }

        // If allowlist is empty, include everything (except blocklisted)
        if self.allowlist.is_empty() {
            return true;
        }

        // Check allowlist
        self.allowlist.iter().any(|pattern| name.contains(pattern))
    }
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Sanitize an identifier to be valid Rust
///
/// - Escapes Rust keywords with r# prefix
/// - Converts invalid characters to underscores
#[must_use]
pub fn sanitize_rust_identifier(name: &str) -> String {
    // Check if it's a Rust keyword
    if is_rust_keyword(name) {
        return format!("r#{name}");
    }

    // Replace invalid characters
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    // Ensure it doesn't start with a digit
    if sanitized.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("_{sanitized}")
    } else {
        sanitized
    }
}

/// Check if a string is a Rust keyword
#[must_use]
fn is_rust_keyword(s: &str) -> bool {
    matches!(
        s,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "try"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_keywords() {
        assert_eq!(sanitize_rust_identifier("type"), "r#type");
        assert_eq!(sanitize_rust_identifier("match"), "r#match");
        assert_eq!(sanitize_rust_identifier("async"), "r#async");
    }

    #[test]
    fn test_sanitize_valid_names() {
        assert_eq!(sanitize_rust_identifier("valid_name"), "valid_name");
        assert_eq!(sanitize_rust_identifier("ValidName"), "ValidName");
        assert_eq!(sanitize_rust_identifier("name123"), "name123");
    }

    #[test]
    fn test_sanitize_invalid_chars() {
        assert_eq!(sanitize_rust_identifier("invalid-name"), "invalid_name");
        assert_eq!(sanitize_rust_identifier("invalid.name"), "invalid_name");
        assert_eq!(sanitize_rust_identifier("invalid name"), "invalid_name");
    }

    #[test]
    fn test_sanitize_starts_with_digit() {
        assert_eq!(sanitize_rust_identifier("123name"), "_123name");
        assert_eq!(sanitize_rust_identifier("456"), "_456");
    }
}
