# Available recipes
default:
    @just --list

# Run all tests in workspace
test:
    cargo test --all --all-features

# Generate code coverage report
coverage:
    ./scripts/coverage.sh

# Run clippy linter on all crates
clippy:
    cargo clippy --all --all-targets --all-features -- -D warnings

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
    cargo build --all --all-features

# Build all crates in release mode
build-release:
    cargo build --all --release --all-features

# Generate documentation for all crates
doc:
    cargo doc --all --all-features --no-deps --open

# Clean build artifacts
clean:
    cargo clean
    rm -rf coverage/
    rm -f tarpaulin-report.html cobertura.xml
