//! Dynamic CDR-to-JSON decoder.
//!
//! Reads raw CDR bytes using a [`TypeDescriptionMsg`] to interpret the binary layout.

use crate::error::{DynamicError, Result};
use byteorder::{BigEndian, ByteOrder, LittleEndian};
use ros2_types::types::*;
use serde_json::{Map, Value};

/// Decode a CDR-encoded message into a JSON value.
///
/// The `data` must include the 4-byte CDR encapsulation header.
/// The `type_desc` provides the structural schema for decoding.
pub fn decode_cdr(data: &[u8], type_desc: &TypeDescriptionMsg) -> Result<Value> {
    if data.len() < 4 {
        return Err(DynamicError::BufferTooShort {
            offset: 0,
            need: 4,
            have: data.len(),
        });
    }

    let rep_id = u16::from_be_bytes([data[0], data[1]]);
    let little_endian = match rep_id {
        0x0001 => true,  // CDR_LE
        0x0000 => false, // CDR_BE
        other => return Err(DynamicError::UnsupportedRepresentation(other)),
    };

    let mut reader = CdrReader {
        buf: &data[4..],
        pos: 0,
        little_endian,
        referenced: &type_desc.referenced_type_descriptions,
    };

    reader.read_struct(&type_desc.type_description)
}

/// Stateful CDR reader that tracks position and alignment.
struct CdrReader<'a> {
    buf: &'a [u8],
    pos: usize,
    little_endian: bool,
    referenced: &'a [IndividualTypeDescription],
}

impl<'a> CdrReader<'a> {
    /// Align the current position to the given boundary.
    fn align(&mut self, alignment: usize) {
        let rem = self.pos % alignment;
        if rem != 0 {
            self.pos += alignment - rem;
        }
    }

    /// Check that `n` bytes are available at the current position.
    fn ensure(&self, n: usize) -> Result<()> {
        if self.pos + n > self.buf.len() {
            Err(DynamicError::BufferTooShort {
                offset: self.pos,
                need: n,
                have: self.buf.len(),
            })
        } else {
            Ok(())
        }
    }

    /// Read `n` raw bytes and advance position.
    fn read_bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        self.ensure(n)?;
        let slice = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8> {
        let b = self.read_bytes(1)?;
        Ok(b[0])
    }

    fn read_i8(&mut self) -> Result<i8> {
        Ok(self.read_u8()? as i8)
    }

    fn read_bool(&mut self) -> Result<bool> {
        Ok(self.read_u8()? != 0)
    }

    fn read_u16(&mut self) -> Result<u16> {
        self.align(2);
        let b = self.read_bytes(2)?;
        Ok(if self.little_endian {
            LittleEndian::read_u16(b)
        } else {
            BigEndian::read_u16(b)
        })
    }

    fn read_i16(&mut self) -> Result<i16> {
        Ok(self.read_u16()? as i16)
    }

    fn read_u32(&mut self) -> Result<u32> {
        self.align(4);
        let b = self.read_bytes(4)?;
        Ok(if self.little_endian {
            LittleEndian::read_u32(b)
        } else {
            BigEndian::read_u32(b)
        })
    }

    fn read_i32(&mut self) -> Result<i32> {
        Ok(self.read_u32()? as i32)
    }

    fn read_u64(&mut self) -> Result<u64> {
        self.align(8);
        let b = self.read_bytes(8)?;
        Ok(if self.little_endian {
            LittleEndian::read_u64(b)
        } else {
            BigEndian::read_u64(b)
        })
    }

    fn read_i64(&mut self) -> Result<i64> {
        Ok(self.read_u64()? as i64)
    }

    fn read_f32(&mut self) -> Result<f32> {
        self.align(4);
        let b = self.read_bytes(4)?;
        Ok(if self.little_endian {
            LittleEndian::read_f32(b)
        } else {
            BigEndian::read_f32(b)
        })
    }

    fn read_f64(&mut self) -> Result<f64> {
        self.align(8);
        let b = self.read_bytes(8)?;
        Ok(if self.little_endian {
            LittleEndian::read_f64(b)
        } else {
            BigEndian::read_f64(b)
        })
    }

    /// Read a CDR string: u32 length (including null terminator) + bytes + null.
    fn read_string(&mut self) -> Result<String> {
        let len = self.read_u32()? as usize;
        if len == 0 {
            return Ok(String::new());
        }
        let bytes = self.read_bytes(len)?;
        // Strip null terminator if present
        let str_bytes = if bytes.last() == Some(&0) {
            &bytes[..bytes.len() - 1]
        } else {
            bytes
        };
        Ok(String::from_utf8(str_bytes.to_vec())?)
    }

    /// Read a struct from the type description.
    fn read_struct(&mut self, desc: &IndividualTypeDescription) -> Result<Value> {
        let mut map = Map::new();
        for field in &desc.fields {
            let value = self.read_field_type(&field.field_type)?;
            map.insert(field.name.clone(), value);
        }
        Ok(Value::Object(map))
    }

    /// Read a single field value based on its FieldType.
    fn read_field_type(&mut self, ft: &FieldType) -> Result<Value> {
        let (base_id, kind) = decompose_type_id(ft.type_id);

        match kind {
            'p' => self.read_primitive_or_nested(base_id, ft),
            'a' => {
                // Fixed-size array: capacity elements, no length prefix
                let count = ft.capacity as usize;
                self.read_array_elements(base_id, ft, count)
            }
            'b' | 'u' => {
                // Bounded or unbounded sequence: u32 count prefix
                let count = self.read_u32()? as usize;
                self.read_array_elements(base_id, ft, count)
            }
            _ => Err(DynamicError::UnknownFieldType(ft.type_id)),
        }
    }

    /// Read a scalar primitive or nested type.
    fn read_primitive_or_nested(&mut self, base_id: u8, ft: &FieldType) -> Result<Value> {
        match base_id {
            FIELD_TYPE_BOOLEAN => Ok(Value::Bool(self.read_bool()?)),
            FIELD_TYPE_BYTE | FIELD_TYPE_UINT8 | FIELD_TYPE_CHAR => {
                Ok(Value::Number(self.read_u8()?.into()))
            }
            FIELD_TYPE_INT8 => Ok(Value::Number(self.read_i8()?.into())),
            FIELD_TYPE_UINT16 | FIELD_TYPE_WCHAR => Ok(Value::Number(self.read_u16()?.into())),
            FIELD_TYPE_INT16 => Ok(Value::Number(self.read_i16()?.into())),
            FIELD_TYPE_UINT32 => Ok(Value::Number(self.read_u32()?.into())),
            FIELD_TYPE_INT32 => Ok(Value::Number(self.read_i32()?.into())),
            FIELD_TYPE_UINT64 => Ok(Value::Number(self.read_u64()?.into())),
            FIELD_TYPE_INT64 => Ok(Value::Number(self.read_i64()?.into())),
            FIELD_TYPE_FLOAT => {
                let v = self.read_f32()?;
                Ok(serde_json::Number::from_f64(v as f64)
                    .map(Value::Number)
                    .unwrap_or(Value::Null))
            }
            FIELD_TYPE_DOUBLE | FIELD_TYPE_LONG_DOUBLE => {
                let v = self.read_f64()?;
                Ok(serde_json::Number::from_f64(v)
                    .map(Value::Number)
                    .unwrap_or(Value::Null))
            }
            FIELD_TYPE_STRING
            | FIELD_TYPE_BOUNDED_STRING
            | FIELD_TYPE_FIXED_STRING
            | FIELD_TYPE_WSTRING
            | FIELD_TYPE_BOUNDED_WSTRING
            | FIELD_TYPE_FIXED_WSTRING => Ok(Value::String(self.read_string()?)),
            FIELD_TYPE_NESTED_TYPE => {
                let nested_desc = self.find_referenced_type(&ft.nested_type_name)?;
                self.read_struct(nested_desc)
            }
            _ => Err(DynamicError::UnknownFieldType(base_id)),
        }
    }

    /// Read `count` elements of the given base type into a JSON array.
    fn read_array_elements(&mut self, base_id: u8, ft: &FieldType, count: usize) -> Result<Value> {
        let mut arr = Vec::with_capacity(count);
        for _ in 0..count {
            arr.push(self.read_primitive_or_nested(base_id, ft)?);
        }
        Ok(Value::Array(arr))
    }

    /// Look up a referenced type by its fully qualified name.
    fn find_referenced_type(&self, type_name: &str) -> Result<&'a IndividualTypeDescription> {
        self.referenced
            .iter()
            .find(|t| t.type_name == type_name)
            .ok_or_else(|| DynamicError::ReferencedTypeNotFound(type_name.to_string()))
    }
}

/// Extract base type_id and kind from a composite type_id.
/// Returns (base_type_id, kind) where kind is:
/// - 'p' for plain/scalar (0–48)
/// - 'a' for fixed array (49–96)
/// - 'b' for bounded sequence (97–144)
/// - 'u' for unbounded sequence (145–192)
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

    /// Build a CDR LE header.
    fn cdr_le_header() -> Vec<u8> {
        vec![0x00, 0x01, 0x00, 0x00]
    }

    fn make_simple_type_desc(fields: Vec<Field>) -> TypeDescriptionMsg {
        TypeDescriptionMsg {
            type_description: IndividualTypeDescription {
                type_name: "test/msg/Test".to_string(),
                fields,
            },
            referenced_type_descriptions: vec![],
        }
    }

    #[test]
    fn test_decode_uint32() {
        let mut data = cdr_le_header();
        data.extend_from_slice(&42u32.to_le_bytes());

        let desc = make_simple_type_desc(vec![Field::new(
            "value",
            FieldType::primitive(FIELD_TYPE_UINT32),
        )]);

        let result = decode_cdr(&data, &desc).unwrap();
        assert_eq!(result["value"], 42);
    }

    #[test]
    fn test_decode_string() {
        let mut data = cdr_le_header();
        // String "hello" = length 6 (5 chars + null terminator)
        data.extend_from_slice(&6u32.to_le_bytes());
        data.extend_from_slice(b"hello\0");

        let desc = make_simple_type_desc(vec![Field::new(
            "text",
            FieldType::primitive(FIELD_TYPE_STRING),
        )]);

        let result = decode_cdr(&data, &desc).unwrap();
        assert_eq!(result["text"], "hello");
    }

    #[test]
    fn test_decode_bool_and_float() {
        let mut data = cdr_le_header();
        data.push(1); // bool = true
        // Align to 4 for f32
        data.extend_from_slice(&[0, 0, 0]); // 3 bytes padding
        data.extend_from_slice(&std::f32::consts::PI.to_le_bytes());

        let desc = make_simple_type_desc(vec![
            Field::new("flag", FieldType::primitive(FIELD_TYPE_BOOLEAN)),
            Field::new("value", FieldType::primitive(FIELD_TYPE_FLOAT)),
        ]);

        let result = decode_cdr(&data, &desc).unwrap();
        assert_eq!(result["flag"], true);
        let v = result["value"].as_f64().unwrap();
        assert!((v - std::f64::consts::PI).abs() < 0.001);
    }

    #[test]
    fn test_decode_fixed_array() {
        let mut data = cdr_le_header();
        // 3 x u32
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&2u32.to_le_bytes());
        data.extend_from_slice(&3u32.to_le_bytes());

        let desc = make_simple_type_desc(vec![Field::new(
            "values",
            FieldType::array(FIELD_TYPE_UINT32, 3),
        )]);

        let result = decode_cdr(&data, &desc).unwrap();
        assert_eq!(result["values"], serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn test_decode_unbounded_sequence() {
        let mut data = cdr_le_header();
        // sequence<u16>: count=2, then 2 x u16
        data.extend_from_slice(&2u32.to_le_bytes());
        data.extend_from_slice(&10u16.to_le_bytes());
        data.extend_from_slice(&20u16.to_le_bytes());

        let desc = make_simple_type_desc(vec![Field::new(
            "items",
            FieldType::sequence(FIELD_TYPE_UINT16),
        )]);

        let result = decode_cdr(&data, &desc).unwrap();
        assert_eq!(result["items"], serde_json::json!([10, 20]));
    }

    #[test]
    fn test_decode_nested_type() {
        // Main type has a field "header" of type "std_msgs/msg/Header"
        // Header has fields: stamp (nested), frame_id (string)
        // We'll simplify: Header = { frame_id: string }
        let mut data = cdr_le_header();
        // frame_id = "map" → length 4 (3 chars + null)
        data.extend_from_slice(&4u32.to_le_bytes());
        data.extend_from_slice(b"map\0");

        let header_desc = IndividualTypeDescription {
            type_name: "std_msgs/msg/Header".to_string(),
            fields: vec![Field::new(
                "frame_id",
                FieldType::primitive(FIELD_TYPE_STRING),
            )],
        };

        let desc = TypeDescriptionMsg {
            type_description: IndividualTypeDescription {
                type_name: "test/msg/Test".to_string(),
                fields: vec![Field::new(
                    "header",
                    FieldType::nested("std_msgs/msg/Header"),
                )],
            },
            referenced_type_descriptions: vec![header_desc],
        };

        let result = decode_cdr(&data, &desc).unwrap();
        assert_eq!(result["header"]["frame_id"], "map");
    }

    #[test]
    fn test_decode_big_endian() {
        let mut data = vec![0x00, 0x00, 0x00, 0x00]; // CDR BE header
        data.extend_from_slice(&42u32.to_be_bytes());

        let desc = make_simple_type_desc(vec![Field::new(
            "value",
            FieldType::primitive(FIELD_TYPE_UINT32),
        )]);

        let result = decode_cdr(&data, &desc).unwrap();
        assert_eq!(result["value"], 42);
    }

    #[test]
    fn test_decode_alignment_across_fields() {
        // u8 field + u64 field → must pad to 8-byte alignment after the u8
        let mut data = cdr_le_header();
        data.push(0xFF); // u8 = 255
        data.extend_from_slice(&[0; 7]); // 7 bytes padding to align to 8
        data.extend_from_slice(&123456789u64.to_le_bytes());

        let desc = make_simple_type_desc(vec![
            Field::new("small", FieldType::primitive(FIELD_TYPE_UINT8)),
            Field::new("big", FieldType::primitive(FIELD_TYPE_UINT64)),
        ]);

        let result = decode_cdr(&data, &desc).unwrap();
        assert_eq!(result["small"], 255);
        assert_eq!(result["big"], 123456789u64);
    }

    #[test]
    fn test_decode_multiple_strings() {
        let mut data = cdr_le_header();
        // First string: "ab" → len=3 (2 + null)
        data.extend_from_slice(&3u32.to_le_bytes());
        data.extend_from_slice(b"ab\0");
        // Padding to align u32 for next string length: pos after first string =
        // 4(len) + 3(bytes) = 7, need align to 4 → pad 1
        data.push(0);
        // Second string: "cd" → len=3
        data.extend_from_slice(&3u32.to_le_bytes());
        data.extend_from_slice(b"cd\0");

        let desc = make_simple_type_desc(vec![
            Field::new("a", FieldType::primitive(FIELD_TYPE_STRING)),
            Field::new("b", FieldType::primitive(FIELD_TYPE_STRING)),
        ]);

        let result = decode_cdr(&data, &desc).unwrap();
        assert_eq!(result["a"], "ab");
        assert_eq!(result["b"], "cd");
    }

    #[test]
    fn test_buffer_too_short() {
        let data = vec![0x00, 0x01, 0x00, 0x00]; // header only, no payload

        let desc = make_simple_type_desc(vec![Field::new(
            "value",
            FieldType::primitive(FIELD_TYPE_UINT32),
        )]);

        let result = decode_cdr(&data, &desc);
        assert!(result.is_err());
    }
}
