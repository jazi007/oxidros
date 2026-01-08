//! Shared attribute parsing for ROS2 derive macros

use darling::{FromDeriveInput, FromField};

/// Container-level attributes for ROS2 types
///
/// Used by both `TypeDescription` and `Ros2Msg` derives.
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(ros2), supports(struct_named))]
pub struct Ros2TypeOpts {
    pub ident: syn::Ident,
    pub generics: syn::Generics,

    /// ROS2 package name (e.g., "std_msgs", "geometry_msgs")
    #[darling(default = "default_package")]
    pub package: String,

    /// Interface type: "msg", "srv", or "action"
    #[darling(default = "default_interface_type")]
    pub interface_type: String,

    /// Path prefix for unique_identifier_msgs package (used in actions)
    ///
    /// For example:
    /// - `"super::super::super"` for flat generated structure (default)
    /// - `"crate::ros2msg"` for oxidros-msg style
    /// - `"crate"` for when unique_identifier_msgs is at crate root
    #[darling(default)]
    pub uuid_path: Option<String>,

    /// Skip generating service/action wrappers
    ///
    /// When true, the derive will not generate the wrapper struct
    /// (e.g., `AddTwoInts` for services or `Fibonacci` for actions).
    /// Useful when:
    /// - You want to generate the wrapper manually via `ros2_service!` or `ros2_action!`
    /// - Testing individual message types without full service/action infrastructure
    #[darling(default)]
    pub skip_wrapper: bool,
}

fn default_package() -> String {
    "default_pkg".to_string()
}

fn default_interface_type() -> String {
    "msg".to_string()
}

/// Field-level attributes for ROS2 type metadata
///
/// Used by both `TypeDescription` and `Ros2Msg` derives.
#[derive(Debug, Clone, FromField)]
#[darling(attributes(ros2))]
pub struct Ros2FieldOpts {
    pub ident: Option<syn::Ident>,
    pub ty: syn::Type,

    /// Override the ROS2 type (e.g., "byte", "char", "wstring")
    #[darling(default)]
    pub ros2_type: Option<String>,

    /// Capacity for bounded strings/sequences
    #[darling(default)]
    pub capacity: Option<u64>,

    /// Explicitly mark this field as a sequence.
    /// Use `#[ros2(sequence)]` on the field (flag-style).
    #[darling(default)]
    pub sequence: bool,

    /// Explicitly mark this field as a string type.
    /// Use `#[ros2(string)]` on the field (flag-style).
    #[darling(default)]
    pub string: bool,

    /// Explicitly mark this field as a wide string type.
    /// Use `#[ros2(wstring)]` on the field (flag-style).
    #[darling(default)]
    pub wstring: bool,

    /// Default value for the field
    #[darling(default)]
    pub default: Option<String>,
}

/// Parse fields from a struct's data
pub fn parse_fields(input: &syn::DeriveInput) -> Result<Vec<Ros2FieldOpts>, syn::Error> {
    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "ROS2 derives can only be used on structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "ROS2 derives can only be used on structs",
            ));
        }
    };

    fields
        .iter()
        .map(|f| {
            Ros2FieldOpts::from_field(f).map_err(|e| {
                syn::Error::new_spanned(f, format!("Failed to parse field attributes: {}", e))
            })
        })
        .collect()
}
