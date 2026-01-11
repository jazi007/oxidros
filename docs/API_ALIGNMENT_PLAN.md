# API Alignment Plan: oxidros-rcl vs oxidros-zenoh

This document details the public API differences between `oxidros-rcl` and `oxidros-zenoh`, and provides a phased plan to align them for a unified developer experience.

---

## Table of Contents

1. [API Differences](#api-differences)
   - [Context](#1-context)
   - [Node](#2-node)
   - [Publisher](#3-publisher)
   - [Subscriber](#4-subscriber)
   - [Client (Service)](#5-client-service)
   - [Server (Service)](#6-server-service)
   - [Selector](#7-selector)
   - [Action (Client/Server)](#8-action-clientserver)
   - [Parameter Server](#9-parameter-server)
   - [Logger](#10-logger)
   - [Additional Types](#11-additional-types--re-exports)
   - [Error Types](#12-error-types)
   - [zenoh-ext Usage](#13-zenoh-ext-crate-not-used)
2. [Alignment Plan](#alignment-plan)

---

## API Differences

### 1. Context

| API | `oxidros-rcl` | `oxidros-zenoh` | Status |
|-----|---------------|-----------------|--------|
| `new()` | `fn new() -> Result<Arc<Self>>` | `fn new() -> Result<Arc<Self>>` | ✅ Aligned |
| `create_node()` | `fn create_node(&Arc<Self>, name: &str, namespace: Option<&str>, options: NodeOptions) -> OResult<Arc<Node>>` | `fn create_node(&Arc<Self>, name: &str, namespace: Option<&str>) -> Result<Arc<Node>>` | ⚠️ Different (NodeOptions param) |
| `create_selector()` | `fn create_selector(&Arc<Self>) -> OResult<Selector>` | ❌ Missing | ❌ Missing in zenoh |
| `with_args()` | ❌ Missing | `fn with_args(ros2_args: Ros2Args) -> Result<Arc<Self>>` | ❌ Missing in rcl |
| `with_domain_id()` | ❌ Missing | `fn with_domain_id(domain_id: u32) -> Result<Arc<Self>>` | ❌ Missing in rcl |
| `with_config()` | ❌ Missing | `fn with_config(domain_id: u32, config: zenoh::Config) -> Result<Arc<Self>>` | 🔧 Backend-specific |
| `with_args_and_domain_id()` | ❌ Missing | `fn with_args_and_domain_id(ros2_args, domain_id) -> Result<Arc<Self>>` | ❌ Missing in rcl |
| `with_full_config()` | ❌ Missing | `fn with_full_config(ros2_args, domain_id, config) -> Result<Arc<Self>>` | 🔧 Backend-specific |
| `domain_id()` | ❌ Missing | `fn domain_id(&self) -> u32` | ❌ Missing in rcl |
| `session_id()` | ❌ Missing | `fn session_id(&self) -> &str` | 🔧 Zenoh-specific |
| `session()` | ❌ Missing | `fn session(&self) -> &Session` | 🔧 Zenoh-specific |
| `ros2_args()` | ❌ Missing | `fn ros2_args(&self) -> &Ros2Args` | ❌ Missing in rcl |
| `enclave()` | ❌ Missing | `fn enclave(&self) -> Option<&str>` | ❌ Missing in rcl |
| `graph_cache()` | ❌ Missing | `fn graph_cache(&self) -> GraphCache` | 🔧 Zenoh-specific |

**Summary:**
- `create_node()` has different signatures (NodeOptions in rcl)
- zenoh has more flexible constructors
- rcl is missing domain_id, ros2_args, enclave accessors
- zenoh is missing `create_selector()` on Context

---

### 2. Node

| API | `oxidros-rcl` | `oxidros-zenoh` | Status |
|-----|---------------|-----------------|--------|
| `get_name()` | `fn get_name(&self) -> OResult<String>` | `fn get_name(&self) -> &str` | ⚠️ Different return type |
| `get_namespace()` | `fn get_namespace(&self) -> OResult<String>` | `fn get_namespace(&self) -> &str` | ⚠️ Different return type |
| `get_fully_qualified_name()` | `fn get_fully_qualified_name(&self) -> OResult<String>` | `fn get_fully_qualified_name(&self) -> String` | ⚠️ Different return type |
| `name()` | ❌ Missing | `fn name(&self) -> &str` | ❌ Missing in rcl |
| `namespace()` | ❌ Missing | `fn namespace(&self) -> &str` | ❌ Missing in rcl |
| `fully_qualified_name()` | ❌ Missing | `fn fully_qualified_name(&self) -> String` | ❌ Missing in rcl |
| `create_publisher()` | `fn create_publisher<T>(..., qos: Option<Profile>) -> OResult<Publisher<T>>` | `fn create_publisher<T>(..., qos: Option<Profile>) -> Result<Publisher<T>>` | ✅ Aligned |
| `create_publisher_disable_loaned_message()` | `fn create_publisher_disable_loaned_message<T>(...) -> OResult<Publisher<T>>` | ❌ Missing | 🔧 RCL-specific |
| `create_subscriber()` | `fn create_subscriber<T>(..., qos: Option<Profile>) -> OResult<Subscriber<T>>` | `fn create_subscriber<T>(..., qos: Option<Profile>) -> Result<Subscriber<T>>` | ✅ Aligned |
| `create_subscriber_disable_loaned_message()` | `fn create_subscriber_disable_loaned_message<T>(...) -> OResult<Subscriber<T>>` | ❌ Missing | 🔧 RCL-specific |
| `create_server()` | `fn create_server<T>(..., qos: Option<Profile>) -> OResult<Server<T>>` | `fn create_server<T>(..., qos: Option<Profile>) -> Result<Server<T>>` | ✅ Aligned |
| `create_client()` | `fn create_client<T>(..., qos: Option<Profile>) -> OResult<Client<T>>` | `fn create_client<T>(..., qos: Option<Profile>) -> Result<Client<T>>` | ✅ Aligned |
| `create_parameter_server()` | `fn create_parameter_server(&Arc<Self>) -> Result<ParameterServer>` | `fn create_parameter_server(&Arc<Self>) -> Result<ParameterServer>` | ✅ Aligned |
| `gid()` | ❌ Missing | `fn gid(&self) -> &[u8; 16]` | ❌ Missing in rcl |
| `node_id()` | ❌ Missing | `fn node_id(&self) -> u32` | 🔧 Zenoh-specific |
| `enclave()` | ❌ Missing | `fn enclave(&self) -> &str` | ❌ Missing in rcl |
| `context()` | ❌ Missing | `fn context(&self) -> &Arc<Context>` | ❌ Missing in rcl |
| `expand_and_remap_name()` | ❌ Missing | `fn expand_and_remap_name(&self, name, kind) -> Result<String>` | ❌ Missing in rcl |

**Summary:**
- `get_*` methods return `OResult<String>` in rcl but `&str`/`String` in zenoh
- rcl is missing short-form accessors (`name()`, `namespace()`, `fully_qualified_name()`)
- Loaned message APIs are rcl-specific (shared memory)
- rcl is missing `gid()`, `context()`, `enclave()`, `expand_and_remap_name()`

---

### 3. Publisher

| API | `oxidros-rcl` | `oxidros-zenoh` | Status |
|-----|---------------|-----------------|--------|
| `topic_name()` | `fn topic_name(&self) -> Result<Cow<'_, String>>` | `fn topic_name(&self) -> Result<Cow<'_, String>>` | ✅ Aligned |
| `fully_qualified_topic_name()` | `fn fully_qualified_topic_name(&self) -> Result<Cow<'_, String>>` | `fn fully_qualified_topic_name(&self) -> Result<Cow<'_, String>>` | ✅ Aligned |
| `send()` | `fn send(&self, msg: &T) -> Result<()>` | `fn send(&self, msg: &T) -> Result<()>` | ✅ Aligned |
| `send_loaned()` | `fn send_loaned(&self, msg: PublisherLoanedMessage<T>) -> Result<()>` | ❌ Missing | 🔧 RCL-specific |
| `send_raw()` | `unsafe fn send_raw(&self, msg: &[u8]) -> Result<()>` | ❌ Missing | 🔧 RCL-specific |
| `can_loan_messages()` | `fn can_loan_messages(&self) -> bool` | ❌ Missing | 🔧 RCL-specific |
| `borrow_loaned_message()` | `fn borrow_loaned_message(&self) -> OResult<PublisherLoanedMessage<T>>` | ❌ Missing | 🔧 RCL-specific |
| `gid()` | ❌ Missing | `fn gid(&self) -> &[u8; GID_SIZE]` | 🔧 Zenoh-specific |
| `node()` | ❌ Missing | `fn node(&self) -> &Arc<Node>` | 🔧 Zenoh-specific |
| `statistics()` | `fn statistics(&self) -> SerializableTimeStat` (feature-gated) | ❌ Missing | 🔧 RCL-specific |

**Summary:**
- ✅ Topic name accessors aligned (`topic_name()`, `fully_qualified_topic_name()`)
- Loaned message APIs are rcl-specific (shared memory)
- `gid()` and `node()` are zenoh-specific

---

### 4. Subscriber

| API | `oxidros-rcl` | `oxidros-zenoh` | Status |
|-----|---------------|-----------------|--------|
| `topic_name()` | `fn topic_name(&self) -> Result<Cow<'_, String>>` | `fn topic_name(&self) -> Result<Cow<'_, String>>` | ✅ Aligned |
| `fully_qualified_topic_name()` | `fn fully_qualified_topic_name(&self) -> Result<Cow<'_, String>>` | `fn fully_qualified_topic_name(&self) -> Result<Cow<'_, String>>` | ✅ Aligned |
| `try_recv()` | `fn try_recv(&self) -> Result<Option<Message<T>>>` | `fn try_recv(&mut self) -> Result<Option<Message<T>>>` | ✅ Aligned (return type) |
| `recv()` | `async fn recv(&mut self) -> Result<Message<T>>` | `async fn recv(&mut self) -> Result<Message<T>>` | ✅ Aligned |
| `recv_blocking()` | `fn recv_blocking(&self) -> Result<Message<T>>` | `fn recv_blocking(&self) -> Result<Message<T>>` | ✅ Aligned |
| `gid()` | ❌ Missing | `fn gid(&self) -> &[u8; GID_SIZE]` | 🔧 Zenoh-specific |
| `node()` | ❌ Missing | `fn node(&self) -> &Arc<Node>` | 🔧 Zenoh-specific |
| `statistics()` | `fn statistics(&self) -> SerializableTimeStat` (feature-gated) | ❌ Missing | 🔧 RCL-specific |

**Summary:**
- ✅ Both backends now return unified `Message<T>` from `recv()` and `try_recv()`
- ✅ Topic name accessors aligned (`topic_name()`, `fully_qualified_topic_name()`)
- rcl has `recv_blocking()`, zenoh does not
- `gid()` and `node()` are zenoh-specific

---

### 5. Client (Service)

| API | `oxidros-rcl` | `oxidros-zenoh` | Status |
|-----|---------------|-----------------|--------|
| `is_service_available()` | `fn is_service_available(&self) -> bool` | `fn is_service_available(&self) -> bool` | ✅ Aligned |
| `call()` | `async fn call(&mut self, request) -> Result<Message<Response>>` | `async fn call(&mut self, request) -> Result<Message<Response>>` | ✅ Aligned |
| `service_name()` | `fn service_name(&self) -> Result<Cow<'_, String>>` | `fn service_name(&self) -> Result<Cow<'_, String>>` | ✅ Aligned |
| `fully_qualified_service_name()` | `fn fully_qualified_service_name(&self) -> Result<Cow<'_, String>>` | `fn fully_qualified_service_name(&self) -> Result<Cow<'_, String>>` | ✅ Aligned |
| `send()` | `fn send(&mut self, data: &Request) -> Result<ClientRecv<T>>` | ❌ Missing | 🔧 RCL-specific (advanced) |
| `send_ret_seq()` | `fn send_ret_seq(&mut self, data: &Request) -> Result<(ClientRecv<T>, i64)>` | ❌ Missing | 🔧 RCL-specific (advanced) |
| `gid()` | ❌ Missing | `fn gid(&self) -> &[u8; GID_SIZE]` | 🔧 Zenoh-specific |
| `node()` | ❌ Missing | `fn node(&self) -> &Arc<Node>` | 🔧 Zenoh-specific |

**Summary:**
- ✅ Both backends have unified `call()` returning `Message<T::Response>`
- ✅ Service name accessors aligned (`service_name()`, `fully_qualified_service_name()`)
- ✅ `is_service_available()` returns `bool` in both backends
- rcl keeps `send()` / `send_ret_seq()` for advanced two-step pattern
- `gid()` and `node()` are zenoh-specific

---

### 6. Server (Service)

| API | `oxidros-rcl` | `oxidros-zenoh` | Status |
|-----|---------------|-----------------|--------|
| `recv()` | `async fn recv(&mut self) -> Result<ServiceRequest<T>>` | `async fn recv(&mut self) -> Result<ServiceRequest<T>>` | ✅ Aligned |
| `try_recv()` | `fn try_recv(&mut self) -> Result<Option<ServiceRequest<T>>>` | `fn try_recv(&mut self) -> Result<Option<ServiceRequest<T>>>` | ✅ Aligned |
| `service_name()` | `fn service_name(&self) -> Result<Cow<'_, String>>` | `fn service_name(&self) -> Result<Cow<'_, String>>` | ✅ Aligned |
| `fully_qualified_service_name()` | `fn fully_qualified_service_name(&self) -> Result<Cow<'_, String>>` | `fn fully_qualified_service_name(&self) -> Result<Cow<'_, String>>` | ✅ Aligned |
| `configure_introspection()` | `fn configure_introspection(&self, clock, qos, state) -> Result<()>` (jazzy) | ❌ Missing | 🔧 RCL-specific |
| `gid()` | ❌ Missing | `fn gid(&self) -> &[u8; GID_SIZE]` | 🔧 Zenoh-specific |
| `node()` | ❌ Missing | `fn node(&self) -> &Arc<Node>` | 🔧 Zenoh-specific |

**ServiceRequest<T> (both backends):**
- `request: Message<T::Request>` - the request data with metadata
- `send(response)` - send a response
- `split()` - separate sender and request for advanced use

**Summary:**
- ✅ Both backends have unified `ServiceRequest<T>` with same interface
- ✅ `recv()` and `try_recv()` return `ServiceRequest<T>` in both backends
- ✅ Service name accessors aligned (`service_name()`, `fully_qualified_service_name()`)
- `gid()` and `node()` are zenoh-specific

---

### 7. Selector

| API | `oxidros-rcl` | `oxidros-zenoh` | Status |
|-----|---------------|-----------------|--------|
| `new()` | `pub(crate) fn new(context: Arc<Context>) -> Result<Self>` | `pub(crate) fn new() -> Self` | ✅ Both private |
| `Context::create_selector()` | `fn create_selector(&Arc<Self>) -> Result<Selector>` | `fn create_selector(&self) -> Selector` | ✅ Aligned |
| `add_subscriber()` | `fn add_subscriber<T>(&mut self, subscriber, handler) -> bool` | `fn add_subscriber<T>(&mut self, subscriber, handler) -> bool` | ✅ Aligned |
| `add_server()` | `fn add_server<T>(&mut self, server, handler) -> bool` | ❌ Stub only | ❌ Missing in zenoh |
| `add_parameter_server()` | `fn add_parameter_server(&mut self, param_server, handler)` | ❌ Stub only | ❌ Missing in zenoh |
| `add_timer()` | `fn add_timer(&mut self, duration, handler) -> u64` (one-shot) | `fn add_timer(&mut self, duration, handler) -> u64` (one-shot) | ✅ Aligned |
| `add_wall_timer()` | `fn add_wall_timer(&mut self, name, period, handler) -> u64` | `fn add_wall_timer(&mut self, name, period, handler) -> u64` | ✅ Aligned |
| `remove_timer()` | `fn remove_timer(&mut self, id: u64)` | `fn remove_timer(&mut self, id: u64)` | ✅ Aligned |
| `wait()` | `fn wait(&mut self) -> Result<()>` | `fn wait(&mut self) -> Result<()>` | ✅ Aligned |
| `wait_timeout()` | `fn wait_timeout(&mut self, timeout) -> Result<bool>` | `fn wait_timeout(&mut self, timeout) -> Result<bool>` | ✅ Aligned |
| `add_action_server()` | `fn add_action_server<T, GR, A, CR>(...) -> bool` | ❌ Returns error | ❌ Not supported in zenoh |
| `add_action_client()` | `fn add_action_client(&mut self, client) -> bool` | ❌ Returns error | ❌ Not supported in zenoh |
| `add_guard_condition()` | `fn add_guard_condition(&mut self, cond, handler, is_once)` | ❌ Missing | 🔧 RCL-specific |
| `statistics()` | `fn statistics(&self) -> Statistics` (feature-gated) | ❌ Missing | 🔧 RCL-specific |

**Summary:**
- ✅ Both backends create Selector via `Context::create_selector()`
- ✅ `Selector::new()` is private (`pub(crate)`) in both backends
- ✅ Timer APIs aligned: `add_timer()` (one-shot), `add_wall_timer()` (periodic), `remove_timer()`
- zenoh is missing server/parameter server/action handlers

---

### 8. Action (Client/Server)

| Module | `oxidros-rcl` | `oxidros-zenoh` | Status |
|--------|---------------|-----------------|--------|
| `action::client::Client` | ✅ Full implementation | ❌ Stub only | ❌ Not supported |
| `action::server::Server` | ✅ Full implementation | ❌ Stub only | ❌ Not supported |
| `action::handle::GoalHandle` | ✅ Full implementation | ❌ Stub only | ❌ Not supported |

**Note:** Actions are not yet supported in the Zenoh backend.

---

### 9. Parameter Server

| API | `oxidros-rcl` | `oxidros-zenoh` | Status |
|-----|---------------|-----------------|--------|
| `new()` | `fn new(node: Arc<Node>) -> Result<Self>` | `fn new(node: Arc<Node>) -> Result<Self>` | ✅ Aligned |
| `wait()` | `async fn wait(&mut self) -> Result<BTreeSet<String>>` | ❌ Missing | ⚠️ Different pattern |
| `spin()` | ❌ Missing | `async fn spin(&mut self) -> Result<()>` | ⚠️ Different pattern |
| `params` field | `pub params: Arc<RwLock<Parameters>>` | Private | ⚠️ Different visibility |
| `params()` | ❌ Missing (field is public) | `fn params(&self) -> &Arc<RwLock<Parameters>>` | ⚠️ Different access pattern |
| `cond_callback` | `pub cond_callback: GuardCondition` | ❌ Missing | 🔧 RCL-specific |

---

### 10. Logger

| Module | `oxidros-rcl` | `oxidros-zenoh` | Status |
|--------|---------------|-----------------|--------|
| `Logger` struct | ✅ Full implementation | ❌ Missing entirely | ❌ Missing in zenoh |
| `pr_debug!` macro | ✅ Available | ❌ Missing | ❌ Missing in zenoh |
| `pr_info!` macro | ✅ Available | ❌ Missing | ❌ Missing in zenoh |
| `pr_warn!` macro | ✅ Available | ❌ Missing | ❌ Missing in zenoh |
| `pr_error!` macro | ✅ Available | ❌ Missing | ❌ Missing in zenoh |
| `pr_fatal!` macro | ✅ Available | ❌ Missing | ❌ Missing in zenoh |

---

### 11. Additional Types & Re-exports

| Type | `oxidros-rcl` | `oxidros-zenoh` | Status |
|------|---------------|-----------------|--------|
| `Message<T>` | ✅ Unified type with `sample: MessageData<T>` + `info: MessageInfo` | ✅ Same | ✅ Aligned |
| `MessageData<T>` | `enum { Copied(T), Loaned(...) }` | `enum { Copied(T), Loaned(...) }` | ✅ Aligned |
| `MessageInfo` | `struct { sequence_number, source_timestamp_ns, publisher_gid }` | Same | ✅ Aligned |
| `RecvResult<T>` | ❌ Removed | ❌ N/A | ✅ Removed |
| `TakenMsg<T>` | ❌ Replaced by `Message<T>` | ❌ Replaced by `Message<T>` | ✅ Unified |
| `Attachment` | ❌ N/A | Internal only (wire encoding for MessageInfo) | 🔧 Internal |
| `Header` (service) | ✅ Includes timestamps, sequence, guid | ❌ Missing | ❌ Missing in zenoh |
| `NodeOptions` | ✅ Full struct | ❌ Missing | 🔧 RCL-specific |
| `GraphCache` | ❌ Missing | ✅ Full implementation | 🔧 Zenoh-specific |
| `QosMapping` | ❌ Missing | ✅ Full implementation | 🔧 Zenoh-specific |
| `ST<T>` | ✅ Single-threaded container | ❌ Missing | 🔧 RCL-specific |
| `is_halt()` | ✅ Signal handler | ❌ Missing | ❌ Missing in zenoh |
| `PublisherLoanedMessage` | ✅ Available | ❌ Missing | 🔧 RCL-specific |
| `SubscriberLoanedMessage` | ✅ Available | ❌ Missing | 🔧 RCL-specific |
| `ServiceRequest` (zenoh) | ❌ Missing | ✅ Unified request+sender | ⚠️ Different |

---

### 12. Error Types ✅

| Type | `oxidros-rcl` | `oxidros-zenoh` | Status |
|------|---------------|-----------------|--------|
| Result alias | `Result<T>` = `Result<T, Error>` | `Result<T>` = `Result<T, Error>` | ✅ Aligned |
| Re-exports | `Error`, `Result`, `ActionError`, `RclError` | `Error`, `Result`, `ActionError`, `RclError` | ✅ Aligned |

---

### 13. zenoh-ext Crate ✅

**Status:** ✅ Completed - zenoh-ext is now properly used for TRANSIENT_LOCAL durability support.

```toml
# In Cargo.toml - now actively used:
zenoh-ext = { version = "1.0", features = ["unstable"] }
```

#### What zenoh-ext Provides (and we're missing):

| Feature | Purpose | ROS2 QoS Support |
|---------|---------|------------------|
| `AdvancedPublisher` | Publisher with message caching | Required for `TRANSIENT_LOCAL` durability |
| `AdvancedSubscriber` | Subscriber with history query | Required for late-joining subscribers |
| `CacheConfig` | Configure publisher cache size | Maps to `KeepLast(n)` history depth |
| `HistoryConfig` | Query historical samples | Late-joiner receives cached messages |
| `RecoveryConfig` | Message retransmission | Required for `RELIABLE` reliability |
| `MissDetectionConfig` | Detect missed samples | Sequence number gap detection |
| `SampleMissListener` | Callback on missed samples | Reliability monitoring |
| `z_serialize`/`z_deserialize` | Zenoh serialization format | Potential alternative to CDR |

#### Current Consequences:

| QoS Policy | Expected Behavior | Actual Behavior |
|------------|-------------------|-----------------|
| `TRANSIENT_LOCAL` | Late joiners receive cached messages | ❌ Late joiners miss messages |
| `RELIABLE` + `KeepAll` | No message loss, blocking on congestion | ⚠️ Partial (congestion control only) |
| `KeepLast(n)` with durability | Publisher caches n samples | ❌ No caching implemented |
| Message loss detection | Gaps in sequence numbers detected | ❌ Not implemented |

**Status:** 🔴 Critical - Must implement to be compatible with ROS2 QoS semantics.

---

## Alignment Plan

The goal is to create a unified API that allows users to write backend-agnostic code while still providing access to backend-specific features when needed.

### Phase 1: Foundation - Unified Traits & Error Types

**Goal:** Establish common traits in `oxidros-core` that both backends implement.

#### Step 1.1: Unify Error Types ✅
- [x] Ensure both backends use `oxidros_core::Error` as the primary error type
- [x] Rename rcl's `OResult<T>` to `Result<T>` for consistency
- [x] Keep backend-specific error variants (RclError, ZenohError) as enum variants in unified Error
- [x] Re-export error types at crate root for both backends (`Error`, `Result`, `ActionError`, `RclError`)

#### Step 1.2: Unify RecvResult Pattern ✅
- [x] Decide on unified pattern: Either `RecvResult<T>` enum or `Result<Option<T>>`
- [x] Recommendation: Use `Result<Option<T>>` as it's more idiomatic Rust
- [x] Update rcl's `try_recv()` to return `Result<Option<T>>` instead of `RecvResult<T>`
- [x] Remove `RecvResult` enum completely

#### Step 1.3: Review & Extend Core Traits
- [ ] Audit `oxidros_core::api` traits (RosContext, RosNode, RosPublisher, etc.)
- [ ] Ensure all common methods are defined in traits
- [ ] Add missing trait methods identified in this document

---

### Phase 2: Context & Node Alignment

**Goal:** Align Context and Node APIs for consistent creation and access patterns.

#### Step 2.1: Context Creation ✅
- [x] Zenoh `create_node()` signature matches rcl (name, namespace) ✅
- [x] Add `create_selector()` to zenoh Context (wraps `Selector::new()`)
- Note: Backend-specific constructors remain (zenoh's `with_config()`, `with_domain_id()`, etc.)
- Note: Backend-specific accessors remain (zenoh's `session()`, `session_id()`, `graph_cache()`, etc.)

#### Step 2.2: Node Creation ✅
- [x] rcl has `create_node(name, namespace)` and `create_node_with_opt(name, namespace, options)`
- [x] zenoh has `create_node(name, namespace)` - aligned with rcl
- Note: `NodeOptions` is rcl-specific (not needed in zenoh)

#### Step 2.3: Node Accessors ✅
- [x] Renamed `get_name()`, `get_namespace()`, `get_fully_qualified_name()` to `name()`, `namespace()`, `fully_qualified_name()`
- [x] Both rcl and zenoh return `Result<String>` - aligned
- Note: `gid()`, `context()`, `enclave()`, `expand_and_remap_name()`, `node_id()` are zenoh-specific

---

### Phase 3: Topic API Alignment (Publisher/Subscriber)

**Goal:** Unify naming conventions and return types.

#### Step 3.1: Naming Convention ✅
- [x] Both rcl and zenoh have `topic_name()` returning the short topic name
- [x] Both rcl and zenoh have `fully_qualified_topic_name()` returning the full path with namespace
- [x] rcl computes `topic_name()` from `fully_qualified_topic_name()` (extracts last segment)
- [x] Added tests in `oxidros-rcl/tests/test_fqn.rs`

#### Step 3.2: Publisher Accessors
- Note: `gid()` and `node()` are zenoh-specific (not available in rcl)

#### Step 3.3: Subscriber Alignment ✅
- [x] Both backends return `Result<Option<Message<T>>>` from `try_recv()`
- [x] Both backends return `Result<Message<T>>` from `recv()`
- [x] Both backends have `recv_blocking()` returning `Result<Message<T>>`
- Note: `gid()` and `node()` are zenoh-specific (not available in rcl)

#### Step 3.4: Message Types ✅
- [x] Created unified `Message<T>` in oxidros-core with:
  - `sample: MessageData<T>` - the message data (Copied or Loaned)
  - `info: MessageInfo` - metadata (sequence_number, source_timestamp_ns, publisher_gid)
- [x] `MessageData<T>` enum replaces old `TakenMsg<T>` with `Copied(T)` and `Loaned(...)` variants
- [x] `MessageInfo` struct with: `sequence_number: i64`, `source_timestamp_ns: i64`, `publisher_gid: [u8; 16]`
- [x] `Message<T>` implements `Deref<Target=T>` for ergonomic access
- [x] zenoh: Fills `MessageInfo` from `Attachment` via `From<Attachment> for MessageInfo`
- [x] rcl: Fills `MessageInfo` from `rmw_message_info_t` via conversion
- [x] zenoh's `ReceivedMessage<T>` remains internal (wire format)
- [x] Both `RosSubscriber` trait and `Selector` updated to use `Message<T>`

---

### Phase 4: Service API Alignment (Client/Server)

**Goal:** Create a unified service pattern that works across both backends.

#### Step 4.1: Client API ✅
- [x] Add `call()` async method to rcl Client (uses `send()` + `recv()` internally)
- [x] Removed `call_with_timeout()` (not needed, users can use `tokio::time::timeout`)
- [x] Add `service_name()` accessor to rcl Client (returns `Result<Cow<'_, String>>`)
- [x] Add `fully_qualified_service_name()` accessor to rcl Client
- [x] Change rcl `is_service_available()` to return `bool` (not `Result`)
- [x] Keep `send()` / `send_ret_seq()` pattern available in rcl for advanced use cases
- Note: `gid()` is zenoh-specific (not available in rcl)
- Note: `node()` not needed in unified API

#### Step 4.2: Server API ✅
- [x] Both backends have `ServiceRequest<T>` with same interface (not shared type, but same API)
- [x] `ServiceRequest<T>` contains: `request: Message<T::Request>`, `send(response)`, `split()`
- [x] `recv()` returns `Result<ServiceRequest<T>>` in both backends
- [x] `try_recv()` returns `Result<Option<ServiceRequest<T>>>` in both backends
- [x] Add `service_name()` accessor to rcl Server (returns `Result<Cow<'_, String>>`)
- [x] Add `fully_qualified_service_name()` accessor to rcl Server
- Note: `gid()` is zenoh-specific (not available in rcl)
- Note: `node()` not needed in unified API

#### Step 4.3: Header/Attachment Unification ✅
- [x] Reuse unified `Message<T>` type (contains `MessageInfo` with sequence number, timestamp, sender GID)
- [x] rcl: `rmw_service_info_t` → `MessageInfo` conversion in `oxidros-rcl/src/rcl/conversions.rs`
- [x] zenoh: `Attachment` → `MessageInfo` conversion (via `From<Attachment> for MessageInfo`)
- [x] Client `call()` returns `Message<T::Response>` (both backends)
- [x] Server `recv()` returns request wrapped with `Message<T::Request>` (both backends)
- [x] `RosClient::call_service()` trait returns `Result<Message<T::Response>>`
- [x] Added `Debug` impl for `Message<T>` and `MessageData<T>` when `T: Debug`

---

### Phase 5: Selector Alignment

**Goal:** Make Selector creation and timer APIs consistent.

#### Step 5.1: Selector Creation ✅
- [x] Add `create_selector()` to zenoh Context
- [x] Add `create_selector()` to rcl Context (already existed)
- [x] Make `Selector::new()` private (`pub(crate)`) in both backends
- [x] Both backends create Selector via `Context::create_selector()`

#### Step 5.2: Timer APIs ✅
- [x] Both backends have `add_timer()` for one-shot timers (fires once)
- [x] Both backends have `add_wall_timer()` for periodic timers
- [x] Both backends have `remove_timer()` to remove timers by ID
- Note: zenoh updated to match rcl semantics (add_timer is one-shot, add_wall_timer is periodic)

#### Step 5.3: Server Handler ✅
- [x] Implement `add_server()` properly in zenoh Selector
- [x] Use polling pattern similar to subscriber handling
- Note: zenoh Selector now polls `server_handlers` alongside `subscriber_handlers` in `wait_timeout_internal()`
- Handler receives `Message<T::Request>` and returns `T::Response` (via `ServerCallback<T>`)

#### Step 5.4: Parameter Server Handler ✅
- [x] Implement `add_parameter_server()` properly in zenoh Selector
- [x] Added `try_process_once()` to zenoh ParameterServer for non-blocking polling
- [x] Polls all 6 parameter services using `try_recv()`
- [x] Calls handler with `(&mut Parameters, BTreeSet<String>)` when params are updated
- Note: Uses `take_updated()` from core Parameters to get updated parameter names

---

### Phase 6: Parameter Server Alignment

**Goal:** Unify parameter server access patterns.

#### Step 6.1: Access Pattern ✅
- [x] Aligned zenoh to match rcl's API
- [x] Made `params` field public in zenoh (like rcl)
- [x] Removed `params()` accessor from zenoh (not needed with public field)
- [x] Removed `spin()` from zenoh (not in rcl API)
- Both backends now have: `pub params: Arc<RwLock<Parameters>>`
- Note: zenoh keeps `process_once()` and `try_process_once()` for async/sync processing

#### Step 6.2: Selector Integration ✅
- [x] Both parameter servers work with their respective Selectors
- rcl: Uses guard condition for notification, stores `param_server` in Selector
- zenoh: Uses polling with `try_process_once()`, stores handler closure
- Both call handler with `(&mut Parameters, BTreeSet<String>)` when params change
- Both use `take_updated()` from core Parameters to get updated names

---

### Phase 7: Logger Support for Zenoh

**Goal:** Add ROS2 logging capability to zenoh backend.

#### Step 7.1: Logger Implementation
- [ ] Create `Logger` struct in zenoh crate
- [ ] Use `tracing` crate or custom implementation
- [ ] Match log levels with rcl: debug, info, warn, error, fatal

#### Step 7.2: Logging Macros
- [ ] Add `pr_debug!`, `pr_info!`, `pr_warn!`, `pr_error!`, `pr_fatal!` macros

#### Step 7.3: Alternative
- [ ] Or: Move Logger to `oxidros-core` as a shared component
- [ ] Backends provide platform-specific sinks

---

### Phase 8: Signal Handling for Zenoh

**Goal:** Add graceful shutdown support to zenoh backend.

#### Step 8.1: Signal Handler
- [ ] Add `is_halt()` function to zenoh
- [ ] Register SIGINT/SIGTERM handlers

#### Step 8.2: Integration
- [ ] Ensure async operations check halt flag
- [ ] Selector should exit on signal

---

### Phase 9: Documentation & Deprecation

**Goal:** Document the unified API and deprecation timeline.

#### Step 9.1: API Documentation
- [ ] Document unified traits in `oxidros-core`
- [ ] Create migration guide for existing users
- [ ] Add examples showing backend-agnostic code

#### Step 9.2: Deprecation Notices
- [ ] Mark old method names as `#[deprecated]`
- [ ] Provide deprecation timeline (e.g., remove in next major version)

#### Step 9.3: Feature Flags
- [ ] Document backend-specific features (loaned messages, actions, etc.)
- [ ] Create feature flags for optional capabilities

---

### Phase 10: Use zenoh-ext for Advanced Pub/Sub ✅ COMPLETED

**Goal:** Leverage `zenoh-ext` crate for proper ROS2 QoS support.

**Status:** ✅ Completed on 2026-01-09

**Implementation Summary:**
- Publisher uses `AdvancedPublisher` with `.cache(depth)` - 0 for volatile, actual depth for transient local
- Subscriber uses `AdvancedSubscriber` with `.history(depth)` - 0 for volatile, actual depth for transient local
- No enum variants needed - advanced pub/sub works for all cases

**Background:** The `zenoh-ext` crate was declared as a dependency in `oxidros-zenoh/Cargo.toml` but was **not actually used** anywhere in the code. This has been fixed. `zenoh-ext` provides critical features required for proper ROS2 QoS compatibility:

#### What zenoh-ext Provides:

| Feature | Description | ROS2 QoS Mapping |
|---------|-------------|------------------|
| `AdvancedPublisher` | Publisher with caching support | `TRANSIENT_LOCAL` durability |
| `AdvancedSubscriber` | Subscriber with history query | `TRANSIENT_LOCAL` durability |
| `CacheConfig` | Configure publisher cache size | `KeepLast(n)` history depth |
| `HistoryConfig` | Configure historical data query | Late-joining subscriber support |
| `RecoveryConfig` | Configure retransmission | `RELIABLE` reliability |
| `MissDetectionConfig` | Detect missed samples | Sequence number tracking |
| `SampleMissListener` | Listen for missed samples | Reliability monitoring |

#### Current Problem:

The current implementation uses basic `zenoh::Publisher` and `zenoh::Subscriber`, which means:
- ❌ `TRANSIENT_LOCAL` durability is not properly implemented
- ❌ Late-joining subscribers don't receive cached messages
- ❌ No sample miss detection for reliable delivery
- ❌ No message recovery for unreliable networks

#### Step 10.1: AdvancedPublisher for TRANSIENT_LOCAL ✅
- [x] Use `AdvancedPublisher` when QoS durability is `TransientLocal`
- [x] Configure cache size from QoS history depth
- [x] Use `CacheConfig` to set `max_samples` based on `KeepLast(n)`

#### Step 10.2: AdvancedSubscriber for Late Joiners ✅
- [x] Use `AdvancedSubscriber` when QoS durability is `TransientLocal`
- [x] Configure `HistoryConfig` to query historical samples on subscription
- [x] Match history depth with publisher's cache

#### Step 10.3: Update Publisher Implementation ✅
```rust
// Before (current):
let zenoh_publisher = session.declare_publisher(key_expr).wait()?;

// After (with zenoh-ext):
use zenoh_ext::AdvancedPublisherBuilderExt;

let builder = session.declare_publisher(key_expr);
let zenoh_publisher = if QosMapping::is_transient_local(&qos) {
    builder
        .cache(CacheConfig::default().max_samples(QosMapping::effective_depth(&qos)))
        .wait()?
} else {
    builder.wait()?
};
```

#### Step 10.4: Update Subscriber Implementation ✅
```rust
// Before (current):
let zenoh_subscriber = session.declare_subscriber(key_expr).wait()?;

// After (with zenoh-ext):
use zenoh_ext::AdvancedSubscriberBuilderExt;

let builder = session.declare_subscriber(key_expr);
let zenoh_subscriber = if QosMapping::is_transient_local(&qos) {
    builder
        .history(HistoryConfig::default().max_samples(QosMapping::effective_depth(&qos)))
        .wait()?
} else {
    builder.wait()?
};
```

---

### Phase 11: Action Support for Zenoh (Future)

**Goal:** Implement ROS2 action support in zenoh backend.

#### Step 11.1: Research
- [ ] Study rmw_zenoh action implementation (if available)
- [ ] Design action protocol over Zenoh

#### Step 11.2: Implementation
- [ ] Implement action client
- [ ] Implement action server
- [ ] Implement goal handle

#### Step 11.3: Selector Integration
- [ ] Add action handlers to zenoh Selector

---

## Priority Matrix

| Phase | Priority | Effort | Impact |
|-------|----------|--------|--------|
| Phase 1: Foundation | 🔴 Critical | Medium | High |
| Phase 2: Context & Node | 🔴 Critical | Medium | High |
| Phase 3: Topic API | 🟡 High | Low | Medium |
| Phase 4: Service API | 🟡 High | Medium | High |
| Phase 5: Selector | 🟡 High | Medium | Medium |
| Phase 6: Parameter Server | 🟢 Medium | Low | Low |
| Phase 7: Logger | 🟢 Medium | Low | Medium |
| Phase 8: Signal Handling | 🟢 Medium | Low | Medium |
| Phase 9: Documentation | 🟡 High | Medium | High |
| **Phase 10: zenoh-ext** | **✅ Done** | **Medium** | **High** |
| Phase 11: Actions | 🔵 Low | High | Medium |

---

## Success Criteria

1. **API Compatibility**: User code written against `oxidros-core` traits works with both backends
2. **Feature Parity**: Common features work identically across backends
3. **Clear Documentation**: Backend-specific features are clearly documented
4. **Backward Compatibility**: Existing code continues to work with deprecation warnings
5. **Type Safety**: Compile-time errors for incompatible backend usage
