use std::future::Future;

use futures::FutureExt as _;
use futures::future::BoxFuture;
use take_once::TakeOnce;

use crate::actor::ActorMessage;
use crate::actor::SyncTrait;
use crate::envelope::Envelope;
use crate::handler::Call;
use crate::handler::Exec;
use crate::handler::Handler;
use crate::link::ActorLike;

pub trait MultiHandler<A>
where
	Self: Send + Sync + 'static,
	A: ActorLike,
{
	fn handle<'a>(self: Box<Self>, state: &'a mut A, ctx: Exec<'a, A>) -> BoxFuture<'a, ()>;
}

pub struct Multi<A>
where
	A: ActorLike,
{
	pub handler: Box<dyn MultiHandler<A>>,
}

impl<A: ActorLike> ActorMessage<A> for Multi<A> {
	fn handle<'a>(
		self,
		state: &'a mut A,
		ctx: Exec<'a, A>,
	) -> impl Future<Output = ()> + Send + 'a {
		self.handler.handle(state, ctx)
	}
}

struct MultiEnvelope<M: SyncTrait, A: Handler<M>> {
	pub envelope: Envelope<M, <A as Handler<M>>::Reply>,
}

impl<A: ActorLike> Multi<A> {
	pub fn new<M: SyncTrait>(envelope: Envelope<M, <A as Handler<M>>::Reply>) -> Self
	where
		A: Handler<M>,
	{
		let runner = MultiEnvelope::<M, A> { envelope };
		Multi {
			handler: Box::new(runner),
		}
	}
}

impl<A, M> MultiHandler<A> for MultiEnvelope<M, A>
where
	M: SyncTrait,
	A: Handler<M>,
{
	fn handle<'a>(self: Box<Self>, state: &'a mut A, ctx: Exec<'a, A>) -> BoxFuture<'a, ()> {
		let (msg, reply) = self.envelope.split();

		async move {
			// `once` lives on the wrapper future's stack; `Call` borrows it,
			// so there is no per-message `Arc` allocation.
			let once = TakeOnce::new();
			let _ = once.store(reply);

			let value = {
				let context = Call {
					// Reborrow the actor context to the (shorter) lifetime of `once`.
					ctx: Exec { ctx: &mut *ctx.ctx },
					reply: &once,
				};
				Handler::<M>::handle(&mut *state, context, msg).await
			};

			// If the handler took the sender (manual `take_reply` or `reply_async`),
			// `once` is empty and delivery is the handler's responsibility.
			// Otherwise deliver the returned value now.
			if let Some(tx) = once.take() {
				let _ = tx.send(value);
			}
		}
		.boxed()
	}
}
