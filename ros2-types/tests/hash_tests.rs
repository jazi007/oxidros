//! Tests for RIHS01 type hash calculation

use ros2_types::types::*;
use ros2_types::*;

#[test]
fn test_simple_message_hash() {
    let type_desc = IndividualTypeDescription::new(
        "test_pkg/msg/SimpleMsg",
        vec![
            Field::new("field1", FieldType::primitive(FIELD_TYPE_INT32)),
            Field::new("field2", FieldType::primitive(FIELD_TYPE_STRING)),
        ],
    );

    let msg = TypeDescriptionMsg::new(type_desc, vec![]);
    let hash = calculate_type_hash(&msg).unwrap();

    assert!(hash.starts_with("RIHS01_"));
    assert_eq!(hash.len(), 71); // "RIHS01_" (7) + SHA256 hex (64)
}

#[test]
fn test_nested_type_hash() {
    let inner_type = IndividualTypeDescription::new(
        "test_pkg/msg/InnerMsg",
        vec![Field::new("value", FieldType::primitive(FIELD_TYPE_DOUBLE))],
    );

    let outer_type = IndividualTypeDescription::new(
        "test_pkg/msg/OuterMsg",
        vec![
            Field::new("id", FieldType::primitive(FIELD_TYPE_UINT32)),
            Field::new("inner", FieldType::nested("test_pkg/msg/InnerMsg")),
        ],
    );

    let msg = TypeDescriptionMsg::new(outer_type, vec![inner_type]);
    let hash = calculate_type_hash(&msg).unwrap();

    assert!(hash.starts_with("RIHS01_"));
}

#[test]
fn test_array_types() {
    let type_desc = IndividualTypeDescription::new(
        "test_pkg/msg/ArrayMsg",
        vec![
            Field::new("fixed_array", FieldType::array(FIELD_TYPE_INT32, 10)),
            Field::new("dynamic_array", FieldType::array(FIELD_TYPE_FLOAT, 0)),
            Field::new("bounded_array", FieldType::array(FIELD_TYPE_UINT64, 100)),
        ],
    );

    let msg = TypeDescriptionMsg::new(type_desc, vec![]);
    let hash = calculate_type_hash(&msg).unwrap();

    assert!(hash.starts_with("RIHS01_"));
}

#[test]
fn test_string_types() {
    let type_desc = IndividualTypeDescription::new(
        "test_pkg/msg/StringMsg",
        vec![
            Field::new("unbounded_string", FieldType::primitive(FIELD_TYPE_STRING)),
            Field::new(
                "bounded_string",
                FieldType::string_with_capacity(FIELD_TYPE_BOUNDED_STRING, 255),
            ),
            Field::new("wstring", FieldType::primitive(FIELD_TYPE_WSTRING)),
        ],
    );

    let msg = TypeDescriptionMsg::new(type_desc, vec![]);
    let hash = calculate_type_hash(&msg).unwrap();

    assert!(hash.starts_with("RIHS01_"));
}

#[test]
fn test_all_primitive_types() {
    let type_desc = IndividualTypeDescription::new(
        "test_pkg/msg/AllPrimitives",
        vec![
            Field::new("bool_field", FieldType::primitive(FIELD_TYPE_BOOLEAN)),
            Field::new("byte_field", FieldType::primitive(FIELD_TYPE_BYTE)),
            Field::new("char_field", FieldType::primitive(FIELD_TYPE_CHAR)),
            Field::new("int8_field", FieldType::primitive(FIELD_TYPE_INT8)),
            Field::new("uint8_field", FieldType::primitive(FIELD_TYPE_UINT8)),
            Field::new("int16_field", FieldType::primitive(FIELD_TYPE_INT16)),
            Field::new("uint16_field", FieldType::primitive(FIELD_TYPE_UINT16)),
            Field::new("int32_field", FieldType::primitive(FIELD_TYPE_INT32)),
            Field::new("uint32_field", FieldType::primitive(FIELD_TYPE_UINT32)),
            Field::new("int64_field", FieldType::primitive(FIELD_TYPE_INT64)),
            Field::new("uint64_field", FieldType::primitive(FIELD_TYPE_UINT64)),
            Field::new("float_field", FieldType::primitive(FIELD_TYPE_FLOAT)),
            Field::new("double_field", FieldType::primitive(FIELD_TYPE_DOUBLE)),
        ],
    );

    let msg = TypeDescriptionMsg::new(type_desc, vec![]);
    let hash = calculate_type_hash(&msg).unwrap();

    assert!(hash.starts_with("RIHS01_"));
}

#[test]
fn test_parse_rihs_string() {
    let hash = "RIHS01_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let (version, value) = parse_rihs_string(hash).unwrap();

    assert_eq!(version, 1);
    assert_eq!(
        value,
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
}

#[test]
fn test_parse_rihs_invalid_format() {
    assert!(parse_rihs_string("invalid").is_err());
    assert!(parse_rihs_string("RIHS_nope").is_err());
    assert!(parse_rihs_string("RIHS01").is_err());
    assert!(parse_rihs_string("notrihs_abc").is_err());
}

#[test]
fn test_hash_deterministic() {
    let type_desc = IndividualTypeDescription::new(
        "test_pkg/msg/DeterministicMsg",
        vec![
            Field::new("field_a", FieldType::primitive(FIELD_TYPE_INT32)),
            Field::new("field_b", FieldType::primitive(FIELD_TYPE_STRING)),
        ],
    );

    let msg1 = TypeDescriptionMsg::new(type_desc.clone(), vec![]);
    let msg2 = TypeDescriptionMsg::new(type_desc, vec![]);

    let hash1 = calculate_type_hash(&msg1).unwrap();
    let hash2 = calculate_type_hash(&msg2).unwrap();

    assert_eq!(hash1, hash2, "Hashes should be deterministic");
}

#[test]
fn test_hash_changes_with_field_order() {
    let type_desc1 = IndividualTypeDescription::new(
        "test_pkg/msg/OrderTest",
        vec![
            Field::new("field_a", FieldType::primitive(FIELD_TYPE_INT32)),
            Field::new("field_b", FieldType::primitive(FIELD_TYPE_STRING)),
        ],
    );

    let type_desc2 = IndividualTypeDescription::new(
        "test_pkg/msg/OrderTest",
        vec![
            Field::new("field_b", FieldType::primitive(FIELD_TYPE_STRING)),
            Field::new("field_a", FieldType::primitive(FIELD_TYPE_INT32)),
        ],
    );

    let msg1 = TypeDescriptionMsg::new(type_desc1, vec![]);
    let msg2 = TypeDescriptionMsg::new(type_desc2, vec![]);

    let hash1 = calculate_type_hash(&msg1).unwrap();
    let hash2 = calculate_type_hash(&msg2).unwrap();

    assert_ne!(hash1, hash2, "Field order should affect hash");
}

#[test]
fn test_default_values() {
    let type_desc = IndividualTypeDescription::new(
        "test_pkg/msg/DefaultMsg",
        vec![
            Field::with_default("field1", FieldType::primitive(FIELD_TYPE_INT32), "42"),
            Field::with_default("field2", FieldType::primitive(FIELD_TYPE_STRING), "hello"),
        ],
    );

    let msg = TypeDescriptionMsg::new(type_desc, vec![]);
    let hash = calculate_type_hash(&msg).unwrap();

    assert!(hash.starts_with("RIHS01_"));
}

// Test the TypeDescription trait implementation
struct TestMessage;

impl TypeDescription for TestMessage {
    fn type_description() -> TypeDescriptionMsg {
        let type_desc = IndividualTypeDescription::new(
            "test/msg/TestMessage",
            vec![Field::new("data", FieldType::primitive(FIELD_TYPE_INT32))],
        );
        TypeDescriptionMsg::new(type_desc, vec![])
    }

    fn message_type_name() -> MessageTypeName {
        MessageTypeName::new("msg", "test", "TestMessage")
    }
}

#[test]
fn test_trait_compute_hash() {
    let hash = TestMessage::compute_hash().unwrap();
    assert!(hash.starts_with("RIHS01_"));
    assert_eq!(hash.len(), 71);
}
