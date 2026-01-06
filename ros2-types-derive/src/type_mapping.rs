//! Type mapping utilities for ROS2 derive macros

use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

use crate::attrs::Ros2FieldOpts;

/// Extract the nested type from a field type
/// For Vec<T>, returns T. For T, returns T if it's a nested type.
pub fn extract_nested_type(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                let type_name = segment.ident.to_string();
                // For Vec<T>, extract T
                if type_name == "Vec" {
                    use syn::PathArguments;
                    if let PathArguments::AngleBracketed(args) = &segment.arguments
                        && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
                    {
                        // Recursively extract in case of Vec<Vec<T>>
                        return extract_nested_type(inner_ty);
                    }
                    return None;
                }

                // For regular types, check if it's nested
                if is_nested_type(ty) {
                    return Some(ty);
                }
            }
            None
        }
        _ => None,
    }
}

/// Check if a type is a nested custom type (not a primitive)
/// For Vec<T>, checks if T is a nested type
pub fn is_nested_type(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) => {
            // Check if it's a std:: type (these are never nested ROS types)
            if let Some(first_segment) = type_path.path.segments.first()
                && first_segment.ident == "std"
            {
                return false;
            }

            if let Some(segment) = type_path.path.segments.last() {
                let type_name = segment.ident.to_string();

                // Special handling for Vec<T> - check if T is nested
                if type_name == "Vec" {
                    use syn::PathArguments;
                    if let PathArguments::AngleBracketed(args) = &segment.arguments
                        && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
                    {
                        return is_nested_type(inner_ty);
                    }
                    return false;
                }

                // If it's a complex path (like std_msgs::msg::string::String)
                // it's definitely a nested type, not a primitive
                if type_path.path.segments.len() > 1 {
                    return true;
                }

                // Check if it's NOT a primitive or standard type
                !matches!(
                    type_name.as_str(),
                    "i8" | "u8"
                        | "i16"
                        | "u16"
                        | "i32"
                        | "u32"
                        | "i64"
                        | "u64"
                        | "f32"
                        | "f64"
                        | "bool"
                        | "Vec"
                        | "String"
                        | "c_char"
                        | "c_schar"
                        | "c_uchar"
                        // ROS FFI types - these are NOT nested ROS message types
                        | "RosString"
                        | "RosWString"
                        | "BoolSeq"
                        | "I8Seq"
                        | "I16Seq"
                        | "I32Seq"
                        | "I64Seq"
                        | "U8Seq"
                        | "U16Seq"
                        | "U32Seq"
                        | "U64Seq"
                        | "F32Seq"
                        | "F64Seq"
                        | "RosStringSeq"
                        | "RosWStringSeq"
                )
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Collect referenced type descriptions from nested types
pub fn collect_referenced_types(field_opts: &[Ros2FieldOpts]) -> Vec<TokenStream> {
    let mut referenced = Vec::new();

    for field_opt in field_opts {
        // Extract the actual nested type, unwrapping Vec if needed
        let nested_ty = extract_nested_type(&field_opt.ty);

        if let Some(ty) = nested_ty {
            // Call type_description() on the nested type to get its FULL description
            // which includes both the type itself and its transitive dependencies
            referenced.push(quote! {
                {
                    let full_desc = <#ty as ros2_types::TypeDescription>::type_description();
                    // Collect both the main type and all its references
                    let mut types = vec![full_desc.type_description];
                    types.extend(full_desc.referenced_type_descriptions);
                    types
                }
            });
        }
    }

    referenced
}

/// Map Rust types to ROS2 FieldType for TypeDescription
pub fn map_rust_type_to_field_type(
    ty: &Type,
    ros2_type: Option<&str>,
    ros2_capacity: Option<u64>,
) -> TokenStream {
    use syn::{PathArguments, PathSegment};

    // Handle fixed-size arrays [T; N]
    if let Type::Array(type_array) = ty {
        let elem_ty = &type_array.elem;
        let len = &type_array.len;

        // Check if element is nested type
        if is_nested_type(elem_ty) {
            // For [NestedType; N], use nested_array
            return quote! {
                {
                    let desc = <#elem_ty as ros2_types::TypeDescription>::type_description();
                    ros2_types::types::FieldType::nested_array(&desc.type_description.type_name, #len as u64)
                }
            };
        } else {
            // For [primitive; N], use array with encoded type_id
            let elem_field_type = map_rust_type_to_field_type(elem_ty, ros2_type, None);

            return quote! {
                {
                    let inner = #elem_field_type;
                    ros2_types::types::FieldType::array(inner.type_id, #len as u64)
                }
            };
        }
    }

    // First, check if this is a DIRECT nested type (multi-segment path like crate::generated::...)
    // NOT Vec<T> - that's handled in the match statement below
    if is_nested_type(ty)
        && let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && segment.ident != "Vec"
    {
        // It's a direct nested type (not Vec)
        return quote! {
            {
                let desc = <#ty as ros2_types::TypeDescription>::type_description();
                ros2_types::types::FieldType::nested(&desc.type_description.type_name)
            }
        };
    }

    // Extract the type name for Path types
    let type_name = match ty {
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                segment.ident.to_string()
            } else {
                return quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_NOT_SET) };
            }
        }
        _ => {
            return quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_NOT_SET) };
        }
    };

    // Special case: check for ::std::string::String explicitly
    if let Type::Path(type_path) = ty {
        let path_str = quote!(#type_path).to_string().replace(' ', "");
        if path_str.contains("std") && path_str.ends_with("String") {
            if matches!(ros2_type, Some("wstring")) {
                if let Some(capacity) = ros2_capacity {
                    return quote! { ros2_types::types::FieldType::bounded_wstring(#capacity) };
                } else {
                    return quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_WSTRING) };
                }
            } else if let Some(capacity) = ros2_capacity {
                return quote! { ros2_types::types::FieldType::bounded_string(#capacity) };
            } else {
                return quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_STRING) };
            }
        }
    }

    match type_name.as_str() {
        "i8" => {
            if matches!(ros2_type, Some("char")) {
                quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_CHAR) }
            } else {
                quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_INT8) }
            }
        }
        "u8" => {
            if matches!(ros2_type, Some("byte")) {
                quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_BYTE) }
            } else {
                quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_UINT8) }
            }
        }
        "i16" => {
            quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_INT16) }
        }
        "u16" => {
            quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_UINT16) }
        }
        "i32" => {
            quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_INT32) }
        }
        "u32" => {
            quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_UINT32) }
        }
        "i64" => {
            quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_INT64) }
        }
        "u64" => {
            quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_UINT64) }
        }
        "f32" => {
            quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_FLOAT) }
        }
        "f64" => {
            quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_DOUBLE) }
        }
        "bool" => {
            quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_BOOLEAN) }
        }
        "String" => {
            if ros2_type == Some("wstring") {
                if let Some(capacity) = ros2_capacity {
                    quote! { ros2_types::types::FieldType::bounded_wstring(#capacity) }
                } else {
                    quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_WSTRING) }
                }
            } else if let Some(capacity) = ros2_capacity {
                quote! { ros2_types::types::FieldType::bounded_string(#capacity) }
            } else {
                quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_STRING) }
            }
        }
        "c_char" => {
            quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_UINT8) }
        }
        "c_schar" => {
            quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_INT8) }
        }
        "c_uchar" => {
            quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_UINT8) }
        }
        "Vec" => {
            // Handle Vec<T> - unbounded sequence
            if let Type::Path(type_path) = ty
                && let Some(PathSegment {
                    arguments: PathArguments::AngleBracketed(args),
                    ..
                }) = type_path.path.segments.last()
                && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
            {
                let inner_field_type = map_rust_type_to_field_type(inner_ty, ros2_type, None);

                if let Some(capacity) = ros2_capacity {
                    return quote! {
                        {
                            let inner = #inner_field_type;
                            if inner.type_id == ros2_types::FIELD_TYPE_NESTED_TYPE {
                                ros2_types::types::FieldType::nested_bounded_sequence(&inner.nested_type_name, #capacity)
                            } else {
                                ros2_types::types::FieldType::bounded_sequence(inner.type_id, #capacity)
                            }
                        }
                    };
                } else {
                    return quote! {
                        {
                            let inner = #inner_field_type;
                            if inner.type_id == ros2_types::FIELD_TYPE_NESTED_TYPE {
                                ros2_types::types::FieldType::nested_sequence(&inner.nested_type_name)
                            } else {
                                ros2_types::types::FieldType::sequence(inner.type_id)
                            }
                        }
                    };
                }
            }
            quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_NOT_SET) }
        }
        _ => {
            // It's a nested type
            quote! {
                {
                    let desc = <#ty as ros2_types::TypeDescription>::type_description();
                    ros2_types::types::FieldType::nested(&desc.type_description.type_name)
                }
            }
        }
    }
}
