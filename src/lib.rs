//! # Actor12 Framework
//!
//! A lightweight, type-safe actor framework for Rust built on async/await.
//!
//! ## Overview
//!
//! Actor12 provides a simple yet powerful actor model implementation that leverages 
//! Rust's type system and async capabilities. Actors are isolated units of computation
//! that communicate through message passing, ensuring thread safety and preventing
//! data races.
//!
//! ## Key Features
//!
//! - **Type Safety**: Compile-time guarantees for message types and actor interactions
//! - **Async/Await**: Built on Tokio for high-performance async execution
//! - **Flexible Messaging**: Multiple patterns for different use cases
//! - **Hierarchical Cancellation**: Clean shutdown and resource management
//! - **Zero-Cost Abstractions**: Minimal runtime overhead
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use runy_actor::prelude::*;
//! use runy_actor::{spawn, Envelope, Multi, MpscChannel};
//!
//! // Define your actor
//! struct Counter {
//!     count: i32,
//! }
//!
//! impl Actor for Counter {
//!     type Message = Multi<Self>;
//!     type Spec = ();
//!     type Channel = MpscChannel<Self::Message>;
//!     type Cancel = ();
//!     type State = ();
//!
//!     fn state(_spec: &Self::Spec) -> Self::State {
//!         ()
//!     }
//!
//!     fn init(_ctx: Init<'_, Self>) -> impl std::future::Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
//!         std::future::ready(Ok(Counter { count: 0 }))
//!     }
//! }
//!
//! // Spawn the actor
//! let link = spawn::<Counter>(());
//! ```
//!
//! ## Architecture
//!
//! The framework is built around several core concepts:
//!
//! - [`Actor`]: The core trait defining actor behavior
//! - [`Link`]: Strong reference to an actor for sending messages
//! - [`Envelope`]: Type-safe message containers for request-response patterns
//! - [`Handler`]: Trait for polymorphic message handling
//! - [`Multi`]: Support for handling multiple message types in a single actor
//!
//! ## Examples
//!
//! See the `examples/` directory for comprehensive usage patterns including:
//! - Basic request-response communication
//! - State management
//! - Multiple message types
//! - Error handling
//! - Worker pools

mod actor;
mod channel;
mod drop;
mod envelope;
mod error;
mod handler;
mod link;
mod multi;
mod proxy;
mod weak;
mod cancel;
mod countme;
mod message;

/// Common imports for working with the Actor12 framework.
///
/// This module re-exports the most commonly used types and traits,
/// allowing for convenient imports with `use runy_actor::prelude::*;`
pub mod prelude {
	pub use super::actor::Actor;
	pub use super::actor::Init;
	pub use super::actor::InitFuture;
	pub use super::handler::Exec;
	pub use super::handler::Handler;
}

pub use actor::Actor;
pub use actor::ActorContext;
pub use actor::Init;
pub use channel::MpscChannel;
pub use drop::DropHandle;
pub use envelope::Envelope;
pub use envelope::NoReply;
pub use error::ActorError;
pub use handler::Call;
pub use handler::Exec;
pub use handler::Handler;
pub use link::DynLink;
pub use link::Link;
pub use multi::Multi;
pub use proxy::Proxy;
pub use weak::WeakLink;
pub use message::{MessageHandle, MessageError, SendableMessage, WeakSendableMessage, HandlerMessage, MessageHandleExt, EnvelopeMessage, RelayMessage};

/// Spawn a new actor instance with the given specification.
///
/// This is a convenience function that delegates to the actor's associated
/// `spawn` method, providing a unified interface for actor creation.
///
/// # Arguments
///
/// * `spec` - The specification required to initialize the actor
///
/// # Returns
///
/// A [`Link<A>`] that can be used to send messages to the spawned actor.
///
/// # Examples
///
/// ```rust,no_run
/// use runy_actor::{spawn, Actor, Init, Exec, MpscChannel, Multi};
///
/// struct MyActor;
///
/// impl Actor for MyActor {
///     type Message = Multi<Self>;
///     type Spec = ();
///     type Channel = MpscChannel<Self::Message>;
///     type Cancel = ();
///     type State = ();
///
///     fn state(_spec: &Self::Spec) -> Self::State {
///         ()
///     }
///
///     fn init(_ctx: Init<'_, Self>) -> impl std::future::Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
///         std::future::ready(Ok(MyActor))
///     }
/// }
///
/// // Spawn the actor
/// let link = spawn::<MyActor>(());
/// ```
pub fn spawn<A: Actor>(spec: A::Spec) -> Link<A> {
	A::spawn(spec)
}
