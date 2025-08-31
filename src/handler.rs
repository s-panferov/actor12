use std::future::Future;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;

use downcast_rs::DowncastSync;
use take_once::TakeOnce;
use tokio::sync::oneshot::error::RecvError;

use crate::actor::Actor;
use crate::actor::ActorContext;
use crate::actor::SyncTrait;
use crate::error::ActorError;
use crate::error::ActorSendError;
use crate::error::FromError;
use crate::link::ActorLike;

pub trait ActorReply: DowncastSync {
    fn is_async(&self) -> bool;
}

downcast_rs::impl_downcast!(sync ActorReply);

impl<T> ActorReply for anyhow::Result<T>
where
    T: Send + Sync + 'static,
{
    fn is_async(&self) -> bool {
        match self {
            Ok(_) => false,
            Err(e) => match e.downcast_ref::<ActorError>() {
                Some(err) => matches!(err, ActorError::AsyncReply),
                None => false,
            },
        }
    }
}

pub trait Handler<M>: ActorLike
where
    M: SyncTrait,
{
    type Reply: Send
        + Sync
        + 'static
        + FromError<ActorSendError<Self>>
        + FromError<RecvError>
        + FromError<ActorError>
        + ActorReply;

    fn handle<'a>(
        &'a mut self,
        ctx: Call<'a, Self, Self::Reply>,
        ev: M,
    ) -> impl Future<Output = Self::Reply> + use<'a, M, Self> + Send;
}

pub struct Exec<'a, A: ActorLike> {
    pub(crate) ctx: &'a mut ActorContext<A>,
}

pub struct Call<'a, A: ActorLike, R> {
    pub(crate) reply: Arc<TakeOnce<tokio::sync::oneshot::Sender<R>>>,
    pub ctx: Exec<'a, A>,
}

impl<'a, A, R> Deref for Call<'a, A, R>
where
    A: Actor,
{
    type Target = Exec<'a, A>;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl<'a, A, R> DerefMut for Call<'a, A, R>
where
    A: Actor,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ctx
    }
}

impl<A, R> Call<'_, A, R>
where
    A: Actor,
    R: Send + Sync + 'static,
    R: FromError<ActorError> + ActorReply,
{
    pub fn take_reply(&mut self) -> tokio::sync::oneshot::Sender<R> {
        self.reply.take().unwrap()
    }

    pub fn reply_async<F>(&mut self, future: F) -> R
    where
        F: Future<Output = R> + Send + 'static,
    {
        let reply = self.reply.take();
        let Some(reply) = reply else {
            return R::from_err(ActorError::ReplyTaken);
        };

        self.ctx.spawn(async move {
            let result = future.await;
            let _ = reply.send(result);
        });

        return R::from_err(ActorError::AsyncReply);
    }
}

impl<'a, A> Exec<'a, A>
where
    A: Actor,
{
    pub fn new(ctx: &'a mut ActorContext<A>) -> Self {
        Self { ctx }
    }
}

impl<'a, A> Deref for Exec<'a, A>
where
    A: Actor,
{
    type Target = ActorContext<A>;

    fn deref(&self) -> &Self::Target {
        self.ctx
    }
}

impl<'a, A> DerefMut for Exec<'a, A>
where
    A: Actor,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ctx
    }
}
