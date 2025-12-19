/// # T4 IDL Parser
///
/// A parser for the interface definition language (IDL) specified by Object Management Group (OMG)  written in Rust. This supports IDL version 4.2.
///
/// # Example
///
/// ```
/// use t4_idl_parser::parse;
/// use nom_greedyerror::convert_error;
///
/// let input = r#"
/// // generated from rosidl_adapter/resource/msg.idl.em
/// // with input from example_msg/bar/Buz.msg
/// // generated code does not contain a copyright notice
///
/// module example_msg {
///     module msg {
///         struct Buz {
///             string c;
///
///             @verbatim (language="comment", text="http://wiki.ros.org/std_msgs")
///             sequence<int32> o;
///         };
///     };
/// };"#;
///
/// match parse(input) {
///     Ok(result) => {
///         println!("{:?}", result);
///     }
///     Err(e) => {
///         eprintln!("{e}");
///         panic!();
///     }
/// }
/// ```
pub(crate) mod character;
pub mod expr;
pub(crate) mod parser;

pub use parser::parse;
