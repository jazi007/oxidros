//! Type resolution with service call fallback.
//!
//! Resolves a DDS type name to a `TypeDescriptionMsg` by:
//! 1. Checking the build-time `phf` registry (O(1), no I/O)
//! 2. Falling back to a Zenoh `get_type_description` service call on a discovered node

use crate::type_registry;
use oxidros_zenoh::{Context, EntityKind, GraphCache};
use ros2_types::types::{Field, FieldType, IndividualTypeDescription, TypeDescriptionMsg};
use std::time::Duration;
use zenoh::bytes::ZBytes;
use zenoh::query::QueryTarget;

/// Resolve a DDS type name to its `TypeDescriptionMsg`.
///
/// Tries the build-time registry first (instant). If not found, discovers a node
/// that advertises the given type and calls its `get_type_description` service.
///
/// # Arguments
/// * `dds_name` — DDS type name, e.g. `"std_msgs::msg::dds_::String_"`
/// * `type_hash` — RIHS01 hash string for the type
/// * `ctx` — Zenoh context (needed for the service call fallback)
/// * `graph` — Graph cache with discovered entities
pub async fn resolve(
    dds_name: &str,
    type_hash: &str,
    ctx: &Context,
    graph: &GraphCache,
) -> Option<TypeDescriptionMsg> {
    // 1. Try build-time registry
    if let Some(desc) = type_registry::lookup_dds(dds_name) {
        return Some(desc.clone());
    }

    // 2. Fall back to service call
    tracing::debug!("Type {dds_name} not in build-time registry, trying service call");
    query_type_description(dds_name, type_hash, ctx, graph).await
}

/// Convert a DDS type name to the ROS2 fully-qualified type name.
/// `"my_pkg::msg::dds_::MyType_"` → `"my_pkg/msg/MyType"`
pub(crate) fn dds_to_ros_type_name(dds_name: &str) -> Option<String> {
    // Format: "pkg::interface_type::dds_::TypeName_"
    let parts: Vec<&str> = dds_name.split("::").collect();
    if parts.len() < 4 || parts[2] != "dds_" {
        return None;
    }
    let pkg = parts[0];
    let iface = parts[1]; // "msg", "srv", "action"
    let type_name = parts[3].strip_suffix('_').unwrap_or(parts[3]);
    Some(format!("{}/{}/{}", pkg, iface, type_name))
}

/// Query a node's `get_type_description` service via raw Zenoh query.
async fn query_type_description(
    dds_name: &str,
    type_hash: &str,
    ctx: &Context,
    graph: &GraphCache,
) -> Option<TypeDescriptionMsg> {
    let ros_type_name = dds_to_ros_type_name(dds_name)?;

    // Find a node that publishes/subscribes this type — we'll call its
    // get_type_description service. Any node will do since they all expose it.
    // First try to find any service server for get_type_description
    let entities = graph.get_all_entities();
    let svc_entity = entities.iter().find(|e| {
        e.kind == EntityKind::ServiceServer
            && e.topic_name
                .as_deref()
                .is_some_and(|t| t.ends_with("/get_type_description"))
    })?;

    let svc_name = svc_entity.topic_name.as_deref()?;
    let svc_type = svc_entity.type_name.as_deref()?;
    let svc_hash = svc_entity.type_hash.as_deref()?;

    // Build key expression: <domain_id>/<service_name>/<srv_type>/<srv_hash>
    let svc_name_stripped = svc_name.strip_prefix('/').unwrap_or(svc_name);
    let key_expr = format!(
        "{}/{}/{}/{}",
        ctx.domain_id(),
        svc_name_stripped,
        svc_type,
        svc_hash,
    );

    // CDR-encode the request: type_name (string), type_hash (string), include_type_sources (bool)
    let request_payload = encode_get_type_description_request(&ros_type_name, type_hash);

    // Build attachment (33 bytes): seq=0, current timestamp, random GID
    let attachment_bytes = build_attachment();

    let replies = ctx
        .session()
        .get(&key_expr)
        .payload(ZBytes::from(request_payload))
        .attachment(ZBytes::from(attachment_bytes.to_vec()))
        .target(QueryTarget::All)
        .timeout(Duration::from_secs(2))
        .await
        .ok()?;

    // Wait for the first reply
    let reply = replies.recv_async().await.ok()?;
    let sample = reply.result().ok()?;
    let response_bytes = sample.payload().to_bytes();

    decode_get_type_description_response(&response_bytes)
}

// ============================================================================
// CDR encoding for GetTypeDescription_Request
// ============================================================================

/// CDR-encode a GetTypeDescription request.
///
/// Wire format (CDR little-endian):
/// - 4 bytes: CDR header (0x00, 0x01, 0x00, 0x00)
/// - u32 + bytes + null: type_name string
/// - u32 + bytes + null: type_hash string
/// - u8: include_type_sources (0 = false)
fn encode_get_type_description_request(type_name: &str, type_hash: &str) -> Vec<u8> {
    let mut buf = Vec::new();

    // CDR LE header
    buf.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);

    // type_name (CDR string = u32 len including null + data + null)
    write_cdr_string(&mut buf, type_name);

    // type_hash
    write_cdr_string(&mut buf, type_hash);

    // include_type_sources = false (we only need the type description)
    buf.push(0u8);

    buf
}

fn write_cdr_string(buf: &mut Vec<u8>, s: &str) {
    let len = (s.len() + 1) as u32; // +1 for null terminator
    // Align to 4 bytes
    while !(buf.len() - 4).is_multiple_of(4) {
        // offset from start of payload (after 4-byte CDR header)
        buf.push(0);
    }
    buf.extend_from_slice(&len.to_le_bytes());
    buf.extend_from_slice(s.as_bytes());
    buf.push(0); // null terminator
}

// ============================================================================
// CDR decoding for GetTypeDescription_Response
// ============================================================================

/// Decode a GetTypeDescription_Response CDR payload → TypeDescriptionMsg.
///
/// Response layout (CDR little-endian after 4-byte header):
///   bool successful           (u8)
///   padding to 4              (3 bytes)
///   string failure_reason
///   TypeDescription type_description
///   TypeSource[] type_sources      (ignored)
///   KeyValue[] extra_information   (ignored)
///
/// We manually walk the CDR bytes to extract `type_description`.
fn decode_get_type_description_response(cdr_bytes: &[u8]) -> Option<TypeDescriptionMsg> {
    if cdr_bytes.len() < 5 {
        return None;
    }

    let mut reader = CdrReader::new(cdr_bytes);

    // Skip CDR header (4 bytes) — endianness from rep_id
    reader.skip(4);

    // successful: bool (u8)
    let successful = reader.read_u8()? != 0;
    if !successful {
        return None;
    }

    // failure_reason: string (skip it)
    reader.align(4);
    reader.skip_string()?;

    // type_description: TypeDescription
    //   IndividualTypeDescription type_description
    //   IndividualTypeDescription[] referenced_type_descriptions
    let main_desc = reader.read_individual_type_description()?;

    // referenced_type_descriptions: sequence<IndividualTypeDescription>
    reader.align(4);
    let ref_count = reader.read_u32()? as usize;
    let mut referenced = Vec::with_capacity(ref_count);
    for _ in 0..ref_count {
        referenced.push(reader.read_individual_type_description()?);
    }

    Some(TypeDescriptionMsg {
        type_description: main_desc,
        referenced_type_descriptions: referenced,
    })
}

/// Minimal CDR little-endian reader for parsing the response.
struct CdrReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> CdrReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn skip(&mut self, n: usize) {
        self.pos += n;
    }

    fn align(&mut self, n: usize) {
        // Align relative to start of CDR payload (after 4-byte header)
        let payload_pos = self.pos.saturating_sub(4);
        let rem = payload_pos % n;
        if rem != 0 {
            self.pos += n - rem;
        }
    }

    fn read_u8(&mut self) -> Option<u8> {
        if self.remaining() < 1 {
            return None;
        }
        let v = self.data[self.pos];
        self.pos += 1;
        Some(v)
    }

    fn read_u32(&mut self) -> Option<u32> {
        self.align(4);
        if self.remaining() < 4 {
            return None;
        }
        let v = u32::from_le_bytes(self.data[self.pos..self.pos + 4].try_into().ok()?);
        self.pos += 4;
        Some(v)
    }

    fn read_u64(&mut self) -> Option<u64> {
        self.align(8);
        if self.remaining() < 8 {
            return None;
        }
        let v = u64::from_le_bytes(self.data[self.pos..self.pos + 8].try_into().ok()?);
        self.pos += 8;
        Some(v)
    }

    fn read_string(&mut self) -> Option<String> {
        let len = self.read_u32()? as usize;
        if len == 0 || self.remaining() < len {
            return None;
        }
        // len includes null terminator
        let s = std::str::from_utf8(&self.data[self.pos..self.pos + len - 1]).ok()?;
        self.pos += len;
        Some(s.to_string())
    }

    fn skip_string(&mut self) -> Option<()> {
        let len = self.read_u32()? as usize;
        if self.remaining() < len {
            return None;
        }
        self.pos += len;
        Some(())
    }

    fn read_individual_type_description(&mut self) -> Option<IndividualTypeDescription> {
        // type_name: string<=255
        let type_name = self.read_string()?;

        // fields: sequence<Field>
        self.align(4);
        let field_count = self.read_u32()? as usize;
        let mut fields = Vec::with_capacity(field_count);
        for _ in 0..field_count {
            fields.push(self.read_field()?);
        }

        Some(IndividualTypeDescription { type_name, fields })
    }

    fn read_field(&mut self) -> Option<Field> {
        // name: string
        let name = self.read_string()?;
        // type: FieldType
        let field_type = self.read_field_type()?;
        // default_value: string
        let default_value = self.read_string()?;

        Some(Field {
            name,
            field_type,
            default_value,
        })
    }

    fn read_field_type(&mut self) -> Option<FieldType> {
        // type_id: uint8
        let type_id = self.read_u8()?;
        // capacity: uint64
        let capacity = self.read_u64()?;
        // string_capacity: uint64
        let string_capacity = self.read_u64()?;
        // nested_type_name: string<=255
        let nested_type_name = self.read_string()?;

        Some(FieldType {
            type_id,
            capacity,
            string_capacity,
            nested_type_name,
        })
    }
}

// ============================================================================
// Attachment builder
// ============================================================================

/// Build a 33-byte service call attachment.
pub(crate) fn build_attachment() -> [u8; 33] {
    use std::time::{SystemTime, UNIX_EPOCH};

    let mut bytes = [0u8; 33];
    // seq = 0 (first call)
    bytes[0..8].copy_from_slice(&0i64.to_le_bytes());
    // timestamp
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0);
    bytes[8..16].copy_from_slice(&ts.to_le_bytes());
    // GID length
    bytes[16] = 16;
    // Random GID
    let gid: [u8; 16] = std::array::from_fn(|_| rand_byte());
    bytes[17..33].copy_from_slice(&gid);
    bytes
}

/// Simple pseudo-random byte using thread-local state.
fn rand_byte() -> u8 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;

    thread_local! {
        static COUNTER: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    }

    COUNTER.with(|c| {
        let val = c.get().wrapping_add(1);
        c.set(val);
        let mut hasher = DefaultHasher::new();
        val.hash(&mut hasher);
        SystemTime::now().hash(&mut hasher);
        std::thread::current().id().hash(&mut hasher);
        hasher.finish() as u8
    })
}
