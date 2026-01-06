//! Callback trait for customizing code generation

/// Trait for customizing code generation behavior
///
/// Similar to bindgen's `ParseCallbacks`, this trait allows users to customize
/// various aspects of the code generation process.
///
/// # Example
///
/// ```
/// use ros2msg::generator::{ParseCallbacks, ItemInfo, FieldInfo};
///
/// struct MyCallbacks;
///
/// impl ParseCallbacks for MyCallbacks {
///     fn item_name(&self, info: &ItemInfo) -> Option<String> {
///         // Prefix all types with "Ros"
///         Some(format!("Ros{}", info.name()))
///     }
///
///     fn field_name(&self, field_info: &FieldInfo) -> Option<String> {
///         // Convert to snake_case if needed
///         Some(field_info.field_name().to_lowercase())
///     }
///
///     fn add_derives(&self, info: &ItemInfo) -> Vec<String> {
///         // Add Serialize/Deserialize to all types
///         vec!["serde::Serialize".to_string(), "serde::Deserialize".to_string()]
///     }
/// }
/// ```
pub trait ParseCallbacks: Send + Sync {
    /// Customize the name of a generated item (struct, enum, etc.)
    ///
    /// Return `None` to use the default name, or `Some(name)` to override.
    fn item_name(&self, _info: &ItemInfo) -> Option<String> {
        None
    }

    /// Customize the name of a field
    ///
    /// Return `None` to use the default name, or `Some(name)` to override.
    fn field_name(&self, _info: &FieldInfo) -> Option<String> {
        None
    }

    /// Customize the module name
    ///
    /// Return `None` to use the default name, or `Some(name)` to override.
    fn module_name(&self, _info: &ItemInfo) -> Option<String> {
        None
    }

    /// Add custom derives to a generated item
    ///
    /// The returned derives will be added in addition to the standard ones.
    fn add_derives(&self, _info: &ItemInfo) -> Vec<String> {
        Vec::new()
    }

    /// Add custom attributes to a generated item
    ///
    /// The returned attributes will be added before the struct/enum definition.
    ///
    /// # Example
    ///
    /// ```ignore
    /// fn add_attributes(&self, info: &ItemInfo) -> Vec<String> {
    ///     vec![
    ///         format!("#[ros2(package = \"{}\", type = \"{}\")]", info.package(), info.file_type())
    ///     ]
    /// }
    /// ```
    fn add_attributes(&self, _info: &ItemInfo) -> Vec<String> {
        Vec::new()
    }

    /// Add custom attributes to a specific field
    ///
    /// The returned attributes will be added before the field definition.
    ///
    /// # Example
    ///
    /// ```ignore
    /// fn add_field_attributes(&self, field_info: &FieldInfo) -> Vec<String> {
    ///     if field_info.field_name() == "covariance" {
    ///         vec!["#[serde(with = \"serde_big_array::BigArray\")]".to_string()]
    ///     } else {
    ///         vec![]
    ///     }
    /// }
    /// ```
    fn add_field_attributes(&self, _field_info: &FieldInfo) -> Vec<String> {
        Vec::new()
    }

    /// Add custom implementations as a string
    ///
    /// Return Rust code that will be appended after the generated type.
    /// This is a simpler alternative to `implement_trait` when you don't need `TokenStream`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// fn custom_impl(&self, info: &ItemInfo) -> Option<String> {
    ///     Some(format!(r#"
    /// impl Default for {} {{
    ///     fn default() -> Self {{
    ///         Self {{ /* ... */ }}
    ///     }}
    /// }}
    /// "#, info.name()))
    /// }
    /// ```
    fn custom_impl(&self, _info: &ItemInfo) -> Option<String> {
        None
    }

    /// Add custom implementations using `TokenStream`
    ///
    /// Return a `TokenStream` that will be appended after the generated type.
    /// This is useful when you want to generate complex implementations programmatically.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use proc_macro2::TokenStream;
    /// use quote::quote;
    ///
    /// fn custom_impl_tokens(&self, info: &ItemInfo) -> Option<TokenStream> {
    ///     let name = syn::Ident::new(info.name(), proc_macro2::Span::call_site());
    ///     Some(quote! {
    ///         impl MyTrait for #name {
    ///             fn my_method(&self) -> &str {
    ///                 stringify!(#name)
    ///             }
    ///         }
    ///     })
    /// }
    /// ```
    fn custom_impl_tokens(&self, _info: &ItemInfo) -> Option<proc_macro2::TokenStream> {
        None
    }

    /// Called when processing includes to determine if an item should be included
    ///
    /// Return `true` to include, `false` to skip. Default includes everything.
    fn include_item(&self, _info: &ItemInfo) -> bool {
        true
    }

    /// Customize the Rust type for a sequence (Vec by default)
    ///
    /// Called when mapping a ROS2 sequence type to Rust.
    /// Return `None` to use the default (`Vec<T>`), or `Some(type_string)` to override.
    ///
    /// # Arguments
    ///
    /// * `element_type` - The Rust type of the sequence elements (e.g., "u8", "f64", "`MyMessage`")
    /// * `max_size` - `Some(n)` for bounded sequences with max size n, `None` for unbounded
    ///
    /// # Example
    ///
    /// ```ignore
    /// fn sequence_type(&self, element_type: &str, max_size: Option<u32>) -> Option<String> {
    ///     // Use a custom bounded vector type for bounded sequences
    ///     if let Some(size) = max_size {
    ///         Some(format!("BoundedVec<{}, {}>", element_type, size))
    ///     } else {
    ///         None // Use default Vec<T>
    ///     }
    /// }
    /// ```
    fn sequence_type(&self, _element_type: &str, _max_size: Option<u32>) -> Option<String> {
        None
    }

    /// Customize the Rust type for a string (String by default)
    ///
    /// Called when mapping a ROS2 string type to Rust.
    /// Return `None` to use the default (`::std::string::String`), or `Some(type_string)` to override.
    ///
    /// # Arguments
    ///
    /// * `max_size` - `Some(n)` for bounded strings with max size n, `None` for unbounded
    ///
    /// # Example
    ///
    /// ```ignore
    /// fn string_type(&self, max_size: Option<u32>) -> Option<String> {
    ///     // Use a custom bounded string type for bounded strings
    ///     if let Some(size) = max_size {
    ///         Some(format!("BoundedString<{}>", size))
    ///     } else {
    ///         None // Use default String
    ///     }
    /// }
    /// ```
    fn string_type(&self, _max_size: Option<u32>) -> Option<String> {
        None
    }

    /// Customize the Rust type for a wstring (String by default)
    ///
    /// Called when mapping a ROS2 wstring (wide string) type to Rust.
    /// Return `None` to use the default (`::std::string::String`), or `Some(type_string)` to override.
    ///
    /// # Arguments
    ///
    /// * `max_size` - `Some(n)` for bounded wstrings with max size n, `None` for unbounded
    ///
    /// # Example
    ///
    /// ```ignore
    /// fn wstring_type(&self, max_size: Option<u32>) -> Option<String> {
    ///     // Use a custom wide string type
    ///     if let Some(size) = max_size {
    ///         Some(format!("BoundedWString<{}>", size))
    ///     } else {
    ///         Some("WString".to_string())
    ///     }
    /// }
    /// ```
    fn wstring_type(&self, _max_size: Option<u32>) -> Option<String> {
        None
    }

    /// Add content before a `pub mod xxx;` declaration
    ///
    /// Called when generating mod.rs files before each module declaration.
    /// Return `None` to add nothing, or `Some(content)` to insert before `pub mod xxx;`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// fn pre_module(&self, info: &ModuleInfo) -> Option<String> {
    ///     // Add a doc comment before type modules
    ///     if matches!(info.module_level(), ModuleLevel::Type(_)) {
    ///         Some(format!("/// Module for {} type\n", info.module_name()))
    ///     } else {
    ///         None
    ///     }
    /// }
    /// ```
    fn pre_module(&self, _info: &ModuleInfo) -> Option<String> {
        None
    }

    /// Add content after a `pub mod xxx;` declaration
    ///
    /// Called when generating mod.rs files after each module declaration.
    /// Return `None` to add nothing, or `Some(content)` to insert after `pub mod xxx;`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// fn post_module(&self, info: &ModuleInfo) -> Option<String> {
    ///     // Re-export all items from type modules
    ///     if matches!(info.module_level(), ModuleLevel::Type(_)) {
    ///         Some(format!("pub use {}::*;\n", info.module_name()))
    ///     } else {
    ///         None
    ///     }
    /// }
    /// ```
    fn post_module(&self, _info: &ModuleInfo) -> Option<String> {
        None
    }

    /// Add content before a `pub mod xxx;` declaration using `TokenStream`
    ///
    /// Same as `pre_module` but returns a `TokenStream` for programmatic generation.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use proc_macro2::TokenStream;
    /// use quote::quote;
    ///
    /// fn pre_module_tokens(&self, info: &ModuleInfo) -> Option<TokenStream> {
    ///     let module_name = info.module_name();
    ///     Some(quote! {
    ///         #[doc = concat!("Module for ", #module_name)]
    ///     })
    /// }
    /// ```
    fn pre_module_tokens(&self, _info: &ModuleInfo) -> Option<proc_macro2::TokenStream> {
        None
    }

    /// Add content after a `pub mod xxx;` declaration using `TokenStream`
    ///
    /// Same as `post_module` but returns a `TokenStream` for programmatic generation.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use proc_macro2::TokenStream;
    /// use quote::quote;
    ///
    /// fn post_module_tokens(&self, info: &ModuleInfo) -> Option<TokenStream> {
    ///     let mod_ident = syn::Ident::new(info.module_name(), proc_macro2::Span::call_site());
    ///     Some(quote! {
    ///         pub use #mod_ident::*;
    ///     })
    /// }
    /// ```
    fn post_module_tokens(&self, _info: &ModuleInfo) -> Option<proc_macro2::TokenStream> {
        None
    }
}

/// The level/type of module being generated
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleLevel {
    /// Package module (e.g., `pub mod std_msgs;`)
    Package,
    /// Interface kind module (e.g., `pub mod msg;`, `pub mod srv;`, `pub mod action;`)
    InterfaceKind(super::InterfaceKind),
    /// Type module (e.g., `pub mod header;`) with its parent interface kind
    Type(super::InterfaceKind),
}

/// Information about a module being generated in mod.rs files
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// The module name (e.g., "header", "msg", "`std_msgs`")
    module_name: String,
    /// The full parent path (e.g., "`std_msgs::msg`" for type modules, "`std_msgs`" for interface modules)
    parent_path: String,
    /// The package name
    package: String,
    /// The module level/type
    module_level: ModuleLevel,
}

impl ModuleInfo {
    /// Create a new `ModuleInfo`
    #[must_use]
    pub(crate) fn new(
        module_name: String,
        parent_path: String,
        package: String,
        module_level: ModuleLevel,
    ) -> Self {
        Self {
            module_name,
            parent_path,
            package,
            module_level,
        }
    }

    /// Get the module name
    #[must_use]
    pub fn module_name(&self) -> &str {
        &self.module_name
    }

    /// Get the full parent path
    #[must_use]
    pub fn parent_path(&self) -> &str {
        &self.parent_path
    }

    /// Get the package name
    #[must_use]
    pub fn package(&self) -> &str {
        &self.package
    }

    /// Get the module level
    #[must_use]
    pub fn module_level(&self) -> ModuleLevel {
        self.module_level
    }

    /// Get the full module path (`parent_path::module_name` or just `module_name` if at root)
    #[must_use]
    pub fn full_path(&self) -> String {
        if self.parent_path.is_empty() {
            self.module_name.clone()
        } else {
            format!("{}::{}", self.parent_path, self.module_name)
        }
    }
}

/// Information about a generated item
#[derive(Debug, Clone)]
pub struct ItemInfo {
    /// The item name (struct/enum name)
    name: String,
    /// The source file path
    source_file: String,
    /// The package name
    package: String,
    /// The interface kind (msg, srv, action)
    interface_kind: super::InterfaceKind,
}

impl ItemInfo {
    /// Create a new `ItemInfo`
    #[must_use]
    pub(super) fn new(
        name: String,
        source_file: String,
        package: String,
        interface_kind: super::InterfaceKind,
    ) -> Self {
        Self {
            name,
            source_file,
            package,
            interface_kind,
        }
    }

    /// Get the item name (struct/enum name)
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the source file path
    #[must_use]
    pub fn source_file(&self) -> &str {
        &self.source_file
    }

    /// Get the package name
    #[must_use]
    pub fn package(&self) -> &str {
        &self.package
    }

    /// Get the interface kind (msg, srv, action)
    #[must_use]
    pub fn interface_kind(&self) -> super::InterfaceKind {
        self.interface_kind
    }
}

/// Information about a field in a generated struct
#[derive(Debug, Clone)]
pub struct FieldInfo {
    /// The field name
    field_name: String,
    /// The field type as a string (Rust type)
    field_type: String,
    /// The parent struct/type name
    parent_name: String,
    /// The package name
    package: String,
    /// The original ROS type name (e.g., "byte", "uint8", "string")
    ros_type_name: String,
    /// The array size if this is a fixed-size array (None for unbounded sequences)
    array_size: Option<u32>,
    /// ROS2 type override (e.g., "byte", "char", "wstring")
    ros2_type_override: Option<String>,
    /// Capacity for bounded strings/sequences
    capacity: Option<u32>,
    /// Default value for the field
    default_value: Option<String>,
}

impl FieldInfo {
    /// Create a new `FieldInfo`
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        field_name: String,
        field_type: String,
        parent_name: String,
        package: String,
        ros_type_name: String,
        array_size: Option<u32>,
        ros2_type_override: Option<String>,
        capacity: Option<u32>,
        default_value: Option<String>,
    ) -> Self {
        Self {
            field_name,
            field_type,
            parent_name,
            package,
            ros_type_name,
            array_size,
            ros2_type_override,
            capacity,
            default_value,
        }
    }

    /// Get the field name
    #[must_use]
    pub fn field_name(&self) -> &str {
        &self.field_name
    }

    /// Get the field type as a string (Rust type)
    #[must_use]
    pub fn field_type(&self) -> &str {
        &self.field_type
    }

    /// Get the parent struct/type name
    #[must_use]
    pub fn parent_name(&self) -> &str {
        &self.parent_name
    }

    /// Get the package name
    #[must_use]
    pub fn package(&self) -> &str {
        &self.package
    }

    /// Get the original ROS type name
    #[must_use]
    pub fn ros_type_name(&self) -> &str {
        &self.ros_type_name
    }

    /// Get the array size if this is a fixed-size array
    #[must_use]
    pub fn array_size(&self) -> Option<u32> {
        self.array_size
    }

    /// Get the ROS2 type override (e.g., "byte", "char", "wstring")
    #[must_use]
    pub fn ros2_type_override(&self) -> Option<&str> {
        self.ros2_type_override.as_deref()
    }

    /// Get the capacity for bounded strings/sequences
    #[must_use]
    pub fn capacity(&self) -> Option<u32> {
        self.capacity
    }

    /// Get the default value for the field
    #[must_use]
    pub fn default_value(&self) -> Option<&str> {
        self.default_value.as_deref()
    }
}
