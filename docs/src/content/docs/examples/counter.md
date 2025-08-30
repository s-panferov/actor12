---
title: Basic Counter Example
description: Learn Actor12 fundamentals with a simple counter actor
---

# Basic Counter Example

This example demonstrates the fundamentals of Actor12 with a simple counter actor that can increment its value and report the current count.

## Complete Example

```rust
use actor12::{Actor, Handler, Multi, Call, spawn, HandlerMessage};
use std::future::Future;

// The actor struct holds the state
pub struct CounterActor {
    value: i32,
}

// Messages the actor can handle
#[derive(Debug)]
pub struct Increment(pub i32);

#[derive(Debug)]
pub struct GetValue;

#[derive(Debug)]
pub struct Reset;

// Implement the Actor trait
impl Actor for CounterActor {
    type Spec = i32; // Initial value
    type Message = Multi<Self>; // Handle multiple message types
    type Channel = actor12::MpscChannel<Self::Message>;
    type Cancel = ();
    type State = ();

    fn state(_spec: &Self::Spec) -> Self::State {}

    fn init(ctx: actor12::Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        let initial_value = ctx.spec;
        futures::future::ready(Ok(CounterActor { value: initial_value }))
    }
}

// Implement handlers for each message type
impl Handler<Increment> for CounterActor {
    type Reply = Result<i32, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: Increment) -> Self::Reply {
        self.value += msg.0;
        println!("Counter incremented by {}, now: {}", msg.0, self.value);
        Ok(self.value)
    }
}

impl Handler<GetValue> for CounterActor {
    type Reply = Result<i32, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _msg: GetValue) -> Self::Reply {
        Ok(self.value)
    }
}

impl Handler<Reset> for CounterActor {
    type Reply = Result<(), anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _msg: Reset) -> Self::Reply {
        let old_value = self.value;
        self.value = 0;
        println!("Counter reset from {} to 0", old_value);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Counter Actor Demo ===");

    // Spawn the actor with initial value 10
    let counter = spawn::<CounterActor>(10);
    
    // Get initial value
    let initial = counter.send_and_reply(HandlerMessage::new(GetValue)).await??;
    println!("Initial value: {}", initial);

    // Increment using two-step API
    println!("\nUsing two-step API:");
    let handle = counter.send_message(HandlerMessage::new(Increment(5))).await;
    
    if handle.was_sent() {
        let result = handle.reply().await??;
        println!("Increment result: {}", result);
    } else {
        println!("Failed to send increment: {:?}", handle.send_error());
    }

    // Increment using convenience method
    println!("\nUsing convenience method:");
    let result = counter.send_and_reply(HandlerMessage::new(Increment(3))).await??;
    println!("Increment result: {}", result);

    // Fire-and-forget increment
    println!("\nFire-and-forget increment:");
    counter.send_and_forget(HandlerMessage::new(Increment(2))).await;
    
    // Check the value after fire-and-forget
    tokio::time::sleep(std::time::Duration::from_millis(10)).await; // Give it time
    let final_value = counter.send_and_reply(HandlerMessage::new(GetValue)).await??;
    println!("Value after fire-and-forget: {}", final_value);

    // Reset the counter
    println!("\nResetting counter:");
    counter.send_and_reply(HandlerMessage::new(Reset)).await??;
    
    let reset_value = counter.send_and_reply(HandlerMessage::new(GetValue)).await??;
    println!("Value after reset: {}", reset_value);

    Ok(())
}
```

## Key Concepts Demonstrated

### 1. Actor State
```rust
pub struct CounterActor {
    value: i32,  // Private state, only accessible by the actor
}
```

The actor encapsulates its state - no external code can directly access or modify the `value` field.

### 2. Message Types
```rust
#[derive(Debug)]
pub struct Increment(pub i32);  // Command with data

#[derive(Debug)]
pub struct GetValue;            // Query with no data

#[derive(Debug)]
pub struct Reset;               // Command with no data
```

Each message type represents a different operation the actor can perform.

### 3. Handler Implementation
```rust
impl Handler<Increment> for CounterActor {
    type Reply = Result<i32, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: Increment) -> Self::Reply {
        self.value += msg.0;        // Modify actor state
        Ok(self.value)              // Return new value
    }
}
```

Each message type gets its own handler with a specific reply type.

### 4. Two-Step API Usage
```rust
// Step 1: Send the message
let handle = counter.send_message(HandlerMessage::new(Increment(5))).await;

// Step 2: Handle the response  
if handle.was_sent() {
    let result = handle.reply().await??;
    println!("Result: {}", result);
}
```

The two-step API gives you control over response handling.

### 5. Convenience Methods
```rust
// Send and wait for response
let result = counter.send_and_reply(HandlerMessage::new(GetValue)).await??;

// Send and forget
counter.send_and_forget(HandlerMessage::new(Increment(1))).await;
```

For common patterns, convenience methods reduce boilerplate.

## Running the Example

To run this example:

1. Add dependencies to `Cargo.toml`:
```toml
[dependencies]
actor12 = "0.1"
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
futures = "0.3"
```

2. Save the code as `examples/counter.rs`

3. Run with:
```bash
cargo run --example counter
```

## Expected Output

```
=== Counter Actor Demo ===
Initial value: 10

Using two-step API:
Counter incremented by 5, now: 15
Increment result: 15

Using convenience method:
Counter incremented by 3, now: 18
Increment result: 18

Fire-and-forget increment:
Counter incremented by 2, now: 20
Value after fire-and-forget: 20

Resetting counter:
Counter reset from 20 to 0
Value after reset: 0
```

## Variations and Extensions

### Error Handling
```rust
impl Handler<Increment> for CounterActor {
    type Reply = Result<i32, CounterError>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: Increment) -> Self::Reply {
        if msg.0 < 0 {
            return Err(CounterError::NegativeIncrement);
        }
        
        if self.value.checked_add(msg.0).is_none() {
            return Err(CounterError::Overflow);
        }
        
        self.value += msg.0;
        Ok(self.value)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CounterError {
    #[error("Cannot increment by negative value")]
    NegativeIncrement,
    #[error("Counter overflow")]
    Overflow,
}
```

### Timeouts
```rust
use std::time::Duration;

// Increment with timeout
match counter.send_with_timeout(
    HandlerMessage::new(Increment(5)), 
    Duration::from_millis(100)
).await {
    Ok(Ok(result)) => println!("Success: {}", result),
    Ok(Err(e)) => println!("Actor error: {}", e),
    Err(actor12::MessageError::Timeout { timeout }) => {
        println!("Timed out after {:?}", timeout);
    }
    Err(e) => println!("Send error: {}", e),
}
```

### WeakLink Usage
```rust
// Create weak reference
let weak_counter = counter.downgrade();

// Use same API - gracefully handles dead actors
match weak_counter.send_and_reply(HandlerMessage::new(GetValue)).await {
    Ok(Ok(value)) => println!("Value: {}", value),
    Ok(Err(e)) => println!("Actor died: {:?}", e),
    Err(e) => println!("Send failed: {}", e),
}
```

## Next Steps

- Try the [Message Passing Example](/examples/messaging) to see inter-actor communication
- Learn about [Error Handling](/examples/errors) patterns  
- Explore [Timeouts](/examples/timeouts) for robust applications