---
title: Two-Step API
description: Complete guide to Actor12's ergonomic two-step messaging API
---

# Two-Step API

Actor12's signature feature is the **two-step messaging API** that separates sending a message from handling the response. This gives you fine-grained control over message handling while maintaining simplicity.

## The Two Steps

### Step 1: Send Message
Send the message and get a `MessageHandle`:
```rust
let handle = actor.send_message(message).await;
```

### Step 2: Handle Response  
Decide what to do with the response:
```rust
let response = handle.reply().await?;           // Wait for response
// OR
handle.forget();                                // Ignore response  
// OR  
let response = handle.reply_timeout(duration).await?; // Wait with timeout
```

## Core Methods

### `send_message<M>(message: M) -> MessageHandle<M::Reply>`
The foundational method that sends any message and returns a handle:

```rust
use actor12::{spawn, HandlerMessage};

let actor = spawn::<MyActor>(());
let handle = actor.send_message(HandlerMessage::new(MyMessage)).await;

// Handle is ready - now decide what to do
if handle.was_sent() {
    let response = handle.reply().await?;
    println!("Got response: {:?}", response);
} else {
    println!("Send failed: {:?}", handle.send_error());
}
```

### `MessageHandle<R>` Methods

#### Core Response Methods
```rust
// Wait for response (consumes handle)
async fn reply(self) -> Result<R, MessageError>

// Wait with timeout
async fn reply_timeout(self, timeout: Duration) -> Result<R, MessageError>

// Fire-and-forget (explicit)
fn forget(self)

// Non-blocking check
fn try_reply(self) -> Result<Option<R>, MessageError>
```

#### Status Methods
```rust
// Check if message was sent successfully
fn was_sent(&self) -> bool

// Get send error if any
fn send_error(&self) -> Option<&str>
```

#### Advanced Methods
```rust
// Transform the response
async fn map_reply<U, F>(self, f: F) -> Result<U, MessageError>
where F: FnOnce(R) -> U + Send

// Chain another async operation
async fn then<U, F, Fut>(self, f: F) -> Result<U, MessageError>  
where F: FnOnce(R) -> Fut + Send, Fut: Future<Output = Result<U, MessageError>>

// Transform errors
async fn map_err<F>(self, f: F) -> Result<R, MessageError>
where F: FnOnce(MessageError) -> MessageError + Send
```

## Convenience Methods

For common patterns, Actor12 provides convenience methods that combine both steps:

### `send_and_reply<M>(message: M) -> Result<M::Reply, MessageError>`
Send message and wait for response immediately:

```rust
// Equivalent to: send_message(msg).await.reply().await
let response = actor.send_and_reply(HandlerMessage::new(GetValue)).await?;
```

### `send_and_forget<M>(message: M)`  
Send message and don't wait for response:

```rust
// Equivalent to: send_message(msg).await.forget()
actor.send_and_forget(HandlerMessage::new(LogEvent("something happened"))).await;
```

### `send_with_timeout<M>(message: M, timeout: Duration) -> Result<M::Reply, MessageError>`
Send message and wait with timeout:

```rust
use std::time::Duration;

// Equivalent to: send_message(msg).await.reply_timeout(timeout).await
let response = actor.send_with_timeout(
    HandlerMessage::new(SlowOperation),
    Duration::from_secs(5)
).await?;
```

## Error Handling

The two-step API provides detailed error information:

### `MessageError` Types
```rust
pub enum MessageError {
    SendFailed(String),    // Message couldn't be sent
    RecvFailed(RecvError), // Response channel failed  
    Timeout { timeout: Duration }, // Operation timed out
    AlreadyConsumed,       // Handle already used
}
```

### Error Handling Patterns
```rust
// Check send success before waiting
let handle = actor.send_message(message).await;
if !handle.was_sent() {
    eprintln!("Failed to send: {}", handle.send_error().unwrap_or("Unknown error"));
    return;
}

match handle.reply().await {
    Ok(response) => println!("Success: {:?}", response),
    Err(MessageError::RecvFailed(_)) => eprintln!("Actor died before responding"),
    Err(MessageError::Timeout { timeout }) => eprintln!("Timed out after {:?}", timeout),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Advanced Patterns

### Conditional Response Handling
```rust
let handle = actor.send_message(message).await;

// Only wait if send succeeded
if handle.was_sent() {
    match handle.try_reply() {
        Ok(Some(response)) => {
            // Response ready immediately
            println!("Immediate response: {:?}", response);
        }
        Ok(None) => {
            // Response not ready yet - decide whether to wait
            if should_wait {
                let response = handle.reply().await?;
                println!("Waited for response: {:?}", response);
            } else {
                handle.forget(); // Don't wait
                println!("Not waiting for response");
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

### Response Transformation
```rust
let formatted_response = actor
    .send_message(HandlerMessage::new(GetUserInfo(user_id)))
    .await
    .map_reply(|user_info| {
        format!("{} <{}>", user_info.name, user_info.email)
    })
    .await?;

println!("User: {}", formatted_response);
```

### Operation Chaining
```rust
let final_result = actor
    .send_message(HandlerMessage::new(LoadData(id)))
    .await
    .then(|data| async move {
        // Process the data asynchronously
        let processed = process_data(data).await?;
        Ok(format!("Processed: {:?}", processed))
    })
    .await?;
```

### Batch Operations
```rust
// Send multiple messages, collect handles
let handles: Vec<_> = messages.into_iter()
    .map(|msg| actor.send_message(HandlerMessage::new(msg)))
    .collect();
    
// Wait for all handles
let mut responses = Vec::new();
for handle in handles {
    let handle = handle.await;
    if handle.was_sent() {
        responses.push(handle.reply().await?);
    }
}
```

## Pattern Comparison

| Pattern | Use Case | Pros | Cons |
|---------|----------|------|------|  
| `send_message().reply()` | Need control over response handling | Maximum flexibility | More verbose |
| `send_and_reply()` | Simple request-response | Concise, clear intent | Less control |
| `send_and_forget()` | Fire-and-forget notifications | Simple, efficient | No response |
| `send_with_timeout()` | Operations that might hang | Built-in timeout | Still blocks |

## Raw Message Support

For performance-critical scenarios, you can send raw messages:

```rust
// Send raw message directly
let handle = actor.send_raw_message(raw_message).await;
let result = handle.reply().await?; // Returns Result<(), ActorSendError>
```

## WeakLink Support  

The two-step API works seamlessly with WeakLinks:

```rust
let weak_actor = actor.downgrade();

// Same API, gracefully handles dead actors
let response = weak_actor.send_and_reply(HandlerMessage::new(message)).await?;
```

## Performance Considerations

### When to Use Each Pattern

**Two-step (`send_message().reply()`):**
- You need conditional response handling
- You want to check send success before waiting
- You need advanced response transformation
- You're implementing retry logic

**Convenience methods:**
- Simple request-response patterns
- Fire-and-forget notifications  
- Operations with known timeouts

### Efficiency Tips

```rust
// Efficient: reuse message handle
let handle = actor.send_message(message).await;
if handle.was_sent() {
    // Only create timeout future if needed
    let response = handle.reply_timeout(timeout).await?;
}

// Less efficient: always create timeout
let response = actor.send_with_timeout(message, timeout).await?;
```

## Next Steps

- Learn about [Envelope Messages](/api/envelope) - simple request-response patterns
- Explore [Handler Messages](/api/handler) - type-safe multi-message actors
- Check out [WeakLink](/api/weaklink) - weak references to actors
- See [Advanced Patterns](/api/advanced) - complex messaging scenarios