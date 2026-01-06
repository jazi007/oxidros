# Available recipes
default:
    @just --list

# Run all tests in workspace
test:
    cargo test --all

# Run clippy linter on all crates
clippy:
    cargo clippy --all --all-targets -- -D warnings

# Format code with rustfmt (check only)
fmt:
    cargo fmt --all -- --check

# Format code with rustfmt (apply changes)
fmt-fix:
    cargo fmt --all

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
