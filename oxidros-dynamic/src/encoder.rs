//! Dynamic JSON-to-CDR encoder.
//!
//! Given a `serde_json::Value` and a [`TypeDescriptionMsg`], produces CDR bytes
//! (including the 4-byte encapsulation header). This is the mirror of
//! [`decode_cdr`](crate::decode_cdr).

use crate::error::{DynamicError, Result};
use ros2_types::types::*;
use serde_json::Value;

/// Encode a JSON value into CDR bytes (little-endian).
///
/// The returned `Vec<u8>` includes the 4-byte CDR encapsulation header.
/// The `type_desc` provides the structural schema for encoding.
pub fn encode_cdr(value: &Value, type_desc: &TypeDescriptionMsg) -> Result<Vec<u8>> {
    let mut writer = CdrWriter {
        buf: Vec::with_capacity(256),
        referenced: &type_desc.referenced_type_descriptions,
    };

    // CDR LE header
    writer.buf.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);

    writer.write_struct(value, &type_desc.type_description)?;

    Ok(writer.buf)
}

/// Stateful CDR writer that tracks position and handles alignment.
struct CdrWriter<'a> {
    buf: Vec<u8>,
    referenced: &'a [IndividualTypeDescription],
}

impl<'a> CdrWriter<'a> {
    /// Current position within the CDR payload (after the 4-byte header).
    fn payload_pos(&self) -> usize {
        self.buf.len() - 4
    }

    /// Pad the buffer to align to `alignment` bytes (relative to payload start).
    fn align(&mut self, alignment: usize) {
        let rem = self.payload_pos() % alignment;
        if rem != 0 {
            let padding = alignment - rem;
            self.buf.extend(std::iter::repeat_n(0u8, padding));
        }
    }

    fn write_u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    fn write_i8(&mut self, v: i8) {
        self.buf.push(v as u8);
    }

    fn write_bool(&mut self, v: bool) {
        self.buf.push(if v { 1 } else { 0 });
    }

    fn write_u16(&mut self, v: u16) {
        self.align(2);
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn write_i16(&mut self, v: i16) {
        self.write_u16(v as u16);
    }

    fn write_u32(&mut self, v: u32) {
        self.align(4);
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn write_i32(&mut self, v: i32) {
        self.write_u32(v as u32);
    }

    fn write_u64(&mut self, v: u64) {
        self.align(8);
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn write_i64(&mut self, v: i64) {
        self.write_u64(v as u64);
    }

    fn write_f32(&mut self, v: f32) {
        self.align(4);
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn write_f64(&mut self, v: f64) {
        self.align(8);
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// Write a CDR string: u32 length (including null) + bytes + null.
    fn write_string(&mut self, s: &str) {
        let len = (s.len() + 1) as u32; // +1 for null terminator
        self.write_u32(len);
        self.buf.extend_from_slice(s.as_bytes());
        self.buf.push(0); // null terminator
    }

    /// Write a struct from the type description.
    fn write_struct(&mut self, value: &Value, desc: &IndividualTypeDescription) -> Result<()> {
        let obj = value
            .as_object()
            .ok_or_else(|| DynamicError::EncoderTypeMismatch {
                expected: "object".to_string(),
                got: value_type_name(value),
                field: desc.type_name.clone(),
            })?;

        for field in &desc.fields {
            let field_value = obj.get(&field.name).unwrap_or(&Value::Null);
            self.write_field_type(field_value, &field.field_type)?;
        }
        Ok(())
    }

    /// Write a single field value based on its FieldType.
    fn write_field_type(&mut self, value: &Value, ft: &FieldType) -> Result<()> {
        let (base_id, kind) = decompose_type_id(ft.type_id);

        match kind {
            'p' => self.write_primitive_or_nested(value, base_id, ft),
            'a' => {
                // Fixed-size array: exactly capacity elements, no length prefix
                let count = ft.capacity as usize;
                self.write_array_elements(value, base_id, ft, Some(count))
            }
            'b' | 'u' => {
                // Bounded or unbounded sequence: u32 count prefix
                let arr = value.as_array().unwrap_or(&EMPTY_ARRAY);
                self.write_u32(arr.len() as u32);
                self.write_array_elements(value, base_id, ft, None)
            }
            _ => Err(DynamicError::UnknownFieldType(ft.type_id)),
        }
    }

    /// Write a scalar primitive or nested type.
    fn write_primitive_or_nested(
        &mut self,
        value: &Value,
        base_id: u8,
        ft: &FieldType,
    ) -> Result<()> {
        match base_id {
            FIELD_TYPE_BOOLEAN => {
                self.write_bool(as_bool(value));
                Ok(())
            }
            FIELD_TYPE_BYTE | FIELD_TYPE_UINT8 | FIELD_TYPE_CHAR => {
                self.write_u8(as_u64(value) as u8);
                Ok(())
            }
            FIELD_TYPE_INT8 => {
                self.write_i8(as_i64(value) as i8);
                Ok(())
            }
            FIELD_TYPE_UINT16 | FIELD_TYPE_WCHAR => {
                self.write_u16(as_u64(value) as u16);
                Ok(())
            }
            FIELD_TYPE_INT16 => {
                self.write_i16(as_i64(value) as i16);
                Ok(())
            }
            FIELD_TYPE_UINT32 => {
                self.write_u32(as_u64(value) as u32);
                Ok(())
            }
            FIELD_TYPE_INT32 => {
                self.write_i32(as_i64(value) as i32);
                Ok(())
            }
            FIELD_TYPE_UINT64 => {
                self.write_u64(as_u64(value));
                Ok(())
            }
            FIELD_TYPE_INT64 => {
                self.write_i64(as_i64(value));
                Ok(())
            }
            FIELD_TYPE_FLOAT => {
                self.write_f32(as_f64(value) as f32);
                Ok(())
            }
            FIELD_TYPE_DOUBLE | FIELD_TYPE_LONG_DOUBLE => {
                self.write_f64(as_f64(value));
                Ok(())
            }
            FIELD_TYPE_STRING
            | FIELD_TYPE_BOUNDED_STRING
            | FIELD_TYPE_FIXED_STRING
            | FIELD_TYPE_WSTRING
            | FIELD_TYPE_BOUNDED_WSTRING
            | FIELD_TYPE_FIXED_WSTRING => {
                let s = value.as_str().unwrap_or("");
                self.write_string(s);
                Ok(())
            }
            FIELD_TYPE_NESTED_TYPE => {
                let nested_desc = self.find_referenced_type(&ft.nested_type_name)?;
                let nested_value = if value.is_null() {
                    &DEFAULT_OBJECT
                } else {
                    value
                };
                self.write_struct(nested_value, nested_desc)
            }
            _ => Err(DynamicError::UnknownFieldType(base_id)),
        }
    }

    /// Write `count` array/sequence elements.
    fn write_array_elements(
        &mut self,
        value: &Value,
        base_id: u8,
        ft: &FieldType,
        fixed_count: Option<usize>,
    ) -> Result<()> {
        let arr = value.as_array().unwrap_or(&EMPTY_ARRAY);
        let count = fixed_count.unwrap_or(arr.len());
        for i in 0..count {
            let elem = arr.get(i).unwrap_or(&Value::Null);
            self.write_primitive_or_nested(elem, base_id, ft)?;
        }
        Ok(())
    }

    /// Look up a referenced type by its fully qualified name.
    fn find_referenced_type(&self, type_name: &str) -> Result<&'a IndividualTypeDescription> {
        self.referenced
            .iter()
            .find(|t| t.type_name == type_name)
            .ok_or_else(|| DynamicError::ReferencedTypeNotFound(type_name.to_string()))
    }
}

// ============================================================================
// Helpers for extracting values from JSON with defaults
// ============================================================================

use std::sync::LazyLock;

static EMPTY_ARRAY: Vec<Value> = Vec::new();
static DEFAULT_OBJECT: LazyLock<Value> = LazyLock::new(|| Value::Object(serde_json::Map::new()));

fn as_bool(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_u64().unwrap_or(0) != 0,
        _ => false,
    }
}

fn as_u64(v: &Value) -> u64 {
    match v {
        Value::Number(n) => n.as_u64().unwrap_or(0),
        Value::Bool(b) => *b as u64,
        _ => 0,
    }
}

fn as_i64(v: &Value) -> i64 {
    match v {
        Value::Number(n) => n.as_i64().unwrap_or(0),
        Value::Bool(b) => *b as i64,
        _ => 0,
    }
}

fn as_f64(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}

fn value_type_name(v: &Value) -> String {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
    .to_string()
}

/// Extract base type_id and kind from a composite type_id.
fn decompose_type_id(type_id: u8) -> (u8, char) {
    match type_id {
        0..=48 => (type_id, 'p'),
        49..=96 => (type_id - 48, 'a'),
        97..=144 => (type_id - 96, 'b'),
        145..=192 => (type_id - 144, 'u'),
        _ => (type_id, 'p'),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode_cdr;

    fn make_simple_type_desc(fields: Vec<Field>) -> TypeDescriptionMsg {
        TypeDescriptionMsg {
            type_description: IndividualTypeDescription {
                type_name: "test/msg/Test".to_string(),
                fields,
            },
            referenced_type_descriptions: vec![],
        }
    }

    /// Round-trip test: encode JSON → CDR → decode → JSON, compare.
    fn roundtrip(desc: &TypeDescriptionMsg, input: &Value) -> Value {
        let cdr = encode_cdr(input, desc).expect("encode failed");
        decode_cdr(&cdr, desc).expect("decode failed")
    }

    #[test]
    fn test_encode_uint32() {
        let desc = make_simple_type_desc(vec![Field::new(
            "value",
            FieldType::primitive(FIELD_TYPE_UINT32),
        )]);
        let input = serde_json::json!({"value": 42});
        assert_eq!(roundtrip(&desc, &input)["value"], 42);
    }

    #[test]
    fn test_encode_string() {
        let desc = make_simple_type_desc(vec![Field::new(
            "data",
            FieldType::primitive(FIELD_TYPE_STRING),
        )]);
        let input = serde_json::json!({"data": "hello"});
        assert_eq!(roundtrip(&desc, &input)["data"], "hello");
    }

    #[test]
    fn test_encode_bool() {
        let desc = make_simple_type_desc(vec![Field::new(
            "flag",
            FieldType::primitive(FIELD_TYPE_BOOLEAN),
        )]);
        let input = serde_json::json!({"flag": true});
        assert_eq!(roundtrip(&desc, &input)["flag"], true);
    }

    #[test]
    fn test_encode_float64() {
        let desc = make_simple_type_desc(vec![Field::new(
            "val",
            FieldType::primitive(FIELD_TYPE_DOUBLE),
        )]);
        let input = serde_json::json!({"val": std::f64::consts::PI});
        let result = roundtrip(&desc, &input);
        let diff = (result["val"].as_f64().unwrap() - std::f64::consts::PI).abs();
        assert!(diff < 1e-10);
    }

    #[test]
    fn test_encode_int8_array() {
        let desc = make_simple_type_desc(vec![Field::new(
            "data",
            FieldType::array(FIELD_TYPE_INT8, 3),
        )]);
        let input = serde_json::json!({"data": [1, 2, 3]});
        let result = roundtrip(&desc, &input);
        assert_eq!(result["data"], serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn test_encode_unbounded_sequence() {
        let desc = make_simple_type_desc(vec![Field::new(
            "items",
            FieldType::sequence(FIELD_TYPE_UINT32),
        )]);
        let input = serde_json::json!({"items": [10, 20, 30]});
        let result = roundtrip(&desc, &input);
        assert_eq!(result["items"], serde_json::json!([10, 20, 30]));
    }

    #[test]
    fn test_encode_nested_type() {
        let inner_desc = IndividualTypeDescription {
            type_name: "test/msg/Inner".to_string(),
            fields: vec![
                Field::new("x", FieldType::primitive(FIELD_TYPE_FLOAT)),
                Field::new("y", FieldType::primitive(FIELD_TYPE_FLOAT)),
            ],
        };
        let desc = TypeDescriptionMsg {
            type_description: IndividualTypeDescription {
                type_name: "test/msg/Outer".to_string(),
                fields: vec![Field::new("point", FieldType::nested("test/msg/Inner"))],
            },
            referenced_type_descriptions: vec![inner_desc],
        };
        let input = serde_json::json!({"point": {"x": 1.0, "y": 2.0}});
        let result = roundtrip(&desc, &input);
        let diff_x = (result["point"]["x"].as_f64().unwrap() - 1.0).abs();
        let diff_y = (result["point"]["y"].as_f64().unwrap() - 2.0).abs();
        assert!(diff_x < 1e-6);
        assert!(diff_y < 1e-6);
    }

    #[test]
    fn test_encode_multi_field_alignment() {
        let desc = make_simple_type_desc(vec![
            Field::new("a", FieldType::primitive(FIELD_TYPE_BOOLEAN)),
            Field::new("b", FieldType::primitive(FIELD_TYPE_UINT32)),
            Field::new("c", FieldType::primitive(FIELD_TYPE_STRING)),
        ]);
        let input = serde_json::json!({"a": true, "b": 99, "c": "test"});
        let result = roundtrip(&desc, &input);
        assert_eq!(result["a"], true);
        assert_eq!(result["b"], 99);
        assert_eq!(result["c"], "test");
    }

    #[test]
    fn test_encode_missing_fields_default() {
        let desc = make_simple_type_desc(vec![
            Field::new("x", FieldType::primitive(FIELD_TYPE_INT32)),
            Field::new("y", FieldType::primitive(FIELD_TYPE_STRING)),
        ]);
        // Only provide x, y should default to empty string
        let input = serde_json::json!({"x": 5});
        let result = roundtrip(&desc, &input);
        assert_eq!(result["x"], 5);
        assert_eq!(result["y"], "");
    }
}
