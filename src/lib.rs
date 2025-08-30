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

pub fn spawn<A: Actor>(spec: A::Spec) -> Link<A> {
	A::spawn(spec)
}
