use std::fmt::Debug;
use std::sync::Arc;

use downcast_rs::DowncastSync;
use downcast_rs::impl_downcast;
use futures::FutureExt;
use futures::future::BoxFuture;
use crate::cancel::CancelToken;
use tokio::sync::OnceCell;
use tokio::sync::oneshot::error::RecvError;
use tokio::task::JoinHandle;

use crate::Call;
use crate::actor::Actor;
use crate::actor::SyncTrait;
use crate::channel::ActorChannel;
use crate::channel::ActorSender;
use crate::envelope::Envelope;
use crate::error::ActorSendError;
use crate::error::FromError;
use crate::handler::Handler;
use crate::multi::Multi;

pub trait ActorLike: 'static + Send + Sync + Sized {
	type Cancel: Clone + Default + Send + Sync + 'static;
	type Message: Send + Sync + 'static;
	type Channel: ActorChannel<Message = Self::Message>;
	type State: Send + Sync + 'static;
}

impl<A> ActorLike for A
where
	A: Actor,
{
	type Cancel = <Self as Actor>::Cancel;
	type Message = <Self as Actor>::Message;
	type Channel = <Self as Actor>::Channel;
	type State = <Self as Actor>::State;
}

pub struct LinkState<A: ActorLike> {
	pub tx: <A::Channel as ActorChannel>::Sender,
	pub token: CancelToken<A::Cancel>,
	pub monitor: OnceCell<JoinHandle<()>>,
	pub state: A::State,
}

pub struct Link<A: ActorLike> {
	pub(crate) state: Arc<LinkState<A>>,
}

impl<A: ActorLike> Debug for Link<A>
where
	A::State: Debug,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Link")
			.field("state", &self.state.state)
			.finish()
	}
}

impl<A: ActorLike> Clone for Link<A> {
	fn clone(&self) -> Self {
		let state = self.state.clone();
		Self { state }
	}
}

impl<A: ActorLike> Link<A> {
	pub fn new(
		tx: <A::Channel as ActorChannel>::Sender,
		token: CancelToken<A::Cancel>,
		state: A::State,
	) -> Self {
		let state = Arc::new(LinkState {
			tx,
			token,
			monitor: Default::default(),
			state,
		});
		Self { state }
	}

	pub fn alive(&self) -> bool {
		!self.state.tx.is_closed()
	}

	pub fn state(&self) -> &A::State {
		&self.state.state
	}

	pub(crate) fn set_monitor(&mut self, handle: JoinHandle<()>) {
		self.state.monitor.set(handle).unwrap();
	}
}

impl<A: ActorLike> Link<A> {
	pub async fn ask_dyn_async<T>(&self, message: T) -> BoxFuture<'static, <A as Handler<T>>::Reply>
	where
		T: SyncTrait,
		A: Handler<T>,
		A: ActorLike<Message = Multi<A>>,
	{
		let (envelope, rx) = Envelope::<T, <A as Handler<T>>::Reply>::new(message);
		match self.state.tx.send(Multi::new(envelope)).await {
			Ok(()) => {}
			Err(e) => {
				return std::future::ready(<A as Handler<T>>::Reply::from_err(e)).boxed();
			}
		}

		async move {
			match rx.await {
				Ok(response) => response,
				Err(e) => <A as Handler<T>>::Reply::from_err(e),
			}
		}
		.boxed()
	}

	pub async fn tell_dyn<T>(&self, message: T) -> ()
	where
		T: SyncTrait,
		A: Handler<T>,
		A: ActorLike<Message = Multi<A>>,
	{
		let (envelope, _) = Envelope::<T, <A as Handler<T>>::Reply>::new(message);
		if let Ok(()) = self.state.tx.send(Multi::new(envelope)).await {}
	}

	pub async fn relay_dyn<T>(&self, envelope: Envelope<T, <A as Handler<T>>::Reply>)
	where
		T: SyncTrait,
		A: Handler<T>,
		A: ActorLike<Message = Multi<A>>,
	{
		let _ = self.state.tx.send(Multi::new(envelope)).await;
	}

	pub async fn ask_dyn<T>(&self, message: T) -> <A as Handler<T>>::Reply
	where
		T: SyncTrait,
		A: Handler<T>,
		A: ActorLike<Message = Multi<A>>,
	{
		let (envelope, rx) = Envelope::<T, <A as Handler<T>>::Reply>::new(message);
		match self.state.tx.send(Multi::new(envelope)).await {
			Ok(()) => {}
			Err(e) => {
				return <A as Handler<T>>::Reply::from_err(e);
			}
		}

		match rx.await {
			Ok(response) => response,
			Err(e) => <A as Handler<T>>::Reply::from_err(e),
		}
	}

	pub async fn send<T, R>(&self, message: T) -> R
	where
		A: ActorLike<Message = Envelope<T, R>>,
		T: Send + Sync + 'static,
		R: Send + Sync + 'static,
		R: FromError<ActorSendError<A>>,
		R: FromError<RecvError>,
	{
		let (envelope, rx) = Envelope::<T, R>::new(message);
		match self.state.tx.send(envelope).await {
			Ok(()) => {}
			Err(e) => {
				return R::from_err(e);
			}
		}

		match rx.await {
			Ok(response) => response,
			Err(e) => R::from_err(e),
		}
	}

	pub async fn send_raw(&self, message: A::Message) -> Result<(), ActorSendError<A>> {
		self.state.tx.send(message).await
	}

	pub fn cancel(&self, reason: A::Cancel) {
		self.state.token.cancel(reason)
	}

	pub async fn cancel_and_wait(&self, reason: A::Cancel) {
		self.state.token.cancel(reason);
		self.state.tx.closed().await
	}

	pub fn to_dyn<M>(&self) -> DynLink<M>
	where
		M: Send + Sync + 'static,
		A: Handler<M>,
		A: ActorLike<Message = Multi<A>>,
	{
		DynLink {
			state: self.state.clone(),
		}
	}
}

impl<A: ActorLike> Drop for LinkState<A> {
	fn drop(&mut self) {
		self.token.cancel(A::Cancel::default())
	}
}

pub trait DynamicLink<T>: DowncastSync {
	fn cancel(&self);
	fn tell_dyn(&self, message: T) -> BoxFuture<'_, ()>;
	fn cancel_and_wait(&'_ self) -> BoxFuture<'_, ()>;
}

impl_downcast!(sync DynamicLink<M>);

pub struct DynLink<M: Send + Sync + 'static> {
	state: Arc<dyn DynamicLink<M>>,
}

impl<M: Send + Sync + 'static> DynLink<M> {
	pub fn is<A: Handler<M> + ActorLike<Message = Multi<A>>>(&self) -> bool {
		self.state.is::<LinkState<A>>()
	}

	pub fn cancel(&self) {
		self.state.cancel();
	}

	pub fn tell_dyn(&self, message: M) -> BoxFuture<'_, ()> {
		self.state.tell_dyn(message)
	}

	pub fn cancel_and_wait(&'_ self) -> BoxFuture<'_, ()> {
		self.state.cancel_and_wait()
	}

	pub fn to<A: Handler<M> + ActorLike<Message = Multi<A>>>(&self) -> Link<A> {
		Link {
			state: self
				.state
				.clone()
				.downcast_arc::<LinkState<A>>()
				.map_err(|_| ())
				.unwrap(),
		}
	}
}

impl<A: ActorLike, M: Send + Sync + 'static> DynamicLink<M> for LinkState<A>
where
	A::Cancel: Default,
	A: Handler<M>,
	A: ActorLike<Message = Multi<A>>,
{
	fn cancel(&self) {
		let value: A::Cancel = Default::default();
		self.token.cancel(value);
	}

	fn tell_dyn(&self, message: M) -> BoxFuture<'_, ()> {
		let (envelope, _) = Envelope::<M, <A as Handler<M>>::Reply>::new(message);
		self.tx.send(Multi::new(envelope)).map(|_| ()).boxed()
	}

	fn cancel_and_wait(&'_ self) -> BoxFuture<'_, ()> {
		<LinkState<A> as DynamicLink<M>>::cancel(self);
		self.tx.closed().boxed()
	}
}

struct Noop;

impl<A: ActorLike> Handler<Noop> for A {
	type Reply = anyhow::Result<()>;

	async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _message: Noop) -> Self::Reply {
		Ok(())
	}
}
