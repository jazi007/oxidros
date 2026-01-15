//! Code generation implementation

// Allow some clippy lints for this module
#![allow(clippy::unused_self)] // Methods use &self for API consistency
#![allow(clippy::too_many_lines)] // Some code generation methods are inherently long

use super::{
    FileType, GeneratedCode, GeneratorResult, InterfaceKind,
    config::GeneratorConfig,
    token_gen::{self, ConstantDef, FieldDefault, StructField},
    types::TypeMapper,
};
use crate::idl::parse_idl_string;
use crate::idl::types::{IdlContent, IdlType, Message};
use crate::idl_adapter::{action_to_idl, message_to_idl, service_to_idl};
use crate::{BaseType, Type, parse_action_file, parse_message_file, parse_service_file};
use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Code generator implementation
pub struct CodeGenerator {
    config: GeneratorConfig,
    type_mapper: TypeMapper,
}

impl CodeGenerator {
    /// Create a new code generator with the given configuration
    #[must_use]
    pub fn new(config: GeneratorConfig) -> Self {
        let type_mapper = if let Some(prefix) = &config.ctypes_prefix {
            TypeMapper::with_ctypes_prefix(prefix)
        } else {
            TypeMapper::new()
        };

        Self {
            config,
            type_mapper,
        }
    }

    /// Generate code from a file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file has no extension
    /// - The package name cannot be extracted
    /// - The file cannot be parsed
    /// - Code generation fails
    pub fn generate_from_file(&self, path: &Path) -> GeneratorResult<GeneratedCode> {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or(super::ConfigError::NoFileExtension)?;

        let package_name = self.extract_package_name(path)?;
        let module_name = self.get_module_name(path)?;

        let file_type = FileType::from_extension(extension).ok_or_else(|| {
            super::ConfigError::UnsupportedFileExtension {
                extension: extension.to_string(),
            }
        })?;

        // Convert source file to IDL string
        let idl_string = self.convert_to_idl(path, &package_name, file_type)?;

        // Parse IDL and generate code, detecting the interface kind from content
        let (code, dependencies, interface_kind) =
            self.generate_from_idl_string(&idl_string, &package_name)?;

        Ok(GeneratedCode {
            code,
            source_file: path.to_path_buf(),
            package_name,
            module_name,
            file_type,
            interface_kind,
            dependencies,
        })
    }

    /// Extract package name from file path
    fn extract_package_name(&self, path: &Path) -> GeneratorResult<String> {
        // Try to find pattern like: .../share/package_name/msg|srv|action|idl/...
        let components: Vec<_> = path.components().collect();

        for (i, component) in components.iter().enumerate() {
            if let Some("share") = component.as_os_str().to_str()
                && i + 1 < components.len()
                && let Some(pkg) = components[i + 1].as_os_str().to_str()
            {
                return Ok(self.config.transform_module_name(pkg, None, None));
            }
        }

        // Fallback: look for pattern package_name/msg|srv|action|idl/filename
        // Walk backwards to find msg/srv/action/idl directory
        for (i, component) in components.iter().enumerate().rev() {
            if let Some(dir) = component.as_os_str().to_str()
                && matches!(dir, "msg" | "srv" | "action" | "idl")
                && i > 0
                && let Some(pkg) = components[i - 1].as_os_str().to_str()
            {
                return Ok(self.config.transform_module_name(pkg, None, None));
            }
        }

        Err(super::ConfigError::PackageNameExtractionFailed {
            path: path.to_path_buf(),
        }
        .into())
    }

    /// Get module name from file path (basename without extension)
    fn get_module_name(&self, path: &Path) -> GeneratorResult<String> {
        Ok(path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| self.config.transform_module_name(s, None, None))
            .ok_or(super::ConfigError::ModuleNameExtractionFailed)?)
    }

    /// Extract original field types from a `MessageSpecification`
    ///
    /// Returns a map of field name -> original type name for fields that need
    /// special handling (like char types which get converted to uint8 in IDL)
    /// Convert a source file (.msg, .srv, .action, .idl) to IDL string
    fn convert_to_idl(
        &self,
        path: &Path,
        package_name: &str,
        file_type: FileType,
    ) -> GeneratorResult<String> {
        match file_type {
            FileType::Message => {
                let msg_spec = parse_message_file(package_name, path)?;
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown.msg");
                let input_file = format!("msg/{file_name}");
                Ok(message_to_idl(&msg_spec, package_name, &input_file))
            }
            FileType::Service => {
                let srv_spec = parse_service_file(package_name, path)?;
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown.srv");
                let input_file = format!("srv/{file_name}");
                Ok(service_to_idl(&srv_spec, package_name, &input_file))
            }
            FileType::Action => {
                let action_spec = parse_action_file(package_name, path)?;
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown.action");
                let input_file = format!("action/{file_name}");
                Ok(action_to_idl(&action_spec, package_name, &input_file))
            }
            FileType::Idl => Ok(std::fs::read_to_string(path)?),
        }
    }

    /// Generate Rust code from an IDL string
    ///
    /// This is the unified generation entry point. It parses the IDL and generates
    /// appropriate code based on the content (messages, services, or actions).
    ///
    /// Returns a tuple of (code, dependencies, `interface_kind`).
    fn generate_from_idl_string(
        &self,
        idl_string: &str,
        package_name: &str,
    ) -> GeneratorResult<(String, Vec<String>, InterfaceKind)> {
        // Extract typedefs from the IDL
        let typedef_map = Self::extract_typedefs_from_idl(idl_string);

        // Parse the IDL to get the structured representation
        let idl_file = parse_idl_string(idl_string)?;

        // Extract dependencies from IDL includes
        let mut dependencies =
            Self::extract_dependencies_from_includes(idl_file.content.get_includes(), package_name);

        // Generate code based on content type and detect interface kind
        let (code, interface_kind) = self.generate_from_idl_content(
            &idl_file.content,
            package_name,
            &typedef_map,
            &mut dependencies,
        )?;

        // Extract just package names for recursive dependency discovery
        let mut package_deps: HashSet<String> = HashSet::new();
        for (pkg, _, _) in &dependencies {
            package_deps.insert(pkg.clone());
        }
        let dep_strings: Vec<String> = package_deps.into_iter().collect();

        Ok((code, dep_strings, interface_kind))
    }

    /// Generate Rust code from parsed IDL content
    ///
    /// Handles all interface kinds: messages, services, and actions.
    /// Returns the generated code and the detected interface kind.
    fn generate_from_idl_content(
        &self,
        content: &IdlContent,
        package_name: &str,
        typedef_map: &HashMap<String, Type>,
        dependencies: &mut HashSet<(String, String, String)>,
    ) -> GeneratorResult<(String, InterfaceKind)> {
        let mut output = String::new();

        // Add header
        if let Some(header) = &self.config.header {
            output.push_str(header);
            output.push_str("\n\n");
        }

        // Add raw lines
        for line in &self.config.raw_lines {
            output.push_str(line);
            output.push('\n');
        }
        if !self.config.raw_lines.is_empty() {
            output.push('\n');
        }

        // Check for services first (they take priority as they contain messages)
        let services = content.get_services();
        if !services.is_empty() {
            for service in &services {
                // Collect dependencies from service messages
                for member in &service.request_message.structure.members {
                    Self::collect_dependencies_from_type(&member.member_type, dependencies);
                }
                for member in &service.response_message.structure.members {
                    Self::collect_dependencies_from_type(&member.member_type, dependencies);
                }

                // Generate request and response structs
                output.push_str(&self.generate_struct_from_idl_with_typedefs(
                    &service.request_message,
                    package_name,
                    InterfaceKind::Service,
                    typedef_map,
                    dependencies,
                ));
                output.push('\n');
                output.push_str(&self.generate_struct_from_idl_with_typedefs(
                    &service.response_message,
                    package_name,
                    InterfaceKind::Service,
                    typedef_map,
                    dependencies,
                ));
            }
            return Ok((output, InterfaceKind::Service));
        }

        // Check for actions
        let actions = content.get_actions();
        if !actions.is_empty() {
            for action in &actions {
                // Collect dependencies from action messages
                for member in &action.goal.structure.members {
                    Self::collect_dependencies_from_type(&member.member_type, dependencies);
                }
                for member in &action.result.structure.members {
                    Self::collect_dependencies_from_type(&member.member_type, dependencies);
                }
                for member in &action.feedback.structure.members {
                    Self::collect_dependencies_from_type(&member.member_type, dependencies);
                }
                // Note: SendGoal_*, GetResult_*, and FeedbackMessage dependencies
                // (UUID, Time) are handled by the ros2_action! macro

                // Generate goal, result, and feedback structs
                // Note: SendGoal_*, GetResult_*, and FeedbackMessage types are generated
                // by the ros2_action! macro from ros2-type-hash-derive when using the rcl feature.
                // This avoids duplicating the field definitions which are fixed by the ROS2 spec.
                output.push_str(&self.generate_struct_from_idl_with_typedefs(
                    &action.goal,
                    package_name,
                    InterfaceKind::Action,
                    typedef_map,
                    dependencies,
                ));
                output.push('\n');
                output.push_str(&self.generate_struct_from_idl_with_typedefs(
                    &action.result,
                    package_name,
                    InterfaceKind::Action,
                    typedef_map,
                    dependencies,
                ));
                output.push('\n');
                output.push_str(&self.generate_struct_from_idl_with_typedefs(
                    &action.feedback,
                    package_name,
                    InterfaceKind::Action,
                    typedef_map,
                    dependencies,
                ));
            }
            return Ok((output, InterfaceKind::Action));
        }

        // Fall back to messages
        let messages = content.get_messages();
        if messages.is_empty() {
            return Err(super::GenerationError::NoMessageInIdl.into());
        }

        for message in &messages {
            // Collect dependencies from message
            for member in &message.structure.members {
                Self::collect_dependencies_from_type(&member.member_type, dependencies);
            }

            output.push_str(&self.generate_struct_from_idl_with_typedefs(
                message,
                package_name,
                InterfaceKind::Message,
                typedef_map,
                dependencies,
            ));
            if messages.len() > 1 {
                output.push('\n');
            }
        }

        Ok((output, InterfaceKind::Message))
    }

    /// Extract typedefs from IDL string
    /// Parses typedef declarations like "typedef double `double__36`[36];"
    fn extract_typedefs_from_idl(idl_string: &str) -> HashMap<String, Type> {
        let mut typedefs = HashMap::new();

        // Match typedef declarations: typedef <base_type> <name>[<size>];
        for line in idl_string.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("typedef ") && trimmed.ends_with(';') {
                // Parse: "typedef double double__36[36];"
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 3 {
                    let base_type = parts[1]; // "double"
                    let declaration = parts[2].trim_end_matches(';'); // "double__36[36]"

                    if let Some(bracket_pos) = declaration.find('[') {
                        let typedef_name = &declaration[..bracket_pos]; // "double__36"
                        let size_part = &declaration[bracket_pos + 1..declaration.len() - 1]; // "36"

                        if let Ok(size) = size_part.parse::<u32>() {
                            // Map IDL type to ROS type
                            let ros_type = Self::idl_typename_to_ros(base_type);

                            typedefs.insert(
                                typedef_name.to_string(),
                                Type {
                                    base_type: BaseType {
                                        pkg_name: None,
                                        type_name: ros_type,
                                        string_upper_bound: None,
                                    },
                                    is_array: true,
                                    array_size: Some(size),
                                    is_upper_bound: false,
                                },
                            );
                        }
                    }
                }
            }
        }

        typedefs
    }

    /// Extract dependencies from IDL includes
    /// Format: "package/msg/Type.idl" -> ("package", "msg", "Type")
    fn extract_dependencies_from_includes(
        includes: Vec<&crate::idl::types::Include>,
        current_package: &str,
    ) -> std::collections::HashSet<(String, String, String)> {
        let mut dependencies = std::collections::HashSet::new();

        for include in includes {
            let locator = &include.locator;
            // Handle both "package/msg/Type.idl" and "/msg/Type.idl" (same package)
            if let Some((pkg_ftype, type_idl)) = locator.rsplit_once('/')
                && let Some(type_name) = type_idl.strip_suffix(".idl")
                && let Some((pkg, ftype)) = pkg_ftype.rsplit_once('/')
            {
                if pkg.is_empty() {
                    // Same-package reference with empty pkg
                    dependencies.insert((
                        current_package.to_string(),
                        ftype.to_string(),
                        type_name.to_string(),
                    ));
                } else {
                    // Cross-package reference
                    dependencies.insert((
                        pkg.to_string(),
                        ftype.to_string(),
                        type_name.to_string(),
                    ));
                }
            }
        }

        dependencies
    }

    /// Collect dependencies from an IDL type
    /// Returns a set of tuples (`package_name`, `file_type`, `type_name`)
    fn collect_dependencies_from_type(
        idl_type: &IdlType,
        dependencies: &mut std::collections::HashSet<(String, String, String)>,
    ) {
        match idl_type {
            IdlType::Namespaced(ns_type) => {
                // Extract package, file_type, and type name from namespace
                // Format: package_name::msg/srv/action::type_name
                if let (Some(pkg), Some(ftype)) =
                    (ns_type.namespaces.first(), ns_type.namespaces.get(1))
                    && (ftype == "msg" || ftype == "srv" || ftype == "action")
                {
                    dependencies.insert((pkg.clone(), ftype.clone(), ns_type.name.clone()));
                }
            }
            IdlType::Array(arr) => {
                Self::collect_dependencies_from_type(&arr.value_type, dependencies);
            }
            IdlType::BoundedSequence(seq) => {
                Self::collect_dependencies_from_type(&seq.value_type, dependencies);
            }
            IdlType::UnboundedSequence(seq) => {
                Self::collect_dependencies_from_type(&seq.value_type, dependencies);
            }
            _ => {}
        }
    }

    /// Format a dependency tuple (`pkg`, `ftype`, `type_name`) as a use path
    /// Returns: `pkg::ftype::type_name_snake::TypeName`
    fn format_dependency_path(pkg: &str, ftype: &str, type_name: &str) -> String {
        let module_name = type_name.to_snake_case();
        format!("{pkg}::{ftype}::{module_name}::{type_name}")
    }

    /// Determine if a field is a fixed-size array and get its size.
    /// Note: `BoundedSequence` is NOT an array - it's a sequence with max capacity.
    /// Arrays are [T; N] in Rust, sequences are Vec<T>.
    fn get_array_info(member_type: &IdlType, typedef_map: &HashMap<String, Type>) -> Option<u32> {
        match member_type {
            IdlType::Array(arr) => Some(arr.size),
            // BoundedSequence is NOT an array - it uses capacity, not array_size
            IdlType::Named(n) => {
                // Check if this is a typedef - if so, look it up in the map
                let trimmed_name = n.name.trim();
                if let Some(expanded_type) = typedef_map.get(trimmed_name) {
                    // Use the expanded typedef
                    if expanded_type.is_array {
                        expanded_type.array_size
                    } else {
                        None
                    }
                } else {
                    // Not a typedef, check for array syntax in the name
                    if n.name.contains('[')
                        && n.name.contains(']')
                        && let Some(start) = n.name.find('[')
                        && let Some(end) = n.name.find(']')
                        && let Ok(size) = n.name[start + 1..end].parse::<u32>()
                    {
                        return Some(size);
                    }
                    None
                }
            }
            _ => None,
        }
    }

    /// Get the default value for a Rust type
    fn get_type_default_value(field_type: &str) -> String {
        // Handle Vec and arrays
        if field_type.starts_with("Vec<") {
            return "::std::vec::Vec::new()".to_string();
        }
        if field_type.starts_with('[') {
            // Extract array size to check if > 32
            if let Some(semicolon_pos) = field_type.find(';')
                && let Some(bracket_pos) = field_type.rfind(']')
            {
                let size_str = field_type[semicolon_pos + 1..bracket_pos].trim();
                // For arrays > 32, we need unsafe zeroed because Default is not implemented
                // SAFETY: This is only used for repr(C) primitive types which are valid when zeroed
                if let Ok(size) = size_str.parse::<usize>()
                    && size > 32
                {
                    return "unsafe { ::std::mem::zeroed() }".to_string();
                }
            }
            // For arrays <= 32, use Default::default()
            return "::core::default::Default::default()".to_string();
        }

        // Handle primitive types
        match field_type {
            "bool" => "false".to_string(),
            "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128" => {
                "0".to_string()
            }
            "f32" | "f64" => "0.0".to_string(),
            "String" => "::std::string::String::new()".to_string(),
            t if t.contains("::c_char") => "0".to_string(),
            // For custom types, use Default::default()
            _ => "::core::default::Default::default()".to_string(),
        }
    }

    /// Format a default value from annotation for the given field type
    fn format_default_value(field_type: &str, default_val: &str) -> String {
        // Handle Vec
        if field_type.starts_with("Vec<") {
            // Parse array literal like [1, 2, 3]
            if default_val.starts_with('[') && default_val.ends_with(']') {
                return format!("vec!{default_val}");
            }
            return "::std::vec::Vec::new()".to_string();
        }

        // Handle arrays
        if field_type.starts_with('[') {
            return default_val.to_string();
        }

        // Handle primitive types
        match field_type {
            "String" => format!("\"{default_val}\".to_string()"),
            _ => default_val.to_string(),
        }
    }

    /// Resolve field type with full paths for nested types to avoid ambiguity
    fn resolve_field_type_with_dependencies(
        field_type: String,
        interface_kind: InterfaceKind,
        dependencies: &HashSet<(String, String, String)>,
    ) -> String {
        // For nested types (types from other packages/modules), prepend the full module path
        // to avoid ambiguity with std types (like String vs std::string::String)
        if field_type.starts_with("Vec<") {
            // Handle Vec<T> - extract inner type and apply full path if it's a dependency
            if let Some(inner_start) = field_type.find('<')
                && let Some(inner_end) = field_type.rfind('>')
            {
                let inner_type = &field_type[inner_start + 1..inner_end];
                let inner_type_clean = inner_type.split_whitespace().next().unwrap_or(inner_type);
                if dependencies
                    .iter()
                    .any(|(_, _, type_name)| type_name == inner_type_clean)
                    && let Some((pkg, ftype, _)) = dependencies
                        .iter()
                        .find(|(_, _, tn)| tn == inner_type_clean)
                {
                    let depth = interface_kind.import_depth();
                    let super_path = "super::".repeat(depth);
                    let full_path = format!(
                        "{super_path}{}",
                        Self::format_dependency_path(pkg, ftype, inner_type_clean)
                    );
                    return format!("Vec<{full_path}>");
                }
            }
        } else if field_type.starts_with('[') {
            // Handle arrays [T; N] - extract inner type and apply full path if it's a dependency
            if let Some(semicolon_pos) = field_type.find(';') {
                let inner_type = field_type[1..semicolon_pos].trim();
                let array_size_part = &field_type[semicolon_pos..];
                if dependencies
                    .iter()
                    .any(|(_, _, type_name)| type_name == inner_type)
                    && let Some((pkg, ftype, _)) =
                        dependencies.iter().find(|(_, _, tn)| tn == inner_type)
                {
                    let depth = interface_kind.import_depth();
                    let super_path = "super::".repeat(depth);
                    let full_path = format!(
                        "{}{}",
                        super_path,
                        Self::format_dependency_path(pkg, ftype, inner_type)
                    );
                    return format!("[{full_path}{array_size_part}");
                }
            }
        } else if field_type.ends_with('>') && field_type.contains("Seq<") {
            // Handle custom Seq types like FieldSeq<0>, SetParametersResultSeq<0>
            // Pattern: {TypeName}Seq<{N}>
            if let Some(seq_pos) = field_type.find("Seq<") {
                let base_type_name = &field_type[..seq_pos];
                let generic_part = &field_type[seq_pos + 3..]; // "<N>"
                if dependencies
                    .iter()
                    .any(|(_, _, type_name)| type_name == base_type_name)
                    && let Some((pkg, ftype, _)) =
                        dependencies.iter().find(|(_, _, tn)| tn == base_type_name)
                {
                    let depth = interface_kind.import_depth();
                    let super_path = "super::".repeat(depth);
                    let full_path = format!(
                        "{}{}",
                        super_path,
                        Self::format_dependency_path(pkg, ftype, base_type_name)
                    );
                    return format!("{full_path}Seq{generic_part}");
                }
            }
        } else if !field_type.contains("::") {
            // Direct field type (not Vec, not array, not already a path)
            if dependencies
                .iter()
                .any(|(_, _, type_name)| type_name == &field_type)
                && let Some((pkg, ftype, _)) =
                    dependencies.iter().find(|(_, _, tn)| tn == &field_type)
            {
                let depth = interface_kind.import_depth();
                let super_path = "super::".repeat(depth);
                return format!(
                    "{}{}",
                    super_path,
                    Self::format_dependency_path(pkg, ftype, &field_type)
                );
            }
        }
        field_type
    }

    /// Generate a Rust struct from an IDL Message with typedef resolution
    ///
    /// Uses TokenStream-based code generation with prettyplease formatting
    #[allow(clippy::too_many_lines)]
    fn generate_struct_from_idl_with_typedefs(
        &self,
        message: &Message,
        package_name: &str,
        interface_kind: InterfaceKind,
        typedef_map: &HashMap<String, Type>,
        dependencies: &HashSet<(String, String, String)>,
    ) -> String {
        let struct_name = self.config.transform_item_name(
            &message.structure.namespaced_type.name,
            package_name,
            interface_kind,
        );

        // Collect all derives (standard + callback-provided)
        let mut all_derives = self.config.derives.clone();
        let mut custom_attributes = Vec::new();

        if let Some(cb) = &self.config.parse_callbacks {
            use super::callbacks::ItemInfo;
            let info = ItemInfo::new(
                struct_name.clone(),
                String::new(),
                package_name.to_string(),
                interface_kind,
            );
            all_derives.extend(cb.add_derives(&info));
            custom_attributes = cb.add_attributes(&info);
        }

        // Check if Default is in derives - if so, we'll implement it manually
        let has_default = all_derives.iter().any(|d| d == "Default");
        if has_default {
            all_derives.retain(|d| d != "Default");
        }

        // Collect fields
        let mut fields: Vec<StructField> = Vec::new();
        let mut field_defaults: Vec<FieldDefault> = Vec::new();

        for member in &message.structure.members {
            let mut field_type =
                self.map_idl_type_to_rust_with_typedefs(&member.member_type, typedef_map);

            // Resolve field types with full paths for dependencies to avoid ambiguity
            field_type = Self::resolve_field_type_with_dependencies(
                field_type,
                interface_kind,
                dependencies,
            );

            // Note: ROS2 .msg 'char' type is actually uint8, NOT the IDL char type.
            // The IDL char type (type_id 13) is only used for IDL files with explicit 'char'.
            // For .msg files, 'char' is converted to uint8 in IDL and should remain as u8 in Rust.
            // We no longer need special handling for char from .msg files.

            // Determine if this is an array and get the size (needed for both callbacks and field name transform)
            let array_size = Self::get_array_info(&member.member_type, typedef_map);
            let ros_type_name = Self::get_ros_type_name(&member.member_type);

            // Determine ROS2 type override for byte/char/wstring
            // Note: "char" here refers to IDL char type, not .msg char (which is uint8)
            let ros2_type_override = match ros_type_name.as_str() {
                "octet" | "octet[]" => Some("byte".to_string()),
                "char" | "char[]" => Some("char".to_string()),
                "wstring" | "wstring[]" => Some("wstring".to_string()),
                _ => None,
            };

            // Get capacity for bounded types (sequence capacity)
            let capacity = Self::get_capacity(&member.member_type);

            // Get string capacity for bounded strings within sequences
            let string_capacity = Self::get_string_capacity(&member.member_type);

            // Get default value
            let default_value_annotation = member.annotations.get_default_value().clone();

            let field_name = self.config.transform_field_name(
                &member.name,
                &struct_name,
                package_name,
                &field_type,
                &ros_type_name,
                array_size,
            );

            // Collect field-level attributes via callback
            let mut field_attrs = Vec::new();
            if let Some(cb) = &self.config.parse_callbacks {
                use super::callbacks::FieldInfo;

                let field_info = FieldInfo::new(
                    field_name.clone(),
                    field_type.clone(),
                    struct_name.clone(),
                    package_name.to_string(),
                    ros_type_name.clone(),
                    array_size,
                    ros2_type_override,
                    capacity,
                    string_capacity,
                    default_value_annotation.clone(),
                );
                field_attrs = cb.add_field_attributes(&field_info);
            }

            // Create field
            fields.push(StructField {
                name: field_name.clone(),
                rust_type: field_type.clone(),
                attributes: field_attrs,
            });

            // Collect default value for Default impl
            if has_default {
                let default_value = if let Some(default_val) = &default_value_annotation {
                    Self::format_default_value(&field_type, default_val)
                } else {
                    Self::get_type_default_value(&field_type)
                };
                field_defaults.push(FieldDefault::new(field_name, default_value));
            }
        }

        // Collect constants
        let constants: Vec<ConstantDef> = message
            .constants
            .iter()
            .map(|constant| {
                let const_name =
                    self.config
                        .transform_item_name(&constant.name, package_name, interface_kind);
                let const_type = self.map_idl_type_to_rust_for_constant(&constant.constant_type);
                let const_value = Self::format_idl_value(&constant.value);
                ConstantDef::new(const_name, const_type, const_value)
            })
            .collect();

        // Generate TokenStreams
        let mut tokens_vec: Vec<TokenStream> = Vec::new();

        // Generate struct
        let struct_tokens =
            token_gen::generate_struct(&struct_name, &all_derives, &custom_attributes, &fields);
        tokens_vec.push(struct_tokens);

        // Generate Default impl if requested
        if has_default {
            let default_tokens = token_gen::generate_default_impl(&struct_name, &field_defaults);
            tokens_vec.push(default_tokens);
        }

        // Add custom implementations via callback
        if let Some(cb) = &self.config.parse_callbacks {
            use super::callbacks::ItemInfo;
            let info = ItemInfo::new(
                struct_name.clone(),
                String::new(),
                package_name.to_string(),
                interface_kind,
            );
            // Check custom_impl (string-based)
            if let Some(custom_impl) = cb.custom_impl(&info) {
                // Parse the custom impl string into tokens
                if let Ok(impl_tokens) = custom_impl.parse::<TokenStream>() {
                    tokens_vec.push(impl_tokens);
                }
            }
            // Check custom_impl_tokens (TokenStream-based)
            if let Some(impl_tokens) = cb.custom_impl_tokens(&info) {
                tokens_vec.push(impl_tokens);
            }
        }

        // Generate constants impl block
        if !constants.is_empty() {
            let const_tokens = token_gen::generate_constants_impl(&struct_name, &constants);
            tokens_vec.push(const_tokens);
        }

        // Format all tokens together
        match token_gen::format_token_streams(tokens_vec) {
            Ok(formatted) => formatted,
            Err(e) => {
                // Fallback: if formatting fails, just convert tokens to string
                eprintln!("Warning: prettyplease formatting failed: {e}");
                String::new()
            }
        }
    }

    /// Generate a Rust struct from an IDL Message (without typedef resolution)
    /// Map an IDL type to Rust type using the `TypeMapper`
    #[must_use]
    pub fn map_idl_type_to_rust(&self, idl_type: &IdlType) -> String {
        // Convert IDL type to old Type format and use TypeMapper
        let ros_type = Self::idl_type_to_ros_type(idl_type);
        self.type_mapper.map_type_in_context(&ros_type)
    }

    /// Map an IDL type to Rust type for constant declarations
    /// String constants should use &str instead of String
    #[must_use]
    pub fn map_idl_type_to_rust_for_constant(&self, idl_type: &IdlType) -> String {
        // Check if it's a string type
        if matches!(idl_type, IdlType::String(_) | IdlType::WString(_)) {
            return "&str".to_string();
        }

        // For other types, use the normal mapping
        self.map_idl_type_to_rust(idl_type)
    }

    /// Map an IDL type to Rust type with typedef resolution
    fn map_idl_type_to_rust_with_typedefs(
        &self,
        idl_type: &IdlType,
        typedef_map: &HashMap<String, Type>,
    ) -> String {
        // Convert IDL type to old Type format with typedef resolution
        let ros_type = Self::idl_type_to_ros_type_with_typedefs(idl_type, typedef_map);
        // Pass callbacks to type mapper for custom type mapping
        let callbacks = self
            .config
            .parse_callbacks
            .as_ref()
            .map(std::convert::AsRef::as_ref);
        self.type_mapper
            .map_type_in_context_with_callbacks(&ros_type, callbacks)
    }

    /// Convert an IDL type to the old Type representation with typedef resolution
    fn idl_type_to_ros_type_with_typedefs(
        idl_type: &IdlType,
        typedef_map: &HashMap<String, Type>,
    ) -> Type {
        match idl_type {
            IdlType::Named(named) => {
                // Trim the name to handle any trailing/leading whitespace from the parser
                let trimmed_name = named.name.trim();

                // Check if this is a typedef
                if let Some(expanded_type) = typedef_map.get(trimmed_name) {
                    return expanded_type.clone();
                }

                // Not a typedef, treat as normal named type
                Type {
                    base_type: BaseType {
                        pkg_name: None,
                        // Use idl_typename_to_ros to map IDL types to ROS types (e.g., octet -> byte)
                        type_name: Self::idl_typename_to_ros(trimmed_name),
                        string_upper_bound: None,
                    },
                    is_array: false,
                    array_size: None,
                    is_upper_bound: false,
                }
            }
            // For all other types, delegate to the original method
            _ => Self::idl_type_to_ros_type(idl_type),
        }
    }

    /// Convert an IDL type to the old Type representation
    fn idl_type_to_ros_type(idl_type: &IdlType) -> Type {
        match idl_type {
            IdlType::Basic(basic) => Type {
                base_type: BaseType {
                    pkg_name: None,
                    type_name: Self::idl_typename_to_ros(basic.typename()),
                    string_upper_bound: None,
                },
                is_array: false,
                array_size: None,
                is_upper_bound: false,
            },
            IdlType::String(s) => Type {
                base_type: BaseType {
                    pkg_name: None,
                    type_name: "string".to_string(),
                    string_upper_bound: s.maximum_size(),
                },
                is_array: false,
                array_size: None,
                is_upper_bound: false,
            },
            IdlType::WString(s) => Type {
                base_type: BaseType {
                    pkg_name: None,
                    type_name: "wstring".to_string(),
                    string_upper_bound: s.maximum_size(),
                },
                is_array: false,
                array_size: None,
                is_upper_bound: false,
            },
            IdlType::Array(arr) => {
                let mut inner_type = Self::idl_type_to_ros_type(&arr.value_type);
                inner_type.is_array = true;
                inner_type.array_size = Some(arr.size);
                inner_type.is_upper_bound = false;
                inner_type
            }
            IdlType::BoundedSequence(seq) => {
                let mut inner_type = Self::idl_type_to_ros_type(&seq.value_type);
                inner_type.is_array = true;
                inner_type.array_size = Some(seq.maximum_size);
                inner_type.is_upper_bound = true;
                inner_type
            }
            IdlType::UnboundedSequence(seq) => {
                let mut inner_type = Self::idl_type_to_ros_type(&seq.value_type);
                inner_type.is_array = true;
                inner_type.array_size = None;
                inner_type.is_upper_bound = false;
                inner_type
            }
            IdlType::Namespaced(ns_type) => {
                // Extract package name from namespaces (first element)
                let pkg_name = if ns_type.namespaces.is_empty() {
                    None
                } else {
                    Some(ns_type.namespaces[0].clone())
                };

                Type {
                    base_type: BaseType {
                        pkg_name,
                        type_name: ns_type.name.clone(),
                        string_upper_bound: None,
                    },
                    is_array: false,
                    array_size: None,
                    is_upper_bound: false,
                }
            }
            IdlType::Named(named) => Type {
                base_type: BaseType {
                    pkg_name: None,
                    // Trim the name to handle trailing/leading whitespace from the parser
                    type_name: Self::idl_typename_to_ros(named.name.trim()),
                    string_upper_bound: None,
                },
                is_array: false,
                array_size: None,
                is_upper_bound: false,
            },
        }
    }

    /// Convert IDL type name to ROS type name
    fn idl_typename_to_ros(idl_type: &str) -> String {
        match idl_type {
            "boolean" => "bool",
            "octet" => "byte",
            "char" => "char",
            "int8" => "int8",
            "uint8" => "uint8",
            "int16" | "short" => "int16",
            "uint16" | "unsigned short" => "uint16",
            "int32" | "long" => "int32",
            "uint32" | "unsigned long" => "uint32",
            "int64" | "long long" => "int64",
            "uint64" | "unsigned long long" => "uint64",
            "float" => "float32",
            "double" => "float64",
            _ => idl_type,
        }
        .to_string()
    }

    /// Get ROS type name from IDL type (for callbacks)
    fn get_ros_type_name(idl_type: &IdlType) -> String {
        match idl_type {
            IdlType::Basic(basic) => basic.typename().to_string(),
            IdlType::String(_) => "string".to_string(),
            IdlType::WString(_) => "wstring".to_string(),
            IdlType::Namespaced(ns) => ns.name.clone(),
            IdlType::Named(named) => named.name.clone(),
            IdlType::Array(arr) => format!("{}[]", Self::get_ros_type_name(&arr.value_type)),
            IdlType::BoundedSequence(seq) => {
                format!("{}[]", Self::get_ros_type_name(&seq.value_type))
            }
            IdlType::UnboundedSequence(seq) => {
                format!("{}[]", Self::get_ros_type_name(&seq.value_type))
            }
        }
    }

    /// Get the capacity of bounded types (strings, wstrings, sequences)
    fn get_capacity(idl_type: &IdlType) -> Option<u32> {
        match idl_type {
            IdlType::String(s) => s.maximum_size(),
            IdlType::WString(w) => w.maximum_size(),
            IdlType::BoundedSequence(seq) => Some(seq.maximum_size),
            _ => None,
        }
    }

    /// Get the string capacity for bounded strings within sequences
    /// Returns the string's maximum size if the sequence element is a bounded string
    fn get_string_capacity(idl_type: &IdlType) -> Option<u32> {
        match idl_type {
            IdlType::BoundedSequence(seq) => match seq.value_type.as_ref() {
                IdlType::String(s) => s.maximum_size(),
                IdlType::WString(w) => w.maximum_size(),
                _ => None,
            },
            IdlType::UnboundedSequence(seq) => match seq.value_type.as_ref() {
                IdlType::String(s) => s.maximum_size(),
                IdlType::WString(w) => w.maximum_size(),
                _ => None,
            },
            _ => None,
        }
    }

    /// Format an IDL value as Rust code
    /// Format an IDL value as Rust code
    fn format_idl_value(value: &crate::idl::values::IdlValue) -> String {
        use crate::idl::values::IdlValue;
        match value {
            IdlValue::Bool(b) => b.to_string(),
            IdlValue::Int8(i) => i.to_string(),
            IdlValue::UInt8(u) => u.to_string(),
            IdlValue::Int16(i) => i.to_string(),
            IdlValue::UInt16(u) => u.to_string(),
            IdlValue::Int32(i) => i.to_string(),
            IdlValue::UInt32(u) => u.to_string(),
            IdlValue::Int64(i) => i.to_string(),
            IdlValue::UInt64(u) => u.to_string(),
            IdlValue::Float32(f) => f.to_string(),
            IdlValue::Float64(f) => f.to_string(),
            IdlValue::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
            IdlValue::Char(c) => format!("'{c}'"),
            IdlValue::Array(_) => "/* array */".to_string(),
            IdlValue::Object(_) => "/* object */".to_string(),
            IdlValue::Null => "/* null */".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::idl::parse_idl_string;

    #[test]
    fn test_typedef_extraction_and_resolution() {
        let idl = r"
module test {
  module msg {
    typedef double double__9[9];
    struct TestArray {
      double__9 data;
    };
  };
};";

        // Test extraction
        let typedef_map = CodeGenerator::extract_typedefs_from_idl(idl);
        assert!(!typedef_map.is_empty(), "Should extract typedef from IDL");
        assert!(
            typedef_map.contains_key("double__9"),
            "Should have double__9 typedef"
        );

        // Test IDL parsing and typedef resolution
        let parsed = parse_idl_string(idl).unwrap();
        let messages = parsed.content.get_messages();
        assert_eq!(messages.len(), 1, "Should parse one message");

        let field = &messages[0].structure.members[0];

        // Verify typedef resolution works (handles trailing whitespace from parser)
        // Test typedef resolution
        let resolved_type =
            CodeGenerator::idl_type_to_ros_type_with_typedefs(&field.member_type, &typedef_map);
        assert!(resolved_type.is_array, "Should resolve to array type");
        assert_eq!(
            resolved_type.array_size,
            Some(9),
            "Should be array of size 9"
        );
        assert_eq!(resolved_type.base_type.type_name, "float64");
    }
}
