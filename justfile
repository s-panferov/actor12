# Actor12 Development Tasks

# Build documentation with rustdoc
docs-build:
    cargo doc --no-deps --document-private-items

# Build and open documentation
docs-open:
    cargo doc --no-deps --document-private-items --open

# Test documentation (compile doc tests)
docs-test:
    cargo test --doc

# Clean documentation build artifacts
docs-clean:
    rm -rf target/doc

# Check documentation for broken links and warnings
docs-check:
    cargo doc --no-deps

# Typecheck and test everything (code, tests, and docs)
check-all:
    cargo check
    cargo nextest run
    cargo test --doc
    cargo doc --no-deps

# Run Rust tests
test:
    cargo test

# Run examples
example name:
    cargo run --example {{name}}

# Run the comprehensive API coverage test
test-api-coverage:
    cargo run --example api_coverage_test

# Format code
fmt:
    cargo fmt

# Check code (no formatting)
check:
    cargo check

# Lint code
clippy:
    cargo clippy -- -D warnings

# Clean Rust build artifacts
clean:
    cargo clean

# Full development setup
setup:
    cargo build

# Run all checks
ci:
    cargo fmt --check
    cargo clippy -- -D warnings  
    cargo test
    just test-api-coverage
    just docs-test
    just docs-check

# Preview what history cleaning would do (safe)
preview-clean-history:
    deno run --allow-run scripts/clean-history.ts --preview

# Clean git history to remove Claude references (DESTRUCTIVE!)
clean-history:
    deno run --allow-run --allow-read --allow-write scripts/clean-history.ts