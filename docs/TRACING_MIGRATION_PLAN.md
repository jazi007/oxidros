# Tracing Migration Plan

This document outlines the step-by-step plan to migrate from statistics features and custom logging to the `tracing` crate across oxidros crates.

## Overview

**Goals:**
1. Remove `statistics`/`rcl_stat` features from `oxidros-rcl`
2. Replace all statistics collection with `tracing::debug!` instrumentation
3. Remove custom `logger::Logger` and `pr_*!` macros, replacing with `tracing` macros
4. Add tracing instrumentation to `oxidros-zenoh`
5. Add tracing instrumentation to `oxidros-core` traits

---

## Phase 1: oxidros-rcl - Remove Statistics Features

### Step 1.1: Update Cargo.toml

**File:** [oxidros-rcl/Cargo.toml](../oxidros-rcl/Cargo.toml)

Remove the following features:
```toml
# Remove these lines:
rcl_stat = ["statistics"]
statistics = ["serde", "serde_json"]
```

Remove optional dependencies used only for statistics:
```toml
# Remove optional marker for serde/serde_json if only used for statistics
```

### Step 1.2: Remove Statistics Module

**File:** [oxidros-rcl/src/helper.rs](../oxidros-rcl/src/helper.rs#L3-L75)

- Remove the entire `#[cfg(feature = "statistics")] pub(crate) mod statistics { ... }` block
- Keep only `pub(crate) const fn is_unpin<T: Unpin>() {}`

### Step 1.3: Update Publisher

**File:** [oxidros-rcl/src/topic/publisher.rs](../oxidros-rcl/src/topic/publisher.rs)

Changes required:
1. Remove imports:
   ```rust
   // Remove:
   #[cfg(feature = "rcl_stat")]
   use crate::helper::statistics::{SerializableTimeStat, TimeStatistics};
   
   #[cfg(feature = "rcl_stat")]
   use parking_lot::Mutex;
   ```

2. Remove struct field from `Publisher<T>`:
   ```rust
   // Remove:
   #[cfg(feature = "rcl_stat")]
   latency_publish: Mutex<TimeStatistics<4096>>,
   ```

3. Remove initialization in `new()` and `new_disable_loaned_message()`:
   ```rust
   // Remove:
   #[cfg(feature = "rcl_stat")]
   latency_publish: Mutex::new(TimeStatistics::new()),
   ```

4. Replace stats collection with tracing in `send()`, `send_raw()`, `send_serialized()`:
   ```rust
   // Replace this pattern:
   let start = std::time::SystemTime::now();
   // ... operation ...
   #[cfg(feature = "rcl_stat")]
   {
       if let Ok(dur) = start.elapsed() {
           let mut guard = self.latency_publish.lock();
           guard.add(dur);
       }
   }
   
   // With:
   let start = std::time::Instant::now();
   // ... operation ...
   tracing::debug!(
       target: "oxidros::publisher",
       latency_us = start.elapsed().as_micros(),
       "rcl_publish completed"
   );
   ```

5. Remove `statistics()` method:
   ```rust
   // Remove entire method:
   #[cfg(feature = "rcl_stat")]
   pub fn statistics(&self) -> SerializableTimeStat { ... }
   ```

### Step 1.4: Update Subscriber

**File:** [oxidros-rcl/src/topic/subscriber.rs](../oxidros-rcl/src/topic/subscriber.rs)

Changes required:
1. Remove imports (lines 180-183):
   ```rust
   // Remove:
   #[cfg(feature = "rcl_stat")]
   use crate::helper::statistics::{SerializableTimeStat, TimeStatistics};
   ```

2. Remove stats field from `RCLSubscription` (line 188-189):
   ```rust
   // Remove:
   #[cfg(feature = "rcl_stat")]
   pub latency_take: Mutex<TimeStatistics<4096>>,
   ```

3. Remove initialization in constructors (lines 249-250, 281-282):
   ```rust
   // Remove:
   #[cfg(feature = "rcl_stat")]
   latency_take: Mutex::new(TimeStatistics::new()),
   ```

4. Replace stats in `try_recv()` with tracing (around lines 336-350):
   ```rust
   // Replace:
   #[cfg(feature = "rcl_stat")]
   let start = std::time::SystemTime::now();
   // ... take operation ...
   #[cfg(feature = "rcl_stat")]
   {
       if let Ok(dur) = start.elapsed() {
           let mut guard = self.subscription.latency_take.lock();
           guard.add(dur);
       }
   }
   
   // With:
   let start = std::time::Instant::now();
   // ... take operation ...
   tracing::debug!(
       target: "oxidros::subscriber",
       latency_us = start.elapsed().as_micros(),
       "rcl_take completed"
   );
   ```

5. Remove `statistics()` method (lines 428-433):
   ```rust
   // Remove entire method:
   #[cfg(feature = "rcl_stat")]
   pub fn statistics(&self) -> SerializableTimeStat { ... }
   ```

### Step 1.5: Update Selector

**File:** [oxidros-rcl/src/selector/mod.rs](../oxidros-rcl/src/selector/mod.rs)

Changes required:
1. Remove all `#[cfg(feature = "statistics")]` blocks
2. Remove `#[cfg_attr(feature = "statistics", allow(dead_code))]` attributes
3. Remove `TimeStat` struct and related statistics code
4. Replace with tracing instrumentation where timing info is useful:
   ```rust
   tracing::debug!(
       target: "oxidros::selector",
       callback_name = %name,
       "executing callback"
   );
   ```

---

## Phase 2: oxidros-rcl - Replace Logger with Tracing

### Step 2.1: Update Internal Logger Usage

Replace all internal uses of `Logger`, `pr_info_in!`, `pr_error_in!`, etc. with tracing macros.

**Files to update:**

1. **[oxidros-rcl/src/signal_handler.rs](../oxidros-rcl/src/signal_handler.rs#L148-L172)**
   ```rust
   // Replace:
   let logger = Logger::new("oxidros");
   pr_info_in!(logger, "Received signal: {signal}");
   
   // With:
   tracing::info!(target: "oxidros", signal = signal, "Received signal");
   ```

2. **[oxidros-rcl/src/action/handle.rs](../oxidros-rcl/src/action/handle.rs#L127)**
   ```rust
   // Replace pr_error_in! with tracing::error!
   ```

3. **[oxidros-rcl/src/action/server.rs](../oxidros-rcl/src/action/server.rs#L577)**
   ```rust
   // Replace pr_error_in! with tracing::error!
   ```

### Step 2.2: Keep Logger Module for User-Facing API

The `Logger` struct and `pr_*!` macros are part of the public API (used in examples and docs). These should be kept but marked as deprecated with migration guidance.

**File:** [oxidros-rcl/src/logger.rs](../oxidros-rcl/src/logger.rs)

Add deprecation notices:
```rust
/// Logger for ROS2.
/// 
/// # Deprecated
/// 
/// This type is deprecated. Use `tracing` macros directly instead:
/// - `pr_info!` → `tracing::info!`
/// - `pr_warn!` → `tracing::warn!`
/// - `pr_error!` → `tracing::error!`
/// - `pr_debug!` → `tracing::debug!`
#[deprecated(since = "0.5.0", note = "Use tracing macros directly")]
pub struct Logger { ... }
```

### Step 2.3: Update Documentation Examples

Update all documentation examples to use tracing instead of `pr_*!` macros:

**Files with doc examples to update:**
- [oxidros-rcl/src/lib.rs](../oxidros-rcl/src/lib.rs) (lines 113, 124, 189, 206, 211)
- [oxidros-rcl/src/topic/subscriber.rs](../oxidros-rcl/src/topic/subscriber.rs) (lines 47-49, 92-97)
- [oxidros-rcl/src/service/server.rs](../oxidros-rcl/src/service/server.rs) (multiple examples)
- [oxidros-rcl/src/parameter.rs](../oxidros-rcl/src/parameter.rs) (lines 74, 155)

Example transformation:
```rust
// Before:
pr_info!(logger, "Received: msg = {}", msg.data);

// After:
tracing::info!(data = %msg.data, "Received message");
```

---

## Phase 3: oxidros-zenoh - Add Tracing Instrumentation

### Step 3.1: Add Debug Tracing to Publisher

**File:** [oxidros-zenoh/src/topic/publisher.rs](../oxidros-zenoh/src/topic/publisher.rs)

Add instrumentation:
```rust
impl<T: TypeSupport> Publisher<T> {
    pub(crate) fn new(...) -> Result<Self> {
        tracing::debug!(
            target: "oxidros::zenoh::publisher",
            topic = %fq_topic_name,
            type_name = %type_name,
            "Creating publisher"
        );
        // ... existing code ...
    }

    pub fn send(&self, msg: &T) -> Result<()> {
        let start = std::time::Instant::now();
        // ... publish logic ...
        tracing::debug!(
            target: "oxidros::zenoh::publisher",
            topic = %self.fq_topic_name,
            seq = seq_num,
            latency_us = start.elapsed().as_micros(),
            "Published message"
        );
        Ok(())
    }
}
```

### Step 3.2: Add Debug Tracing to Subscriber

**File:** [oxidros-zenoh/src/topic/subscriber.rs](../oxidros-zenoh/src/topic/subscriber.rs)

Add instrumentation:
```rust
impl<T: TypeSupport> Subscriber<T> {
    pub(crate) fn new(...) -> Result<Self> {
        tracing::debug!(
            target: "oxidros::zenoh::subscriber",
            topic = %fq_topic_name,
            type_name = %type_name,
            "Creating subscriber"
        );
        // ... existing code ...
    }

    pub async fn recv(&mut self) -> Result<Message<T>> {
        // ... receive logic ...
        tracing::debug!(
            target: "oxidros::zenoh::subscriber",
            topic = %self.fq_topic_name,
            seq = ?msg.sequence_number,
            "Received message"
        );
        // ...
    }
}
```

### Step 3.3: Add Debug Tracing to Service Client/Server

**Files:**
- [oxidros-zenoh/src/service/client.rs](../oxidros-zenoh/src/service/client.rs)
- [oxidros-zenoh/src/service/server.rs](../oxidros-zenoh/src/service/server.rs)

Replace existing `tracing::error!` and `tracing::warn!` with more comprehensive instrumentation:
```rust
// Add debug for request/response flow:
tracing::debug!(
    target: "oxidros::zenoh::service",
    service = %service_name,
    "Sending request"
);

tracing::debug!(
    target: "oxidros::zenoh::service",
    service = %service_name,
    latency_ms = elapsed.as_millis(),
    "Received response"
);
```

### Step 3.4: Add Tracing to Node/Context

**Files:**
- [oxidros-zenoh/src/node.rs](../oxidros-zenoh/src/node.rs)
- [oxidros-zenoh/src/context.rs](../oxidros-zenoh/src/context.rs)

Add lifecycle tracing:
```rust
tracing::debug!(
    target: "oxidros::zenoh",
    node = %name,
    namespace = %namespace,
    "Node created"
);

tracing::debug!(
    target: "oxidros::zenoh",
    domain_id = domain_id,
    "Context initialized"
);
```

---

## Phase 4: oxidros-core - Add Tracing Instrumentation

### Step 4.1: Add Tracing Dependency

**File:** [oxidros-core/Cargo.toml](../oxidros-core/Cargo.toml)

```toml
[dependencies]
# ... existing deps ...
tracing = "0.1"

[features]
default = []
# Remove statistics feature
```

### Step 4.2: Add #[instrument] to Core Traits

Consider adding `#[tracing::instrument]` attributes to trait method implementations for automatic span creation.

**File:** [oxidros-core/src/api/mod.rs](../oxidros-core/src/api/mod.rs)

Since traits can't have default instrumentation, document recommended patterns:

```rust
/// A ROS2 publisher that can send messages to a topic.
///
/// # Tracing
///
/// Implementations should emit tracing events at key points:
/// - `tracing::debug!` on `send()` completion
/// - `tracing::error!` on failures
pub trait RosPublisher<T: TypeSupport>: Send + Sync {
    // ...
}
```

### Step 4.3: Add Instrumentation Helper Module

**File:** Create `oxidros-core/src/tracing_helper.rs`

```rust
//! Tracing instrumentation helpers for oxidros.
//!
//! Provides common tracing targets and span helpers.

/// Target for publisher-related tracing events.
pub const TARGET_PUBLISHER: &str = "oxidros::publisher";

/// Target for subscriber-related tracing events.
pub const TARGET_SUBSCRIBER: &str = "oxidros::subscriber";

/// Target for service client/server tracing events.
pub const TARGET_SERVICE: &str = "oxidros::service";

/// Target for action-related tracing events.
pub const TARGET_ACTION: &str = "oxidros::action";

/// Target for node lifecycle tracing events.
pub const TARGET_NODE: &str = "oxidros::node";
```

---

## Phase 5: Testing & Validation

### Step 5.1: Build All Crates Without Statistics

```bash
# Verify clean build without statistics feature
cargo build --workspace

# Verify no compilation errors
cargo check --workspace
```

### Step 5.2: Run Test Suite

```bash
# Run all tests
cargo test --workspace

# Run with tracing enabled
RUST_LOG=debug cargo test --workspace
```

### Step 5.3: Verify Tracing Output

Create a simple test to verify tracing output:
```rust
#[test]
fn test_tracing_output() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    
    // Create publisher/subscriber and verify debug output
}
```

---

## Summary Checklist

### oxidros-rcl
- [x] Remove `rcl_stat` and `statistics` features from Cargo.toml
- [x] Remove `statistics` module from helper.rs
- [x] Remove stats from Publisher struct and methods
- [x] Remove stats from Subscriber struct and methods  
- [x] Remove stats from Selector
- [x] Replace internal `pr_*_in!` with `tracing::*!`
- [x] Deprecate public `Logger` and `pr_*!` macros
- [ ] Update documentation examples

### oxidros-zenoh
- [x] Add tracing to Publisher (new, send)
- [x] Add tracing to Subscriber (new, recv)
- [x] Add tracing to Client (call)
- [x] Add tracing to Server (serve)
- [x] Add tracing to Node/Context lifecycle

### oxidros-core
- [x] Add tracing dependency
- [x] Remove statistics feature
- [x] Document tracing conventions for trait implementors
- [x] Add tracing target constants

---

## Migration Guide for Users

Users who rely on `Logger` and `pr_*!` macros should migrate to tracing:

```rust
// Before (deprecated)
use oxidros_rcl::{logger::Logger, pr_info};
let logger = Logger::new("my_node");
pr_info!(logger, "Message: {}", value);

// After (recommended)
use tracing::info;
info!(target: "my_node", value = %value, "Message");
```

To capture tracing output, initialize a subscriber:
```rust
fn main() {
    tracing_subscriber::fmt::init();
    // or for more control:
    tracing_subscriber::fmt()
        .with_env_filter("oxidros=debug,my_node=info")
        .init();
}
```
