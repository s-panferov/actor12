---
title: Migrating from Old API
description: How to migrate from the old Actor12 API to the new two-step API
---

# Migrating from Old API

This guide helps you migrate from Actor12's old API to the new unified two-step messaging API. The new API provides better ergonomics while maintaining full backward compatibility.

## API Mapping

### Handler-Style Messages

#### Old API → New API

```rust
// OLD: ask_dyn (wait for response)
let response = actor.ask_dyn(MyMessage).await;

// NEW: send_and_reply (equivalent)
let response = actor.send_and_reply(HandlerMessage::new(MyMessage)).await?;

// NEW: two-step pattern (more control)
let handle = actor.send_message(HandlerMessage::new(MyMessage)).await;
let response = handle.reply().await?;
```

```rust
// OLD: tell_dyn (fire-and-forget)
actor.tell_dyn(MyMessage).await;

// NEW: send_and_forget (equivalent)
actor.send_and_forget(HandlerMessage::new(MyMessage)).await;

// NEW: two-step pattern (explicit)
let handle = actor.send_message(HandlerMessage::new(MyMessage)).await;
handle.forget();
```

```rust
// OLD: ask_dyn_async (get future)
let future = actor.ask_dyn_async(MyMessage).await;
let response = future.await;

// NEW: send_message (get handle immediately)
let handle = actor.send_message(HandlerMessage::new(MyMessage)).await;
let response = handle.reply().await?;
```

### Envelope-Style Messages

```rust
// OLD: send (simple request-response)
let response = actor.send(MyMessage).await;

// NEW: send_and_reply (equivalent)
let response = actor.send_and_reply(EnvelopeMessage::new(MyMessage)).await?;

// NEW: two-step pattern
let handle = actor.send_message(EnvelopeMessage::new(MyMessage)).await;
let response = handle.reply().await?;
```

### Raw Messages

```rust
// OLD: send_raw
let result = actor.send_raw(message).await;

// NEW: send_raw_message
let handle = actor.send_raw_message(message).await;
let result = handle.reply().await?;
```

### Relay Operations

```rust
// OLD: relay_dyn
actor.relay_dyn(envelope).await;

// NEW: RelayMessage
actor.send_and_forget(RelayMessage::new(envelope)).await;
```

## WeakLink Migration

### Handler-Style WeakLink

```rust
// OLD: weak.ask_dyn
let response = weak_actor.ask_dyn(MyMessage).await;

// NEW: weak.send_and_reply
let response = weak_actor.send_and_reply(HandlerMessage::new(MyMessage)).await?;

// OLD: weak.tell_dyn
weak_actor.tell_dyn(MyMessage).await;

// NEW: weak.send_and_forget  
weak_actor.send_and_forget(HandlerMessage::new(MyMessage)).await;
```

### Envelope-Style WeakLink

```rust
// OLD: weak.send
let response = weak_actor.send(MyMessage).await;

// NEW: weak.send_and_reply
let response = weak_actor.send_and_reply(EnvelopeMessage::new(MyMessage)).await?;
```

## Migration Strategy

### 1. Gradual Migration (Recommended)

Start by migrating to convenience methods, which are direct replacements:

```rust
// Step 1: Replace old methods with new convenience methods
// actor.ask_dyn(msg).await → actor.send_and_reply(HandlerMessage::new(msg)).await?
// actor.tell_dyn(msg).await → actor.send_and_forget(HandlerMessage::new(msg)).await
// actor.send(msg).await → actor.send_and_reply(EnvelopeMessage::new(msg)).await?

// Step 2: Add error handling
match actor.send_and_reply(HandlerMessage::new(msg)).await {
    Ok(response) => {
        // Handle successful response
    }
    Err(e) => {
        // Handle send/receive errors
        eprintln!("Message failed: {}", e);
    }
}

// Step 3: Migrate to two-step API for better control
let handle = actor.send_message(HandlerMessage::new(msg)).await;
if handle.was_sent() {
    let response = handle.reply().await?;
    // Process response
} else {
    eprintln!("Failed to send: {:?}", handle.send_error());
}
```

### 2. Complete Migration

For new code or major refactoring, use the two-step API directly:

```rust
// Before
async fn process_request(actor: &Link<MyActor>, request: Request) -> Result<Response, Error> {
    let response = actor.ask_dyn(ProcessRequest(request)).await;
    match response {
        Ok(result) => Ok(result),
        Err(e) => Err(Error::ProcessingFailed(e)),
    }
}

// After  
async fn process_request(actor: &Link<MyActor>, request: Request) -> Result<Response, ProcessError> {
    let handle = actor.send_message(HandlerMessage::new(ProcessRequest(request))).await;
    
    if !handle.was_sent() {
        return Err(ProcessError::SendFailed(
            handle.send_error().unwrap_or("Unknown").to_string()
        ));
    }
    
    match handle.reply_timeout(Duration::from_secs(5)).await {
        Ok(Ok(response)) => Ok(response),
        Ok(Err(actor_error)) => Err(ProcessError::ActorError(actor_error)),
        Err(MessageError::Timeout { .. }) => Err(ProcessError::Timeout),
        Err(e) => Err(ProcessError::MessageError(e)),
    }
}

#[derive(Debug, Error)]
enum ProcessError {
    #[error("Failed to send message: {0}")]
    SendFailed(String),
    #[error("Actor returned error: {0}")]
    ActorError(String),
    #[error("Request timed out")]
    Timeout,
    #[error("Message error: {0}")]
    MessageError(#[from] MessageError),
}
```

## Error Handling Changes

### Old API Error Handling

The old API embedded errors in response types:

```rust
// Old way - errors were embedded in actor responses
impl Handler<GetUser> for UserActor {
    type Reply = Result<User, UserError>; // Actor errors
    
    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: GetUser) -> Self::Reply {
        // Actor logic errors returned here
        self.find_user(msg.user_id).ok_or(UserError::NotFound)
    }
}

// Usage had limited error information
match actor.ask_dyn(GetUser { user_id }).await {
    Ok(user) => println!("Found user: {:?}", user),
    Err(user_error) => println!("User error: {}", user_error),
    // No way to distinguish between send errors and actor errors
}
```

### New API Error Handling

The new API separates send errors from actor response errors:

```rust
// Same actor implementation
impl Handler<GetUser> for UserActor {
    type Reply = Result<User, UserError>; // Actor errors (unchanged)
    
    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: GetUser) -> Self::Reply {
        // Same logic
        self.find_user(msg.user_id).ok_or(UserError::NotFound)
    }
}

// Usage has better error distinction
match actor.send_and_reply(HandlerMessage::new(GetUser { user_id })).await {
    Ok(Ok(user)) => println!("Found user: {:?}", user),
    Ok(Err(user_error)) => println!("User not found: {}", user_error),
    Err(MessageError::SendFailed(e)) => println!("Couldn't send message: {}", e),
    Err(MessageError::RecvFailed(_)) => println!("Actor died before responding"),
    Err(MessageError::Timeout { timeout }) => println!("Timed out after {:?}", timeout),
    Err(e) => println!("Other message error: {}", e),
}
```

## New Capabilities

The new API provides capabilities that weren't available before:

### Timeouts
```rust
// Built-in timeout support
let response = actor.send_with_timeout(
    HandlerMessage::new(SlowOperation), 
    Duration::from_secs(10)
).await?;
```

### Conditional Response Handling
```rust
let handle = actor.send_message(HandlerMessage::new(msg)).await;

// Check if we actually want to wait
if should_wait_for_response {
    let response = handle.reply().await?;
    process_response(response);
} else {
    handle.forget(); // Explicitly ignore
}
```

### Non-blocking Checks
```rust
let handle = actor.send_message(HandlerMessage::new(msg)).await;

// Check if response is ready immediately
match handle.try_reply() {
    Ok(Some(response)) => {
        // Got response immediately
        process_immediate_response(response);
    }
    Ok(None) => {
        // Response not ready - decide whether to wait
        if urgent {
            let response = handle.reply().await?;
            process_response(response);
        } else {
            handle.forget(); // Come back later
        }
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

### Response Transformation
```rust
// Transform response before getting it
let formatted = actor
    .send_message(HandlerMessage::new(GetUserInfo(id)))
    .await
    .map_reply(|info| format!("{} <{}>", info.name, info.email))
    .await?;
```

## Backward Compatibility

**Important**: All old API methods are still available and work exactly as before. You can migrate gradually:

```rust
// This still works exactly as before
let response = actor.ask_dyn(MyMessage).await;
let response = actor.send(MyMessage).await;
actor.tell_dyn(MyMessage).await;

// WeakLink old API also still works
let response = weak_actor.ask_dyn(MyMessage).await;
let response = weak_actor.send(MyMessage).await;
```

## Checklist for Migration

- [ ] Identify all uses of old API methods (`ask_dyn`, `tell_dyn`, `send`, `relay_dyn`)
- [ ] Replace with equivalent convenience methods first
- [ ] Add proper error handling for new `MessageError` types  
- [ ] Consider using two-step API for complex scenarios
- [ ] Add timeouts where appropriate
- [ ] Test error scenarios (actor death, timeouts, send failures)
- [ ] Update WeakLink usage
- [ ] Remove old API usage once migration is complete

## Common Migration Patterns

### Pattern 1: Simple Request-Response

```rust
// Before
async fn get_balance(bank: &Link<BankActor>, account: AccountId) -> Money {
    bank.ask_dyn(GetBalance(account)).await.unwrap_or(Money::ZERO)
}

// After
async fn get_balance(bank: &Link<BankActor>, account: AccountId) -> Result<Money, BalanceError> {
    match bank.send_and_reply(HandlerMessage::new(GetBalance(account))).await {
        Ok(Ok(balance)) => Ok(balance),
        Ok(Err(bank_error)) => Err(BalanceError::BankError(bank_error)),
        Err(msg_error) => Err(BalanceError::MessageError(msg_error)),
    }
}

#[derive(Debug, Error)]
enum BalanceError {
    #[error("Bank error: {0}")]
    BankError(BankError),
    #[error("Message error: {0}")]  
    MessageError(#[from] MessageError),
}
```

### Pattern 2: Fire-and-Forget with Error Logging

```rust
// Before
async fn log_event(logger: &Link<LoggerActor>, event: Event) {
    logger.tell_dyn(LogEvent(event)).await;
    // No way to know if it failed
}

// After
async fn log_event(logger: &Link<LoggerActor>, event: Event) {
    let handle = logger.send_message(HandlerMessage::new(LogEvent(event))).await;
    if !handle.was_sent() {
        eprintln!("Failed to send log event: {:?}", handle.send_error());
    }
    handle.forget(); // Don't wait for response
}
```

### Pattern 3: Timeout-Sensitive Operations

```rust
// Before - no built-in timeout support
async fn process_with_timeout(processor: &Link<ProcessorActor>, data: Data) -> Option<Result> {
    match tokio::time::timeout(Duration::from_secs(5), processor.ask_dyn(Process(data))).await {
        Ok(result) => Some(result),
        Err(_) => None, // Timeout
    }
}

// After - built-in timeout
async fn process_with_timeout(processor: &Link<ProcessorActor>, data: Data) -> Result<ProcessResult, ProcessError> {
    match processor.send_with_timeout(HandlerMessage::new(Process(data)), Duration::from_secs(5)).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(process_error)) => Err(ProcessError::ProcessingFailed(process_error)),
        Err(MessageError::Timeout { timeout }) => Err(ProcessError::Timeout(timeout)),
        Err(e) => Err(ProcessError::MessageFailed(e)),
    }
}
```

The new API provides better error handling, more control, and additional capabilities while maintaining full backward compatibility. Migrate at your own pace!