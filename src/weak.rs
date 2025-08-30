use std::sync::Arc;

use futures::FutureExt;
use futures::future::BoxFuture;
use tokio::sync::oneshot::error::RecvError;

use crate::Link;
use crate::actor::Actor;
use crate::envelope::Envelope;
use crate::error::ActorError;
use crate::error::ActorSendError;
use crate::error::FromError;
use crate::handler::Handler;
use crate::link::ActorLike;
use crate::link::LinkState;
use crate::multi::Multi;

pub struct WeakLink<A: ActorLike> {
	state: std::sync::Weak<LinkState<A>>,
}

impl<A: Actor> Clone for WeakLink<A> {
	fn clone(&self) -> Self {
		Self {
			state: self.state.clone(),
		}
	}
}

impl<A: Actor> WeakLink<A> {
	pub fn upgrade(&self) -> Option<Link<A>> {
		self.state.upgrade().map(|state| Link { state })
	}

	pub fn cancel(&self, reason: A::Cancel) {
		if let Some(link) = self.upgrade() {
			link.cancel(reason);
		}
	}

	pub async fn cancel_and_wait<'a>(&'a self, reason: A::Cancel) {
		if let Some(link) = self.upgrade() {
			link.cancel_and_wait(reason).await
		}
	}

	pub async fn ask_dyn_async<T>(&self, message: T) -> BoxFuture<'static, <A as Handler<T>>::Reply>
	where
		T: Send + Sync + 'static,
		A: Handler<T>,
		A: ActorLike<Message = Multi<A>>,
	{
		if let Some(link) = self.upgrade() {
			link.ask_dyn_async(message).await
		} else {
			std::future::ready(<A as Handler<T>>::Reply::from_err(ActorError::Dead)).boxed()
		}
	}

	pub async fn tell_dyn<T>(&self, message: T)
	where
		T: Send + Sync + 'static,
		A: Handler<T>,
		A: ActorLike<Message = Multi<A>>,
	{
		if let Some(link) = self.upgrade() {
			link.tell_dyn(message).await;
		}
	}

	pub async fn relay_dyn<T>(&self, envelope: Envelope<T, <A as Handler<T>>::Reply>)
	where
		T: Send + Sync + 'static,
		A: Handler<T>,
		A: ActorLike<Message = Multi<A>>,
	{
		if let Some(link) = self.upgrade() {
			link.relay_dyn(envelope).await;
		}
	}

	pub async fn ask_dyn<T>(&self, message: T) -> <A as Handler<T>>::Reply
	where
		T: Send + Sync + 'static,
		A: Handler<T>,
		A: ActorLike<Message = Multi<A>>,
	{
		if let Some(link) = self.upgrade() {
			link.ask_dyn(message).await
		} else {
			<A as Handler<T>>::Reply::from_err(ActorError::Dead)
		}
	}

	pub async fn send<T, R>(&self, message: T) -> R
	where
		A: Actor<Message = Envelope<T, R>>,
		T: Send + Sync + 'static,
		R: Send + Sync + 'static,
		R: FromError<ActorSendError<A>>,
		R: FromError<RecvError>,
		R: FromError<ActorError>,
	{
		if let Some(link) = self.upgrade() {
			link.send(message).await
		} else {
			R::from_err(ActorError::Dead)
		}
	}
}

impl<A: Actor> Link<A> {
	pub fn downgrade(&self) -> WeakLink<A> {
		WeakLink {
			state: Arc::downgrade(&self.state),
		}
	}
}
