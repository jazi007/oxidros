# oxidros-core

Core traits and types for the oxidros ROS2 client library.

## Overview

`oxidros-core` provides the foundational abstractions for ROS2 functionality without
depending on any specific backend implementation. It defines the trait interfaces that
backends (RCL, Zenoh, etc.) implement.

## Key Components

- **API Traits** — `RosContext`, `RosNode`, `RosPublisher`, `RosSubscriber`, `RosClient`, `RosServer`, `RosSelector`
- **Message Traits** — `Message`, `MessageData`, `TypeSupport`, `ServiceMsg`, `ActionMsg`
- **QoS Profiles** — Quality of Service policies (reliability, durability, history, liveliness)
- **Parameter System** — `Parameter`, `Value`, `Descriptor` with range constraints
- **Time Types** — `UnsafeTime`, `UnsafeDuration`
- **Error Types** — `Error`, `RclError`, `ActionError`

## Feature Flags

| Feature   | Description |
|-----------|-------------|
| `yaml`    | YAML parameter file parsing |
| `logging` | ROS2-compatible logging via `tracing` |
