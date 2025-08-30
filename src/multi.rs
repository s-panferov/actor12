use std::future::Future;
use std::sync::Arc;

use futures::FutureExt as _;
use futures::future::BoxFuture;
use take_once::TakeOnce;

use crate::actor::ActorMessage;
use crate::actor::SyncTrait;
use crate::envelope::Envelope;
use crate::handler::ActorReply;
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
		let once = TakeOnce::new();
		let _ = once.store(reply);
		let once = Arc::new(once);
		let context = Call {
			ctx: ctx,
			reply: once.clone(),
		};

		let future = Handler::<M>::handle(state, context, msg);

		let future = async move {
			let value = future.await;
			if value.is_async() {
				return;
			} else {
				let _ = once
					.take()
					.expect("Reply channel should not be copied")
					.send(value);
			}
		};

		future.boxed()
	}
}
