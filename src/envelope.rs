use std::fmt::Debug;
use std::future::Future;

use tokio::sync::oneshot;

use crate::actor::Actor;
use crate::actor::ActorMessage;
use crate::handler::Exec;

pub struct Envelope<T, R> {
	pub value: T,
	pub reply: oneshot::Sender<R>,
}

impl<T: Debug, R> Debug for Envelope<T, R> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Debug::fmt(&self.value, f)
	}
}

impl<T, R> Envelope<T, R> {
	pub fn new(value: T) -> (Self, oneshot::Receiver<R>) {
		let (reply, rx) = oneshot::channel();
		(Self { value, reply }, rx)
	}



	pub fn relay(value: T, reply: oneshot::Sender<R>) -> Self {
		Self { value, reply }
	}

	pub fn send(self, value: R) -> Result<(), R> {
		self.reply.send(value)
	}

	pub fn split(self) -> (T, oneshot::Sender<R>) {
		let value = self.value;
		let reply = self.reply;
		(value, reply)
	}

	pub fn map<U>(self, func: impl FnOnce(T) -> U) -> Envelope<U, R> {
		Envelope {
			value: func(self.value),
			reply: self.reply,
		}
	}

	pub async fn reply_fut<F: Future<Output = R>>(self, fut: F)
	where
		T: Send + 'static,
		R: Send + 'static,
		F: Send + 'static,
	{
		let value = fut.await;
		let _ = self.reply.send(value);
	}

	pub async fn reply<F: Future<Output = R>>(self, func: impl Fn(T) -> F + Send + 'static)
	where
		T: Send + 'static,
		R: Send + 'static,
		F: Send,
	{
		let value = func(self.value).await;
		let _ = self.reply.send(value);
	}
}

impl<T, R, A: Actor> ActorMessage<A> for Envelope<T, R>
where
	T: Send + Sync + 'static,
	R: Send + Sync + 'static,
{
	fn handle(self, _: &mut A, _: Exec<A>) -> impl Future<Output = ()> + Send {
		futures::future::ready(())
	}
}

pub struct NoReply<T>(pub T);

impl<T> NoReply<T> {
	pub fn new(value: T) -> Self {
		Self(value)
	}
}

impl<T: Debug> Debug for NoReply<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Debug::fmt(&self.0, f)
	}
}
impl<T, A: Actor> ActorMessage<A> for NoReply<T>
where
	T: Send + Sync + 'static,
{
	fn handle(self, _: &mut A, _: Exec<A>) -> impl Future<Output = ()> + Send {
		futures::future::ready(())
	}
}
