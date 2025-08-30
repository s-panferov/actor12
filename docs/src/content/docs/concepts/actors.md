---
title: Actors
description: Understanding actors in Actor12
---

# Actors

Actors are the fundamental building blocks of Actor12. An actor is an isolated entity that:

- **Holds private state** that can only be accessed by the actor itself
- **Processes messages sequentially** in the order they arrive
- **Communicates only through message passing** with other actors
- **Can create new actors** and manage their lifecycle

## Actor Trait

Every actor must implement the `Actor` trait:

```rust
use actor12::{Actor, Init, MpscChannel};
use std::future::Future;

pub struct MyActor {
    // Private state here
    counter: i32,
    name: String,
}

impl Actor for MyActor {
    // What you pass to spawn() to create the actor
    type Spec = (String, i32);
    
    // The type of messages this actor can receive
    type Message = MyMessage;
    
    // Channel implementation for message delivery
    type Channel = MpscChannel<Self::Message>;
    
    // Type for cancellation reasons
    type Cancel = ();
    
    // Shared state accessible from Links
    type State = String; // e.g., actor name
    
    // Initialize shared state from spec
    fn state(spec: &Self::Spec) -> Self::State {
        spec.0.clone() // Return the name
    }
    
    // Create the actor instance
    fn init(ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        let (name, counter) = ctx.spec;
        futures::future::ready(Ok(MyActor { counter, name }))
    }
}
```

## Associated Types

### `Spec`
The data needed to create an actor instance. Passed to `spawn()`:
```rust
type Spec = i32; // Simple counter starting value
type Spec = (String, Vec<u8>); // Name and initial data
type Spec = MyConfig; // Custom configuration struct
```

### `Message`
The type of messages the actor can receive:
```rust
// Single message type (envelope style)
type Message = Envelope<MyMessage, MyResponse>;

// Multiple message types (handler style) 
type Message = Multi<Self>;

// Custom message enum
type Message = MyMessageEnum;
```

### `Channel`
How messages are delivered to the actor:
```rust
// Standard MPSC channel (most common)
type Channel = MpscChannel<Self::Message>;

// Could be custom channel implementations
```

### `Cancel`
The type used for cancellation reasons:
```rust
type Cancel = (); // No cancellation data
type Cancel = String; // Cancellation reason
type Cancel = MyCancelType; // Custom cancellation info
```

### `State`
Shared state accessible from Links without sending messages:
```rust
type State = (); // No shared state
type State = String; // Actor name or ID
type State = Arc<MySharedData>; // Complex shared data
```

## Actor Lifecycle

### 1. Spawning
```rust
use actor12::spawn;

// Create actor with spec
let actor_link = spawn::<MyActor>((
    "my-actor".to_string(),
    42
));
```

### 2. Running
The actor processes messages in its message loop until:
- It's cancelled
- All Links are dropped
- An error occurs

### 3. Cleanup
Actors automatically clean up when they stop. You can implement custom cleanup in the handler if needed.

## Message Processing

Actors process messages sequentially - one message at a time. This guarantees:
- **No race conditions** on actor state
- **Deterministic behavior** - same messages in same order = same result
- **Simple reasoning** about actor behavior

```rust
// These messages will be processed in order:
actor.send_and_forget(Message1).await;
actor.send_and_forget(Message2).await; 
actor.send_and_forget(Message3).await;
// Actor will process: Message1 → Message2 → Message3
```

## Error Handling

Actors can handle errors in several ways:

### Init Errors
Return `Err` from the `init` function:
```rust
fn init(ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
    let config = ctx.spec;
    if config.is_valid() {
        futures::future::ready(Ok(MyActor::new(config)))
    } else {
        futures::future::ready(Err(())) // Actor creation fails
    }
}
```

### Message Handler Errors
Handle errors in individual message responses:
```rust
impl Handler<MyMessage> for MyActor {
    type Reply = Result<String, MyError>;
    
    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: MyMessage) -> Self::Reply {
        if msg.is_valid() {
            Ok("Success".to_string())
        } else {
            Err(MyError::InvalidMessage)
        }
    }
}
```

## Best Practices

### Keep Actors Small and Focused
```rust
// Good: focused responsibility
struct UserCache { 
    users: HashMap<UserId, User> 
}

// Less good: too many responsibilities  
struct MegaActor {
    users: HashMap<UserId, User>,
    orders: Vec<Order>,
    payments: PaymentProcessor,
    email_service: EmailClient,
}
```

### Use Appropriate Message Types
- **Handler style** for type-safe, multiple message types
- **Envelope style** for simple request-response patterns
- **Raw messages** for performance-critical paths

### Design for Testability
```rust
// Easy to test - pure message handling logic
impl Handler<CalculateTotal> for PriceCalculator {
    type Reply = Result<Price, CalculationError>;
    
    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: CalculateTotal) -> Self::Reply {
        // Pure calculation logic
        Ok(msg.items.iter().map(|item| item.price).sum())
    }
}
```

### Handle Backpressure
Actors automatically provide backpressure - if an actor is slow, senders will wait. Monitor actor performance and scale horizontally by creating multiple actor instances if needed.

## Next Steps

- Learn about [Messages](/concepts/messages) - the data actors exchange
- Understand [Links](/concepts/links) - how you communicate with actors  
- Explore [Handlers](/concepts/handlers) - type-safe message processing