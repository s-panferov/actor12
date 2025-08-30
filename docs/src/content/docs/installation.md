---
title: Installation
description: How to install and set up Actor12 in your Rust project
---

# Installation

## Prerequisites

Actor12 requires Rust 1.70+ and works with the latest stable version of Rust. Make sure you have Rust installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable
```

## Adding Actor12 to Your Project

Add Actor12 to your `Cargo.toml`:

```toml
[dependencies]
actor12 = "0.1"
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"  # For error handling in examples
```

## Features

Actor12 has minimal dependencies and no optional features - everything you need is included by default:

- **Core actor framework**: Actor trait, Link, WeakLink
- **Message handling**: Envelope and Handler patterns  
- **Async runtime**: Built on tokio
- **Cancellation**: Hierarchical cancellation tokens
- **Error handling**: Comprehensive error types

## Development Setup

If you want to contribute to Actor12 or run the examples:

```bash
git clone https://github.com/yourusername/actor12.git
cd actor12
cargo test
cargo run --example basic_counter
```

## IDE Support

Actor12 works great with:
- **Rust Analyzer**: Full type checking and completion
- **VS Code**: With the rust-analyzer extension
- **IntelliJ IDEA**: With the Rust plugin

## Next Steps

Now that you have Actor12 installed, check out the [Quick Start](/quick-start) guide to create your first actor!