use std::ops::Deref;
use std::ops::DerefMut;

use tokio::task::JoinHandle;

#[derive(Debug)]
pub struct DropHandle<T>(pub JoinHandle<T>);

impl<T> DropHandle<T> {
	pub fn abort(&self) {
		self.0.abort();
	}
}

impl<T> Deref for DropHandle<T> {
	type Target = JoinHandle<T>;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T> DerefMut for DropHandle<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl<T> Drop for DropHandle<T> {
	fn drop(&mut self) {
		self.0.abort()
	}
}
