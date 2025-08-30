// Proof of Concept: Two-Step Link API Implementation
// This is a conceptual implementation showing how the new API could work

use std::future::Future;
use std::time::Duration;
use tokio::sync::oneshot;
use thiserror::Error;

// =============================================================================
// Core Types
// =============================================================================

/// Handle returned by send() - allows waiting for reply or forgetting
pub struct MessageHandle<R> {
    receiver: Option<oneshot::Receiver<R>>,
    sent_successfully: bool,
    send_error: Option<String>, // Simplified for POC
}

#[derive(Debug, Error)]
pub enum MessageError {
    #[error("Failed to send message: {0}")]
    SendFailed(String),
    #[error("Failed to receive reply: {0}")]
    RecvFailed(#[from] oneshot::error::RecvError),
    #[error("Reply timed out")]
    Timeout,
}

// =============================================================================
// Message Handle Implementation
// =============================================================================

impl<R> MessageHandle<R> {
    /// Wait for the reply from the actor
    pub async fn reply(mut self) -> Result<R, MessageError> {
        if !self.sent_successfully {
            return Err(MessageError::SendFailed(
                self.send_error.unwrap_or_else(|| "Unknown send error".to_string())
            ));
        }
        
        let receiver = self.receiver.take()
            .ok_or_else(|| MessageError::SendFailed("Receiver already consumed".to_string()))?;
            
        receiver.await.map_err(MessageError::RecvFailed)
    }
    
    /// Wait for reply with timeout
    pub async fn reply_timeout(self, timeout: Duration) -> Result<R, MessageError> {
        match tokio::time::timeout(timeout, self.reply()).await {
            Ok(result) => result,
            Err(_) => Err(MessageError::Timeout),
        }
    }
    
    /// Fire and forget - drop the response receiver
    pub fn forget(mut self) {
        self.receiver.take(); // Explicitly drop the receiver
    }
    
    /// Check if message was sent successfully
    pub fn was_sent(&self) -> bool {
        self.sent_successfully
    }
    
    /// Get send error if any
    pub fn send_error(&self) -> Option<&str> {
        self.send_error.as_deref()
    }
    
    /// Create a successful handle
    fn success(receiver: oneshot::Receiver<R>) -> Self {
        Self {
            receiver: Some(receiver),
            sent_successfully: true,
            send_error: None,
        }
    }
    
    /// Create a failed handle
    fn failed(error: String) -> Self {
        Self {
            receiver: None,
            sent_successfully: false,
            send_error: Some(error),
        }
    }
}

// =============================================================================
// Message Trait - Unifies Different Message Types
// =============================================================================

pub trait Message {
    /// The actor type this message can be sent to
    type Actor;
    
    /// The reply type for this message
    type Reply: Send + 'static;
    
    /// Convert this message into an envelope for sending
    fn into_envelope(self) -> (Box<dyn MessageEnvelope<Self::Reply>>, oneshot::Receiver<Self::Reply>);
}

/// Type-erased envelope for sending through channels
pub trait MessageEnvelope<R>: Send {
    /// Execute this message on the given actor context
    fn execute(self: Box<Self>, actor: &mut dyn std::any::Any) -> impl Future<Output = R> + Send;
}

// =============================================================================
// Simple Implementation for Typed Messages
// =============================================================================

/// Simple message implementation
pub struct TypedMessage<T, R> {
    payload: T,
    handler: fn(&mut dyn std::any::Any, T) -> R,
}

impl<T, R> TypedMessage<T, R> 
where
    T: Send + 'static,
    R: Send + 'static,
{
    pub fn new(payload: T, handler: fn(&mut dyn std::any::Any, T) -> R) -> Self {
        Self { payload, handler }
    }
}

struct TypedEnvelope<T, R> {
    payload: Option<T>,
    handler: fn(&mut dyn std::any::Any, T) -> R,
    reply_tx: Option<oneshot::Sender<R>>,
}

impl<T, R> MessageEnvelope<R> for TypedEnvelope<T, R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    async fn execute(mut self: Box<Self>, actor: &mut dyn std::any::Any) -> R {
        let payload = self.payload.take().unwrap();
        let result = (self.handler)(actor, payload);
        
        if let Some(tx) = self.reply_tx.take() {
            let _ = tx.send(result.clone()); // Clone for POC simplicity
        }
        
        result
    }
}

impl<T, R> Message for TypedMessage<T, R>
where
    T: Send + 'static,
    R: Send + Clone + 'static, // Clone for POC simplicity
{
    type Actor = (); // Simplified for POC
    type Reply = R;
    
    fn into_envelope(self) -> (Box<dyn MessageEnvelope<Self::Reply>>, oneshot::Receiver<Self::Reply>) {
        let (tx, rx) = oneshot::channel();
        
        let envelope = Box::new(TypedEnvelope {
            payload: Some(self.payload),
            handler: self.handler,
            reply_tx: Some(tx),
        });
        
        (envelope, rx)
    }
}

// =============================================================================
// Link Implementation with New API
// =============================================================================

pub struct Link<A> {
    // Simplified for POC - in real implementation would have proper channels
    _phantom: std::marker::PhantomData<A>,
}

impl<A> Link<A> 
where
    A: Send + 'static,
{
    /// Unified send method - works for all message types
    pub async fn send<M>(&self, message: M) -> MessageHandle<M::Reply>
    where
        M: Message<Actor = A>,
    {
        let (envelope, receiver) = message.into_envelope();
        
        // Simulate sending - in real implementation would use actor channels
        match self.simulate_send(envelope).await {
            Ok(()) => MessageHandle::success(receiver),
            Err(e) => MessageHandle::failed(e),
        }
    }
    
    // Simulate the actual channel send operation
    async fn simulate_send<R>(&self, _envelope: Box<dyn MessageEnvelope<R>>) -> Result<(), String> {
        // In real implementation, this would send through actor's channel
        // For POC, we'll simulate success most of the time
        if rand::random::<f32>() > 0.1 {
            Ok(())
        } else {
            Err("Simulated send failure".to_string())
        }
    }
}

impl<A> Clone for Link<A> {
    fn clone(&self) -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

// =============================================================================
// Usage Examples
// =============================================================================

#[derive(Debug, Clone)]
pub struct Counter {
    value: i32,
}

#[derive(Debug)]
pub struct IncrementMsg;

#[derive(Debug, Clone)]
pub struct GetValueMsg;

// Message implementations
impl Message for IncrementMsg {
    type Actor = Counter;
    type Reply = i32;
    
    fn into_envelope(self) -> (Box<dyn MessageEnvelope<Self::Reply>>, oneshot::Receiver<Self::Reply>) {
        TypedMessage::new(self, |actor: &mut dyn std::any::Any, _msg: IncrementMsg| {
            let counter = actor.downcast_mut::<Counter>().unwrap();
            counter.value += 1;
            counter.value
        }).into_envelope()
    }
}

impl Message for GetValueMsg {
    type Actor = Counter;
    type Reply = i32;
    
    fn into_envelope(self) -> (Box<dyn MessageEnvelope<Self::Reply>>, oneshot::Receiver<Self::Reply>) {
        TypedMessage::new(self, |actor: &mut dyn std::any::Any, _msg: GetValueMsg| {
            let counter = actor.downcast_ref::<Counter>().unwrap();
            counter.value
        }).into_envelope()
    }
}

// =============================================================================
// Example Usage Patterns
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let link: Link<Counter> = Link { _phantom: std::marker::PhantomData };
    
    // Example 1: Request-Response Pattern
    println!("=== Request-Response Pattern ===");
    match link.send(GetValueMsg).await.reply().await {
        Ok(value) => println!("Current value: {}", value),
        Err(e) => println!("Error getting value: {}", e),
    }
    
    // Example 2: Fire-and-Forget Pattern  
    println!("\n=== Fire-and-Forget Pattern ===");
    link.send(IncrementMsg).await.forget();
    println!("Increment message sent (fire-and-forget)");
    
    // Example 3: Timeout Pattern
    println!("\n=== Timeout Pattern ===");
    match link.send(GetValueMsg).await.reply_timeout(Duration::from_millis(100)).await {
        Ok(value) => println!("Value with timeout: {}", value),
        Err(MessageError::Timeout) => println!("Request timed out"),
        Err(e) => println!("Other error: {}", e),
    }
    
    // Example 4: Conditional Reply Pattern
    println!("\n=== Conditional Reply Pattern ===");
    let handle = link.send(GetValueMsg).await;
    if handle.was_sent() {
        match handle.reply().await {
            Ok(value) => println!("Conditional reply value: {}", value),
            Err(e) => println!("Reply error: {}", e),
        }
    } else {
        println!("Send failed: {:?}", handle.send_error());
        handle.forget();
    }
    
    // Example 5: Batch Operations
    println!("\n=== Batch Operations ===");
    let messages = vec![GetValueMsg, GetValueMsg, GetValueMsg];
    let mut handles = Vec::new();
    
    // Send all messages concurrently
    for msg in messages {
        handles.push(link.send(msg).await);
    }
    
    // Process responses as they come in
    for (i, handle) in handles.into_iter().enumerate() {
        match handle.reply_timeout(Duration::from_secs(1)).await {
            Ok(value) => println!("Batch response {}: {}", i, value),
            Err(e) => println!("Batch error {}: {}", i, e),
        }
    }
    
    println!("\nAll examples completed!");
    Ok(())
}

// =============================================================================
// Advanced Patterns
// =============================================================================

/// Extension trait for additional patterns
pub trait MessageHandleExt<R> {
    /// Try to get reply without waiting (non-blocking)
    fn try_reply(self) -> Result<Option<R>, MessageError>;
    
    /// Map the reply value if/when it arrives
    fn map<U, F>(self, f: F) -> MessageHandle<U>
    where
        F: FnOnce(R) -> U + Send + 'static,
        U: Send + 'static;
        
    /// Chain another operation after this reply
    fn then<U, F, Fut>(self, f: F) -> impl Future<Output = Result<U, MessageError>>
    where
        F: FnOnce(R) -> Fut,
        Fut: Future<Output = Result<U, MessageError>>;
}

impl<R> MessageHandleExt<R> for MessageHandle<R> 
where
    R: Send + 'static,
{
    fn try_reply(mut self) -> Result<Option<R>, MessageError> {
        if !self.sent_successfully {
            return Err(MessageError::SendFailed(
                self.send_error.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }
        
        match self.receiver.take().unwrap().try_recv() {
            Ok(value) => Ok(Some(value)),
            Err(oneshot::error::TryRecvError::Empty) => Ok(None),
            Err(oneshot::error::TryRecvError::Closed) => Err(MessageError::RecvFailed(
                oneshot::error::RecvError(())
            )),
        }
    }
    
    fn map<U, F>(self, f: F) -> MessageHandle<U>
    where
        F: FnOnce(R) -> U + Send + 'static,
        U: Send + 'static,
    {
        // Implementation would create new handle with mapped receiver
        // Simplified for POC
        todo!("Map implementation requires more complex receiver transformation")
    }
    
    async fn then<U, F, Fut>(self, f: F) -> Result<U, MessageError>
    where
        F: FnOnce(R) -> Fut,
        Fut: Future<Output = Result<U, MessageError>>,
    {
        let reply = self.reply().await?;
        f(reply).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_reply_pattern() {
        let link: Link<Counter> = Link { _phantom: std::marker::PhantomData };
        let handle = link.send(GetValueMsg).await;
        
        assert!(handle.was_sent());
        
        // This would work with proper actor implementation
        // let value = handle.reply().await.unwrap();
        // assert_eq!(value, 0);
    }
    
    #[tokio::test] 
    async fn test_forget_pattern() {
        let link: Link<Counter> = Link { _phantom: std::marker::PhantomData };
        let handle = link.send(IncrementMsg).await;
        
        // Should not panic or block
        handle.forget();
    }
    
    #[tokio::test]
    async fn test_timeout_pattern() {
        let link: Link<Counter> = Link { _phantom: std::marker::PhantomData };
        let handle = link.send(GetValueMsg).await;
        
        // With proper implementation, this would test actual timeout behavior
        let result = handle.reply_timeout(Duration::from_millis(1)).await;
        // assert!(matches!(result, Err(MessageError::Timeout)));
    }
}