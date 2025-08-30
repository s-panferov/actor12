use std::future::Future;
use std::time::Duration;
use tokio::sync::oneshot;
use thiserror::Error;

use crate::actor::{SyncTrait, Actor};
use crate::envelope::Envelope;
use crate::error::{ActorSendError, ActorError, FromError};
use crate::handler::Handler;
use crate::link::{ActorLike, Link};
use crate::multi::Multi;
use crate::channel::ActorSender;

/// Handle returned by Link::send() - allows waiting for reply or ignoring response
pub struct MessageHandle<R> {
    receiver: Option<oneshot::Receiver<R>>,
    sent_successfully: bool,
    send_error: Option<String>,
    timeout: Option<Duration>,
}

/// Errors that can occur when handling message responses
#[derive(Debug, Error)]
pub enum MessageError {
    #[error("Failed to send message: {0}")]
    SendFailed(String),
    #[error("Failed to receive reply: {0}")]
    RecvFailed(#[from] oneshot::error::RecvError),
    #[error("Reply timed out after {timeout:?}")]
    Timeout { timeout: Duration },
    #[error("Message handle already consumed")]
    AlreadyConsumed,
}

impl<R> MessageHandle<R> {
    /// Create a handle for a successfully sent message
    pub(crate) fn success(receiver: oneshot::Receiver<R>) -> Self {
        Self {
            receiver: Some(receiver),
            sent_successfully: true,
            send_error: None,
            timeout: None,
        }
    }

    /// Create a handle for a failed message send
    pub(crate) fn failed(error: String) -> Self {
        Self {
            receiver: None,
            sent_successfully: false,
            send_error: Some(error),
            timeout: None,
        }
    }

    /// Set timeout for the reply (chainable)
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Wait for the reply from the actor
    pub async fn reply(mut self) -> Result<R, MessageError> {
        if !self.sent_successfully {
            return Err(MessageError::SendFailed(
                self.send_error.unwrap_or_else(|| "Unknown send error".to_string())
            ));
        }

        let receiver = self.receiver.take()
            .ok_or(MessageError::AlreadyConsumed)?;

        if let Some(timeout) = self.timeout {
            match tokio::time::timeout(timeout, receiver).await {
                Ok(result) => result.map_err(MessageError::RecvFailed),
                Err(_) => Err(MessageError::Timeout { timeout }),
            }
        } else {
            receiver.await.map_err(MessageError::RecvFailed)
        }
    }


    /// Fire and forget - explicitly drop the response receiver
    pub fn forget(mut self) {
        self.receiver.take(); // Drop the receiver to save resources
    }

    /// Check if message was sent successfully
    pub fn was_sent(&self) -> bool {
        self.sent_successfully
    }

    /// Get send error if any
    pub fn send_error(&self) -> Option<&str> {
        self.send_error.as_deref()
    }

    /// Try to get reply without blocking (non-blocking check)
    pub fn try_reply(mut self) -> Result<Option<R>, MessageError> {
        if !self.sent_successfully {
            return Err(MessageError::SendFailed(
                self.send_error.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }

        let mut receiver = self.receiver.take()
            .ok_or(MessageError::AlreadyConsumed)?;

        match receiver.try_recv() {
            Ok(value) => Ok(Some(value)),
            Err(oneshot::error::TryRecvError::Empty) => {
                // Put receiver back for future use
                self.receiver = Some(receiver);
                Ok(None)
            }
            Err(oneshot::error::TryRecvError::Closed) => {
                Err(MessageError::SendFailed("Channel closed".to_string()))
            }
        }
    }
}

/// Trait that defines how to send a message to an actor
pub trait SendableMessage<A: ActorLike> {
    type Reply: Send + Sync + 'static;
    
    /// Send this message to the actor and return a handle for the response
    fn send_to(self, link: &Link<A>) -> impl Future<Output = MessageHandle<Self::Reply>> + Send;
}

/// Trait for sending messages through weak links
pub trait WeakSendableMessage<A: ActorLike> {
    type Reply: Send + Sync + 'static;
    
    /// Send this message through a weak link, handling dead actors gracefully
    fn weak_send_to(self, weak_link: &crate::WeakLink<A>) -> impl Future<Output = MessageHandle<Self::Reply>> + Send;
}

/// Wrapper for Envelope-style messages to work with the unified API
pub struct EnvelopeMessage<T> {
    pub message: T,
}

impl<T> EnvelopeMessage<T> {
    pub fn new(message: T) -> Self {
        Self { message }
    }
}

/// Implementation for Envelope-style messages
impl<A, T, R> SendableMessage<A> for EnvelopeMessage<T>
where
    A: ActorLike<Message = Envelope<T, R>>,
    T: Send + Sync + 'static,
    R: Send + Sync + 'static,
    R: crate::error::FromError<ActorSendError<A>>,
    R: crate::error::FromError<oneshot::error::RecvError>,
{
    type Reply = R;

    async fn send_to(self, link: &Link<A>) -> MessageHandle<Self::Reply> {
        let (envelope, receiver) = Envelope::<T, R>::new(self.message);
        
        match link.sender().send(envelope).await {
            Ok(()) => MessageHandle::success(receiver),
            Err(e) => MessageHandle::failed(format!("Send failed: {:?}", e)),
        }
    }
}

/// Wrapper for Handler-style messages to work with the unified API
pub struct HandlerMessage<T> {
    pub message: T,
}

impl<T> HandlerMessage<T> {
    pub fn new(message: T) -> Self {
        Self { message }
    }
}

/// Wrapper for relaying existing envelopes (equivalent to old relay_dyn)
pub struct RelayMessage<T, R> {
    pub envelope: Envelope<T, R>,
}

impl<T, R> RelayMessage<T, R> {
    pub fn new(envelope: Envelope<T, R>) -> Self {
        Self { envelope }
    }
}

/// Implementation for relaying envelopes through Handler trait
impl<A, T> SendableMessage<A> for RelayMessage<T, <A as Handler<T>>::Reply>
where
    A: ActorLike<Message = Multi<A>> + Handler<T>,
    T: SyncTrait,
{
    type Reply = (); // Relay doesn't wait for response

    async fn send_to(self, link: &Link<A>) -> MessageHandle<Self::Reply> {
        match link.sender().send(Multi::new(self.envelope)).await {
            Ok(()) => {
                let (tx, rx) = oneshot::channel();
                let _ = tx.send(());
                MessageHandle::success(rx)
            }
            Err(_) => {
                MessageHandle::failed("Relay failed".to_string())
            }
        }
    }
}

/// Weak relay support
impl<A, T> WeakSendableMessage<A> for RelayMessage<T, <A as Handler<T>>::Reply>
where
    A: Actor + ActorLike<Message = Multi<A>> + Handler<T>,
    T: SyncTrait,
{
    type Reply = ();

    async fn weak_send_to(self, weak_link: &crate::WeakLink<A>) -> MessageHandle<Self::Reply> {
        if let Some(link) = weak_link.upgrade() {
            self.send_to(&link).await
        } else {
            // Actor is dead - just return success since relay doesn't expect response
            let (tx, rx) = oneshot::channel();
            let _ = tx.send(());
            MessageHandle::success(rx)
        }
    }
}

/// Implementation for Handler-style messages (Multi<A> message type)
impl<A, T> SendableMessage<A> for HandlerMessage<T>
where
    A: ActorLike<Message = Multi<A>> + Handler<T>,
    T: SyncTrait,
{
    type Reply = <A as Handler<T>>::Reply;

    async fn send_to(self, link: &Link<A>) -> MessageHandle<Self::Reply> {
        let (envelope, receiver) = Envelope::<T, <A as Handler<T>>::Reply>::new(self.message);
        
        match link.sender().send(Multi::new(envelope)).await {
            Ok(()) => MessageHandle::success(receiver),
            Err(e) => MessageHandle::failed(format!("Send failed: {:?}", e)),
        }
    }
}

/// Implement WeakSendableMessage for EnvelopeMessage
impl<A, T, R> WeakSendableMessage<A> for EnvelopeMessage<T>
where
    A: Actor + ActorLike<Message = Envelope<T, R>>,
    T: Send + Sync + 'static,
    R: Send + Sync + 'static,
    R: FromError<ActorSendError<A>>,
    R: FromError<oneshot::error::RecvError>,
    R: FromError<ActorError>,
{
    type Reply = R;

    async fn weak_send_to(self, weak_link: &crate::WeakLink<A>) -> MessageHandle<Self::Reply> {
        if let Some(link) = weak_link.upgrade() {
            self.send_to(&link).await
        } else {
            // Actor is dead - create a failed handle
            let dead_error = R::from_err(ActorError::Dead);
            let (tx, rx) = oneshot::channel();
            let _ = tx.send(dead_error);
            MessageHandle::success(rx)
        }
    }
}

/// Implement WeakSendableMessage for HandlerMessage
impl<A, T> WeakSendableMessage<A> for HandlerMessage<T>
where
    A: Actor + ActorLike<Message = Multi<A>> + Handler<T>,
    T: SyncTrait,
{
    type Reply = <A as Handler<T>>::Reply;

    async fn weak_send_to(self, weak_link: &crate::WeakLink<A>) -> MessageHandle<Self::Reply> {
        if let Some(link) = weak_link.upgrade() {
            self.send_to(&link).await
        } else {
            // Actor is dead - create a failed handle with dead reply
            let dead_reply = <A as Handler<T>>::Reply::from_err(ActorError::Dead);
            let (tx, rx) = oneshot::channel();
            let _ = tx.send(dead_reply);
            MessageHandle::success(rx)
        }
    }
}

/// Extension trait for additional message handle patterns
pub trait MessageHandleExt<R>: Sized {
    /// Chain another async operation after this reply
    fn then<U, F, Fut>(self, f: F) -> impl Future<Output = Result<U, MessageError>> + Send
    where
        F: FnOnce(R) -> Fut + Send,
        Fut: Future<Output = Result<U, MessageError>> + Send,
        U: Send;

    /// Map the reply value when it arrives
    fn map_reply<U, F>(self, f: F) -> impl Future<Output = Result<U, MessageError>> + Send
    where
        F: FnOnce(R) -> U + Send,
        U: Send;

    /// Map errors that occur during reply handling
    fn map_err<F>(self, f: F) -> impl Future<Output = Result<R, MessageError>> + Send
    where
        F: FnOnce(MessageError) -> MessageError + Send;
}

impl<R> MessageHandleExt<R> for MessageHandle<R>
where
    R: Send,
{
    async fn then<U, F, Fut>(self, f: F) -> Result<U, MessageError>
    where
        F: FnOnce(R) -> Fut + Send,
        Fut: Future<Output = Result<U, MessageError>> + Send,
        U: Send,
    {
        let reply = self.reply().await?;
        f(reply).await
    }

    async fn map_reply<U, F>(self, f: F) -> Result<U, MessageError>
    where
        F: FnOnce(R) -> U + Send,
        U: Send,
    {
        let reply = self.reply().await?;
        Ok(f(reply))
    }

    async fn map_err<F>(self, f: F) -> Result<R, MessageError>
    where
        F: FnOnce(MessageError) -> MessageError + Send,
    {
        self.reply().await.map_err(f)
    }
}