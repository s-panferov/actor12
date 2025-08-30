# Actor12 Framework

A lightweight, high-performance actor framework for Rust built on Tokio. This is a standalone version of the actor system extracted from the Runy project.

## Features

- **Type-safe messaging**: Strongly typed actor messages with compile-time guarantees
- **Async/await support**: Built on Tokio for efficient async message handling
- **Cancellation**: Hierarchical cancellation system for clean shutdown
- **Memory tracking**: Built-in memory usage tracking for actors
- **Flexible channels**: Support for different message channel types

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
runy-actor = { path = "." }
tokio = { version = "1", features = ["full"] }
anyhow = "1.0"
futures = "0.3"
```

## Basic Example

```rust
use runy_actor::{Actor, Envelope, Exec, Init, MpscChannel, spawn};

// Define your actor
pub struct EchoServer;

// Define the message type
type EchoMessage = Envelope<String, anyhow::Result<String>>;

impl Actor for EchoServer {
    type Spec = ();
    type Message = EchoMessage;
    type Channel = MpscChannel<Self::Message>;
    type Cancel = ();
    type State = ();

    fn state(_spec: &Self::Spec) -> Self::State {}

    fn init(_ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        futures::future::ready(Ok(EchoServer))
    }

    async fn handle(&mut self, _ctx: Exec<'_, Self>, msg: Self::Message) {
        let response = format!("Echo: {}", msg.value);
        msg.reply.send(Ok(response)).unwrap();
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let echo = spawn::<EchoServer>(());
    let response: anyhow::Result<String> = echo.send("Hello".to_string()).await;
    println!("{}", response?);
    Ok(())
}
```

## Examples

Run the examples to see the framework in action:

### Basic Examples
```bash
# Simple echo server
cargo run --example echo_server

# Counter with state management  
cargo run --example simple_counter
```

### Handler Pattern Examples
```bash
# Multiple message types with Handler trait
cargo run --example handler_pattern

# Dynamic dispatch and routing
cargo run --example dynamic_dispatch
```

### Advanced Examples
```bash
# Ping-pong between actors (see examples/ directory)
cargo run --example ping_pong

# Bank account with transactions
cargo run --example bank_account

# Worker pool pattern
cargo run --example worker_pool
```

## Core Concepts

### Actors

Actors are the fundamental unit of computation. Each actor:
- Has its own state
- Processes messages sequentially 
- Can spawn child actors
- Communicates only through message passing

### Messages

Messages are sent through `Envelope<T, R>` where:
- `T` is the message payload type
- `R` is the expected response type

### Links

`Link<A>` provides a handle to send messages to actor `A`. Links are:
- Cloneable and thread-safe
- Used to send messages via `.send(message).await` for `Envelope<T, R>` messages
- Used to send typed messages via `.ask_dyn(message).await` for `Handler<M>` implementations
- Automatically handle response routing

### Handler Pattern

The `Handler<M>` trait allows actors to handle multiple message types:

```rust
impl Handler<String> for MyActor {
    type Reply = Result<String, anyhow::Error>;
    
    async fn handle(&mut self, ctx: Call<'_, Self, Self::Reply>, msg: String) -> Self::Reply {
        Ok(format!("Received: {}", msg))
    }
}
```

For actors using `Multi<A>` as their message type, you can:
- Implement `Handler<T>` for different message types `T`
- Use `.ask_dyn(message).await` to send messages dynamically
- Handle different types of requests in the same actor

### Cancellation

The framework provides hierarchical cancellation:
- Parent actors can cancel child actors
- Cancellation propagates through the actor tree
- Clean shutdown is handled automatically

## Testing

```bash
cargo test
```

## License

This project inherits the license from the original Runy project.