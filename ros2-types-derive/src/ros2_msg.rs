//! Ros2Msg derive macro implementation
//!
//! Generates FFI bindings when `rcl` feature is enabled,
//! or pure Rust implementations when disabled.
//!
//! # Interface Types
//!
//! - **msg**: Simple message types
//! - **srv**: Service message types (Request/Response pairs)
//! - **action**: Action message types (Goal/Result/Feedback with wrapper types)
//!
//! # Service Types
//!
//! For services, you define the Request and Response structs separately and then
//! use `#[ros2_service]` attribute macro to define the service wrapper:
//!
//! ```ignore
//! #[derive(Ros2Msg)]
//! #[ros2(package = "example_interfaces", interface_type = "srv")]
//! pub struct AddTwoInts_Request { pub a: i64, pub b: i64 }
//!
//! #[derive(Ros2Msg)]
//! #[ros2(package = "example_interfaces", interface_type = "srv")]
//! pub struct AddTwoInts_Response { pub sum: i64 }
//!
//! // Service wrapper - define manually or via ros2_service! macro
//! ros2_service!(example_interfaces, AddTwoInts);
//! ```
//!
//! # Action Types
//!
//! For actions, define Goal/Result/Feedback structs and use `#[ros2_action]`:
//!
//! ```ignore
//! #[derive(Ros2Msg)]
//! #[ros2(package = "example_interfaces", interface_type = "action")]
//! pub struct Fibonacci_Goal { pub order: i32 }
//!
//! #[derive(Ros2Msg)]
//! #[ros2(package = "example_interfaces", interface_type = "action")]
//! pub struct Fibonacci_Result { pub sequence: Vec<i32> }
//!
//! #[derive(Ros2Msg)]
//! #[ros2(package = "example_interfaces", interface_type = "action")]
//! pub struct Fibonacci_Feedback { pub partial_sequence: Vec<i32> }
//!
//! // Action wrapper - define manually or via ros2_action! macro
//! ros2_action!(example_interfaces, Fibonacci);
//! ```

use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::DeriveInput;

use crate::attrs::{Ros2FieldOpts, Ros2TypeOpts, parse_fields};

/// Implement the Ros2Msg derive macro
pub fn derive_ros2_msg_impl(input: DeriveInput) -> Result<TokenStream, syn::Error> {
    // Parse container attributes using darling
    let opts = Ros2TypeOpts::from_derive_input(&input)
        .map_err(|e| syn::Error::new_spanned(&input, e.to_string()))?;

    // Parse field attributes
    let field_opts = parse_fields(&input)?;

    // Generate implementations
    let rcl_impl = generate_rcl_impl(&opts, &field_opts);
    let pure_impl = generate_pure_impl(&opts, &field_opts);
    let common_impl = generate_common_impl(&opts);

    // Generate service/action wrappers (must be at module level, not inside const _)
    let wrapper_impl = generate_wrapper_impl(&opts);

    let expanded = quote! {
        // Common implementations (always generated)
        #common_impl

        #[cfg(feature = "rcl")]
        const _: () = {
            #rcl_impl
        };

        #[cfg(not(feature = "rcl"))]
        const _: () = {
            #pure_impl
        };
        // Service/Action wrappers (at module level so they're accessible)
        #wrapper_impl
    };

    Ok(expanded)
}

/// Generate common FFI declarations for serialization (rcutils, rmw functions)
/// and the rcl_serialized_message_t struct.
///
/// This is shared between base message types and action wrapper types.
fn generate_serialization_ffi_decls() -> TokenStream {
    quote! {
        fn rcutils_get_zero_initialized_uint8_array() -> rcl_serialized_message_t;
        fn rcutils_uint8_array_init(
            array: *mut rcl_serialized_message_t,
            size: usize,
            allocator: *const std::ffi::c_void,
        ) -> i32;
        fn rcutils_get_default_allocator() -> std::ffi::c_void;
        fn rcutils_uint8_array_fini(array: *mut rcl_serialized_message_t);
        fn rmw_serialize(
            msg: *const std::ffi::c_void,
            type_support: *const std::ffi::c_void,
            serialized_msg: *mut rcl_serialized_message_t,
        ) -> i32;
        fn rmw_deserialize(
            buffer: *const u8,
            buffer_size: usize,
            type_support: *const std::ffi::c_void,
            msg: *mut std::ffi::c_void,
            bytes_read: *mut usize,
        ) -> i32;
    }
}

/// Generate the rcl_serialized_message_t struct definition.
///
/// This is the C struct used for serialization/deserialization.
fn generate_serialized_message_struct() -> TokenStream {
    quote! {
        #[repr(C)]
        struct rcl_serialized_message_t {
            buffer: *mut u8,
            buffer_length: usize,
            buffer_capacity: usize,
            allocator: *const std::ffi::c_void,
        }
    }
}

/// Get the default expression for a type, handling large arrays specially
///
/// Arrays with more than 32 elements don't implement Default in Rust's std library,
/// so we need to use alternative initialization methods.
fn get_default_expr_for_type(ty: &syn::Type) -> TokenStream {
    if let Some((elem_ty, size)) = get_large_array_info(ty) {
        // For large arrays, use array initialization with default element
        return quote! { [<#elem_ty as Default>::default(); #size] };
    }
    // Fall back to Default::default() for all other types
    quote! { Default::default() }
}

/// Check if a type is a large array (> 32 elements) and return (element_type, size)
fn get_large_array_info(ty: &syn::Type) -> Option<(&syn::Type, usize)> {
    if let syn::Type::Array(array) = ty
        && let syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(lit_int),
            ..
        }) = &array.len
        && let Ok(size) = lit_int.base10_parse::<usize>()
        && size > 32
    {
        return Some((&array.elem, size));
    }
    None
}

/// Generate service/action wrapper implementations
fn generate_wrapper_impl(opts: &Ros2TypeOpts) -> TokenStream {
    // Skip wrapper generation if explicitly requested
    if opts.skip_wrapper {
        return TokenStream::new();
    }

    let name = &opts.ident;
    let package = &opts.package;
    let interface_type = &opts.interface_type;
    let name_str = name.to_string();

    // For service _Request types, generate the service wrapper
    if interface_type == "srv"
        && let Some(service_name) = name_str.strip_suffix("_Request")
    {
        return generate_service_wrapper(package, service_name);
    }

    // For action _Goal types, generate the action wrapper
    if interface_type == "action"
        && let Some(action_name) = name_str.strip_suffix("_Goal")
    {
        // Use uuid_path attribute if provided, otherwise default to super::super::super
        // to go from package/action/file.rs up to generated root
        let uuid_path = opts.uuid_path.as_deref().or(Some("super::super::super"));
        return generate_action_wrapper(package, action_name, uuid_path);
    }

    TokenStream::new()
}

/// Generate FFI implementations for rcl feature
fn generate_rcl_impl(opts: &Ros2TypeOpts, _field_opts: &[Ros2FieldOpts]) -> TokenStream {
    let name = &opts.ident;
    let package = &opts.package;
    let interface_type = &opts.interface_type;

    // Generate the base struct implementation (common for msg/srv/action)
    generate_rcl_base_impl(name, package, interface_type)
}

/// Generate common implementations that are always needed (regardless of rcl feature)
fn generate_common_impl(opts: &Ros2TypeOpts) -> TokenStream {
    let name = &opts.ident;

    // Sequence types
    let seq_raw_type = format_ident!("{}SeqRaw", name);
    let seq_type = format_ident!("{}Seq", name);

    quote! {
        // Sequence raw type alias
        type #seq_raw_type = ros2_types::SequenceRaw<#name>;

        /// Sequence of messages.
        /// `N` is the maximum number of elements.
        /// If `N` is `0`, the size is unlimited.
        #[repr(transparent)]
        #[derive(Debug)]
        pub struct #seq_type<const N: usize>(pub #seq_raw_type);

        impl<const N: usize> std::ops::Deref for #seq_type<N> {
            type Target = #seq_raw_type;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl<const N: usize> std::ops::DerefMut for #seq_type<N> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl<const N: usize> #seq_type<N> {
            /// Create a null/empty sequence
            pub const fn null() -> Self {
                Self(#seq_raw_type::null())
            }
        }

        unsafe impl<const N: usize> Send for #seq_type<N> {}
        unsafe impl<const N: usize> Sync for #seq_type<N> {}

        /// TypeDescription for sequence types delegates to the element type.
        /// This enables proper type hash computation when XxxSeq<N> is used in fields.
        impl<const N: usize> ros2_types::TypeDescription for #seq_type<N> {
            fn type_description() -> ros2_types::types::TypeDescriptionMsg {
                <#name as ros2_types::TypeDescription>::type_description()
            }

            fn message_type_name() -> ros2_types::MessageTypeName {
                <#name as ros2_types::TypeDescription>::message_type_name()
            }
        }
    }
}

/// Generate base FFI implementations for any ROS2 struct type
fn generate_rcl_base_impl(name: &syn::Ident, package: &str, interface_type: &str) -> TokenStream {
    // Create identifiers for FFI functions
    // Format: {package}__{interface_type}__{TypeName}__*
    let ffi_prefix = format!("{}__{}__{}", package, interface_type, name);

    let init_fn = format_ident!("{}__init", ffi_prefix);
    let fini_fn = format_ident!("{}__fini", ffi_prefix);
    let are_equal_fn = format_ident!("{}__are_equal", ffi_prefix);
    let copy_fn = format_ident!("{}__copy", ffi_prefix);
    let seq_init_fn = format_ident!("{}__Sequence__init", ffi_prefix);
    let seq_fini_fn = format_ident!("{}__Sequence__fini", ffi_prefix);
    let seq_are_equal_fn = format_ident!("{}__Sequence__are_equal", ffi_prefix);
    let seq_copy_fn = format_ident!("{}__Sequence__copy", ffi_prefix);
    let type_support_fn = format_ident!(
        "rosidl_typesupport_c__get_message_type_support_handle__{}__{}__{}",
        package,
        interface_type,
        name
    );

    // Sequence types
    let seq_raw_type = format_ident!("{}SeqRaw", name);
    let seq_type = format_ident!("{}Seq", name);
    let ts_impl = generate_rcl_type_support_impl(name, package, interface_type);

    let serialization_ffi = generate_serialization_ffi_decls();
    let serialized_msg_struct = generate_serialized_message_struct();

    quote! {
        // FFI function declarations
        unsafe extern "C" {
            fn #init_fn(msg: *mut #name) -> bool;
            fn #fini_fn(msg: *mut #name);
            fn #are_equal_fn(lhs: *const #name, rhs: *const #name) -> bool;
            fn #copy_fn(lhs: *const #name, rhs: *mut #name) -> bool;
            fn #seq_init_fn(msg: *mut #seq_raw_type, size: usize) -> bool;
            fn #seq_fini_fn(msg: *mut #seq_raw_type);
            fn #seq_are_equal_fn(lhs: *const #seq_raw_type, rhs: *const #seq_raw_type) -> bool;
            fn #seq_copy_fn(lhs: *const #seq_raw_type, rhs: *mut #seq_raw_type) -> bool;
            fn #type_support_fn() -> *const std::ffi::c_void;
            #serialization_ffi
        }

        #serialized_msg_struct

        #ts_impl
        // Constructor using FFI init
        impl #name {
            /// Create a new instance initialized by the ROS2 C library
            pub fn new() -> Option<Self> {
                let mut msg = unsafe { std::mem::zeroed() };
                if unsafe { #init_fn(&mut msg) } {
                    Some(msg)
                } else {
                    None
                }
            }
        }

        // Drop implementation using FFI fini
        impl Drop for #name {
            fn drop(&mut self) {
                unsafe { #fini_fn(self) };
            }
        }

        // PartialEq using FFI are_equal
        impl PartialEq for #name {
            fn eq(&self, other: &Self) -> bool {
                unsafe { #are_equal_fn(self, other) }
            }
        }

        // TryClone using FFI copy
        impl ros2_types::TryClone for #name {
            fn try_clone(&self) -> Option<Self> {
                let mut result = Self::new()?;
                if unsafe { #copy_fn(self, &mut result) } {
                    Some(result)
                } else {
                    None
                }
            }
        }

        // Default implementation using FFI init (panics on failure)
        impl Default for #name {
            fn default() -> Self {
                Self::new().expect("Failed to initialize ROS2 message")
            }
        }

        // Clone implementation using FFI copy (panics on failure)
        impl Clone for #name {
            fn clone(&self) -> Self {
                ros2_types::TryClone::try_clone(self).expect("Failed to clone ROS2 message")
            }
        }

        // Sequence FFI implementations
        impl<const N: usize> #seq_type<N> {
            /// Create a sequence with the given size.
            /// `N` represents the maximum number of elements.
            /// If `N` is `0`, the sequence is unlimited.
            pub fn new(size: usize) -> Option<Self> {
                if N != 0 && size > N {
                    return None;
                }
                let mut msg = #seq_raw_type::null();
                if unsafe { #seq_init_fn(&mut msg, size) } {
                    Some(Self(msg))
                } else {
                    None
                }
            }
        }

        impl<const N: usize> Drop for #seq_type<N> {
            fn drop(&mut self) {
                unsafe { #seq_fini_fn(std::ops::DerefMut::deref_mut(self)) };
            }
        }

        impl<const N: usize> PartialEq for #seq_type<N> {
            fn eq(&self, other: &Self) -> bool {
                unsafe {
                    let msg1 = #seq_raw_type { data: self.data, size: self.size, capacity: self.capacity };
                    let msg2 = #seq_raw_type { data: other.data, size: other.size, capacity: other.capacity };
                    #seq_are_equal_fn(&msg1, &msg2)
                }
            }
        }

        impl<const N: usize> ros2_types::TryClone for #seq_type<N> {
            fn try_clone(&self) -> Option<Self> {
                let mut result = Self::new(self.size)?;
                let msg1 = #seq_raw_type { data: self.data, size: self.size, capacity: self.capacity };
                let mut msg2 = #seq_raw_type { data: result.data, size: result.size, capacity: result.capacity };
                if unsafe { #seq_copy_fn(&msg1, &mut msg2) } {
                    result.0 = msg2;
                    Some(result)
                } else {
                    None
                }
            }
        }

        // Default implementation for sequence (creates empty sequence)
        impl<const N: usize> Default for #seq_type<N> {
            fn default() -> Self {
                Self::null()
            }
        }

        // Clone implementation for sequence using FFI copy (panics on failure)
        impl<const N: usize> Clone for #seq_type<N> {
            fn clone(&self) -> Self {
                ros2_types::TryClone::try_clone(self).expect("Failed to clone ROS2 sequence")
            }
        }
    }
}

/// Generate pure Rust implementations (no FFI)
fn generate_pure_impl(opts: &Ros2TypeOpts, field_opts: &[Ros2FieldOpts]) -> TokenStream {
    let name = &opts.ident;

    // Generate Default implementation
    let default_fields: Vec<_> = field_opts
        .iter()
        .map(|f| {
            let field_name = f.ident.as_ref().unwrap();
            if let Some(ref default_val) = f.default {
                // Parse the default value - handle type coercion for floats
                let ty = &f.ty;
                let is_float = if let syn::Type::Path(tp) = ty {
                    tp.path
                        .segments
                        .last()
                        .map(|s| s.ident == "f32" || s.ident == "f64")
                        .unwrap_or(false)
                } else {
                    false
                };

                // For floats, ensure the literal has a decimal point
                let coerced_val = if is_float && !default_val.contains('.') {
                    format!("{}.0", default_val)
                } else {
                    default_val.clone()
                };

                let default_expr: TokenStream = coerced_val.parse().unwrap_or_else(|_| {
                    quote! { Default::default() }
                });
                quote! { #field_name: #default_expr }
            } else {
                // Check if the type is a large array (size > 32) which doesn't impl Default
                let default_expr = get_default_expr_for_type(&f.ty);
                quote! { #field_name: #default_expr }
            }
        })
        .collect();

    // Generate Clone field copies
    let clone_fields: Vec<_> = field_opts
        .iter()
        .map(|f| {
            let field_name = f.ident.as_ref().unwrap();
            quote! { #field_name: self.#field_name.clone() }
        })
        .collect();

    // Generate PartialEq comparisons
    let eq_comparisons: Vec<_> = field_opts
        .iter()
        .map(|f| {
            let field_name = f.ident.as_ref().unwrap();
            quote! { self.#field_name == other.#field_name }
        })
        .collect();

    let eq_body = if eq_comparisons.is_empty() {
        quote! { true }
    } else {
        quote! { #(#eq_comparisons)&&* }
    };

    // Sequence types
    let seq_raw_type = format_ident!("{}SeqRaw", name);
    let seq_type = format_ident!("{}Seq", name);
    let ts_impl = generate_native_type_support_impl(name, &opts.package, &opts.interface_type);

    quote! {
        // Constructor - always succeeds for pure Rust types
        impl #name {
            /// Create a new instance with default values
            pub fn new() -> Option<Self> {
                Some(Self::default())
            }
        }

        #ts_impl

        impl Default for #name {
            fn default() -> Self {
                Self {
                    #(#default_fields),*
                }
            }
        }

        impl Clone for #name {
            fn clone(&self) -> Self {
                Self {
                    #(#clone_fields),*
                }
            }
        }

        // TryClone - always succeeds for pure Rust types
        impl ros2_types::TryClone for #name {
            fn try_clone(&self) -> Option<Self> {
                Some(self.clone())
            }
        }

        impl PartialEq for #name {
            fn eq(&self, other: &Self) -> bool {
                #eq_body
            }
        }

        // Sequence implementations for pure Rust
        impl<const N: usize> #seq_type<N> {
            /// Create a sequence with the given size, initialized with default values.
            /// `N` represents the maximum number of elements.
            /// If `N` is `0`, the sequence is unlimited.
            pub fn new(size: usize) -> Option<Self>
            where
                #name: Default,
            {
                if N != 0 && size > N {
                    return None;
                }
                let vec: Vec<#name> = (0..size).map(|_| #name::default()).collect();
                Some(Self(#seq_raw_type::from_vec(vec)))
            }

            /// Create a sequence from a Vec (takes ownership)
            pub fn from_vec(vec: Vec<#name>) -> Option<Self> {
                if N != 0 && vec.len() > N {
                    return None;
                }
                Some(Self(#seq_raw_type::from_vec(vec)))
            }

            /// Convert the sequence to a Vec (takes ownership)
            ///
            /// # Safety
            /// Only call this on sequences created with `new()` or `from_vec()`.
            pub unsafe fn into_vec(self) -> Vec<#name> {
                let inner = std::ptr::read(&self.0);
                std::mem::forget(self); // Don't run our Drop
                inner.into_vec()
            }
        }

        // Default implementation for sequence (creates null/empty sequence)
        impl<const N: usize> Default for #seq_type<N> {
            fn default() -> Self {
                Self::null()
            }
        }

        // Clone implementation for sequence
        impl<const N: usize> Clone for #seq_type<N>
        where
            #name: Clone,
        {
            fn clone(&self) -> Self {
                Self(self.0.clone())
            }
        }

        // TryClone for sequence - always succeeds for pure Rust
        impl<const N: usize> ros2_types::TryClone for #seq_type<N>
        where
            #name: Clone,
        {
            fn try_clone(&self) -> Option<Self> {
                Some(self.clone())
            }
        }

        // PartialEq for sequence
        impl<const N: usize> PartialEq for #seq_type<N>
        where
            #name: PartialEq,
        {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }

        // Serde Serialize for sequence
        impl<const N: usize> ros2_types::serde::Serialize for #seq_type<N>
        where
            #name: ros2_types::serde::Serialize,
        {
            fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
            where
                S: ros2_types::serde::Serializer,
            {
                self.0.serialize(serializer)
            }
        }

        // Serde Deserialize for sequence
        impl<'de, const N: usize> ros2_types::serde::Deserialize<'de> for #seq_type<N>
        where
            #name: ros2_types::serde::Deserialize<'de>,
        {
            fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
            where
                D: ros2_types::serde::Deserializer<'de>,
            {
                let inner = #seq_raw_type::deserialize(deserializer)?;
                if N != 0 && inner.len() > N {
                    return Err(ros2_types::serde::de::Error::custom(
                        format!("sequence length {} exceeds maximum {}", inner.len(), N)
                    ));
                }
                Ok(Self(inner))
            }
        }
    }
}

fn generate_native_type_support_impl(
    name: &syn::Ident,
    package: &str,
    interface_type: &str,
) -> TokenStream {
    // DDS type name format: "pkg_name::interface_type::dds_::TypeName_"
    let dds_type_name = format!("{}::{}::dds_::{}_", package, interface_type, name);

    quote! {
        // TypeSupport implementation
        impl ros2_types::TypeSupport for #name {
            fn to_bytes(&self) -> ros2_types::Result<Vec<u8>> {
                <Self as ros2_types::CdrSerde>::serialize(self)
            }

            fn from_bytes(bytes: &[u8]) -> ros2_types::Result<Self> {
                <Self as ros2_types::CdrSerde>::deserialize(bytes)
            }

            fn type_name() -> &'static str {
                #dds_type_name
            }

            fn type_hash() -> ros2_types::Result<::std::string::String> {
                <Self as ros2_types::TypeDescription>::compute_hash()
            }
        }
    }
}

/// Generate native type support impl for action wrapper types that don't have TypeDescription.
/// These types use the default type_hash() implementation (returns empty string).
fn generate_native_type_support_impl_no_hash(
    name: &syn::Ident,
    package: &str,
    interface_type: &str,
) -> TokenStream {
    // DDS type name format: "pkg_name::interface_type::dds_::TypeName_"
    let dds_type_name = format!("{}::{}::dds_::{}_", package, interface_type, name);

    quote! {
        // TypeSupport implementation (without type_hash override)
        impl ros2_types::TypeSupport for #name {
            fn to_bytes(&self) -> ros2_types::Result<Vec<u8>> {
                <Self as ros2_types::CdrSerde>::serialize(self)
            }

            fn from_bytes(bytes: &[u8]) -> ros2_types::Result<Self> {
                <Self as ros2_types::CdrSerde>::deserialize(bytes)
            }

            fn type_name() -> &'static str {
                #dds_type_name
            }
            // Uses default type_hash() implementation
        }
    }
}

fn generate_rcl_type_support_impl(
    name: &syn::Ident,
    package: &str,
    interface_type: &str,
) -> TokenStream {
    // DDS type name format: "pkg_name::interface_type::dds_::TypeName_"
    let dds_type_name = format!("{}::{}::dds_::{}_", package, interface_type, name);
    let type_support_fn = format_ident!(
        "rosidl_typesupport_c__get_message_type_support_handle__{}__{}__{}",
        package,
        interface_type,
        name
    );
    let dds_lit = syn::LitStr::new(&dds_type_name, proc_macro2::Span::call_site());
    quote! {
        // TypeSupport implementation
        impl ros2_types::TypeSupport for #name {
            fn type_support() -> *const std::ffi::c_void {
                unsafe { #type_support_fn() }
            }

            fn to_bytes(&self) -> ros2_types::Result<Vec<u8>> {
                let ts = Self::type_support();
                let mut msg_buf: rcl_serialized_message_t = unsafe { rcutils_get_zero_initialized_uint8_array() };
                let ret_init = unsafe {
                    rcutils_uint8_array_init(
                        &mut msg_buf as *mut rcl_serialized_message_t,
                        0,
                        &rcutils_get_default_allocator() as *const _,
                    )
                };
                if ret_init != 0 {
                    return Err(ros2_types::Error::CdrError("rcutils_uint8_array_init failed".to_string()));
                }
                let ret = unsafe {
                    rmw_serialize(
                        self as *const _ as *const std::ffi::c_void,
                        ts,
                        &mut msg_buf as *mut rcl_serialized_message_t,
                    )
                };
                let result = if ret == 0 {
                    let slice = unsafe { std::slice::from_raw_parts(msg_buf.buffer, msg_buf.buffer_length) };
                    Ok(slice.to_vec())
                } else {
                    Err(ros2_types::Error::CdrError("rmw_serialize failed".to_string()))
                };
                unsafe { rcutils_uint8_array_fini(&mut msg_buf as *mut rcl_serialized_message_t) };
                result
            }

            fn from_bytes(bytes: &[u8]) -> ros2_types::Result<Self> {
                let ts = Self::type_support();
                let mut msg = unsafe { std::mem::zeroed() };
                let mut read = 0usize;
                let ret = unsafe {
                    rmw_deserialize(
                        bytes.as_ptr(),
                        bytes.len(),
                        ts,
                        &mut msg as *mut _ as *mut std::ffi::c_void,
                        &mut read as *mut usize,
                    )
                };
                if ret == 0 {
                    Ok(msg)
                } else {
                    Err(ros2_types::Error::CdrError("rmw_deserialize failed".to_string()))
                }
            }

            fn type_name() -> &'static str {
                #dds_lit
            }
        }
    }
}

/// Generate service wrapper implementation (for use with ros2_service! macro)
///
/// This generates a service wrapper struct with ServiceMsg trait implementation.
pub fn generate_service_wrapper(package: &str, service_name: &str) -> TokenStream {
    let service_ident = format_ident!("{}", service_name);
    let request_ident = format_ident!("{}_Request", service_name);
    let response_ident = format_ident!("{}_Response", service_name);

    let type_support_fn = format_ident!(
        "rosidl_typesupport_c__get_service_type_support_handle__{}__srv__{}",
        package,
        service_name
    );

    let service_doc = format!("Service wrapper for {}", service_name);
    let dds_type_name = format!("{}::srv::dds_::{}_", package, service_name);
    quote! {
        #[doc = #service_doc]
        #[derive(Debug, ros2_types::ServiceTypeDescription)]
        #[ros2(package = #package)]
        pub struct #service_ident;

        #[cfg(feature = "rcl")]
        unsafe extern "C" {
            fn #type_support_fn() -> *const std::ffi::c_void;
        }

        impl ros2_types::ServiceMsg for #service_ident {
            type Request = #request_ident;
            type Response = #response_ident;
            #[cfg(feature = "rcl")]
            fn type_support() -> *const std::ffi::c_void {
                unsafe { #type_support_fn() }
            }
            #[cfg(not(feature = "rcl"))]
            fn type_hash() -> ros2_types::Result<::std::string::String> {
                <Self as ros2_types::ServiceTypeDescription>::compute_hash()
            }
            fn type_name() -> &'static str {
                #dds_type_name
            }
        }
    }
}

/// Generate the action wrapper implementation (for use with ros2_action! macro)
///
/// This generates an action wrapper struct with all required traits:
/// - ActionMsg
/// - ActionGoal (for SendGoal service)
/// - ActionResult (for GetResult service)
/// - Helper structs: SendGoal_Request, SendGoal_Response, GetResult_Request, GetResult_Response, FeedbackMessage
///
/// # Arguments
///
/// * `package` - The ROS2 package name
/// * `action_name` - The action name (e.g., "Fibonacci")
/// * `uuid_path_prefix` - Optional path prefix for unique_identifier_msgs (e.g., "crate" for "crate::unique_identifier_msgs")
///
/// # Required Dependencies
///
/// Actions require `unique_identifier_msgs` and `builtin_interfaces` packages.
pub fn generate_action_wrapper(
    package: &str,
    action_name: &str,
    uuid_path_prefix: Option<&str>,
) -> TokenStream {
    // DDS type name format: "pkg_name::interface_type::dds_::TypeName_"
    let dds_type_name = format!("{}::action::dds_::{}_", package, action_name);
    let action_ident = format_ident!("{}", action_name);
    let goal_ident = format_ident!("{}_Goal", action_name);
    let result_ident = format_ident!("{}_Result", action_name);
    let feedback_ident = format_ident!("{}_Feedback", action_name);

    let send_goal_ident = format_ident!("{}_SendGoal", action_name);
    let send_goal_request_ident = format_ident!("{}_SendGoal_Request", action_name);
    let send_goal_response_ident = format_ident!("{}_SendGoal_Response", action_name);

    let get_result_ident = format_ident!("{}_GetResult", action_name);
    let get_result_request_ident = format_ident!("{}_GetResult_Request", action_name);
    let get_result_response_ident = format_ident!("{}_GetResult_Response", action_name);

    let feedback_message_ident = format_ident!("{}_FeedbackMessage", action_name);

    // Action type support FFI
    let action_type_support_fn = format_ident!(
        "rosidl_typesupport_c__get_action_type_support_handle__{}__action__{}",
        package,
        action_name
    );

    // Service type support FFI
    let send_goal_type_support_fn = format_ident!(
        "rosidl_typesupport_c__get_service_type_support_handle__{}__action__{}_SendGoal",
        package,
        action_name
    );

    let get_result_type_support_fn = format_ident!(
        "rosidl_typesupport_c__get_service_type_support_handle__{}__action__{}_GetResult",
        package,
        action_name
    );

    // Message type support FFI for all helper types
    let send_goal_request_type_support_fn = format_ident!(
        "rosidl_typesupport_c__get_message_type_support_handle__{}__action__{}_SendGoal_Request",
        package,
        action_name
    );
    let send_goal_response_type_support_fn = format_ident!(
        "rosidl_typesupport_c__get_message_type_support_handle__{}__action__{}_SendGoal_Response",
        package,
        action_name
    );
    let get_result_request_type_support_fn = format_ident!(
        "rosidl_typesupport_c__get_message_type_support_handle__{}__action__{}_GetResult_Request",
        package,
        action_name
    );
    let get_result_response_type_support_fn = format_ident!(
        "rosidl_typesupport_c__get_message_type_support_handle__{}__action__{}_GetResult_Response",
        package,
        action_name
    );
    let feedback_message_type_support_fn = format_ident!(
        "rosidl_typesupport_c__get_message_type_support_handle__{}__action__{}_FeedbackMessage",
        package,
        action_name
    );

    // Build the UUID type path based on the prefix
    let uuid_type: syn::Type = if let Some(prefix) = uuid_path_prefix {
        syn::parse_str(&format!("{}::unique_identifier_msgs::msg::UUID", prefix))
            .unwrap_or_else(|_| syn::parse_str("unique_identifier_msgs::msg::UUID").unwrap())
    } else {
        syn::parse_str("unique_identifier_msgs::msg::UUID").unwrap()
    };

    let action_doc = format!("Action wrapper for {}", action_name);
    let ts_send_goal_req_impl_rcl =
        generate_rcl_type_support_impl(&send_goal_request_ident, package, "action");
    let ts_send_goal_resp_impl_rcl =
        generate_rcl_type_support_impl(&send_goal_response_ident, package, "action");
    let ts_get_result_req_impl_rcl =
        generate_rcl_type_support_impl(&get_result_request_ident, package, "action");
    let ts_get_result_resp_impl_rcl =
        generate_rcl_type_support_impl(&get_result_response_ident, package, "action");
    let ts_feedback_message_impl_rcl =
        generate_rcl_type_support_impl(&feedback_message_ident, package, "action");

    let ts_send_goal_req_impl_native =
        generate_native_type_support_impl_no_hash(&send_goal_request_ident, package, "action");
    let ts_send_goal_resp_impl_native =
        generate_native_type_support_impl_no_hash(&send_goal_response_ident, package, "action");
    let ts_get_result_req_impl_native =
        generate_native_type_support_impl_no_hash(&get_result_request_ident, package, "action");
    let ts_get_result_resp_impl_native =
        generate_native_type_support_impl_no_hash(&get_result_response_ident, package, "action");
    let ts_feedback_message_impl_native =
        generate_native_type_support_impl_no_hash(&feedback_message_ident, package, "action");

    let serialization_ffi = generate_serialization_ffi_decls();
    let serialized_msg_struct = generate_serialized_message_struct();

    quote! {
        // =============================================================================
        // FFI declarations
        // =============================================================================

        #[cfg(feature = "rcl")]
        unsafe extern "C" {
            fn #action_type_support_fn() -> *const std::ffi::c_void;
            fn #send_goal_type_support_fn() -> *const std::ffi::c_void;
            fn #get_result_type_support_fn() -> *const std::ffi::c_void;
            // Message type supports for helper types
            fn #send_goal_request_type_support_fn() -> *const std::ffi::c_void;
            fn #send_goal_response_type_support_fn() -> *const std::ffi::c_void;
            fn #get_result_request_type_support_fn() -> *const std::ffi::c_void;
            fn #get_result_response_type_support_fn() -> *const std::ffi::c_void;
            fn #feedback_message_type_support_fn() -> *const std::ffi::c_void;
            #serialization_ffi
        }

        #serialized_msg_struct

        // =============================================================================
        // SendGoal service types
        // =============================================================================

        /// Request message for sending a goal
        #[repr(C)]
        #[derive(Debug)]
        #[cfg_attr(not(feature = "rcl"), derive(ros2_types::serde::Serialize, ros2_types::serde::Deserialize))]
        #[cfg_attr(not(feature = "rcl"), serde(crate = "ros2_types::serde"))]
        pub struct #send_goal_request_ident {
            pub goal_id: #uuid_type,
            pub goal: #goal_ident,
        }

        /// Response message for goal acceptance
        #[repr(C)]
        #[derive(Debug)]
        #[cfg_attr(not(feature = "rcl"), derive(ros2_types::serde::Serialize, ros2_types::serde::Deserialize))]
        #[cfg_attr(not(feature = "rcl"), serde(crate = "ros2_types::serde"))]
        pub struct #send_goal_response_ident {
            pub accepted: bool,
            pub stamp: ros2_types::UnsafeTime,
        }

        /// SendGoal service wrapper
        #[derive(Debug)]
        pub struct #send_goal_ident;

        impl ros2_types::ActionGoal for #send_goal_ident {
            type Request = #send_goal_request_ident;
            type Response = #send_goal_response_ident;

            #[cfg(feature = "rcl")]
            fn type_support() -> *const std::ffi::c_void {
                unsafe { #send_goal_type_support_fn() }
            }
        }

        impl ros2_types::GetUUID for #send_goal_request_ident {
            fn get_uuid(&self) -> &[u8; 16] {
                &self.goal_id.uuid
            }
        }

        #[cfg(feature = "rcl")]
        #ts_send_goal_req_impl_rcl
        #[cfg(not(feature = "rcl"))]
        #ts_send_goal_req_impl_native

        impl ros2_types::GoalResponse for #send_goal_response_ident {
            fn is_accepted(&self) -> bool {
                self.accepted
            }

            fn get_time_stamp(&self) -> ros2_types::UnsafeTime {
                ros2_types::UnsafeTime {
                    sec: self.stamp.sec,
                    nanosec: self.stamp.nanosec,
                }
            }

            fn new(accepted: bool, stamp: ros2_types::UnsafeTime) -> Self {
                Self { accepted, stamp }
            }
        }

        #[cfg(feature = "rcl")]
        #ts_send_goal_resp_impl_rcl
        #[cfg(not(feature = "rcl"))]
        #ts_send_goal_resp_impl_native

        // =============================================================================
        // GetResult service types
        // =============================================================================

        /// Request message for getting action result
        #[repr(C)]
        #[derive(Debug)]
        #[cfg_attr(not(feature = "rcl"), derive(ros2_types::serde::Serialize, ros2_types::serde::Deserialize))]
        #[cfg_attr(not(feature = "rcl"), serde(crate = "ros2_types::serde"))]
        pub struct #get_result_request_ident {
            pub goal_id: #uuid_type,
        }

        /// Response message containing the result
        #[repr(C)]
        #[derive(Debug)]
        #[cfg_attr(not(feature = "rcl"), derive(ros2_types::serde::Serialize, ros2_types::serde::Deserialize))]
        #[cfg_attr(not(feature = "rcl"), serde(crate = "ros2_types::serde"))]
        pub struct #get_result_response_ident {
            pub status: u8,
            pub result: #result_ident,
        }

        /// GetResult service wrapper
        #[derive(Debug)]
        pub struct #get_result_ident;

        impl ros2_types::ActionResult for #get_result_ident {
            type Request = #get_result_request_ident;
            type Response = #get_result_response_ident;

            #[cfg(feature = "rcl")]
            fn type_support() -> *const std::ffi::c_void {
                unsafe { #get_result_type_support_fn() }
            }
        }

        impl ros2_types::GetUUID for #get_result_request_ident {
            fn get_uuid(&self) -> &[u8; 16] {
                &self.goal_id.uuid
            }
        }

        #[cfg(feature = "rcl")]
        #ts_get_result_req_impl_rcl
        #[cfg(not(feature = "rcl"))]
        #ts_get_result_req_impl_native

        impl ros2_types::ResultResponse for #get_result_response_ident {
            fn get_status(&self) -> u8 {
                self.status
            }
        }

        #[cfg(feature = "rcl")]
        #ts_get_result_resp_impl_rcl
        #[cfg(not(feature = "rcl"))]
        #ts_get_result_resp_impl_native

        // =============================================================================
        // Feedback message
        // =============================================================================

        /// Feedback message with goal UUID
        #[repr(C)]
        #[derive(Debug)]
        #[cfg_attr(not(feature = "rcl"), derive(ros2_types::serde::Serialize, ros2_types::serde::Deserialize))]
        #[cfg_attr(not(feature = "rcl"), serde(crate = "ros2_types::serde"))]
        pub struct #feedback_message_ident {
            pub goal_id: #uuid_type,
            pub feedback: #feedback_ident,
        }

        impl ros2_types::GetUUID for #feedback_message_ident {
            fn get_uuid(&self) -> &[u8; 16] {
                &self.goal_id.uuid
            }
        }

        #[cfg(feature = "rcl")]
        #ts_feedback_message_impl_rcl
        #[cfg(not(feature = "rcl"))]
        #ts_feedback_message_impl_native

        // =============================================================================
        // Action wrapper
        // =============================================================================

        #[doc = #action_doc]
        #[derive(Debug, ros2_types::ActionTypeDescription)]
        #[ros2(package = #package)]
        pub struct #action_ident;

        impl ros2_types::ActionMsg for #action_ident {
            type Goal = #send_goal_ident;
            type Result = #get_result_ident;
            type Feedback = #feedback_message_ident;

            fn type_name() -> &'static str {
                #dds_type_name
            }

            #[cfg(feature = "rcl")]
            fn type_support() -> *const std::ffi::c_void {
                unsafe { #action_type_support_fn() }
            }

            type GoalContent = #goal_ident;

            fn new_goal_request(
                goal: Self::GoalContent,
                uuid: [u8; 16],
            ) -> <Self::Goal as ros2_types::ActionGoal>::Request {
                #send_goal_request_ident {
                    goal,
                    goal_id: #uuid_type { uuid },
                }
            }

            type ResultContent = #result_ident;

            fn new_result_response(
                status: u8,
                result: Self::ResultContent,
            ) -> <Self::Result as ros2_types::ActionResult>::Response {
                #get_result_response_ident { status, result }
            }

            type FeedbackContent = #feedback_ident;

            fn new_feedback_message(
                feedback: Self::FeedbackContent,
                uuid: [u8; 16],
            ) -> Self::Feedback {
                #feedback_message_ident {
                    feedback,
                    goal_id: #uuid_type { uuid },
                }
            }
            #[cfg(not(feature = "rcl"))]
            fn type_hash() -> ros2_types::Result<::std::string::String> {
                <Self as ros2_types::ActionTypeDescription>::compute_hash()
            }
        }
    }
}
