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
        // Skip primitive types marked with string/wstring attributes
        if field_opt.string || field_opt.wstring {
            continue;
        }

        // Attempt to extract nested type.
        // For sequences, we need special handling since wrapper types like GoalStatusSeq<0>
        // don't have type generics we can extract.
        let nested_ty = if field_opt.sequence {
            // Try to extract generic inner type for path types like Vec<T> or Sequence<T>
            match &field_opt.ty {
                Type::Path(type_path) => {
                    if let Some(segment) = type_path.path.segments.last() {
                        use syn::PathArguments;
                        if let PathArguments::AngleBracketed(args) = &segment.arguments
                            && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
                        {
                            // Successfully extracted inner type (e.g., from Vec<GoalStatus>)
                            extract_nested_type(inner_ty)
                        } else {
                            // No type generic found - this is likely a wrapper like GoalStatusSeq<0>
                            // with a const generic. We can't directly get the inner type.
                            // Skip this field - the base type should already be collected elsewhere.
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            }
        } else {
            extract_nested_type(&field_opt.ty)
        };

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
    field_opts: &crate::attrs::Ros2FieldOpts,
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
            let elem_field_type = map_rust_type_to_field_type(elem_ty, field_opts);

            return quote! {
                {
                    let inner = #elem_field_type;
                    ros2_types::types::FieldType::array(inner.type_id, #len as u64)
                }
            };
        }
    }

    // Handle explicit string/wstring attributes
    // This allows custom string types (like RosString<N>) to be properly mapped
    if field_opts.string {
        if let Some(cap) = field_opts.capacity {
            return quote! { ros2_types::types::FieldType::bounded_string(#cap) };
        } else {
            return quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_STRING) };
        }
    }

    if field_opts.wstring {
        if let Some(cap) = field_opts.capacity {
            return quote! { ros2_types::types::FieldType::bounded_wstring(#cap) };
        } else {
            return quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_WSTRING) };
        }
    }

    // If the field is explicitly marked as a sequence, try to extract
    // a generic inner type (works for Vec<T>, Sequence<T>, or other
    // wrapper types with angle-bracketed generic args). This makes the
    // derive attribute-driven instead of relying on the outer type
    // being literally `Vec`.
    if field_opts.sequence {
        if let Type::Path(type_path) = ty {
            if let Some(PathSegment {
                ident,
                arguments: PathArguments::AngleBracketed(args),
            }) = type_path.path.segments.last()
            {
                // First, try to extract a type generic (e.g., Vec<GoalStatus>)
                if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                    // Prevent outer `sequence` attr from influencing inner mapping
                    let mut inner_opts = field_opts.clone();
                    inner_opts.sequence = false;
                    let inner_field_type = map_rust_type_to_field_type(inner_ty, &inner_opts);
                    if let Some(cap) = field_opts.capacity {
                        return quote! {
                            {
                                let inner = #inner_field_type;
                                if inner.type_id == ros2_types::FIELD_TYPE_NESTED_TYPE {
                                    ros2_types::types::FieldType::nested_bounded_sequence(&inner.nested_type_name, #cap)
                                } else {
                                    ros2_types::types::FieldType::bounded_sequence(inner.type_id, #cap)
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

                // If no type generic, check if this is a wrapper type like GoalStatusSeq<0>
                // The const generic is the capacity, and we need to infer the element type
                let type_name = ident.to_string();
                if type_name.ends_with("Seq") {
                    let base_type_name = &type_name[..type_name.len() - 3]; // Strip "Seq"

                    // Check if this is a primitive sequence type
                    let primitive_type_id = match base_type_name {
                        "Bool" => Some(quote! { ros2_types::FIELD_TYPE_BOOLEAN }),
                        "I8" => Some(quote! { ros2_types::FIELD_TYPE_INT8 }),
                        "U8" => Some(quote! { ros2_types::FIELD_TYPE_UINT8 }),
                        "I16" => Some(quote! { ros2_types::FIELD_TYPE_INT16 }),
                        "U16" => Some(quote! { ros2_types::FIELD_TYPE_UINT16 }),
                        "I32" => Some(quote! { ros2_types::FIELD_TYPE_INT32 }),
                        "U32" => Some(quote! { ros2_types::FIELD_TYPE_UINT32 }),
                        "I64" => Some(quote! { ros2_types::FIELD_TYPE_INT64 }),
                        "U64" => Some(quote! { ros2_types::FIELD_TYPE_UINT64 }),
                        "F32" => Some(quote! { ros2_types::FIELD_TYPE_FLOAT }),
                        "F64" => Some(quote! { ros2_types::FIELD_TYPE_DOUBLE }),
                        "RosString" => Some(quote! { ros2_types::FIELD_TYPE_STRING }),
                        "RosWString" => Some(quote! { ros2_types::FIELD_TYPE_WSTRING }),
                        _ => None,
                    };

                    if let Some(type_id) = primitive_type_id {
                        // This is a primitive sequence type
                        if let Some(cap) = field_opts.capacity {
                            return quote! { ros2_types::types::FieldType::bounded_sequence(#type_id, #cap) };
                        } else {
                            return quote! { ros2_types::types::FieldType::sequence(#type_id) };
                        }
                    }

                    // This is a nested sequence wrapper type - the element type is inferred
                    // by stripping "Seq" from the type name and looking up that type
                    // We build the element type path by modifying the current path
                    let mut element_path = type_path.path.clone();
                    if let Some(last_seg) = element_path.segments.last_mut() {
                        last_seg.ident = syn::Ident::new(base_type_name, last_seg.ident.span());
                        last_seg.arguments = PathArguments::None;
                    }
                    let element_ty: Type = Type::Path(syn::TypePath {
                        qself: type_path.qself.clone(),
                        path: element_path,
                    });

                    if let Some(cap) = field_opts.capacity {
                        return quote! {
                            {
                                let desc = <#element_ty as ros2_types::TypeDescription>::type_description();
                                ros2_types::types::FieldType::nested_bounded_sequence(&desc.type_description.type_name, #cap)
                            }
                        };
                    } else {
                        return quote! {
                            {
                                let desc = <#element_ty as ros2_types::TypeDescription>::type_description();
                                ros2_types::types::FieldType::nested_sequence(&desc.type_description.type_name)
                            }
                        };
                    }
                }
            }
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
            // Determine if this should be treated as wstring via explicit attribute or ros2_type
            // Only treat as wstring when explicitly overridden via ros2_type="wstring".
            // The `string` attribute is not a marker for wide strings.
            let is_wstring = matches!(field_opts.ros2_type.as_deref(), Some("wstring"));
            if is_wstring {
                if let Some(capacity) = field_opts.capacity {
                    return quote! { ros2_types::types::FieldType::bounded_wstring(#capacity) };
                } else {
                    return quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_WSTRING) };
                }
            } else if let Some(capacity) = field_opts.capacity {
                return quote! { ros2_types::types::FieldType::bounded_string(#capacity) };
            } else {
                return quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_STRING) };
            }
        }
    }

    match type_name.as_str() {
        "i8" => {
            if matches!(field_opts.ros2_type.as_deref(), Some("char")) {
                quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_CHAR) }
            } else {
                quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_INT8) }
            }
        }
        "u8" => {
            if matches!(field_opts.ros2_type.as_deref(), Some("byte")) {
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
            // Fallback for types declared as `String` in the AST but not matching the std:: path check above.
            let is_wstring = matches!(field_opts.ros2_type.as_deref(), Some("wstring"));
            if is_wstring {
                if let Some(capacity) = field_opts.capacity {
                    quote! { ros2_types::types::FieldType::bounded_wstring(#capacity) }
                } else {
                    quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_WSTRING) }
                }
            } else if let Some(capacity) = field_opts.capacity {
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
        // Vec<T> without explicit #[ros2(sequence)] is an error or legacy fallback.
        // With proper generator output, all sequences should have the attribute set.
        // We keep a minimal fallback for backward compatibility but it should rarely be hit.
        "Vec" => {
            // Handle Vec<T> - unbounded sequence (fallback for legacy code)
            if let Type::Path(type_path) = ty
                && let Some(PathSegment {
                    arguments: PathArguments::AngleBracketed(args),
                    ..
                }) = type_path.path.segments.last()
                && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
            {
                let mut inner_opts = field_opts.clone();
                inner_opts.sequence = false;
                let inner_field_type = map_rust_type_to_field_type(inner_ty, &inner_opts);

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
