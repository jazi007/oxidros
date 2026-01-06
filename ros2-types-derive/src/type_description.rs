//! TypeDescription derive macro implementation

use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use crate::attrs::{Ros2TypeOpts, parse_fields};
use crate::type_mapping::{collect_referenced_types, map_rust_type_to_field_type};

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

            // Map Rust type to ROS field type using explicit field attributes
            let field_type_expr = map_rust_type_to_field_type(field_type, f);

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

    // Collect referenced types by checking for nested types
    let referenced_types = collect_referenced_types(&field_opts);

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
    };

    Ok(expanded)
}
