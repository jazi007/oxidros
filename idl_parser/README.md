# T4 IDL Parser

A parser for the interface definition language (IDL) specified by Object Management Group (OMG)  written in Rust. This supports IDL version 4.2.

# Example

```rust
use t4_idl_parser::{parse, Span};
use nom_greedyerror::convert_error;

let input = r#"
// generated from rosidl_adapter/resource/msg.idl.em
// with input from example_msg/bar/Buz.msg
// generated code does not contain a copyright notice

module example_msg {
    module msg {
        struct Buz {
            string c;

            @verbatim (language="comment", text="http://wiki.ros.org/std_msgs")
            sequence<int32> o;
        };
    };
};"#;

match parse(input) {
    Ok(result) => {
        println!("{:#?}", result);
    }
    Err(e) => {
        eprintln!("{e}");
        panic!();
    }
}
```

This will output a result as follows.

```text
[
    Module(
        Module {
            id: "example_msg",
            definitions: [
                Module(
                    Module {
                        id: "msg",
                        definitions: [
                            Type(
                                ConstrType(
                                    Struct(
                                        Def(
                                            StructDef {
                                                id: "Buz",
                                                members: [
                                                    Member {
                                                        type_spec: Template(
                                                            String(
                                                                UnlimitedSize,
                                                            ),
                                                        ),
                                                        declarators: [
                                                            Simple(
                                                                "c",
                                                            ),
                                                        ],
                                                    },
                                                    Member {
                                                        type_spec: Template(
                                                            Sequence(
                                                                Unlimited(
                                                                    PrimitiveType(
                                                                        Int32,
                                                                    ),
                                                                ),
                                                            ),
                                                        ),
                                                        declarators: [
                                                            Simple(
                                                                "o",
                                                            ),
                                                        ],
                                                    },
                                                ],
                                                inheritance: None,
                                            },
                                        ),
                                    ),
                                ),
                            ),
                        ],
                    },
                ),
            ],
        },
    ),
]
```

# Limitation

C/C++ like preprocessor is not supported.
