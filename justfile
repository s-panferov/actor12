# Actor12 Development Tasks

# Install documentation dependencies
docs-install:
    cd docs && npm install

# Start documentation development server
docs-dev:
    cd docs && npm run dev

# Build documentation for production
docs-build:
    cd docs && npm run build

# Preview documentation production build
docs-preview:
    cd docs && npm run preview

# Clean documentation build artifacts
docs-clean:
    rm -rf docs/dist docs/.astro

# Setup documentation (install deps and start dev server)
docs-setup:
    just docs-install
    just docs-dev

# Build and test documentation
docs-test:
    just docs-build
    just docs-preview

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
    just docs-install

# Run all checks
ci:
    cargo fmt --check
    cargo clippy -- -D warnings  
    cargo test
    just test-api-coverage
    just docs-build

# Preview what history cleaning would do (safe)
preview-clean-history:
    deno run --allow-run scripts/clean-history.ts --preview

# Clean git history to remove Claude references (DESTRUCTIVE!)
clean-history:
    deno run --allow-run --allow-read --allow-write scripts/clean-history.ts