# Available recipes
default:
    @just --list

# Detect backend feature from environment
_backend := if env("ROS_DISTRO", "") != "" { "rcl" } else { "zenoh" }
# Exclude RCL-only crates when building with zenoh
_exclude := if env("ROS_DISTRO", "") != "" { "" } else { "--exclude oxidros-wrapper --exclude oxidros-rcl" }

# Run all tests in workspace (backend auto-detected from ROS_DISTRO)
test:
    cargo test --workspace {{ _exclude }} --no-default-features --features {{ _backend }}

# Run clippy linter on all crates (backend auto-detected from ROS_DISTRO)
clippy:
    cargo clippy --workspace {{ _exclude }} --all-targets --no-default-features --features {{ _backend }} -- -D warnings

# Format code with rustfmt (check only)
fmt:
    cargo fmt --all -- --check
    taplo fmt --check

# Format code with rustfmt (apply changes)
fmt-fix:
    cargo fmt --all
    taplo fmt

# Run all checks (fmt, clippy, test)
check: fmt clippy test
    @echo "All checks passed!"

# Build all crates in workspace
build:
    cargo build --all

# Build all crates in release mode
build-release:
    cargo build --all --release

# Generate documentation for all crates
doc:
    cargo doc --all --no-deps --open

# Clean build artifacts
clean:
    cargo clean

# Run IDL parser conformance tests against ROS2
idl-conformance *ARGS:
    ./scripts/run_idl_conformance.sh {{ARGS}}

# Run TypeDescription hash validation against ROS2
type-hash-validation *ARGS:
    ./scripts/run_type_hash_validation.sh {{ARGS}}

# Run all validation tests (IDL conformance + type hash)
validate: idl-conformance type-hash-validation
    @echo "All validation tests passed!"

# Generate API reference documentation comparing wrapper and zenoh backends
# Requires: cargo-public-api, ROS2 sourced (for oxidros-wrapper)
api-docs:
    python3 scripts/generate_api_docs.py

# Generate API docs with custom output path
api-docs-to FILE:
    python3 scripts/generate_api_docs.py --output {{FILE}}

# Generate partial API docs (zenoh-only, no ROS2 required)
api-docs-force:
    python3 scripts/generate_api_docs.py --force
