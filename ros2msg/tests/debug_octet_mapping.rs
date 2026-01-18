// Test to debug octet constant type mapping

use ros2msg::generator::{CodeGenerator, GeneratorConfig};
use ros2msg::idl::types::{BasicType, BasicTypeKind, IdlType};

#[test]
fn test_octet_type_mapping() {
    // Create an octet IdlType
    let octet_type = IdlType::Basic(BasicType::from_kind(BasicTypeKind::Octet));

    // Create a code generator
    let config = GeneratorConfig::default();
    let generator = CodeGenerator::new(config);

    // Test the mapping function
    let rust_type = generator.map_idl_type_to_rust(&octet_type);

    println!("IdlType: {:?}", octet_type);
    println!("Rust type: {}", rust_type);

    assert_eq!(rust_type, "u8", "octet should map to u8");
}

#[test]
fn test_octet_constant_mapping() {
    // Create an octet IdlType
    let octet_type = IdlType::Basic(BasicType::from_kind(BasicTypeKind::Octet));

    // Create a code generator
    let config = GeneratorConfig::default();
    let generator = CodeGenerator::new(config);

    // Test the constant mapping function
    let rust_type = generator.map_idl_type_to_rust_for_constant(&octet_type);

    println!("IdlType for constant: {:?}", octet_type);
    println!("Rust type for constant: {}", rust_type);

    assert_eq!(rust_type, "u8", "octet constant should map to u8");
}
