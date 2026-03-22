# Zenoh-Based ROS2 CLI Tool — Implementation Plan

> **Goal**: Build a standalone `ros2`-compatible CLI that operates purely over Zenoh,
> requiring no ROS2 installation at runtime. Supports introspection, echo, services,
> parameters, and MCAP bag record/play.

---

## Codebase Context

### Existing crates and what they provide

| Crate | Role | Key items for CLI |
|---|---|---|
| `oxidros-zenoh` | Pure-Rust ROS2 middleware over Zenoh | `Context`, `Node`, `Publisher`, `Subscriber`, `Client`, `Server`, `GraphCache` |
| `oxidros-core` | Shared traits (`RosPublisher`, `RosSubscriber`, etc.) | `api::RosPublisher::send_raw()`, `Message<T>`, `MessageInfo` |
| `ros2-types` | Type traits + CDR + type hash | `TypeSupport`, `TypeDescription`, `TypeDescriptionMsg`, `CdrSerde` |
| `ros2-types-derive` | Derive macros | `#[derive(TypeDescription, Ros2Msg)]` |
| `ros2msg` | `.msg`/`.idl` parser + IDL adapter | `idl_adapter::message_to_idl()`, `MessageSpecification` |
| `oxidros-msg` | Pre-generated ROS2 message types | `std_msgs`, `geometry_msgs`, `rcl_interfaces`, etc. |
| `ros2args` | `--ros-args` parser | `Ros2Args`, name expansion, param files |

### Discovery mechanism (oxidros-zenoh)

- **Liveliness tokens** on pattern `@ros2_lv/<domain_id>/**`
- Parsed by `GraphCache` in `context.rs` via a Zenoh liveliness subscriber
- Token format: `@ros2_lv/<domain>/<session_id>/<node_id>/<entity_id>/<kind>/<enclave>/<ns>/<name>[/<topic>/<type>/<hash>/<qos>]`
- Entity kinds: `NN` (Node), `MP` (Publisher), `MS` (Subscriber), `SS` (ServiceServer), `SC` (ServiceClient)

### EntityInfo (graph_cache.rs)

```rust
pub struct EntityInfo {
    pub domain_id: u32,
    pub session_id: String,
    pub node_id: u32,
    pub entity_id: u32,
    pub kind: EntityKind,
    pub enclave: String,
    pub namespace: String,
    pub node_name: String,
    pub topic_name: Option<String>,
    pub type_name: Option<String>,
    pub type_hash: Option<String>,
}
```

`GraphCache` methods: `get_node_names()`, `count_publishers(topic)`, `count_subscribers(topic)`,
`get_publishers_info(topic)`, `get_subscribers_info(topic)`, `is_service_available(name)`.

### Zenoh key expression scheme (keyexpr.rs)

- **Topics/services**: `<domain_id>/<fq_name>/<type_name>/<type_hash>`
- Name mangling: `/` → `%` (via `mangle_name` / `unmangle_name`)
- QoS encoded in liveliness token as compact string

### Message serialization

- CDR encoding via `CdrSerde` trait (4-byte encapsulation header + serde-based payload)
- `TypeSupport::to_bytes()` / `from_bytes()` for typed ser/de
- `RosPublisher::send_raw(&[u8])` exists — adds CDR header to raw bytes
- **`RosSubscriber` has NO `recv_raw()` — always deserializes into `T`** ← must fix

### TypeDescription trait (type_description.rs)

```rust
pub trait TypeDescription {
    fn type_description() -> TypeDescriptionMsg;
    fn message_type_name() -> MessageTypeName;
    fn compute_hash() -> Result<String>;   // default impl
}
```

Also: `ServiceTypeDescription`, `ActionTypeDescription` (same pattern).

### TypeDescriptionMsg (types.rs)

```rust
pub struct TypeDescriptionMsg {
    pub type_description: IndividualTypeDescription,
    pub referenced_type_descriptions: Vec<IndividualTypeDescription>,
}
pub struct IndividualTypeDescription {
    pub type_name: String,        // e.g. "std_msgs/msg/Header"
    pub fields: Vec<Field>,
}
pub struct Field {
    pub name: String,
    pub field_type: FieldType,
    pub default_value: String,
}
pub struct FieldType {
    pub type_id: u8,              // constant from FIELD_TYPE_*
    pub capacity: u64,            // array/seq bound (0 if N/A)
    pub string_capacity: u64,     // bounded string max (0 if N/A)
    pub nested_type_name: String, // fqn for nested types
}
```

Type ID ranges: 1–22 primitives, 49–66 fixed arrays, 97–114 bounded sequences, 145–162 unbounded sequences.

### Existing IDL generation (ros2msg/src/idl_adapter.rs)

- `message_to_idl(spec, pkg) -> String` — converts `MessageSpecification` → OMG IDL
- Handles typedefs for fixed arrays, `sequence<T, N>`, module scoping
- Maps: `bool→boolean`, `byte→octet`, `float32→float`, `float64→double`, etc.
- **Only works from `MessageSpecification` (parsed .msg), NOT from `TypeDescriptionMsg`**

### What MCAP needs

- Per-channel **schema**: IDL or `.msg` text + encoding string (`"ros2idl"` or `"ros2msg"`)
- Per-message: raw CDR bytes + timestamp + channel ID
- Rust crate: [`mcap`](https://crates.io/crates/mcap)

---

## Dependency Graph

```
Phase 0.1 (recv_raw) ──────┬──→ Phase 4.1 (echo)
                            ├──→ Phase 4.2 (hz)
                            ├──→ Phase 4.3 (bw)
                            └──→ Phase 7.2 (bag record)

Phase 0.2 (to_idl)  ───────┬──→ Phase 7.2 (bag record — MCAP schema)
Phase 0.3 (to_msg_def) ────┘

Phase 3.1 (CDR→JSON decoder)──┬──→ Phase 4.1 (echo)
                               ├──→ Phase 5.1 (service call)
                               └──→ Phase 9   (topic pub)

Phase 3.2 (runtime TypeDesc) ─┬──→ Phase 3.1
                               ├──→ Phase 4.1
                               └──→ Phase 7.2

Phase 1.x (crate + clap) ─────→ all commands

Phase 2.x (graph commands)     (independent — ship first)
Phase 6.x (param commands)     (typed clients — ship early)
```

---

## Implementation Steps

### Phase 0 — Foundations

#### Step 0.1: Add `recv_raw()` to subscriber path
- [x] Add `fn recv_raw(&mut self) -> Result<(Vec<u8>, MessageInfo)>` to `RosSubscriber` trait in `oxidros-core/src/api/mod.rs`
- [x] Implement in `oxidros-zenoh/src/topic/subscriber.rs`: return `sample.payload().to_bytes()` + parsed `Attachment` without deserializing via `T::from_bytes()`
- [x] Also added `fn try_recv_raw()` (sync variant) — async `recv_raw()` is the main method
- [x] Implemented in `oxidros-wrapper/src/lib.rs` (delegates to inner subscriber)
- **Files**: `oxidros-core/src/api/mod.rs`, `oxidros-zenoh/src/topic/subscriber.rs`, `oxidros-wrapper/src/lib.rs`
- **Why**: Every dynamic command (echo, hz, bw, record) needs raw CDR bytes

#### Step 0.2: Add `TypeDescriptionMsg::to_idl()` method
- [x] Implement `fn to_idl(&self) -> String` on `TypeDescriptionMsg` in `ros2-types/src/types.rs`
- [x] Map `type_id` → IDL type name via `base_type_to_idl()` (20 match arms)
- [x] Detect arrays (type_id 49–66) → emit `typedef T T__N[N];`
- [x] Detect bounded sequences (97–114) → `sequence<T, N>`
- [x] Detect unbounded sequences (145–162) → `sequence<T>`
- [x] Handle nested types via `nested_type_name` → `pkg::msg::TypeName` IDL qualified form
- [x] Wrap in IDL module scoping: `module pkg { module msg { struct TypeName { ... }; }; };`
- [x] Inline all referenced types in one IDL blob (referenced first, then main type)
- [x] Helper: `decompose_type_id()` extracts base + kind (plain/array/bounded/unbounded)
- **Files**: `ros2-types/src/types.rs`
- **Why**: MCAP schema requires IDL text

#### Step 0.3: Add `TypeDescriptionMsg::to_msg_definition()` method
- [x] Implement `fn to_msg_definition(&self) -> String` on `TypeDescriptionMsg`
- [x] Output `.msg` format: `type_name field_name` per line via `base_type_to_msg()`
- [x] Append referenced type definitions separated by `===` lines + `MSG: fqn` header (Foxglove convention)
- [x] Handles arrays `T[N]`, bounded sequences `T[<=N]`, unbounded sequences `T[]`, bounded strings `string<=N`
- **Files**: `ros2-types/src/types.rs`
- **Why**: Alternative schema encoding for MCAP (`"ros2msg"`), simpler, widely supported

---

### Phase 1 — CLI Crate Skeleton

#### Step 1.1: Create `oxidros-cli` crate
- [x] New crate at `oxidros-cli/` with `Cargo.toml` (binary target `ros2`)
- [x] Dependencies: `oxidros-zenoh`, `oxidros-core`, `ros2-types`, `clap` (derive+env), `tokio`, `tracing`, `tracing-subscriber`
- [x] Top-level subcommands via clap: `node`, `topic`, `service` (param/bag deferred to later phases)
- **Files**: `oxidros-cli/Cargo.toml`, `oxidros-cli/src/main.rs`

#### Step 1.2: Bootstrap Zenoh context
- [x] `main()`: parse clap args, init `oxidros_zenoh::Context`, wait ~200ms for graph discovery
- [x] Pass `graph_cache` snapshot to subcommand handlers
- [x] Support `--domain-id` and `ROS_DOMAIN_ID` env var via clap `env` feature
- **Files**: `oxidros-cli/src/main.rs`

---

### Phase 2 — Graph Introspection Commands

#### Step 2.1: `node list`
- [x] Call `graph_cache.get_node_names()` → print sorted `namespace/name` per line
- **Files**: `oxidros-cli/src/node.rs`

#### Step 2.2: `node info <name>`
- [x] Filter all entities matching fqn via `get_all_entities()`
- [x] Group by `EntityKind` → print publishers, subscribers, service servers, service clients
- [x] Show topic name + type for each
- **Files**: `oxidros-cli/src/node.rs`

#### Step 2.3: `topic list` / `topic list -t` (show types)
- [x] `graph.get_topic_names_and_types()` → sorted unique `(topic, type)` pairs
- [x] `-t` flag: print type name next to each topic
- [x] `-v` flag: print publisher/subscriber counts
- **Files**: `oxidros-cli/src/topic.rs`

#### Step 2.4: `topic info <name>`
- [x] `get_publishers_info(topic)` and `get_subscribers_info(topic)`
- [x] Print: type, publisher count, subscriber count, list of nodes for each
- **Files**: `oxidros-cli/src/topic.rs`

#### Step 2.5: `service list`
- [x] `graph.get_service_names_and_types()` with optional `-t` for types
- **Files**: `oxidros-cli/src/service.rs`

**Additional changes for Phase 2:**
- [x] Added `get_topic_names_and_types()`, `get_service_names_and_types()`, `get_all_entities()` to `GraphCache`
- [x] Re-exported `EntityInfo` and `EntityKind` from `oxidros-zenoh`

---

### Phase 3 — Dynamic Message Decoding

#### Step 3.1: CDR-to-JSON dynamic decoder
- [x] New crate: `oxidros-dynamic` with `decode_cdr()` public API
- [x] Input: `&[u8]` (raw CDR with header) + `&TypeDescriptionMsg`
- [x] Output: `serde_json::Value`
- [x] Parse CDR encapsulation header (4 bytes) → determine endianness (big-endian u16 rep ID)
- [x] Walk `fields` in order, read each according to `type_id`:
  - Primitives: read 1/2/4/8 bytes with correct alignment
  - Strings: read u32 length + UTF-8 bytes + null terminator
  - Nested: look up `nested_type_name` in referenced types, recurse
  - Fixed arrays: read N elements (no length prefix)
  - Sequences: read u32 count, then count elements
  - Bounded strings: same as strings (bound is validation only)
- [x] CDR alignment rules: each primitive aligned to its own size (bool=1, u16=2, u32/f32=4, u64/f64=8)
- [x] 10 unit tests: uint32, string, bool+float, fixed array, unbounded sequence, nested type, big endian, alignment, multiple strings, buffer too short
- **Files**: `oxidros-dynamic/Cargo.toml`, `oxidros-dynamic/src/{lib,decoder,error}.rs`
- **Why**: Needed for echo, service call, topic pub — any command displaying message content

#### Step 3.2: Obtain TypeDescriptionMsg at runtime
- [x] **Build-time type registry**: `build.rs` parses all `.msg` files from ROS2 share directories using `ros2msg`, converts `MessageSpecification` → `TypeDescriptionMsg` JSON, and generates a `phf::Map<&'static str, &'static str>` (DDS name → JSON). At runtime, JSON is deserialized into `TypeDescriptionMsg` on first access via `LazyLock<HashMap>` cache.
- [x] **Fallback — Service call**: `type_resolve::resolve()` tries build-time registry first, then discovers a node's `get_type_description` service from the graph cache, sends a raw Zenoh query with CDR-encoded request, and manually decodes the CDR response to extract `TypeDescriptionMsg`. Supports custom/user-defined message types not in the build-time registry.
- **Files**: `oxidros-cli/build.rs`, `oxidros-cli/src/type_registry.rs`, `oxidros-cli/src/type_resolve.rs`
- **Registry**: 205 message types from all ROS2 packages (auto-discovered)

---

### Phase 4 — Echo / Hz / Bw Commands

#### Step 4.1: `topic echo <topic>`
- [x] Query graph → extract `type_name`, `type_hash` from `EntityInfo`
- [x] Create raw Zenoh subscriber on key expression `<domain_id>/<topic>/<type>/<hash>`
- [x] For each raw sample:
  1. Get `TypeDescriptionMsg` (Step 3.2) via `type_resolve::resolve()`
  2. Decode CDR → JSON (Step 3.1) via `oxidros_dynamic::decode_cdr()`
  3. Print YAML-formatted output (matching `ros2 topic echo` style) with `print_yaml()`
- [x] Flags: `--once`, `--json`, `-n/--max-count`
- **Files**: `oxidros-cli/src/topic.rs`
- **Depends on**: Step 0.1, Step 3.1, Step 3.2

#### Step 4.2: `topic hz <topic>`
- [x] Raw subscribe, track receive timestamps with rolling window
- [x] Print rolling average frequency, min/max period, std deviation
- **Files**: `oxidros-cli/src/topic.rs`
- **Depends on**: Step 0.1

#### Step 4.3: `topic bw <topic>`
- [x] Raw subscribe, track payload sizes with rolling window
- [x] Print rolling bandwidth (auto-scaled B/s, KB/s, MB/s), mean/min/max size
- **Files**: `oxidros-cli/src/topic.rs`
- **Depends on**: Step 0.1

---

### Phase 5 — Service Call ✅

#### Step 5.1: `service call <service> <type> <yaml>`
- [x] Parse YAML value input
- [x] Obtain `TypeDescriptionMsg` for the request type
- [x] Encode YAML → CDR bytes using dynamic encoder (reverse of Step 3.1)
- [x] Send as raw Zenoh `get()` query on the service key expression
- [x] Decode response CDR → JSON, print
- **Files**: `oxidros-cli/src/service.rs`, `oxidros-dynamic/src/encoder.rs`
- **Depends on**: Step 3.1, Step 3.2
- **Additional**: Dynamic CDR encoder (`encode_cdr`) added in `oxidros-dynamic` with 9 round-trip tests

---

### Phase 6 — Parameter Commands ✅

#### Step 6.1: `param list <node>`
- [x] Call `<node>/list_parameters` service via raw Zenoh query
- [x] Print parameter names, optional `--prefix` filter
- **Files**: `oxidros-cli/src/param.rs`

#### Step 6.2: `param get <node> <name>`
- [x] Call `<node>/get_parameters` service
- [x] Print parameter value with type (from `ParameterValue` type discriminant)

#### Step 6.3: `param set <node> <name> <value>`
- [x] Call `<node>/set_parameters` service with YAML-parsed value
- [x] Auto-infer parameter type from YAML (bool, integer, double, string, arrays)

#### Step 6.4: `param describe <node> <name>`
- [x] Call `<node>/describe_parameters` service, print descriptor
- [x] Shows type, description, constraints, read_only, ranges

#### Step 6.5: `param dump <node>`
- [x] List all params + get all → output YAML (node_name/ros__parameters format)

- **Files**: `oxidros-cli/src/param.rs`, `oxidros-cli/src/service.rs` (refactored `raw_call`)
- **Approach**: Uses dynamic CDR encode/decode via `service::raw_call()` (same as service call)

---

### Phase 7 — Bag Record (MCAP)

#### Step 7.1: Add `mcap` dependency
- [x] Add `mcap` crate to `oxidros-cli/Cargo.toml`
- **Files**: `oxidros-cli/Cargo.toml`

#### Step 7.2: `bag record <topics...>` / `bag record -a`
- [x] Discover topics from graph cache
- [x] For each topic:
  1. Obtain `TypeDescriptionMsg` (Step 3.2)
  2. Generate schema via `to_msg_definition()` (Step 0.3)
  3. Register MCAP channel: `schema_encoding="ros2msg"`, `message_encoding="cdr"`, schema data = msg definition string
- [x] Subscribe raw (Step 0.1) to each topic
- [x] For each sample: write MCAP message (CDR payload, timestamp from attachment, channel ID)
- [x] Flags: `--output <file>`, `--duration`, `--compression {zstd,lz4,none}`
- [x] Ctrl+C handler: finalize MCAP (write summary section)
- **Files**: `oxidros-cli/src/bag.rs`
- **Depends on**: Step 0.1, Step 0.2 or 0.3, Step 3.2

#### Step 7.3: `bag info <file>`
- [x] Read MCAP file summary
- [x] Print: duration, message count, topics + types + message counts, compression, schema info
- **Files**: `oxidros-cli/src/bag.rs`

---

### Phase 8 — Bag Play

#### Step 8.1: `bag play <file>`
- [x] Open MCAP, read channels + schemas
- [x] For each channel: create Zenoh publisher with correct key expression
- [x] Iterate messages in timestamp order
- [x] Replay with timing: `tokio::time::sleep()` matching original intervals
- [x] Flags: `--rate <float>`, `--loop`, `--topics <filter>`, `--start-offset`
- [x] Ctrl+C handler for clean shutdown
- [x] Channel metadata (dds_type, type_hash) stored during record, read during play
- **Files**: `oxidros-cli/src/bag.rs`
- **Depends on**: `send_raw()` (already exists)

---

### Phase 9 — Additional Commands

#### Step 9.1: `topic pub <topic> <type> <yaml>`
- [ ] Dynamic CDR encoder (YAML → CDR bytes given `TypeDescriptionMsg`)
- [ ] Publish via `send_raw()`
- **Depends on**: Dynamic encoder from Step 5.1

#### Step 9.2: `action list` / `action info` / `action send_goal`
- [ ] Similar to service commands, using `ActionTypeDescription`

#### Step 9.3: `doctor`
- [ ] Check Zenoh connectivity, enumerate nodes, verify liveliness

#### Step 9.4: Shell completions
- [ ] `clap_complete` for bash/zsh/fish

---

## Recommended Implementation Order

| Priority | Phase | What | Blocked by |
|----------|-------|------|------------|
| 1 | 0.1 | `recv_raw()` on subscriber | — |
| 2 | 0.2, 0.3 | `to_idl()`, `to_msg_definition()` | — |
| 3 | 1.1, 1.2 | CLI crate skeleton + Zenoh bootstrap | — |
| 4 | 2.1–2.5 | Graph introspection (`node list`, `topic list`, etc.) | Phase 1 |
| 5 | 6.1–6.5 | Parameter commands (typed — no dynamic decode) | Phase 1 |
| 6 | 3.1 | Dynamic CDR → JSON decoder | — |
| 7 | 3.2 | Runtime TypeDescriptionMsg resolution | Phase 1 |
| 8 | 4.1–4.3 | `echo`, `hz`, `bw` | Phase 0.1, 3.1, 3.2 |
| 9 | 7.1–7.3 | Bag record + info (MCAP) | Phase 0.1, 0.2, 3.2 |
| 10 | 8.1 | Bag play | Phase 1 |
| 11 | 5.1 | Service call (dynamic) | Phase 3.1, 3.2 |
| 12 | 9.x | topic pub, actions, doctor, completions | Phase 3.1 |

---

## Information Loss Accepted

When generating IDL/msg from `TypeDescriptionMsg`, the following are **not available** but are
**not needed** for MCAP playback or echo:

- Constants (not in `TypeDescriptionMsg`)
- Comments / annotations
- Complex default values (only `default_value: String` exists)

MCAP consumers (Foxglove, PlotJuggler) only need the structural schema to decode CDR.

---

## Key Files Reference

| File | What to edit |
|------|-------------|
| `oxidros-core/src/api/mod.rs` | `RosSubscriber` trait — add `recv_raw()` |
| `oxidros-zenoh/src/topic/subscriber.rs` | Implement `recv_raw()` |
| `ros2-types/src/types.rs` | `TypeDescriptionMsg::to_idl()`, `to_msg_definition()` |
| `ros2-types/src/lib.rs` | Re-export new methods |
| `ros2msg/src/idl_adapter.rs` | Reference for IDL type mapping |
| `oxidros-zenoh/src/graph_cache.rs` | `EntityInfo`, discovery API |
| `oxidros-zenoh/src/keyexpr.rs` | Key expression construction |
| `oxidros-zenoh/src/context.rs` | Zenoh session + graph discovery init |
| `oxidros-zenoh/src/attachment.rs` | Message metadata (seq, timestamp, GID) |
