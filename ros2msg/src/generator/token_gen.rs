//! TokenStream-based code generation utilities
//!
//! This module provides helper functions for generating Rust code using
//! proc-macro2 `TokenStreams` and the quote macro. The generated code is
//! formatted using prettyplease for consistent output.

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::str::FromStr;

/// Parse a type string into a `TokenStream`
///
/// Handles complex types like:
/// - `u32`, `i64`, `bool`
/// - `Vec<u8>`
/// - `[f64; 9]`
/// - `::std::string::String`
/// - `libc::c_char`
/// - `super::super::pkg::msg::Type`
pub(super) fn parse_type(type_str: &str) -> TokenStream {
    // Remove comments from type strings (e.g., "Vec<u8> /* max_size: 10 */")
    // Handle multiple comments and comments embedded in array types
    let type_str = remove_comments(type_str);
    let type_str = type_str.trim();

    // Try to parse directly - this handles most cases
    if let Ok(tokens) = TokenStream::from_str(type_str) {
        return tokens;
    }

    // Fallback: create as identifier (shouldn't happen with valid types)
    let ident = Ident::new(type_str, Span::call_site());
    quote! { #ident }
}

/// Remove all /* */ comments from a string, preserving the rest
fn remove_comments(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '/' && chars.peek() == Some(&'*') {
            // Skip the '*' and everything until '*/'
            chars.next(); // consume '*'
            while let Some(c2) = chars.next() {
                if c2 == '*' && chars.peek() == Some(&'/') {
                    chars.next(); // consume '/'
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Parse a derive name into an identifier
pub(super) fn parse_derive(derive_str: &str) -> TokenStream {
    // Handle derives with paths like "serde::Serialize"
    if let Ok(tokens) = TokenStream::from_str(derive_str) {
        return tokens;
    }

    let ident = Ident::new(derive_str, Span::call_site());
    quote! { #ident }
}

/// Create a field identifier, handling Rust keywords (raw identifiers like r#type)
fn field_ident(name: &str) -> TokenStream {
    // Handle raw identifiers (r#type, r#match, etc.)
    if let Some(raw_name) = name.strip_prefix("r#") {
        let ident = Ident::new_raw(raw_name, Span::call_site());
        return quote! { #ident };
    }

    let ident = Ident::new(name, Span::call_site());
    quote! { #ident }
}

/// Create a struct identifier
fn struct_ident(name: &str) -> Ident {
    Ident::new(name, Span::call_site())
}

/// Create a constant identifier
fn const_ident(name: &str) -> Ident {
    Ident::new(name, Span::call_site())
}

/// Parse an attribute string into tokens
///
/// The input should be a complete attribute like `#[serde(skip)]`
fn parse_attribute(attr_str: &str) -> Option<TokenStream> {
    TokenStream::from_str(attr_str).ok()
}

/// Parse a default value expression
fn parse_default_value(value_str: &str) -> TokenStream {
    if let Ok(tokens) = TokenStream::from_str(value_str) {
        return tokens;
    }

    // Fallback for complex expressions
    let ident = Ident::new("Default", Span::call_site());
    quote! { #ident::default() }
}

/// Generate a struct definition with derives and attributes
pub(super) fn generate_struct(
    name: &str,
    derives: &[String],
    custom_attrs: &[String],
    fields: &[StructField],
) -> TokenStream {
    let struct_name = struct_ident(name);

    // Parse derives
    let derive_tokens: Vec<TokenStream> = derives.iter().map(|d| parse_derive(d)).collect();

    // Parse custom attributes
    let attr_tokens: Vec<TokenStream> = custom_attrs
        .iter()
        .filter_map(|a| parse_attribute(a))
        .collect();

    // Generate field tokens
    let field_tokens: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let field_name = field_ident(&f.name);
            let field_type = parse_type(&f.rust_type);
            let attrs: Vec<TokenStream> = f
                .attributes
                .iter()
                .filter_map(|a| parse_attribute(a))
                .collect();

            quote! {
                #(#attrs)*
                pub #field_name: #field_type,
            }
        })
        .collect();

    let derives_attr = if derive_tokens.is_empty() {
        quote! {}
    } else {
        quote! { #[derive(#(#derive_tokens),*)] }
    };

    quote! {
        #[repr(C)]
        #derives_attr
        #(#attr_tokens)*
        pub struct #struct_name {
            #(#field_tokens)*
        }
    }
}

/// Generate a Default implementation
pub(super) fn generate_default_impl(name: &str, field_defaults: &[FieldDefault]) -> TokenStream {
    let struct_name = struct_ident(name);

    let field_inits: Vec<TokenStream> = field_defaults
        .iter()
        .map(|f| {
            let field_name = field_ident(&f.name);
            let default_value = parse_default_value(&f.default_value);

            quote! {
                #field_name: #default_value,
            }
        })
        .collect();

    quote! {
        impl ::core::default::Default for #struct_name {
            #[inline]
            fn default() -> Self {
                Self {
                    #(#field_inits)*
                }
            }
        }
    }
}

/// Generate constants in an impl block
pub(super) fn generate_constants_impl(struct_name: &str, constants: &[ConstantDef]) -> TokenStream {
    if constants.is_empty() {
        return quote! {};
    }

    let name = struct_ident(struct_name);

    let const_tokens: Vec<TokenStream> = constants
        .iter()
        .map(|c| {
            let const_name = const_ident(&c.name);
            let const_type = parse_type(&c.rust_type);
            let const_value = parse_const_value(&c.value, &c.rust_type);

            quote! {
                pub const #const_name: #const_type = #const_value;
            }
        })
        .collect();

    quote! {
        impl #name {
            #(#const_tokens)*
        }
    }
}

/// Parse a constant value based on its type
fn parse_const_value(value: &str, rust_type: &str) -> TokenStream {
    // String constants
    if rust_type == "&str" {
        // Value should already be quoted like `"hello"`
        if let Ok(tokens) = TokenStream::from_str(value) {
            return tokens;
        }
    }

    // Try to parse directly
    if let Ok(tokens) = TokenStream::from_str(value) {
        return tokens;
    }

    // Fallback - just emit as-is
    let ident = Ident::new(value, Span::call_site());
    quote! { #ident }
}

/// Format a `TokenStream` into a pretty-printed string
pub(super) fn format_tokens(tokens: TokenStream) -> Result<String, syn::Error> {
    let syntax_tree = syn::parse2::<syn::File>(tokens)?;
    Ok(prettyplease::unparse(&syntax_tree))
}

/// Format multiple `TokenStreams` into a single pretty-printed string
pub(super) fn format_token_streams(streams: Vec<TokenStream>) -> Result<String, syn::Error> {
    let combined = streams.into_iter().fold(TokenStream::new(), |mut acc, ts| {
        acc.extend(ts);
        acc
    });
    format_tokens(combined)
}

/// Struct field information for code generation
#[derive(Debug, Clone)]
pub(super) struct StructField {
    /// Field name (already sanitized)
    pub name: String,
    /// Rust type string
    pub rust_type: String,
    /// Field-level attributes
    pub attributes: Vec<String>,
}

#[allow(dead_code)]
impl StructField {
    /// Create a new struct field
    fn new(name: impl Into<String>, rust_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            rust_type: rust_type.into(),
            attributes: Vec::new(),
        }
    }

    /// Add an attribute to this field
    fn with_attribute(mut self, attr: impl Into<String>) -> Self {
        self.attributes.push(attr.into());
        self
    }

    /// Add multiple attributes to this field
    fn with_attributes(mut self, attrs: Vec<String>) -> Self {
        self.attributes.extend(attrs);
        self
    }
}

/// Field default value for Default impl generation
#[derive(Debug, Clone)]
pub(super) struct FieldDefault {
    /// Field name
    pub name: String,
    /// Default value expression as string
    pub default_value: String,
}

impl FieldDefault {
    /// Create a new field default
    pub(super) fn new(name: impl Into<String>, default_value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            default_value: default_value.into(),
        }
    }
}

/// Constant definition for impl block generation
#[derive(Debug, Clone)]
pub(super) struct ConstantDef {
    /// Constant name
    pub name: String,
    /// Rust type string
    pub rust_type: String,
    /// Value expression as string
    pub value: String,
}

impl ConstantDef {
    /// Create a new constant definition
    pub(super) fn new(
        name: impl Into<String>,
        rust_type: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            rust_type: rust_type.into(),
            value: value.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_type() {
        let tokens = parse_type("u32");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_parse_vec_type() {
        let tokens = parse_type("Vec<u8>");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_parse_array_type() {
        let tokens = parse_type("[f64; 9]");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_parse_path_type() {
        let tokens = parse_type("::std::string::String");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_parse_type_with_comment() {
        let tokens = parse_type("Vec<u8> /* max_size: 10 */");
        let s = tokens.to_string();
        assert!(s.contains("Vec"));
        assert!(!s.contains("max_size"));
    }

    #[test]
    fn test_generate_simple_struct() {
        let fields = vec![
            StructField::new("x", "f64"),
            StructField::new("y", "f64"),
            StructField::new("z", "f64"),
        ];

        let tokens = generate_struct(
            "Point",
            &["Debug".to_string(), "Clone".to_string()],
            &[],
            &fields,
        );

        let formatted = format_tokens(tokens).unwrap();
        assert!(formatted.contains("pub struct Point"));
        assert!(formatted.contains("pub x: f64"));
        assert!(formatted.contains("#[derive(Debug, Clone)]"));
    }

    #[test]
    fn test_generate_default_impl() {
        let defaults = vec![FieldDefault::new("x", "0.0"), FieldDefault::new("y", "0.0")];

        let tokens = generate_default_impl("Point", &defaults);
        let formatted = format_tokens(tokens).unwrap();
        assert!(formatted.contains("impl ::core::default::Default for Point"));
        assert!(formatted.contains("x: 0.0"));
    }

    #[test]
    fn test_generate_constants_impl() {
        let constants = vec![
            ConstantDef::new("MAX_VALUE", "u32", "100"),
            ConstantDef::new("NAME", "&str", "\"test\""),
        ];

        let tokens = generate_constants_impl("TestStruct", &constants);
        let formatted = format_tokens(tokens).unwrap();
        assert!(formatted.contains("impl TestStruct"));
        assert!(formatted.contains("pub const MAX_VALUE: u32 = 100"));
    }
}
