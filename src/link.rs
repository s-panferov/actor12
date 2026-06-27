//! Handles for talking to a spawned actor.
//!
//! ## Overview
//!
//! A [`Link<A>`] is the user-facing handle returned by
//! [`spawn`](crate::spawn). It is a cheap, cloneable, `Arc`-backed reference to
//! a running actor: cloning a link just bumps a reference count, and every clone
//! points at the same actor. As long as at least one [`Link`] is alive the actor
//! is kept running; when the last link is dropped the actor is cancelled (see
//! [Lifecycle](#lifecycle) below).
//!
//! Links are compared and hashed by identity — two links are equal iff they
//! refer to the same actor instance — so they can be used as keys in maps and
//! sets.
//!
//! ## Messaging
//!
//! [`Link`] exposes several messaging patterns. Which ones are available depends
//! on the actor's [`Message`](crate::Actor::Message) type:
//!
//! - When the message type is [`Multi<A>`](crate::Multi) the actor can handle
//!   many message types through the [`Handler`] trait, and you use the `*_dyn`
//!   methods:
//!   - [`ask_dyn`](Link::ask_dyn) — send a message and await its typed reply.
//!   - [`ask_dyn_async`](Link::ask_dyn_async) — like `ask_dyn`, but resolves the
//!     *send* first and hands back a future for the reply.
//!   - [`tell_dyn`](Link::tell_dyn) — fire-and-forget; no reply is awaited.
//!   - [`relay_dyn`](Link::relay_dyn) — forward a pre-built [`Envelope`] so the
//!     reply is delivered to the original requester.
//! - When the message type is a plain [`Envelope<T, R>`](crate::Envelope) the
//!   actor handles a single message type and you use [`send`](Link::send).
//! - [`send_raw`](Link::send_raw) is the lowest-level escape hatch: it pushes a
//!   raw [`Message`](crate::Actor::Message) into the mailbox and surfaces the
//!   channel error directly.
//!
//! ## Lifecycle
//!
//! - [`alive`](Link::alive) reports whether the actor is still running.
//! - [`wait`](Link::wait) yields a future that completes when it shuts down.
//! - [`cancel`](Link::cancel) requests shutdown with a reason; the
//!   [`cancel_and_wait`](Link::cancel_and_wait) variant also waits for it.
//! - Dropping the last [`Link`] cancels the actor with the *default* cancel
//!   reason via [`LinkState`]'s `Drop` impl.
//!
//! ## Type erasure
//!
//! [`to_dyn`](Link::to_dyn) converts a `Link<A>` into a [`DynLink<M>`], which
//! remembers only the message type `M` and erases the concrete actor type `A`.
//! This lets collections hold links to heterogeneous actors that all accept the
//! same message. A [`DynLink`] can be downcast back to a concrete `Link<A>` with
//! [`DynLink::to`].
//!
//! ## Example
//!
//! ```rust,no_run
//! use actor12::{spawn, Actor, Init, Handler, Call, Multi, MpscChannel};
//! use std::future::Future;
//!
//! struct Counter { count: i32 }
//! struct Increment;
//!
//! impl Actor for Counter {
//!     type Message = Multi<Self>;
//!     type Spec = ();
//!     type Channel = MpscChannel<Self::Message>;
//!     type Cancel = ();
//!     type State = ();
//!
//!     fn state(_spec: &Self::Spec) -> Self::State {}
//!
//!     fn init(_ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
//!         std::future::ready(Ok(Counter { count: 0 }))
//!     }
//! }
//!
//! impl Handler<Increment> for Counter {
//!     type Reply = anyhow::Result<i32>;
//!
//!     async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _msg: Increment) -> Self::Reply {
//!         self.count += 1;
//!         Ok(self.count)
//!     }
//! }
//!
//! # async fn example() -> anyhow::Result<()> {
//! let link = spawn::<Counter>(());
//!
//! // Ask and await the reply.
//! let count: anyhow::Result<i32> = link.ask_dyn(Increment).await;
//!
//! // Fire-and-forget.
//! link.tell_dyn(Increment).await;
//!
//! // Request shutdown.
//! link.cancel(());
//! # Ok(())
//! # }
//! ```

use std::fmt::Debug;
use std::sync::Arc;

use crate::cancel::CancelToken;
use downcast_rs::DowncastSync;
use downcast_rs::impl_downcast;
use futures::FutureExt;
use futures::future::BoxFuture;
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

/// The subset of an [`Actor`]'s associated types that a [`Link`] needs.
///
/// [`Link`] and the messaging machinery are generic over `ActorLike` rather than
/// [`Actor`] directly so they only depend on the four types they actually use.
/// There is a blanket impl for every [`Actor`], so any actor is automatically
/// `ActorLike` — you never implement this trait by hand.
pub trait ActorLike: 'static + Send + Sync + Sized {
    /// Reason value carried by a cancellation; must have a `Default` used when
    /// the actor is cancelled implicitly (e.g. on the last link drop).
    type Cancel: Clone + Default + Send + Sync + 'static;
    /// The type pushed into the actor's mailbox (often [`Multi<Self>`](crate::Multi)
    /// or an [`Envelope`]).
    type Message: Send + Sync + 'static;
    /// The channel transporting messages to the actor.
    type Channel: ActorChannel<Message = Self::Message>;
    /// User-facing state attached to the link and readable via [`Link::state`].
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

/// Shared inner state of a [`Link`], held behind an `Arc`.
///
/// Every clone of a [`Link`] points at the same `LinkState`. When the last
/// reference is dropped its [`Drop`] impl cancels the actor with the default
/// [`Cancel`](crate::Actor::Cancel) reason, so an actor outlives exactly as long as
/// one of its links does.
pub struct LinkState<A: ActorLike> {
    /// Sender used to deliver messages into the actor's mailbox.
    pub tx: <A::Channel as ActorChannel>::Sender,
    /// Token used to request cancellation of the actor.
    pub token: CancelToken<A::Cancel>,
    /// Optional handle to a supervising/monitor task, set once after spawn.
    pub monitor: OnceCell<JoinHandle<()>>,
    /// User-facing state snapshot, exposed through [`Link::state`].
    pub state: A::State,
}

/// A cloneable, reference-counted handle to a running actor.
///
/// `Link` is the main way to interact with an actor: send it messages, observe
/// whether it is still alive, and request its shutdown. Cloning is cheap (an
/// `Arc` bump) and all clones address the same actor.
///
/// Links have identity semantics: [`PartialEq`], [`Eq`] and [`Hash`] are based
/// on the underlying `Arc` pointer, so a link compares equal only to clones of
/// itself and can be used as a map or set key.
///
/// See the module-level documentation for the available messaging patterns.
pub struct Link<A: ActorLike> {
    pub(crate) state: Arc<LinkState<A>>,
}

impl<A: ActorLike> std::hash::Hash for Link<A> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.state).hash(state);
    }
}

impl<A: ActorLike> PartialEq for Link<A> {
    fn eq(&self, other: &Self) -> bool {
        Arc::as_ptr(&self.state) == Arc::as_ptr(&other.state)
    }
}

impl<A: ActorLike> Eq for Link<A> {}

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
    /// Construct a link from its raw parts.
    ///
    /// This is plumbing used by the spawning machinery; in normal code you obtain
    /// a link from [`spawn`](crate::spawn) rather than calling this directly.
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

    /// Returns `true` while the actor is still running.
    ///
    /// Reports `false` once the actor's mailbox has closed, i.e. after the actor
    /// has shut down.
    pub fn alive(&self) -> bool {
        !self.state.tx.is_closed()
    }

    /// Returns a future that completes when the actor shuts down.
    ///
    /// Resolves immediately if the actor has already stopped.
    pub fn wait(&self) -> BoxFuture<'_, ()> {
        self.state.tx.closed().boxed()
    }

    /// Borrows the user-facing [`State`](crate::Actor::State) attached to this link.
    pub fn state(&self) -> &A::State {
        &self.state.state
    }

    /// Records the monitor task handle. Set once, internally, after spawn.
    pub(crate) fn set_monitor(&mut self, handle: JoinHandle<()>) {
        self.state.monitor.set(handle).unwrap();
    }
}

impl<A: ActorLike> Link<A> {
    // ========== EXISTING API - PRESERVED FOR BACKWARD COMPATIBILITY ==========

    /// Sends a message and returns a future that resolves to the reply.
    ///
    /// Unlike [`ask_dyn`](Self::ask_dyn), the `.await` on *this* method only
    /// drives the send; it hands back a `'static` future you can await later to
    /// obtain the reply. This is useful for issuing a request now and collecting
    /// its answer afterwards (e.g. fanning out to many actors before joining).
    ///
    /// Available only when the actor's message type is [`Multi<A>`](crate::Multi)
    /// and `A` implements [`Handler<T>`].
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

    /// Fire-and-forget: sends a message without waiting for a reply.
    ///
    /// The reply channel is dropped, so the handler's reply (if any) is
    /// discarded. Send errors are ignored — if the actor is gone the message is
    /// simply dropped.
    ///
    /// Available only when the actor's message type is [`Multi<A>`](crate::Multi)
    /// and `A` implements [`Handler<T>`].
    ///
    /// ```rust,no_run
    /// # use actor12::{spawn, Actor, Init, Handler, Call, Multi, MpscChannel};
    /// # use std::future::Future;
    /// # struct Counter { count: i32 }
    /// # struct Increment;
    /// # impl Actor for Counter {
    /// #     type Message = Multi<Self>;
    /// #     type Spec = ();
    /// #     type Channel = MpscChannel<Self::Message>;
    /// #     type Cancel = ();
    /// #     type State = ();
    /// #     fn state(_spec: &Self::Spec) -> Self::State {}
    /// #     fn init(_ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
    /// #         std::future::ready(Ok(Counter { count: 0 }))
    /// #     }
    /// # }
    /// # impl Handler<Increment> for Counter {
    /// #     type Reply = anyhow::Result<i32>;
    /// #     async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _msg: Increment) -> Self::Reply {
    /// #         self.count += 1;
    /// #         Ok(self.count)
    /// #     }
    /// # }
    /// # async fn example() {
    /// let link = spawn::<Counter>(());
    /// link.tell_dyn(Increment).await; // reply discarded
    /// # }
    /// ```
    pub async fn tell_dyn<T>(&self, message: T) -> ()
    where
        T: SyncTrait,
        A: Handler<T>,
        A: ActorLike<Message = Multi<A>>,
    {
        let (envelope, _) = Envelope::<T, <A as Handler<T>>::Reply>::new(message);
        if let Ok(()) = self.state.tx.send(Multi::new(envelope)).await {}
    }

    /// Forwards a pre-built [`Envelope`] to the actor.
    ///
    /// Because the envelope already carries its own reply channel, the reply goes
    /// wherever that envelope's channel points — typically the original
    /// requester. Use this to relay a message received by one actor on to
    /// another without intercepting the reply.
    ///
    /// Available only when the actor's message type is [`Multi<A>`](crate::Multi)
    /// and `A` implements [`Handler<T>`].
    pub async fn relay_dyn<T>(&self, envelope: Envelope<T, <A as Handler<T>>::Reply>)
    where
        T: SyncTrait,
        A: Handler<T>,
        A: ActorLike<Message = Multi<A>>,
    {
        let _ = self.state.tx.send(Multi::new(envelope)).await;
    }

    /// Sends a message and awaits the actor's typed reply.
    ///
    /// This is the request-response workhorse for actors built on
    /// [`Multi<A>`](crate::Multi): it delivers `message`, waits for the matching
    /// [`Handler<T>`] to run, and returns its [`Reply`](Handler::Reply). If the
    /// actor is dead or the reply is dropped, the error is converted into the
    /// reply type via `FromError` (e.g. an `Err` for an `anyhow::Result`).
    ///
    /// Available only when the actor's message type is [`Multi<A>`](crate::Multi)
    /// and `A` implements [`Handler<T>`].
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use actor12::{spawn, Actor, Init, Handler, Call, Multi, MpscChannel};
    /// # use std::future::Future;
    /// # struct Counter { count: i32 }
    /// # struct Increment;
    /// # impl Actor for Counter {
    /// #     type Message = Multi<Self>;
    /// #     type Spec = ();
    /// #     type Channel = MpscChannel<Self::Message>;
    /// #     type Cancel = ();
    /// #     type State = ();
    /// #     fn state(_spec: &Self::Spec) -> Self::State {}
    /// #     fn init(_ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
    /// #         std::future::ready(Ok(Counter { count: 0 }))
    /// #     }
    /// # }
    /// # impl Handler<Increment> for Counter {
    /// #     type Reply = anyhow::Result<i32>;
    /// #     async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _msg: Increment) -> Self::Reply {
    /// #         self.count += 1;
    /// #         Ok(self.count)
    /// #     }
    /// # }
    /// # async fn example() -> anyhow::Result<()> {
    /// let link = spawn::<Counter>(());
    /// let count: i32 = link.ask_dyn(Increment).await?;
    /// assert_eq!(count, 1);
    /// # Ok(())
    /// # }
    /// ```
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

    /// Sends a message and awaits its reply, for single-message actors.
    ///
    /// This is the counterpart to [`ask_dyn`](Self::ask_dyn) for actors whose
    /// message type is a plain [`Envelope<T, R>`](crate::Envelope) rather than
    /// [`Multi<A>`](crate::Multi). It wraps `message` in an envelope, sends it,
    /// and returns the reply `R`. Send/receive failures are converted into `R`
    /// through `FromError`.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use actor12::{spawn, Actor, Init, Exec, Envelope, MpscChannel};
    /// # use std::future::Future;
    /// # struct Adder;
    /// # impl Actor for Adder {
    /// #     type Message = Envelope<i32, anyhow::Result<i32>>;
    /// #     type Spec = ();
    /// #     type Channel = MpscChannel<Self::Message>;
    /// #     type Cancel = ();
    /// #     type State = ();
    /// #     fn state(_spec: &Self::Spec) -> Self::State {}
    /// #     fn init(_ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
    /// #         std::future::ready(Ok(Adder))
    /// #     }
    /// #     async fn handle(&mut self, _ctx: Exec<'_, Self>, msg: Self::Message) {
    /// #         let n = msg.value;
    /// #         let _ = msg.reply.send(Ok(n + 1));
    /// #     }
    /// # }
    /// # async fn example() -> anyhow::Result<()> {
    /// let link = spawn::<Adder>(());
    /// let result: i32 = link.send::<i32, anyhow::Result<i32>>(41).await?;
    /// assert_eq!(result, 42);
    /// # Ok(())
    /// # }
    /// ```
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

    /// Sends a raw [`Message`](crate::Actor::Message) into the mailbox.
    ///
    /// The lowest-level send: it performs no envelope wrapping and returns the
    /// channel `ActorSendError` on failure instead of converting it. Prefer
    /// [`ask_dyn`](Self::ask_dyn)/[`tell_dyn`](Self::tell_dyn)/[`send`](Self::send)
    /// unless you need this control.
    pub async fn send_raw(&self, message: A::Message) -> Result<(), ActorSendError<A>> {
        self.state.tx.send(message).await
    }

    /// Requests that the actor shut down, with the given reason.
    ///
    /// Returns immediately after signalling; the actor observes the cancellation
    /// and stops on its own. Use [`cancel_and_wait`](Self::cancel_and_wait) to
    /// also wait for shutdown.
    ///
    /// ```rust,no_run
    /// # use actor12::{spawn, Actor, Init, MpscChannel, Multi};
    /// # use std::future::Future;
    /// # struct W;
    /// # impl Actor for W {
    /// #     type Message = Multi<Self>;
    /// #     type Spec = ();
    /// #     type Channel = MpscChannel<Self::Message>;
    /// #     type Cancel = ();
    /// #     type State = ();
    /// #     fn state(_spec: &Self::Spec) -> Self::State {}
    /// #     fn init(_ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
    /// #         std::future::ready(Ok(W))
    /// #     }
    /// # }
    /// let link = spawn::<W>(());
    /// link.cancel(()); // ask the actor to stop
    /// ```
    pub fn cancel(&self, reason: A::Cancel) {
        self.state.token.cancel(reason)
    }

    /// Requests shutdown and awaits the actor's termination.
    ///
    /// Like [`cancel`](Self::cancel), but the returned future resolves only once
    /// the actor's mailbox has closed.
    pub async fn cancel_and_wait(&self, reason: A::Cancel) {
        self.state.token.cancel(reason);
        self.state.tx.closed().await
    }

    /// Erases the actor type, producing a [`DynLink<M>`] keyed only by message `M`.
    ///
    /// Use this to store links to different actor types in a single collection,
    /// as long as they all handle the message type `M`. Recover the concrete link
    /// later with [`DynLink::to`].
    ///
    /// Available only when the actor's message type is [`Multi<A>`](crate::Multi)
    /// and `A` implements [`Handler<M>`].
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

/// Dropping the last reference to the shared state cancels the actor with the
/// default [`Cancel`](crate::Actor::Cancel) reason, tying the actor's lifetime to
/// its links.
impl<A: ActorLike> Drop for LinkState<A> {
    fn drop(&mut self) {
        self.token.cancel(A::Cancel::default())
    }
}

/// Object-safe interface backing a type-erased [`DynLink`].
///
/// [`LinkState<A>`] implements this for every message type `M` the actor
/// handles, exposing just the operations that don't mention the concrete actor
/// type. It extends [`DowncastSync`] so a `DynLink` can be downcast back to a
/// concrete [`Link`].
pub trait DynamicLink<T>: DowncastSync {
    /// Cancels the actor with its default reason.
    fn cancel(&self);
    /// Fire-and-forget send of `message`, erased over the actor type.
    fn tell_dyn(&self, message: T) -> BoxFuture<'_, ()>;
    /// Cancels the actor and waits for it to terminate.
    fn cancel_and_wait(&'_ self) -> BoxFuture<'_, ()>;
}

impl_downcast!(sync DynamicLink<M>);

/// A link with the concrete actor type erased, remembering only message type `M`.
///
/// Produced by [`Link::to_dyn`]. It supports the actor-type-agnostic subset of
/// the [`Link`] API ([`cancel`](DynLink::cancel),
/// [`tell_dyn`](DynLink::tell_dyn), [`cancel_and_wait`](DynLink::cancel_and_wait))
/// and can be turned back into a concrete [`Link`] via [`to`](DynLink::to).
pub struct DynLink<M: Send + Sync + 'static> {
    state: Arc<dyn DynamicLink<M>>,
}

impl<M: Send + Sync + 'static> DynLink<M> {
    /// Returns `true` if the erased actor is of concrete type `A`.
    ///
    /// Use this to check before calling [`to`](Self::to), which panics on a
    /// mismatch.
    pub fn is<A: Handler<M> + ActorLike<Message = Multi<A>>>(&self) -> bool {
        self.state.is::<LinkState<A>>()
    }

    /// Cancels the actor with its default reason.
    pub fn cancel(&self) {
        self.state.cancel();
    }

    /// Fire-and-forget send of `message` to the erased actor.
    pub fn tell_dyn(&self, message: M) -> BoxFuture<'_, ()> {
        self.state.tell_dyn(message)
    }

    /// Cancels the actor and returns a future that resolves once it has stopped.
    pub fn cancel_and_wait(&'_ self) -> BoxFuture<'_, ()> {
        self.state.cancel_and_wait()
    }

    /// Recovers the concrete [`Link<A>`] from this erased link.
    ///
    /// # Panics
    ///
    /// Panics if the erased actor is not of type `A`. Guard with
    /// [`is`](Self::is) when the type is not statically known.
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

/// Bridges a concrete [`LinkState<A>`] to the erased [`DynamicLink`] interface,
/// dispatching each operation to the actor's typed machinery.
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

/// Internal no-op message; provides a default [`Handler`] impl for every actor.
struct Noop;

impl<A: ActorLike> Handler<Noop> for A {
    type Reply = anyhow::Result<()>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _message: Noop) -> Self::Reply {
        Ok(())
    }
}
