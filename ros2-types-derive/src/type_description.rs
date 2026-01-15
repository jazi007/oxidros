//! TypeDescription derive macro implementation
//!
//! This is a simplified implementation that delegates type mapping to the
//! `RosFieldType` trait, eliminating complex compile-time type path analysis.

use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use crate::attrs::{Ros2FieldOpts, Ros2TypeOpts, parse_fields};

/// Generate field type expression using the RosFieldType trait.
///
/// For most fields, we simply call `T::ros_field_type()`.
/// For fields with override attributes (byte, char, wstring), we use hardcoded values.
/// For sequences, we handle the combination of sequence + ros2_type properly.
fn generate_field_type_expr(field_type: &syn::Type, field_opts: &Ros2FieldOpts) -> TokenStream {
    // Handle explicit overrides for types where Rust type differs from ROS2 type
    if let Some(ref ros2_type) = field_opts.ros2_type {
        // Get the base type_id for the ros2_type
        let type_id = match ros2_type.as_str() {
            "byte" | "octet" => quote! { ros2_types::FIELD_TYPE_BYTE },
            "char" => quote! { ros2_types::FIELD_TYPE_CHAR },
            "wstring" => quote! { ros2_types::FIELD_TYPE_WSTRING },
            _ => quote! { ros2_types::FIELD_TYPE_UINT8 }, // fallback
        };

        // Check if this is also a sequence
        if field_opts.sequence {
            if let Some(capacity) = field_opts.capacity {
                return quote! { ros2_types::types::FieldType::bounded_sequence(#type_id, #capacity) };
            } else {
                return quote! { ros2_types::types::FieldType::sequence(#type_id) };
            }
        }

        // Not a sequence - return primitive or bounded type
        return match ros2_type.as_str() {
            "byte" | "octet" => {
                quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_BYTE) }
            }
            "char" => {
                quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_CHAR) }
            }
            "wstring" => {
                if let Some(capacity) = field_opts.capacity {
                    quote! { ros2_types::types::FieldType::bounded_wstring(#capacity) }
                } else {
                    quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_WSTRING) }
                }
            }
            _ => quote! { <#field_type as ros2_types::RosFieldType>::ros_field_type() },
        };
    }

    // Handle sequence + string (e.g., Vec<String> with #[ros2(sequence, string)])
    // This covers:
    // - sequence<string> → unbounded sequence of unbounded strings
    // - sequence<string, N> → bounded sequence of unbounded strings (capacity = N)
    // - sequence<string<M>> → unbounded sequence of bounded strings (string_capacity = M)
    // - sequence<string<M>, N> → bounded sequence of bounded strings (capacity = N, string_capacity = M)
    if field_opts.sequence && field_opts.string {
        let string_type_id = if field_opts.string_capacity.is_some() {
            // Bounded string element
            quote! { ros2_types::FIELD_TYPE_BOUNDED_STRING }
        } else {
            // Unbounded string element
            quote! { ros2_types::FIELD_TYPE_STRING }
        };

        // Get string_capacity for the FieldType (0 means unbounded)
        let string_cap_val = field_opts.string_capacity.unwrap_or(0);

        if let Some(seq_capacity) = field_opts.capacity {
            // Bounded sequence
            return quote! {
                ros2_types::types::FieldType::bounded_sequence_with_string_capacity(
                    #string_type_id,
                    #seq_capacity,
                    #string_cap_val
                )
            };
        } else {
            // Unbounded sequence
            return quote! {
                ros2_types::types::FieldType::sequence_with_string_capacity(#string_type_id, #string_cap_val)
            };
        }
    }

    // Handle sequence + wstring
    if field_opts.sequence && field_opts.wstring {
        let wstring_type_id = if field_opts.string_capacity.is_some() {
            quote! { ros2_types::FIELD_TYPE_BOUNDED_WSTRING }
        } else {
            quote! { ros2_types::FIELD_TYPE_WSTRING }
        };

        let string_cap_val = field_opts.string_capacity.unwrap_or(0);

        if let Some(seq_capacity) = field_opts.capacity {
            return quote! {
                ros2_types::types::FieldType::bounded_sequence_with_string_capacity(
                    #wstring_type_id,
                    #seq_capacity,
                    #string_cap_val
                )
            };
        } else {
            return quote! {
                ros2_types::types::FieldType::sequence_with_string_capacity(#wstring_type_id, #string_cap_val)
            };
        }
    }

    // Handle explicit string attribute with capacity (for bounded strings)
    if field_opts.string
        && let Some(capacity) = field_opts.capacity
    {
        return quote! { ros2_types::types::FieldType::bounded_string(#capacity) };
    }

    // Handle explicit wstring attribute with capacity
    if field_opts.wstring {
        if let Some(capacity) = field_opts.capacity {
            return quote! { ros2_types::types::FieldType::bounded_wstring(#capacity) };
        } else {
            return quote! { ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_WSTRING) };
        }
    }

    // Handle sequence with capacity (bounded sequence of primitives or nested types)
    // This handles cases like Vec<f64> with #[ros2(sequence, capacity = 3)]
    if field_opts.sequence
        && let Some(capacity) = field_opts.capacity
    {
        // For bounded sequences, we need to extract the inner type from Vec<T>
        // and create a bounded sequence with that element type
        if let syn::Type::Path(type_path) = field_type
            && let Some(segment) = type_path.path.segments.last()
            && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
        {
            // Got the inner type T from Vec<T>
            return quote! {
                {
                    let inner = <#inner_ty as ros2_types::RosFieldType>::ros_field_type();
                    if inner.type_id == ros2_types::FIELD_TYPE_NESTED_TYPE {
                        ros2_types::types::FieldType::nested_bounded_sequence(&inner.nested_type_name, #capacity)
                    } else {
                        ros2_types::types::FieldType::bounded_sequence(inner.type_id, #capacity)
                    }
                }
            };
        }
        // Fallback for non-generic sequence types - shouldn't normally happen
        return quote! { <#field_type as ros2_types::RosFieldType>::ros_field_type() };
    }

    // Default: delegate to the RosFieldType trait
    quote! { <#field_type as ros2_types::RosFieldType>::ros_field_type() }
}

/// Generate referenced types expression using the RosFieldType trait.
fn generate_referenced_types_expr(
    field_type: &syn::Type,
    field_opts: &Ros2FieldOpts,
) -> TokenStream {
    // Skip primitives marked with string/wstring attributes
    if field_opts.string || field_opts.wstring {
        return quote! { Vec::new() };
    }

    // Skip fields with ros2_type override (these are primitives with different ROS2 mapping)
    if field_opts.ros2_type.is_some() {
        return quote! { Vec::new() };
    }

    // Delegate to the RosFieldType trait
    quote! { <#field_type as ros2_types::RosFieldType>::referenced_types() }
}

/// Implement the TypeDescription derive macro
pub fn derive_type_description_impl(input: DeriveInput) -> Result<TokenStream, syn::Error> {
    // Parse container attributes using darling
    let opts = Ros2TypeOpts::from_derive_input(&input)
        .map_err(|e| syn::Error::new_spanned(&input, e.to_string()))?;

    let name = &opts.ident;
    let package = &opts.package;
    let interface_type = &opts.interface_type;
    let generics = &opts.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Construct type name
    let type_name = format!("{}/{}/{}", package, interface_type, name);

    // Parse field attributes
    let field_opts = parse_fields(&input)?;

    // Generate field descriptions
    let field_conversions: Vec<_> = field_opts
        .iter()
        .map(|f| {
            let field_name_raw = f.ident.as_ref().unwrap().to_string();
            // Strip r# prefix for raw identifiers (e.g., r#type -> type)
            let field_name = field_name_raw.strip_prefix("r#").unwrap_or(&field_name_raw);
            let field_type = &f.ty;

            // Generate field type expression using trait delegation
            let field_type_expr = generate_field_type_expr(field_type, f);

            // Generate Field::new() or Field::with_default() based on whether we have a default value
            if let Some(ref default_value) = f.default {
                quote! {
                    ros2_types::types::Field::with_default(
                        #field_name,
                        #field_type_expr,
                        #default_value
                    )
                }
            } else {
                quote! {
                    ros2_types::types::Field::new(
                        #field_name,
                        #field_type_expr
                    )
                }
            }
        })
        .collect();

    // Handle empty structs - add hidden field for C++ compatibility
    let fields_vec = if field_opts.is_empty() {
        quote! {
            vec![
                ros2_types::types::Field::new(
                    "structure_needs_at_least_one_member",
                    ros2_types::types::FieldType::primitive(ros2_types::FIELD_TYPE_UINT8)
                )
            ]
        }
    } else {
        quote! {
            vec![
                #(#field_conversions),*
            ]
        }
    };

    // Collect referenced types using trait delegation
    let referenced_types: Vec<_> = field_opts
        .iter()
        .map(|f| {
            let field_type = &f.ty;
            generate_referenced_types_expr(field_type, f)
        })
        .collect();

    let expanded = quote! {
        impl #impl_generics ros2_types::TypeDescription for #name #ty_generics #where_clause {
            fn type_description() -> ros2_types::types::TypeDescriptionMsg {
                let type_desc = ros2_types::types::IndividualTypeDescription::new(
                    #type_name,
                    #fields_vec
                );

                // Collect referenced types and deduplicate by type_name
                let nested_collections: Vec<Vec<ros2_types::types::IndividualTypeDescription>> = vec![#(#referenced_types),*];
                let all_refs: Vec<ros2_types::types::IndividualTypeDescription> = nested_collections.into_iter().flatten().collect();

                let mut seen = std::collections::HashSet::new();
                let mut unique_refs = Vec::new();

                for ref_desc in all_refs {
                    if seen.insert(ref_desc.type_name.clone()) {
                        unique_refs.push(ref_desc);
                    }
                }

                ros2_types::types::TypeDescriptionMsg::new(type_desc, unique_refs)
            }

            fn message_type_name() -> ros2_types::MessageTypeName {
                ros2_types::MessageTypeName::new(
                    #interface_type,
                    #package,
                    stringify!(#name)
                )
            }
        }

        impl #impl_generics ros2_types::RosFieldType for #name #ty_generics #where_clause {
            fn ros_field_type() -> ros2_types::types::FieldType {
                ros2_types::types::FieldType::nested(#type_name)
            }

            fn referenced_types() -> Vec<ros2_types::types::IndividualTypeDescription> {
                let desc = <Self as ros2_types::TypeDescription>::type_description();
                let mut types = vec![desc.type_description];
                types.extend(desc.referenced_type_descriptions);
                types
            }
        }
    };

    Ok(expanded)
}
