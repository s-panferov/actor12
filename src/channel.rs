use std::future::Future;
use std::marker::PhantomData;

use tokio::sync::mpsc;

pub trait ActorChannel {
	type Message: Send;
	type Receiver: ActorReceiver<Self::Message> + Sync;
	type Sender: ActorSender<Self::Message> + Sync + Clone;

	fn create(buffer: usize) -> (Self::Sender, Self::Receiver);
}

pub struct MpscChannel<T> {
	_t: PhantomData<T>,
}

impl<T> ActorChannel for MpscChannel<T>
where
	T: Send + Sync + 'static,
{
	type Message = T;
	type Receiver = mpsc::Receiver<T>;
	type Sender = mpsc::Sender<T>;
	fn create(buffer: usize) -> (Self::Sender, Self::Receiver) {
		mpsc::channel(buffer)
	}
}

pub trait ActorReceiver<T>: Send
where
	T: Send,
{
	fn recv(&mut self) -> impl Future<Output = Option<T>> + Send;
	#[allow(unused)]
	fn is_closed(&mut self) -> bool;
}

impl<T> ActorReceiver<T> for mpsc::Receiver<T>
where
	T: Send,
{
	async fn recv(&mut self) -> Option<T> {
		mpsc::Receiver::recv(self).await
	}

	fn is_closed(&mut self) -> bool {
		mpsc::Receiver::is_closed(self)
	}
}

pub trait ActorSender<T>: Send
where
	T: Send,
{
	type Error: std::error::Error + std::marker::Send + std::marker::Sync;
	fn send(&self, value: T) -> impl Future<Output = Result<(), Self::Error>> + Send;
	fn closed(&self) -> impl Future<Output = ()> + Send;
	fn is_closed(&self) -> bool;
}

impl<T> ActorSender<T> for mpsc::Sender<T>
where
	T: Send + Sync + 'static,
{
	type Error = mpsc::error::SendError<T>;

	async fn send(&self, value: T) -> Result<(), Self::Error> {
		mpsc::Sender::send(self, value).await
	}

	async fn closed(&self) -> () {
		mpsc::Sender::closed(self).await
	}

	fn is_closed(&self) -> bool {
		mpsc::Sender::is_closed(self)
	}
}
