# Architecture Refactoring Plan

This document outlines the steps to refactor oxidros for a cleaner separation of concerns.

## Goals

- **oxidros-core**: Pure trait definitions with ergonomic async methods
- **oxidros-rcl**: Minimal FFI bindings only (advanced users)
- **oxidros-wrapper**: Implements core traits for rcl types (default rcl experience)
- **oxidros-zenoh**: Implements core traits for zenoh backend
- **oxidros**: Unified entry point with backend selection via features

## New Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         oxidros                              │
│  (unified API, feature-gated backend selection)             │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   ┌─────────────────────┐     ┌─────────────────────┐       │
│   │   oxidros-wrapper   │     │   oxidros-zenoh     │       │
│   │  (impl core traits  │     │  (impl core traits  │       │
│   │   for rcl types)    │     │   for zenoh types)  │       │
│   └──────────┬──────────┘     └─────────────────────┘       │
│              │                                               │
│   ┌──────────▼──────────┐                                   │
│   │    oxidros-rcl      │                                   │
│   │  (FFI only, no      │                                   │
│   │   trait impls)      │                                   │
│   └─────────────────────┘                                   │
│                                                              │
├─────────────────────────────────────────────────────────────┤
│                      oxidros-core                            │
│  (traits: RosPublisher, RosSubscriber, RosClient, etc.)     │
└─────────────────────────────────────────────────────────────┘
```

## Refactoring Steps

### Step 1: Update oxidros-core traits ✅

Add new ergonomic methods to core traits in `oxidros-core/src/api/mod.rs`:

- [x] Add `futures-core` dependency to `oxidros-core/Cargo.toml`
- [x] `RosPublisher<T>`:
  - [x] Add `send_raw()` for raw bytes publishing
  - [x] Add `send_many()` with default impl
- [x] `RosSubscriber<T>`:
  - [x] Add `recv_many()` with default impl
  - [x] Add `into_stream() -> MessageStream<T>`
- [x] `RosClient<T>`:
  - [x] Add `call_with_retry()` - waits for availability + timeout retry
- [x] `RosServer<T>`:
  - [x] Add `serve()` - callback-based serving loop

### Step 2: Remove core trait impls from oxidros-rcl ✅

Remove implementations of `oxidros_core::api::*` traits:

- [x] `oxidros-rcl/src/context.rs` - Remove `impl RosContext for Context`
- [x] `oxidros-rcl/src/node.rs` - Remove `impl RosNode for Node`
- [x] `oxidros-rcl/src/selector/mod.rs` - Remove `impl RosSelector for Selector`
- [x] `oxidros-rcl/src/topic/publisher.rs` - Remove `impl RosPublisher for Publisher`
- [x] `oxidros-rcl/src/topic/subscriber.rs` - Remove `impl RosSubscriber for Subscriber`
- [x] `oxidros-rcl/src/service/client.rs` - Remove `impl RosClient for Client`
- [x] `oxidros-rcl/src/service/server.rs` - Remove `impl RosServer for Server`

### Step 3: Update oxidros-wrapper ✅

Transform oxidros-wrapper to use newtype pattern for core trait implementations:

**Note**: Due to Rust's orphan rules, we use the newtype pattern - wrapper types
defined in oxidros-wrapper that wrap rcl types and implement core traits.

- [x] Create newtype wrappers: `Context`, `Node`, `Publisher`, `Subscriber`, `Client`, `Server`, `Selector`
- [x] Implement `Deref`/`DerefMut` for transparent access to inner types
- [x] Implement all core traits (`RosContext`, `RosNode`, `RosPublisher`, etc.)
- [x] Create `SubscriberStream` wrapper for async streaming
- [x] Create `ServiceRequest` wrapper for service requests
- [x] Re-export core traits and message types for user convenience

### Step 4: Update oxidros-zenoh ✅

Add implementations for new trait methods:

- [x] `RosPublisher::send_raw()` - send raw CDR bytes
- [x] `RosPublisher::send_many()` - uses default implementation
- [x] `RosSubscriber::recv_many()` - uses default implementation
- [x] `RosSubscriber::into_stream()` - implement with `SubscriberStream`
- [x] `RosClient::call_with_retry()` - implement with timeout and retry
- [x] `RosServer::serve()` - implement callback loop

### Step 5: Update oxidros-build with auto-detection

Centralize backend detection in `oxidros-build` so all crates get consistent behavior:

- [ ] Add detection functions to `oxidros-build/src/lib.rs`:
  ```rust
  pub enum Backend {
      Rcl(RosDistro),
      Zenoh,
  }
  
  pub enum RosDistro {
      Humble,
      Jazzy,
      Kilted,
  }
  
  pub fn detect_backend() -> Backend { ... }
  pub fn emit_backend_cfg() { ... }  // Emits cargo:rustc-cfg directives
  ```

- [ ] Update `oxidros-build/Cargo.toml` if needed

#### oxidros-build Implementation

```rust
// oxidros-build/src/lib.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RosDistro {
    Humble,
    Jazzy,
    Kilted,
}

impl RosDistro {
    pub fn as_str(&self) -> &'static str {
        match self {
            RosDistro::Humble => "humble",
            RosDistro::Jazzy => "jazzy",
            RosDistro::Kilted => "kilted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Rcl(RosDistro),
    Zenoh,
}

/// Detect backend from environment and cargo features
pub fn detect_backend() -> Backend {
    // 1. Check for explicit feature flags (set by downstream crate)
    if std::env::var("CARGO_FEATURE_ZENOH").is_ok() {
        return Backend::Zenoh;
    }
    if std::env::var("CARGO_FEATURE_HUMBLE").is_ok() {
        return Backend::Rcl(RosDistro::Humble);
    }
    if std::env::var("CARGO_FEATURE_JAZZY").is_ok() {
        return Backend::Rcl(RosDistro::Jazzy);
    }
    if std::env::var("CARGO_FEATURE_KILTED").is_ok() {
        return Backend::Rcl(RosDistro::Kilted);
    }
    
    // 2. Auto-detect only if "auto" feature is enabled
    if std::env::var("CARGO_FEATURE_AUTO").is_ok() {
        if let Some(backend) = detect_from_environment() {
            return backend;
        }
    }
    
    // 3. Default to Zenoh (no ROS2 installation required)
    Backend::Zenoh
}

fn detect_from_environment() -> Option<Backend> {
    // Check AMENT_PREFIX_PATH
    if let Ok(ament_path) = std::env::var("AMENT_PREFIX_PATH") {
        if let Some(distro) = detect_distro_from_path(&ament_path) {
            return Some(Backend::Rcl(distro));
        }
    }
    
    // Check ROS_DISTRO env var
    if let Ok(distro) = std::env::var("ROS_DISTRO") {
        match distro.as_str() {
            "humble" => return Some(Backend::Rcl(RosDistro::Humble)),
            "jazzy" => return Some(Backend::Rcl(RosDistro::Jazzy)),
            "kilted" => return Some(Backend::Rcl(RosDistro::Kilted)),
            _ => {}
        }
    }
    
    None
}

fn detect_distro_from_path(path: &str) -> Option<RosDistro> {
    for component in path.split(':') {
        if component.contains("/opt/ros/") || component.contains("/ros/") {
            if component.contains("humble") { return Some(RosDistro::Humble); }
            if component.contains("jazzy") { return Some(RosDistro::Jazzy); }
            if component.contains("kilted") { return Some(RosDistro::Kilted); }
        }
    }
    None
}

/// Emit cargo:rustc-cfg directives for conditional compilation
/// Call this from your build.rs
pub fn emit_backend_cfg() {
    let backend = detect_backend();
    
    match backend {
        Backend::Rcl(distro) => {
            println!("cargo:rustc-cfg=backend=\"rcl\"");
            println!("cargo:rustc-cfg=ros_distro=\"{}\"", distro.as_str());
        }
        Backend::Zenoh => {
            println!("cargo:rustc-cfg=backend=\"zenoh\"");
        }
    }
    
    // Re-run if environment changes
    println!("cargo:rerun-if-env-changed=AMENT_PREFIX_PATH");
    println!("cargo:rerun-if-env-changed=ROS_DISTRO");
}
```

### Step 6: Update oxidros main crate

Update feature flags and use `oxidros-build` for detection:

- [ ] Update `Cargo.toml`:
  ```toml
  [features]
  default = ["auto"]
  
  # Auto-detect: build.rs chooses rcl or zenoh based on environment
  auto = []
  
  # Explicit backend selection (mutually exclusive)
  rcl = ["dep:oxidros-wrapper"]
  zenoh = ["dep:oxidros-zenoh"]
  
  # RCL distribution features (imply rcl)
  humble = ["rcl", "oxidros-wrapper/humble"]
  jazzy = ["rcl", "oxidros-wrapper/jazzy"]
  kilted = ["rcl", "oxidros-wrapper/kilted"]
  
  [build-dependencies]
  oxidros-build = { path = "../oxidros-build" }
  ```

- [ ] Simplify `build.rs`:
  ```rust
  // oxidros/build.rs
  fn main() {
      oxidros_build::emit_backend_cfg();
  }
  ```

- [x] Update `lib.rs` to conditionally re-export based on features
- [x] Ensure unified API regardless of backend

Then in `lib.rs`:

```rust
// oxidros/src/lib.rs

#[cfg(backend = "rcl")]
pub use oxidros_wrapper::*;

#[cfg(backend = "zenoh")]
pub use oxidros_zenoh::*;

// Unified re-exports work because both implement the same core traits
pub use oxidros_core::{
    RosContext, RosNode, RosPublisher, RosSubscriber, 
    RosClient, RosServer, RosSelector,
};
```

#### Feature Selection Matrix

| User's Cargo.toml | Environment | Result |
|-------------------|-------------|--------|
| `features = ["auto"]` | ROS2 Jazzy sourced | Uses `oxidros-wrapper` with jazzy |
| `features = ["auto"]` | No ROS2 | Uses `oxidros-zenoh` |
| `features = ["zenoh"]` | Any | Always uses `oxidros-zenoh` |
| `features = ["jazzy"]` | Any | Always uses `oxidros-wrapper` with jazzy |
| `features = ["rcl"]` | ROS2 Humble sourced | Uses `oxidros-wrapper`, auto-detects humble |

#### Benefits of Centralizing in oxidros-build

1. **Consistency**: All crates in the workspace use identical detection logic
2. **Reusability**: Downstream crates can call `oxidros_build::detect_backend()` in their build.rs
3. **Single source of truth**: Update detection logic once, applies everywhere
4. **Feature propagation**: User's feature choice (auto/zenoh/rcl) is respected uniformly

#### Compile-time vs Runtime

- **Compile-time**: Backend is selected at build time via `build.rs`
- **Advantage**: No runtime overhead, dead code elimination
- **Limitation**: Cannot switch backends without recompiling
- **Future**: Could add runtime selection via dynamic loading if needed

### Step 7: Update documentation and examples

- [ ] Update `README.md` with new architecture
- [ ] Update `docs/API_REFERENCE.md`
- [ ] Update examples to use new API
- [ ] Add migration guide for existing users

### Step 8: Testing

- [ ] Run existing tests with rcl backend
- [ ] Run existing tests with zenoh backend
- [ ] Test auto-detection feature
- [ ] Cross-backend interop tests (rcl node ↔ zenoh node)

## New Trait Signatures (Reference)

```rust
// oxidros-core/src/api/mod.rs

pub trait RosPublisher<T: TypeSupport>: Send + Sync {
    fn topic_name(&self) -> Result<Cow<'_, String>>;
    fn send(&self, msg: &T) -> Result<()>;
    
    /// Publish raw serialized bytes directly (no serialization)
    fn send_raw(&self, data: &[u8]) -> Result<()>;
    
    /// Publish multiple messages
    fn send_many<'a>(&self, messages: impl IntoIterator<Item = &'a T>) -> Result<()>
    where T: 'a
    {
        for msg in messages {
            self.send(msg)?;
        }
        Ok(())
    }
}

pub trait RosSubscriber<T: TypeSupport>: Send {
    fn topic_name(&self) -> Result<Cow<'_, String>>;
    fn recv(&mut self) -> impl Future<Output = Result<Message<T>>> + Send;
    fn try_recv(&mut self) -> Result<Option<Message<T>>>;
    
    /// Receive up to `limit` messages without blocking
    fn recv_many(&mut self, limit: usize) -> Result<Vec<Message<T>>> {
        let mut results = Vec::with_capacity(limit.min(64));
        while results.len() < limit {
            match self.try_recv()? {
                Some(msg) => results.push(msg),
                None => break,
            }
        }
        Ok(results)
    }
    
    /// Convert to an async Stream
    fn into_stream(self) -> impl Stream<Item = Result<Message<T>>> + Send
    where Self: Sized + 'static;
}

pub trait RosClient<T: ServiceMsg>: Send {
    fn service_name(&self) -> Result<Cow<'_, String>>;
    fn is_service_available(&self) -> bool;
    fn call(&mut self, request: &T::Request) 
        -> impl Future<Output = Result<Message<T::Response>>> + Send;
    
    /// Call with automatic retry on timeout, waits for service availability
    fn call_with_retry(
        &mut self,
        request: &T::Request,
        timeout: Duration,
    ) -> impl Future<Output = Result<Message<T::Response>>> + Send;
}

pub trait RosServer<T: ServiceMsg>: Send {
    type Request: ServiceRequest<T>;
    fn service_name(&self) -> Result<Cow<'_, String>>;
    fn recv(&mut self) -> impl Future<Output = Result<Self::Request>> + Send;
    fn try_recv(&mut self) -> Result<Option<Self::Request>>;
    
    /// Run a serving loop with the given handler
    fn serve<F>(self, handler: F) -> impl Future<Output = Result<()>> + Send
    where
        Self: Sized,
        F: FnMut(Message<T::Request>) -> T::Response + Send;
}
```

## Notes

- `futures-core` is preferred over `futures` for minimal dependencies
- Default implementations allow backends to override with optimized versions
- `oxidros-rcl` remains available for advanced users needing direct FFI access
- Consider renaming `oxidros-wrapper` to `oxidros-rcl-async` or similar in future
