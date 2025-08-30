---
title: Quick Start
description: Get up and running with Actor12 in minutes
---

# Quick Start

Let's build a simple counter actor to demonstrate Actor12's two-step messaging API.

## Your First Actor

Here's a complete example of a counter actor:

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
        Ok(self.value)
    }
}

impl Handler<GetValue> for CounterActor {
    type Reply = Result<i32, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _msg: GetValue) -> Self::Reply {
        Ok(self.value)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Spawn the actor with initial value 0
    let counter = spawn::<CounterActor>(0);

    // Two-step API: send message, then decide what to do with response
    let handle = counter.send_message(HandlerMessage::new(Increment(5))).await;
    let result = handle.reply().await?;
    println!("After increment: {:?}", result); // Prints: Ok(5)

    // Convenience method: send and reply immediately
    let value = counter.send_and_reply(HandlerMessage::new(GetValue)).await?;
    println!("Current value: {:?}", value); // Prints: Ok(5)

    // Fire-and-forget: increment without waiting for response
    counter.send_and_forget(HandlerMessage::new(Increment(3))).await;

    // Check the final value
    let final_value = counter.send_and_reply(HandlerMessage::new(GetValue)).await?;
    println!("Final value: {:?}", final_value); // Prints: Ok(8)

    Ok(())
}
```

## Breaking It Down

### 1. Actor Definition
```rust
pub struct CounterActor {
    value: i32, // Actor's private state
}
```

### 2. Message Types
```rust
pub struct Increment(pub i32);
pub struct GetValue;
```

### 3. Actor Implementation
```rust
impl Actor for CounterActor {
    type Spec = i32; // What you pass to spawn()
    type Message = Multi<Self>; // Enables handler-style messages
    // ... other required types
}
```

### 4. Message Handlers
```rust
impl Handler<Increment> for CounterActor {
    type Reply = Result<i32, anyhow::Error>;
    
    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: Increment) -> Self::Reply {
        self.value += msg.0;
        Ok(self.value)
    }
}
```

### 5. Using the Two-Step API
```rust
// Step 1: Send the message
let handle = counter.send_message(HandlerMessage::new(Increment(5))).await;

// Step 2: Decide what to do with the response
let result = handle.reply().await?;           // Wait for response
// OR: handle.forget();                        // Ignore response
// OR: handle.reply_timeout(duration).await?; // Wait with timeout
```

## API Patterns

Actor12 provides three main ways to send messages:

### Two-Step Pattern
```rust
let handle = actor.send_message(message).await;
let response = handle.reply().await?;
```

### Convenience Methods
```rust
// Send and wait for reply
let response = actor.send_and_reply(message).await?;

// Send and forget
actor.send_and_forget(message).await;

// Send with timeout
let response = actor.send_with_timeout(message, Duration::from_secs(1)).await?;
```

### Advanced Patterns
```rust
// Non-blocking check
if let Ok(Some(response)) = handle.try_reply() {
    println!("Got immediate response: {:?}", response);
}

// Transform response
let formatted = handle.map_reply(|result| format!("Result: {:?}", result)).await?;

// Chain operations
let chained = handle.then(|result| async move {
    // Do something with result
    Ok(format!("Processed: {:?}", result))
}).await?;
```

## Next Steps

Now that you've seen the basics, explore:

- [Core Concepts](/concepts/actors) - Deeper understanding of actors and messages
- [Two-Step API](/api/two-step) - Complete guide to the messaging API  
- [Examples](/examples/counter) - More complete examples and patterns