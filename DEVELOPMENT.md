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

## Phase 3: API Traits Definition in oxidros-core (3-4 days)

### 3.1 Create Context trait
- [ ] Create `oxidros-core/src/api/mod.rs`
- [ ] Define `trait Context` with:
  - `fn new() -> Result<Arc<Self>>`
  - `fn create_node(&self, name: &str, namespace: Option<&str>) -> Result<Arc<Node>>`
  - `fn domain_id(&self) -> u32`

### 3.2 Create Node trait
- [ ] Define `trait Node` with:
  - `fn name(&self) -> &str`
  - `fn namespace(&self) -> &str`
  - `fn fully_qualified_name(&self) -> String`
  - `fn create_publisher<T>(...) -> Result<Publisher<T>>`
  - `fn create_subscriber<T>(...) -> Result<Subscriber<T>>`
  - `fn create_client<T>(...) -> Result<Client<T>>`
  - `fn create_server<T>(...) -> Result<Server<T>>`
  - `fn create_parameter_server(&self) -> Result<ParameterServer>`

### 3.3 Create Publisher trait
- [ ] Define `trait Publisher<T>` with:
  - `fn topic_name(&self) -> &str`
  - `fn send(&self, msg: &T) -> Result<()>`

### 3.4 Create Subscriber trait
- [ ] Define `trait Subscriber<T>` with:
  - `fn topic_name(&self) -> &str`
  - `async fn recv(&mut self) -> Result<ReceivedMessage<T>>`
  - `fn try_recv(&mut self) -> Result<Option<ReceivedMessage<T>>>`

### 3.5 Create Service traits
- [ ] Define `trait Client<T: ServiceMsg>` with:
  - `fn service_name(&self) -> &str`
  - `async fn call(&self, request: &T::Request) -> Result<T::Response>`
  - `async fn call_with_timeout(&self, request: &T::Request, timeout: Duration) -> Result<T::Response>`

- [ ] Define `trait Server<T: ServiceMsg>` with:
  - `fn service_name(&self) -> &str`
  - `async fn recv(&mut self) -> Result<ServiceRequest<T>>`
  - `fn try_recv(&mut self) -> Result<Option<ServiceRequest<T>>>`

### 3.6 Create common types
- [ ] Define `ReceivedMessage<T>` struct in oxidros-core
- [ ] Define `ServiceRequest<T>` struct in oxidros-core
- [ ] Move/unify `CallbackResult` enum

---

## Phase 4: Implement API Traits (3-4 days)

### 4.1 Implement traits in oxidros-rcl
- [ ] Implement `Context` trait for `context::Context`
- [ ] Implement `Node` trait for `node::Node`
- [ ] Implement `Publisher<T>` trait for `topic::publisher::Publisher<T>`
- [ ] Implement `Subscriber<T>` trait for `topic::subscriber::Subscriber<T>`
- [ ] Implement `Client<T>` trait for `service::client::Client<T>`
- [ ] Implement `Server<T>` trait for `service::server::Server<T>`

### 4.2 Implement traits in oxidros-zenoh
- [ ] Implement `Context` trait for `Context`
- [ ] Implement `Node` trait for `Node`
- [ ] Implement `Publisher<T>` trait for `topic::publisher::Publisher<T>`
- [ ] Implement `Subscriber<T>` trait for `topic::subscriber::Subscriber<T>`
- [ ] Implement `Client<T>` trait for `service::client::Client<T>`
- [ ] Implement `Server<T>` trait for `service::server::Server<T>`

### 4.3 Add compatibility aliases in oxidros-rcl
- [ ] Add `name()` as alias for `get_name()` on Node
- [ ] Add `namespace()` as alias for `get_namespace()` on Node  
- [ ] Add `topic_name()` as alias for `get_topic_name()` on Publisher
- [ ] Deprecate old method names with `#[deprecated]`

---

## Phase 5: Selector Implementation for oxidros-zenoh (2-3 days)

### 5.1 Create Selector using flume
- [ ] Create `oxidros-zenoh/src/selector.rs`
- [ ] Implement `Selector` struct using `flume::Selector`
- [ ] Implement `add_subscriber()` method
- [ ] Implement `add_server()` method
- [ ] Implement `add_wall_timer()` method
- [ ] Implement `add_parameter_server()` method
- [ ] Implement `wait()` method

### 5.2 Add Selector to Context
- [ ] Add `create_selector()` method to zenoh Context
- [ ] Update lib.rs exports

### 5.3 Define Selector trait in oxidros-core
- [ ] Define `trait Selector` with common API
- [ ] Implement trait for both rcl and zenoh Selectors

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
| 1 | Error System Unification | 2-3 days | ⬜ Not Started |
| 2 | Trait Consolidation | 2-3 days | ⬜ Not Started |
| 3 | API Traits Definition | 3-4 days | ⬜ Not Started |
| 4 | Implement API Traits | 3-4 days | ⬜ Not Started |
| 5 | Selector for Zenoh | 2-3 days | ⬜ Not Started |
| 6 | Update oxidros Crate | 1 day | ⬜ Not Started |
| 7 | Testing & Documentation | 1-2 days | ⬜ Not Started |
| 8 | Action System (Future) | 1-2 weeks | ⬜ Deferred |

**Total (Phases 1-7): ~2-3 weeks**

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
