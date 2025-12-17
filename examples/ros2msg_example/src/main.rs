// Re-export msg types needed by generated code
pub mod msg {
    pub use oxidros::msg::{
        BoolSeq, F32Seq, F64Seq, I16Seq, I32Seq, I64Seq, I8Seq, RosString, RosStringSeq, U16Seq,
        U32Seq, U64Seq, U8Seq,
    };
}

// Re-export rcl types needed by generated code
pub mod rcl {
    pub use oxidros::rcl::{rosidl_message_type_support_t, rosidl_service_type_support_t};
    pub use std::os::raw::c_ulong as size_t;
}

include!(concat!(env!("OUT_DIR"), "/mod.rs"));

fn main() {
    println!("Hello, world!");
}
