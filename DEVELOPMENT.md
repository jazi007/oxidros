# oxidros API Alignment Development Plan

This document tracks the progress of aligning `oxidros-rcl` and `oxidros-zenoh` APIs.

**Estimated Total Effort: 2-3 weeks**

---

## Phase 1: Error System Unification (2-3 days) ✅ COMPLETE

### 1.1 Clean up oxidros-core error types ✅
- [x] Create unified `Error` enum in `oxidros-core` covering both RCL and Zenoh cases
- [x] Add `thiserror` derive for better error messages
- [x] Add `From<ros2_types::Error>` impl for CDR errors
- [x] Keep `DynError`, `OError`, `OResult` as deprecated type aliases for backwards compatibility
- [x] Add `RclError` enum for RCL-specific error codes
- [x] Add `ActionError` enum with `Rcl` variant for nested RCL errors

### 1.2 Update oxidros-rcl error handling ✅
- [x] Update `oxidros-rcl/src/rcl/conversions.rs` to use `RclError` instead of `OError`
- [x] Update `oxidros-rcl/src/error.rs` to use `ActionError` instead of `RCLActionError`
- [x] Update `oxidros-rcl/src/action/client.rs` error pattern matching
- [x] Deprecated type aliases still work for backwards compatibility
- [x] Builds successfully with `--features jazzy`

### 1.3 Update oxidros-zenoh error handling ✅
- [x] Replace local `Error` enum with re-export from `oxidros_core::Error`
- [x] Add `Ros2ArgsResultExt` trait for converting ros2args errors
- [x] Update `context.rs` and `node.rs` to use new error handling
- [x] Builds successfully

---

## Phase 2: Trait Consolidation (2-3 days)

## Phase 2: Trait Consolidation (2-3 days) ✅ COMPLETE

**Note**: The original plan to move traits from `ros2-types` to `oxidros-core` was revised.
Moving traits would create a circular dependency since `ros2-types` has derive macros that 
other crates depend on. Instead, the approach is:
- Keep traits defined in `ros2-types` (standalone, low-level crate)
- `oxidros-core` re-exports all traits via `pub use ros2_types::*`
- Downstream crates import from `oxidros_core` instead of `ros2_types`

### 2.1 Verify oxidros-core re-exports ✅
- [x] `oxidros-core/src/lib.rs` has `pub use ros2_types::*` (re-exports all traits)
- [x] `oxidros-core/src/msg.rs` re-exports: `TypeSupport`, `TryClone`, `ServiceMsg`, `ActionMsg`, `ActionGoal`, `ActionResult`, `GetUUID`, `GoalResponse`, `ResultResponse`
- [x] `oxidros-core/src/time.rs` re-exports: `UnsafeTime`, `UnsafeDuration`

### 2.2 Trait source structure verified ✅
- [x] `ros2-types/src/traits.rs` is the source of truth for message traits
- [x] `ros2-types` provides derive macros (`Ros2Msg`, `TypeDescription`, `ros2_service`, `ros2_action`)
- [x] Derive macros reference `ros2_types::` for generated code

### 2.3 Update downstream crates ✅
- [x] `oxidros-rcl` - already imports from `oxidros_core` (no changes needed)
- [x] `oxidros-zenoh` - updated all imports from `ros2_types` to `oxidros_core`
  - `node.rs`: `TypeDescription`, `TypeSupport`
  - `topic/publisher.rs`: `TypeDescription`, `TypeSupport`
  - `topic/subscriber.rs`: `TypeDescription`, `TypeSupport`
  - `service/client.rs`: `TypeDescription`, `TypeSupport`
  - `service/server.rs`: `TypeDescription`, `TypeSupport`
- [x] Removed direct `ros2-types` dependency from `oxidros-zenoh/Cargo.toml`
- [x] `oxidros-msg` - keeps `ros2-types` dependency for derive macros (required)
- [x] All crates build successfully

---

## Phase 3: API Traits Definition in oxidros-core (3-4 days) ✅ COMPLETE

Created `oxidros-core/src/api/mod.rs` with unified API traits that both `oxidros-rcl` and `oxidros-zenoh` can implement.

### 3.1 Create Context trait ✅
- [x] Created `oxidros-core/src/api/mod.rs`
- [x] Defined `trait RosContext` with:
  - `type Node: RosNode` - Associated type for node
  - `fn create_node(&Arc<Self>, name: &str, namespace: Option<&str>) -> Result<Arc<Self::Node>>`
  - `fn domain_id(&self) -> u32`

### 3.2 Create Node trait ✅
- [x] Defined `trait RosNode` with:
  - `type Publisher<T>`, `type Subscriber<T>`, `type Client<T>`, `type Server<T>` - Associated types
  - `fn name(&self) -> &str`
  - `fn namespace(&self) -> &str`
  - `fn fully_qualified_name(&self) -> String`
  - `fn create_publisher<T>(&Arc<Self>, ...) -> Result<Self::Publisher<T>>`
  - `fn create_subscriber<T>(&Arc<Self>, ...) -> Result<Self::Subscriber<T>>`
  - `fn create_client<T>(&Arc<Self>, ...) -> Result<Self::Client<T>>`
  - `fn create_server<T>(&Arc<Self>, ...) -> Result<Self::Server<T>>`

**Note**: `create_parameter_server` deferred - not part of unified API yet

### 3.3 Create Publisher trait ✅
- [x] Defined `trait RosPublisher<T: TypeSupport>` with:
  - `fn topic_name(&self) -> &str`
  - `fn send(&self, msg: &T) -> Result<()>`

### 3.4 Create Subscriber trait ✅
- [x] Defined `trait RosSubscriber<T: TypeSupport, M = ()>` with:
  - `fn topic_name(&self) -> &str`
  - `async fn recv(&mut self) -> Result<ReceivedMessage<T, M>>`
  - `fn try_recv(&mut self) -> Result<Option<ReceivedMessage<T, M>>>`

**Note**: `recv_blocking` deferred - implementations differ too much

### 3.5 Create Service traits ✅
- [x] Defined `trait RosClient<T: ServiceMsg>` with:
  - `fn service_name(&self) -> &str`
  - `fn is_service_available(&self) -> bool`
  - `async fn call(&self, request: &T::Request) -> Result<T::Response>`
  - `async fn call_with_timeout(&self, request: &T::Request, timeout: Duration) -> Result<T::Response>`

- [x] Defined `trait RosServer<T: ServiceMsg>` with:
  - `type Request: ServiceRequest<T>` - Associated type for request handler
  - `fn service_name(&self) -> &str`
  - `async fn recv(&mut self) -> Result<Self::Request>`
  - `fn try_recv(&mut self) -> Result<Option<Self::Request>>`

### 3.6 Create common types ✅
- [x] Defined `ReceivedMessage<T, M>` struct with `data` and optional `metadata`
- [x] Defined `trait ServiceRequest<T: ServiceMsg>` with `request()` and `respond()`

**Note**: `CallbackResult` already exists in `oxidros_core::selector`

### 3.7 Re-export from lib.rs ✅
- [x] Added `pub mod api` to lib.rs
- [x] Re-exported: `ReceivedMessage`, `RosClient`, `RosContext`, `RosNode`, `RosPublisher`, `RosServer`, `RosSubscriber`, `ServiceRequest`

---

## Phase 4: Implement API Traits (3-4 days) ✅ COMPLETE

### 4.1 Implement traits in oxidros-rcl ✅
- [x] Implement `RosContext` trait for `context::Context`
- [x] Implement `RosNode` trait for `node::Node`
- [x] Implement `RosPublisher<T>` trait for `topic::publisher::Publisher<T>`
- [x] Implement `RosSubscriber<T>` trait for `topic::subscriber::Subscriber<T>`
- [x] Implement `RosClient<T>` trait for `service::client::Client<T>`
- [x] Implement `RosServer<T>` trait for `service::server::Server<T>`
- [x] Created `RclServiceRequest<T>` wrapper for `ServiceRequest` trait

### 4.2 Implement traits in oxidros-zenoh ✅
- [x] Implement `RosContext` trait for `Context`
- [x] Implement `RosNode` trait for `Node`
- [x] Implement `RosPublisher<T>` trait for `topic::publisher::Publisher<T>`
- [x] Implement `RosSubscriber<T>` trait for `topic::subscriber::Subscriber<T>`
- [x] Implement `RosClient<T>` trait for `service::client::Client<T>`
- [x] Implement `RosServer<T>` trait for `service::server::Server<T>`
- [x] Created `ZenohServiceRequest<T>` wrapper for `ServiceRequest` trait

### 4.3 TypeSupport and trait improvements ✅
- [x] Added `Send + Sync` as supertraits of `TypeSupport` (in `ros2-types`)
- [x] Added `Send + Sync` as supertraits of `ServiceMsg` (in `ros2-types`)
- [x] Added `type_hash() -> Result<String>` method to `TypeSupport` for RIHS01 hash
- [x] Simplified all API trait bounds to just `T: TypeSupport` or `T: ServiceMsg`
- [x] Updated `RosSubscriber` trait to return `TakenMsg<T>` for zero-copy loaned message support

### 4.4 Compatibility aliases (Deferred)
- [ ] Add `name()` as alias for `get_name()` on Node
- [ ] Add `namespace()` as alias for `get_namespace()` on Node  
- [ ] Add `topic_name()` as alias for `get_topic_name()` on Publisher
- [ ] Deprecate old method names with `#[deprecated]`

**Note**: Trait implementations delegate to existing methods; no API deprecation needed for trait usage.

---

## Phase 5: Selector Implementation (2-3 days) ✅ COMPLETE

### 5.1 Define RosSelector trait in oxidros-core ✅
- [x] Created `RosSelector` trait in `oxidros-core/src/api/mod.rs`
- [x] Associated types: `Subscriber<T>`, `Server<T>`, `ActionServer<T>`, `ActionClient<T>`, `ActionGoalHandle<T>`, `ParameterServer`
- [x] Methods: `add_subscriber()`, `add_server()`, `add_parameter_server()`, `add_timer()`, `add_wall_timer()`, `remove_timer()`
- [x] Action methods: `add_action_server()`, `add_action_client()` (return `Result<bool>` for backend support detection)
- [x] Wait methods: `wait()`, `wait_timeout()`
- [x] Added `NotImplemented` error variant to `oxidros_core::Error` for unsupported features

### 5.2 Add Selector to RosContext trait ✅
- [x] Added `type Selector: RosSelector` associated type to `RosContext`
- [x] Added `fn create_selector(&Arc<Self>) -> Result<Self::Selector>` method
- [x] Updated lib.rs re-exports

### 5.3 Implement RosSelector for oxidros-rcl ✅
- [x] Implemented `RosSelector` trait for `selector::Selector`
- [x] Delegates to existing Selector methods
- [x] Wraps handlers to match RCL signatures (e.g., `ServerCallback` includes `Header`)
- [x] Action server wraps goal_handler for `SendGoalServiceRequest<T>`, cancel_handler for `GoalInfo`
- [x] Added `inner_data()` accessor to `action::client::Client` for selector registration

### 5.4 Create Selector for oxidros-zenoh ✅
- [x] Created `oxidros-zenoh/src/selector.rs`
- [x] Implemented `Selector` struct with poll-based message handling
- [x] Timer implementation using `HashMap<u64, Timer>` with absolute `Instant` fire times
  - **Note**: Did not use `DeltaList` because its linked-list API (`insert`, `pop`, `filter`) differs from the HashMap approach; DeltaList uses relative time deltas optimized for RCL's wait semantics
- [x] Subscriber handlers use `try_recv()` polling in wait loop
- [x] Action methods return `Err(NotImplemented)` - Zenoh doesn't support actions yet
- [x] Implemented `RosSelector` trait for `Selector`
- [x] Updated lib.rs to export `Selector`

### 5.5 Implement RosContext::create_selector for both backends ✅
- [x] oxidros-rcl: delegates to `Context::create_selector()`
- [x] oxidros-zenoh: returns `Ok(Selector::new())`

---

## Phase 6: Update oxidros Crate (1 day)

### 6.1 Feature-gated re-exports
- [ ] Update `oxidros/Cargo.toml` to include oxidros-zenoh dependency
- [ ] Add `zenoh` feature flag
- [ ] Update `oxidros/src/lib.rs`:
  ```rust
  #[cfg(feature = "rcl")]
  pub use oxidros_rcl::*;
  
  #[cfg(feature = "zenoh")]
  pub use oxidros_zenoh::*;
  ```
- [ ] Make `rcl` and `zenoh` features mutually exclusive
- [ ] Update documentation

### 6.2 Update feature organization
- [ ] Rename `humble`/`jazzy`/`kilted` to `rcl-humble`/`rcl-jazzy`/`rcl-kilted`
- [ ] Add convenience features that enable both rcl + distro

---

## Phase 7: Testing & Documentation (1-2 days)

### 7.1 Add API compatibility tests
- [ ] Create test that compiles with both backends
- [ ] Test Context creation
- [ ] Test Node creation
- [ ] Test Publisher/Subscriber
- [ ] Test Client/Server

### 7.2 Update documentation
- [ ] Update oxidros-core README
- [ ] Update oxidros README with backend selection
- [ ] Add migration guide for API changes
- [ ] Document backend-specific features

---

## Phase 8: Action System for oxidros-zenoh (Future - 1-2 weeks)

> **Note:** This phase is deferred and can be done later.

### 8.1 Action Client
- [ ] Create `oxidros-zenoh/src/action/client.rs`
- [ ] Implement goal sending
- [ ] Implement result receiving
- [ ] Implement feedback subscription
- [ ] Implement goal cancellation

### 8.2 Action Server
- [ ] Create `oxidros-zenoh/src/action/server.rs`
- [ ] Implement goal receiving
- [ ] Implement result sending
- [ ] Implement feedback publishing
- [ ] Implement goal state machine

### 8.3 Action trait in oxidros-core
- [ ] Define `trait ActionClient<T: ActionMsg>`
- [ ] Define `trait ActionServer<T: ActionMsg>`
- [ ] Implement for oxidros-rcl
- [ ] Implement for oxidros-zenoh

---

## Summary

| Phase | Description | Estimated Time | Status |
|-------|-------------|----------------|--------|
| 1 | Error System Unification | 2-3 days | ✅ Complete |
| 2 | Trait Consolidation | 2-3 days | ✅ Complete |
| 3 | API Traits Definition | 3-4 days | ✅ Complete |
| 4 | Implement API Traits | 3-4 days | ✅ Complete |
| 5 | Selector Implementation | 2-3 days | ✅ Complete |
| 6 | Update oxidros Crate | 1 day | ⬜ Not Started |
| 7 | Testing & Documentation | 1-2 days | ⬜ Not Started |
| 8 | Action System (Future) | 1-2 weeks | ⬜ Deferred |

**Total (Phases 1-7): ~2-3 weeks**
**Completed: Phases 1-5**

---

## Architecture After Alignment

```
┌─────────────────────────────────────────────────────────────┐
│                         oxidros                              │
│  (feature = "rcl" OR feature = "zenoh")                     │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
┌─────────────────────────┐     ┌─────────────────────────┐
│      oxidros-rcl        │     │     oxidros-zenoh       │
│  (impl API traits)      │     │   (impl API traits)     │
└─────────────────────────┘     └─────────────────────────┘
              │                               │
              └───────────────┬───────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      oxidros-core                            │
│  - API traits (Context, Node, Publisher, Subscriber, etc.)  │
│  - Error types (unified Error enum)                         │
│  - Message traits (TypeSupport, ServiceMsg, ActionMsg)      │
│  - QoS types                                                │
│  - Parameter types                                          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                       ros2-types                             │
│  - TypeDescription trait & derive                           │
│  - RIHS type hash calculation                               │
│  - CDR serialization (native feature)                       │
│  (re-exports traits from oxidros-core)                      │
└─────────────────────────────────────────────────────────────┘
```

---

## Notes

- Backend-specific features (loaned messages in RCL, graph cache in Zenoh) remain on concrete types
- Common API exposed via traits allows generic code
- `oxidros` crate provides unified entry point with feature flags
