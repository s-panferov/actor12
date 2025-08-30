use std::error::Error;

use crate::channel::{ActorChannel, ActorSender};
use crate::link::ActorLike;

pub type ActorSendError<A> = <<<A as ActorLike>::Channel as ActorChannel>::Sender as ActorSender<
	<<A as ActorLike>::Channel as ActorChannel>::Message,
>>::Error;

#[derive(thiserror::Error, Debug)]
pub enum ActorError {
	#[error("Dynamic send error")]
	DynSendError(),

	#[error("Actor is already dead")]
	Dead,

	#[error("Reply taken")]
	ReplyTaken,

	#[error("Async reply")]
	AsyncReply,
}

pub trait FromError<E> {
	fn from_err(err: E) -> Self
	where
		Self: Sized;
}

impl<T, E> FromError<E> for anyhow::Result<T>
where
	T: Send + Sync + 'static,
	E: Error + Send + Sync + 'static,
{
	fn from_err(err: E) -> Self {
		Err(anyhow::Error::new(err))
	}
}
