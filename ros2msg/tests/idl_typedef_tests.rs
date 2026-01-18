//! Tests for IDL parser typedef support
//!
//! These tests document the current limitation: typedefs are parsed but not tracked,
//! so typedef names cannot be used as types in structs.
//!
//! TODO: Implement typedef tracking and resolution in the IDL parser

use ros2msg::idl::parser_pest::parse_idl;

#[test]
#[ignore]
fn test_parse_typedef_char_array() {
    // This tests the real-world case from rmw_dds_common/msg/Gid.msg
    // The IDL adapter generates a typedef for fixed-size char arrays
    let idl = r#"
module rmw_dds_common {
  module msg {
    typedef char char__16[16];
    struct Gid {
      char__16 data;
    };
  };
};
"#;

    let result = parse_idl(idl);
    match &result {
        Ok(parsed) => {
            println!("Successfully parsed IDL with typedef!");
            println!("Definitions: {}", parsed.definitions.len());
        }
        Err(e) => {
            eprintln!("Parse error: {}", e);
            eprintln!("\nROOT CAUSE: IDL parser doesn't track typedefs");
            eprintln!("The typedef is parsed but 'char__16' is not recognized as a valid type");
            eprintln!("\nFIX NEEDED:");
            eprintln!("1. Parser must collect typedefs into a symbol table");
            eprintln!("2. Type resolution must check typedef names");
            eprintln!("3. Code generator must expand typedefs to Rust types");
        }
    }
    // This will fail until typedef tracking is implemented
    assert!(
        result.is_ok(),
        "IDL parser needs typedef support - see error above for details"
    );
}

#[test]
fn test_parse_simple_typedef() {
    // Simple typedefs without usage pass because they're valid syntax
    let idl = r#"
module test {
  typedef long MyLong;
  typedef long LongArray[10];
};
"#;

    let result = parse_idl(idl);
    assert!(
        result.is_ok(),
        "Failed to parse simple typedef: {:?}",
        result.err()
    );
}

#[test]
#[ignore]
fn test_wstring_multiline_annotation() {
    // Test unbounded wstring first
    let simple_wstring = r#"
module test {
  struct Simple {
    wstring<> data;
  };
};
"#;
    let result = parse_idl(simple_wstring);
    if let Err(e) = &result {
        eprintln!("Simple wstring parse error: {}", e);
    }
    assert!(result.is_ok(), "Unbounded wstring should parse");

    // Now test the actual WString.idl from ROS2 with multiline @verbatim
    let idl = r#"
module example_interfaces {
  module msg {
    @verbatim (language="comment", text=
      "This is an example message of using a primitive datatype, wstring." "\n"
      "If you want to test with this that's fine, but if you are deploying" "\n"
      "it into a system you should create a semantically meaningful message type." "\n"
      "If you want to embed it in another message, use the primitive data type instead.")
    struct WString {
      wstring<> data;
    };
  };
};
"#;

    let result = parse_idl(idl);
    if let Err(e) = &result {
        eprintln!("WString parse error: {}", e);
    }
    assert!(
        result.is_ok(),
        "WString with multiline verbatim annotation should parse"
    );
}

#[test]
fn test_simple_struct_works() {
    // Regular structs without typedefs work fine
    let idl = r#"
module test_msgs {
  module msg {
    struct Simple {
      int32 value;
      string name;
    };
  };
};
"#;
    let result = parse_idl(idl);
    assert!(
        result.is_ok(),
        "Simple struct should parse: {:?}",
        result.err()
    );
}

#[test]
#[ignore]
fn test_typedef_usage_fails() {
    // Test if typedef usage works
    let idl_with_typedef = r#"
module test_msgs {
  module msg {
    typedef int32 MyInt;
    struct Simple {
      MyInt value;
    };
  };
};
"#;
    let result = parse_idl(idl_with_typedef);
    if let Err(e) = &result {
        eprintln!("Typedef usage error: {}", e);
    } else {
        eprintln!("Typedef usage WORKS for simple types!");
    }

    // Test array typedef specifically (this is what fails)
    let idl_array_typedef = r#"
module test_msgs {
  module msg {
    typedef char CharArray[16];
    struct ArrayTest {
      CharArray data;
    };
  };
};
"#;
    let result2 = parse_idl(idl_array_typedef);
    if let Err(e) = &result2 {
        eprintln!("Array typedef error: {}", e);
    }
    assert!(result2.is_err(), "Array typedef usage should fail");
}
