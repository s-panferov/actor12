use std::any::type_name;
use std::fmt::Debug;
use std::future::Future;
use std::ops::ControlFlow;

use crate::cancel::CancelReason;
use crate::cancel::CancelToken;
use tokio::task::JoinError;
use tokio::task::JoinSet;
use tracing::Instrument;

use crate::WeakLink;
use crate::channel::ActorChannel;
use crate::channel::ActorReceiver as _;
use crate::handler::Exec;
use crate::link::ActorLike;
use crate::link::Link;

/// Runtime context for an active actor instance.
///
/// This structure contains all the necessary components for an actor's execution,
/// including message reception, cancellation handling, and task spawning capabilities.
pub struct ActorContext<A: ActorLike> {
	/// Message receiver for incoming actor messages
	pub rx: <A::Channel as ActorChannel>::Receiver,
	/// Cancellation token for graceful shutdown
	pub token: CancelToken<A::Cancel>,
	/// Task set for managing spawned futures
	pub futures: JoinSet<()>,
	/// Tracing span for observability
	pub span: tracing::Span,
	/// Weak reference to the actor's link
	pub link: WeakLink<A>,
}

impl<A: Actor> ActorContext<A> {
	/// Spawn a background task within the actor's context.
	///
	/// The spawned task will be automatically cancelled when the actor shuts down.
	pub fn spawn(&mut self, future: impl Future<Output = ()> + Send + 'static) {
		self.futures.spawn(future);
	}
}

/// Initialization context provided to actors during startup.
///
/// This structure contains everything needed for an actor to initialize itself,
/// including the initialization specification and the ability to spawn background tasks.
pub struct Init<'a, A: Actor> {
	/// The specification data required for actor initialization
	pub spec: A::Spec,
	/// Task set for spawning background tasks during initialization
	pub tasks: &'a mut JoinSet<()>,
	/// A strong link to the actor being initialized
	pub link: Link<A>,
	/// Cancellation token for the initialization process
	pub token: CancelToken<A::Cancel>,
}

impl<A: Actor> Init<'_, A> {
	/// Spawn a background task during actor initialization.
	///
	/// The spawned task will run alongside the actor and be automatically
	/// cancelled when the actor shuts down.
	pub fn spawn<F>(&mut self, future: F)
	where
		F: Future<Output = ()> + Send + 'static,
	{
		self.tasks.spawn(future);
	}
}

#[derive(Debug)]
pub enum Terminate {
	ProcessAll,
	Exit,
}

pub trait InitFuture<A: Actor>:
	Future<Output = Result<A, <A as Actor>::Cancel>> + Send + 'static
{
}

impl<A: Actor, F: Future<Output = Result<A, <A as Actor>::Cancel>> + Send + 'static> InitFuture<A>
	for F
{
}

pub trait Actor: Sized + Send + Sync + 'static {
	type Message: ActorMessage<Self>;
	type Spec: Send;
	type Channel: ActorChannel<Message = Self::Message>;
	type Cancel: Clone + Debug + Default + Send + Sync + 'static;
	type State: Send + Sync + 'static;

	fn span(_spec: &Self::Spec) -> tracing::Span {
		tracing::info_span!("Actor")
	}

	fn state(spec: &Self::Spec) -> Self::State;

	fn termination_strategy(&mut self) -> Terminate {
		Terminate::Exit
	}

	fn terminate(
		self,
		_ctx: ActorContext<Self>,
		_reason: CancelReason<Self::Cancel>,
	) -> impl Future<Output = ()> + Send {
		futures::future::ready(())
	}

	fn tick(&mut self) -> impl Future<Output = ()> + Send {
		futures::future::pending()
	}

	fn cycle(
		&mut self,
		ctx: &mut ActorContext<Self>,
	) -> impl Future<Output = ControlFlow<CancelReason<Self::Cancel>, ()>> + Send {
		async {
			tokio::select! {
				reason = ctx.token.cancelled_or_dropped() => {
					return ControlFlow::Break(reason.unwrap_or_default());
				},
				_ = Self::tick(self) => {
					return ControlFlow::Continue(())
				},
				msg = ctx.rx.recv() => {
					match msg {
						Some(msg) => Self::handle(self, Exec { ctx }, msg).await,
						None => return ControlFlow::Break(Default::default()),
					}
				}
			}

			ControlFlow::Continue(())
		}
	}

	fn crash(err: JoinError) -> impl Future<Output = ()> + Send {
		async move {
			tracing::error!("ACTOR DIED: {err:?}");
			std::process::exit(-1);
		}
	}

	fn handle<'a>(
		&'a mut self,
		_ctx: Exec<'a, Self>,
		msg: Self::Message,
	) -> impl Future<Output = ()> + Send + 'a {
		msg.handle(self, _ctx)
	}

	fn init(ctx: Init<'_, Self>) -> impl InitFuture<Self>;

	fn spawn(spec: Self::Spec) -> Link<Self> {
		let count = crate::countme::Count::<Self>::new();

		let (tx, rx) = Self::Channel::create(10);
		let token = CancelToken::<Self::Cancel>::new();

		let mut link: Link<Self> = Link::new(tx, token.clone(), Self::state(&spec));
		let mut join_set = JoinSet::default();

		let weak = link.downgrade();
		let span = Self::span(&spec);

		let state = Self::init(Init {
			spec: spec,
			token: token.clone(),
			tasks: &mut join_set,
			link: link.clone(),
		});

		let handle = tokio::spawn(
			{
				let span = span.clone();
				async move {
					let state = state.in_current_span().await;

					let mut state = match state {
						Ok(state) => state,
						Err(cancel) => {
							tracing::error!(
								reason = ?cancel,
								"Actor terminated before initialization"
							);
							token.cancel(cancel);
							return;
						}
					};

					let mut ctx = ActorContext {
						rx,
						token,
						futures: join_set,
						span: span.clone(),
						link: weak,
					};

					let reason = loop {
						match Self::cycle(&mut state, &mut ctx).in_current_span().await {
							ControlFlow::Continue(_) => {}
							ControlFlow::Break(reason) => break reason,
						}
					};

					Actor::terminate(state, ctx, reason).in_current_span().await;
				}
			}
			.instrument(span.clone()),
		);

		let monitor_handle = tokio::spawn(async move {
			let result = handle.await;
			match result {
				Ok(()) => {
					tracing::info!("Actor {} completed gracefully", type_name::<Self>());
				}
				Err(err) => {
					tracing::error!("Actor {} crashed", type_name::<Self>());
					Self::crash(err).await
				}
			}

			let _count_guard = count;
		});

		link.set_monitor(monitor_handle);
		link
	}
}

pub trait ActorMessage<A: ActorLike>: SyncTrait {
	fn handle<'a>(self, state: &'a mut A, ctx: Exec<'a, A>)
	-> impl Future<Output = ()> + Send + 'a;
}

pub trait SyncTrait: Sized + Send + Sync + 'static {}
impl<T: Send + Sync + 'static> SyncTrait for T {}
