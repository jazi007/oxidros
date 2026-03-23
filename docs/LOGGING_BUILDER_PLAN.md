# Logging Builder Plan

Common `LoggingBuilder` with `with_default_layers()` for composable tracing setup.

## User-Facing API (from `oxidros`)

```rust
use oxidros::logger::{LoggingBuilder, LoggingBuilderExt};

// Simple — same as today, backward compatible
oxidros::logger::init_ros_logging("my_node");

// Builder — default backend layers + custom extras
LoggingBuilder::new("my_node")
    .with_default_layers()          // adds RCL or Zenoh layer (cfg-gated)
    .with_layer(my_otel_layer)      // user can stack more layers
    .with_fmt_layer()               // optionally add stderr fmt output
    .init();

// Builder — fully custom, no backend defaults
LoggingBuilder::new("my_node")
    .with_filter("debug")
    .with_fmt_layer()
    .init();
```

## Phases

### Phase 1 — `LoggingBuilder` in `oxidros-core`

- [x] `oxidros-core/Cargo.toml`: Add `logging` feature with optional deps (`tracing-subscriber`, `tracing`, `tracing-log`)
- [x] New file `oxidros-core/src/logging.rs`: `LoggingBuilder` struct and methods
  - `new(name: &str) -> Self`
  - `name(&self) -> &str`
  - `with_filter(self, filter: &str) -> Self`
  - `with_layer(self, layer: impl Layer<Registry> + Send + Sync + 'static) -> Self`
  - `with_fmt_layer(self) -> Self`
  - `with_log_bridge(self, enabled: bool) -> Self`
  - `init(self)` — assembles registry + all layers, calls `try_init()`, guarded by `OnceLock`
- [x] `oxidros-core/src/lib.rs`: Add `#[cfg(feature = "logging")] pub mod logging;`

### Phase 2 — Refactor `oxidros-rcl` logger

- [x] `oxidros-rcl/Cargo.toml`: Enable `oxidros-core/logging` feature
- [x] `oxidros-rcl/src/logger.rs`: Add `pub fn with_default_layers(builder: LoggingBuilder) -> LoggingBuilder` (adds `RclLayer`)
- [x] `oxidros-rcl/src/logger.rs`: Refactor `init_ros_logging` to use `with_default_layers(LoggingBuilder::new(name)).init()`

### Phase 3 — Refactor `oxidros-zenoh` logger

- [x] `oxidros-zenoh/Cargo.toml`: Enable `oxidros-core/logging` feature
- [x] `oxidros-zenoh/src/logger.rs`: Add `pub fn with_default_layers(builder: LoggingBuilder) -> LoggingBuilder` (adds `ZenohLayer` + `fmt_layer`)
- [x] `oxidros-zenoh/src/logger.rs`: Refactor `init_ros_logging` to use `with_default_layers(LoggingBuilder::new(name)).init()`

### Phase 4 — `oxidros` facade: `with_default_layers()` via extension trait

- [x] `oxidros/Cargo.toml`: Enable `oxidros-core/logging` feature
- [x] `oxidros/src/logger.rs`: Re-export `LoggingBuilder`, add `LoggingBuilderExt` trait with `with_default_layers()`
  - `#[cfg(feature = "rcl")]` delegates to `oxidros_wrapper::logger::with_default_layers`
  - `#[cfg(feature = "zenoh")]` delegates to `oxidros_zenoh::logger::with_default_layers`
- [x] `oxidros/src/logger.rs`: Keep backward-compat `init_ros_logging` re-exports

### Phase 5 — Tests

- [x] `oxidros-core`: Unit test `LoggingBuilder` alone (no backend layers, just filter + fmt)
- [x] `oxidros-rcl` / `oxidros-zenoh`: Verify existing tests still pass (backward compat)
- [x] `oxidros`: Integration test using `LoggingBuilder::new("test").with_default_layers().init()`

## File Change Summary

| File | Action |
|---|---|
| `oxidros-core/Cargo.toml` | Add `logging` feature + optional tracing deps |
| `oxidros-core/src/logging.rs` | **New** — `LoggingBuilder` |
| `oxidros-core/src/lib.rs` | Add `pub mod logging` (feature-gated) |
| `oxidros-rcl/Cargo.toml` | Enable `oxidros-core/logging` |
| `oxidros-rcl/src/logger.rs` | Add `with_default_layers()`, refactor `init_ros_logging` |
| `oxidros-zenoh/Cargo.toml` | Enable `oxidros-core/logging` |
| `oxidros-zenoh/src/logger.rs` | Add `with_default_layers()`, refactor `init_ros_logging` |
| `oxidros/Cargo.toml` | Enable `oxidros-core/logging` |
| `oxidros/src/logger.rs` | Re-export `LoggingBuilder`, add `LoggingBuilderExt` trait |
