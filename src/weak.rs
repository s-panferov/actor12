use std::hash::Hash;
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

/// A non-owning handle to an actor, obtained via [`Link::downgrade`].
///
/// A `WeakLink` does not keep the actor alive. It can also be *empty*
/// ([`WeakLink::new`] / [`Default`]) and pointed at an actor later with
/// [`set`](WeakLink::set) or [`try_set`](WeakLink::try_set). An empty link
/// behaves exactly like a link to a dead actor: [`upgrade`](WeakLink::upgrade)
/// returns `None`, sends resolve to [`ActorError::Dead`] and cancels are no-ops.
///
/// Equality and hashing are by identity; all empty links compare equal.
pub struct WeakLink<A: ActorLike> {
    state: std::sync::Weak<LinkState<A>>,
}

impl<A: ActorLike> WeakLink<A> {
    /// Creates an empty link that points at no actor. Does not allocate.
    pub fn new() -> Self {
        Self {
            state: std::sync::Weak::new(),
        }
    }

    /// Returns `true` if the link has been pointed at an actor, even if that
    /// actor has since shut down. Returns `false` for an empty link.
    pub fn is_set(&self) -> bool {
        !self.state.ptr_eq(&std::sync::Weak::new())
    }

    /// Points the link at `link`'s actor, replacing any previous target.
    pub fn set(&mut self, link: &Link<A>) {
        self.state = Arc::downgrade(&link.state);
    }

    /// Points the link at `link`'s actor only if it is still empty.
    ///
    /// Returns `true` if the link was set, `false` if it already had a target
    /// (which is left unchanged).
    pub fn try_set(&mut self, link: &Link<A>) -> bool {
        if self.is_set() {
            return false;
        }
        self.set(link);
        true
    }

    /// Resets the link to empty.
    pub fn clear(&mut self) {
        self.state = std::sync::Weak::new();
    }

    /// Takes the current target out, leaving the link empty.
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}

impl<A: ActorLike> Default for WeakLink<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: ActorLike> From<&Link<A>> for WeakLink<A> {
    fn from(link: &Link<A>) -> Self {
        Self {
            state: Arc::downgrade(&link.state),
        }
    }
}

impl<A: ActorLike> std::fmt::Debug for WeakLink<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WeakLink")
            .field("set", &self.is_set())
            .field("alive", &(self.state.strong_count() > 0))
            .finish()
    }
}

impl<A: ActorLike> Hash for WeakLink<A> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.state.as_ptr().hash(state);
    }
}

impl<A: ActorLike> PartialEq for WeakLink<A> {
    fn eq(&self, other: &Self) -> bool {
        self.state.as_ptr() == other.state.as_ptr()
    }
}

impl<A: ActorLike> Eq for WeakLink<A> {}

impl<A: ActorLike> Clone for WeakLink<A> {
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

    // ========== EXISTING API - PRESERVED FOR BACKWARD COMPATIBILITY ==========

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
