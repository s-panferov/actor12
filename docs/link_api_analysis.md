# Link API Analysis & Redesign

## Current API Problems

### 1. Multiple Confusing Methods
```rust
// Current API has too many similar methods
link.send(msg).await           // For Envelope<T, R> messages
link.ask_dyn(msg).await        // For Multi<A> + Handler<T> 
link.tell_dyn(msg).await       // Fire-and-forget Multi<A>
link.ask_dyn_async(msg).await  // Returns BoxFuture
link.send_raw(msg).await       // Raw message sending
```

### 2. Type System Complexity
```rust
// Complex generic bounds that are hard to understand
pub async fn send<T, R>(&self, message: T) -> R
where
    A: ActorLike<Message = Envelope<T, R>>,
    T: Send + Sync + 'static,
    R: Send + Sync + 'static,
    R: FromError<ActorSendError<A>>,
    R: FromError<RecvError>,

pub async fn ask_dyn<T>(&self, message: T) -> <A as Handler<T>>::Reply
where
    T: SyncTrait,
    A: Handler<T>,
    A: ActorLike<Message = Multi<A>>,
```

### 3. Wasteful Tell Implementation
```rust
// tell_dyn creates envelope but drops receiver - wasteful
pub async fn tell_dyn<T>(&self, message: T) -> ()
where
    T: SyncTrait,
    A: Handler<T>,
    A: ActorLike<Message = Multi<A>>,
{
    let (envelope, _) = Envelope::<T, <A as Handler<T>>::Reply>::new(message);
    //                 ^ Receiver is dropped, wasting channel allocation
    if let Ok(()) = self.state.tx.send(Multi::new(envelope)).await {}
}
```

### 4. API Inconsistency
- Different methods for different message types
- Some methods return `Result`, others handle errors internally
- Unclear when to use which method

## Proposed Two-Step API Design

### Core Concept
```rust
// Unified API - same for all message types
let response_handle = link.send(message).await?;

// Then choose what to do:
let reply = response_handle.reply().await?;           // Wait for response
let reply = response_handle.reply_timeout(dur).await?; // With timeout
response_handle.forget();                              // Fire-and-forget
let status = response_handle.status();                 // Check if sent
```

### Implementation Design

```rust
/// Result of sending a message - can be used to wait for reply or ignored
pub struct MessageHandle<R> {
    receiver: Option<oneshot::Receiver<R>>,
    sent_successfully: bool,
    send_error: Option<ActorSendError>,
}

impl<R> MessageHandle<R> {
    /// Wait for the reply from the actor
    pub async fn reply(mut self) -> Result<R, MessageError> 
    where 
        R: FromError<RecvError>
    {
        if !self.sent_successfully {
            return Err(MessageError::SendFailed(self.send_error.unwrap()));
        }
        
        match self.receiver.take().unwrap().await {
            Ok(response) => Ok(response),
            Err(e) => Err(MessageError::RecvFailed(e)),
        }
    }
    
    /// Wait for reply with timeout
    pub async fn reply_timeout(self, timeout: Duration) -> Result<R, MessageError> {
        tokio::time::timeout(timeout, self.reply()).await
            .map_err(|_| MessageError::Timeout)?
    }
    
    /// Fire and forget - drop the response receiver
    pub fn forget(mut self) {
        self.receiver.take(); // Drop the receiver
    }
    
    /// Check if message was sent successfully
    pub fn was_sent(&self) -> bool {
        self.sent_successfully
    }
    
    /// Get send error if any
    pub fn send_error(&self) -> Option<&ActorSendError> {
        self.send_error.as_ref()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MessageError {
    #[error("Failed to send message: {0}")]
    SendFailed(ActorSendError),
    #[error("Failed to receive reply: {0}")]
    RecvFailed(RecvError),
    #[error("Reply timed out")]
    Timeout,
}
```

### Unified Link API

```rust
impl<A: ActorLike> Link<A> {
    /// Send a message and get a handle to wait for reply or ignore
    pub async fn send<T>(&self, message: T) -> MessageHandle<T::Reply>
    where
        T: Message<Actor = A>,
    {
        let (envelope, receiver) = T::create_envelope(message);
        
        match self.state.tx.send(envelope).await {
            Ok(()) => MessageHandle {
                receiver: Some(receiver),
                sent_successfully: true,
                send_error: None,
            },
            Err(e) => MessageHandle {
                receiver: None,
                sent_successfully: false, 
                send_error: Some(e),
            }
        }
    }
}

/// Trait that unifies different message types
pub trait Message {
    type Actor: ActorLike;
    type Reply: Send + Sync + 'static;
    type Envelope: Send + Sync + 'static;
    
    fn create_envelope(self) -> (Self::Envelope, oneshot::Receiver<Self::Reply>);
}

// Implementation for Envelope-style messages
impl<T, R> Message for T 
where
    T: Send + Sync + 'static,
    R: Send + Sync + 'static,
{
    type Actor = A; // Need to figure out actor association
    type Reply = R;
    type Envelope = Envelope<T, R>;
    
    fn create_envelope(self) -> (Self::Envelope, oneshot::Receiver<Self::Reply>) {
        Envelope::new(self)
    }
}

// Implementation for Handler-style messages  
impl<T, A> Message for T
where
    T: SyncTrait,
    A: Handler<T> + ActorLike<Message = Multi<A>>,
{
    type Actor = A;
    type Reply = <A as Handler<T>>::Reply;
    type Envelope = Multi<A>;
    
    fn create_envelope(self) -> (Self::Envelope, oneshot::Receiver<Self::Reply>) {
        let (envelope, rx) = Envelope::new(self);
        (Multi::new(envelope), rx)
    }
}
```

## Usage Examples

### Request-Response Pattern
```rust
// Old API
let response = link.ask_dyn(GetUserRequest { id: 123 }).await;

// New API - explicit and clear
let response = link.send(GetUserRequest { id: 123 })
    .await?
    .reply()
    .await?;
```

### Fire-and-Forget Pattern
```rust
// Old API
link.tell_dyn(LogEvent { msg: "hello".into() }).await;

// New API - explicit intent
link.send(LogEvent { msg: "hello".into() })
    .await?
    .forget();
```

### Timeout Pattern
```rust
// Old API - would need separate timeout wrapper
let response = tokio::time::timeout(
    Duration::from_secs(5),
    link.ask_dyn(SlowRequest)
).await??;

// New API - built in
let response = link.send(SlowRequest)
    .await?
    .reply_timeout(Duration::from_secs(5))
    .await?;
```

### Conditional Reply Pattern
```rust
// New API enables patterns that weren't possible before
let handle = link.send(ProcessRequest { data }).await?;

// Can inspect send status before deciding to wait
if handle.was_sent() {
    let response = handle.reply().await?;
    // Process response
} else {
    eprintln!("Failed to send: {:?}", handle.send_error());
    handle.forget();
}
```

### Batch Operations
```rust
// Send multiple messages concurrently
let handles: Vec<_> = messages.into_iter()
    .map(|msg| link.send(msg))
    .collect::<futures::stream::FuturesUnordered<_>>()
    .collect()
    .await;

// Then decide what to do with each
for handle in handles {
    if let Ok(handle) = handle {
        tokio::spawn(async move {
            if let Ok(response) = handle.reply().await {
                // Process response
            }
        });
    }
}
```

## Benefits of Two-Step API

### 1. **Unified Interface**
- One `send()` method for all message types
- Eliminates confusion between `ask_dyn`, `tell_dyn`, `send`
- Consistent behavior across different message patterns

### 2. **Explicit Intent**
- Clear distinction between fire-and-forget (`.forget()`) and request-response (`.reply()`)
- No hidden behavior - you explicitly choose what to do

### 3. **Better Error Handling**
- Send errors separate from receive errors
- Can inspect send status before waiting for reply
- More granular error information

### 4. **Enhanced Flexibility**
- Built-in timeout support
- Conditional reply patterns
- Batch operations support
- Future extensibility (retries, priority, etc.)

### 5. **Performance Benefits**
- No wasted receiver allocation for fire-and-forget
- Can defer reply decision until after checking send status
- Better resource utilization

### 6. **Type Safety**
- Single trait to implement for custom message types
- Compile-time guarantees about actor/message compatibility
- Cleaner generic bounds

## Migration Strategy

### Phase 1: Add New API Alongside Old
```rust
impl<A: ActorLike> Link<A> {
    // New API
    pub async fn send<T>(&self, message: T) -> MessageHandle<T::Reply> { ... }
    
    // Old API - marked deprecated
    #[deprecated(note = "Use send().reply() instead")]
    pub async fn ask_dyn<T>(&self, message: T) -> <A as Handler<T>>::Reply { ... }
    
    #[deprecated(note = "Use send().forget() instead")] 
    pub async fn tell_dyn<T>(&self, message: T) -> () { ... }
}
```

### Phase 2: Update Examples and Documentation
- Rewrite all examples to use new API
- Add migration guide in documentation
- Update README with new patterns

### Phase 3: Remove Old API
- Remove deprecated methods
- Clean up unused code
- Simplify internal implementation

## Implementation Challenges

### 1. **Message Trait Association**
Need to solve how to associate message types with actors:
```rust
// Option A: Explicit actor parameter
link.send::<MyActor>(message).await

// Option B: Message trait includes actor type
trait Message {
    type Actor: ActorLike;
    // ...
}

// Option C: Context-dependent inference
// Rely on link type to infer actor
```

### 2. **Generic Bounds Complexity**
Need to simplify while maintaining type safety:
```rust
// Current complex bounds
where
    T: SyncTrait,
    A: Handler<T>,
    A: ActorLike<Message = Multi<A>>,
    
// Simplified bounds
where
    T: Message<Actor = A>,
```

### 3. **Backward Compatibility**
Need to maintain compatibility during migration period without code duplication.

## Conclusion

The two-step API design addresses major ergonomic issues with the current Link API:
- **Simplifies** the interface from 5+ methods to 1 core method
- **Clarifies** intent through explicit `.reply()` or `.forget()`  
- **Enhances** flexibility with timeouts, conditional patterns, and batch operations
- **Improves** type safety through unified Message trait
- **Reduces** resource waste by avoiding unnecessary receiver allocation

This design maintains all current functionality while providing a much cleaner and more intuitive developer experience.