---
title: Introduction to Actor12
description: A powerful Rust actor framework with an ergonomic two-step messaging API
---

# Introduction to Actor12

Actor12 is a modern Rust actor framework designed to make concurrent programming intuitive and safe. It provides a powerful actor model implementation with a unique two-step messaging API that gives you fine-grained control over message handling while maintaining simplicity.

## Why Actor12?

### 🔄 **Ergonomic Two-Step API**
Send a message, then decide what to do with the response:
```rust
let handle = actor.send_message(MyMessage("hello")).await;
let response = handle.reply().await?; // Or handle.forget() to ignore
```

### 🔗 **Flexible Message Patterns**
- **Envelope-style**: Traditional request-response patterns
- **Handler-style**: Type-safe message handling with automatic routing
- **Raw messages**: Direct actor communication

### 🎯 **Strong Type Safety**
Compile-time guarantees for message types and responses using Rust's powerful type system.

### 🔧 **Advanced Features**
- **Timeouts**: Built-in timeout support for all message patterns
- **WeakLink**: Safe references that don't prevent actor cleanup  
- **Cancellation**: Hierarchical cancellation with CancelToken
- **Error Handling**: Comprehensive error types for different failure modes

### 🚀 **High Performance**
Built on tokio with efficient async/await patterns and minimal overhead.

## Core Concepts

Actor12 is built around a few key concepts:

- **Actors**: Stateful entities that process messages sequentially
- **Messages**: Data sent between actors (envelopes or handler-style)
- **Links**: Strong references to actors for sending messages
- **WeakLinks**: Weak references that allow graceful cleanup
- **Handlers**: Type-safe message processing traits

## Getting Started

Ready to dive in? Check out the [Installation](/installation) guide to add Actor12 to your project, or jump straight to the [Quick Start](/quick-start) to see it in action.

## Community & Support

Actor12 is an open-source project. You can find the source code, report issues, and contribute on [GitHub](https://github.com/yourusername/actor12).