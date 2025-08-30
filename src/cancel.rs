use std::fmt::Debug;
use std::future::Future;
use std::ops::Deref;
use std::panic::Location;
use std::sync::Arc;
use std::sync::OnceLock;

use parking_lot::Mutex;
use tokio::sync::watch::Receiver;
use tokio::sync::watch::Sender;

#[derive(Clone)]
pub struct CancelToken<T: Clone> {
	inner: Arc<TreeNode<T>>,
}

impl<T: Clone> Default for CancelToken<T> {
	fn default() -> Self {
		Self::new()
	}
}

impl<T: Clone + Debug> Debug for CancelToken<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CancelToken")
			.field("state", &*self.inner.state.borrow())
			.finish()
	}
}

impl<T: Clone> CancelToken<T> {
	pub fn new() -> Self {
		CancelToken {
			inner: TreeNode::new(),
		}
	}

	pub fn reset(&self) {
		self.inner.reset()
	}

	#[track_caller]
	pub fn cancel<V: Into<T>>(&self, reason: V)
	where
		T: Clone,
	{
		self.inner.cancel_with_reason(CancelReason::new_with_loc(
			reason.into(),
			Location::caller(),
		))
	}

	pub fn cancel_with_reason(&self, reason: CancelReason<T>)
	where
		T: Clone,
	{
		self.inner.cancel_with_reason(reason)
	}

	pub fn is_cancelled(&self) -> bool {
		let state = self.inner.state.borrow();
		match &*state {
			State::Running => false,
			State::Cancelled(_) => true,
		}
	}

	pub fn reason(&self) -> Option<CancelReason<T>> {
		let state = self.inner.state.borrow();
		match &*state {
			State::Running => None,
			State::Cancelled(reason) => Some(reason.clone()),
		}
	}

	pub fn cancelled(&self) -> impl Future<Output = CancelReason<T>> {
		TreeNode::cancelled(self.inner.state.subscribe())
	}

	pub fn cancelled_or_dropped(&self) -> impl Future<Output = Option<CancelReason<T>>> {
		TreeNode::cancelled_or_dropped(self.inner.state.subscribe())
	}

	pub fn child(&self) -> CancelToken<T> {
		CancelToken {
			inner: self.inner.child(),
		}
	}
}

#[derive(Debug)]
pub enum State<T> {
	Running,
	Cancelled(CancelReason<T>),
}

#[derive(Debug, PartialEq, Eq)]
pub struct CancelReason<T> {
	value: T,
	location: &'static Location<'static>,
}

impl<T: Default> Default for CancelReason<T> {
	fn default() -> Self {
		CancelReason {
			value: Default::default(),
			location: Location::caller(),
		}
	}
}

impl<T> CancelReason<T> {
	#[track_caller]
	pub fn new<V: Into<T>>(value: V) -> Self {
		Self {
			value: value.into(),
			location: Location::caller(),
		}
	}

	pub fn new_with_loc(value: T, location: &'static Location<'static>) -> Self {
		Self { value, location }
	}
}

impl<T> Clone for CancelReason<T>
where
	T: Clone,
{
	fn clone(&self) -> Self {
		CancelReason {
			value: self.value.clone(),
			location: self.location,
		}
	}
}

pub struct TreeNode<T: Clone> {
	pub state: Sender<State<T>>,
	pub children: parking_lot::Mutex<Vec<Arc<TreeNode<T>>>>,
	pub drop: OnceLock<T>,
}

impl<T: Clone> Drop for TreeNode<T> {
	#[track_caller]
	fn drop(&mut self) {
		if let Some(value) = self.drop.take() {
			self.cancel_with_reason(CancelReason::new_with_loc(value, Location::caller()));
		}
	}
}

impl<T: Clone> TreeNode<T> {
	pub fn new() -> Arc<Self> {
		Arc::new(Self {
			state: Sender::new(State::Running),
			children: Mutex::new(Vec::new()),
			drop: OnceLock::new(),
		})
	}

	pub fn reset(&self) {
		self.children.lock().clear();
		self.state.send_replace(State::Running);
	}

	#[allow(dead_code)]
	pub fn on_drop(&self, value: T) {
		let _ = self.drop.set(value);
	}

	pub fn child(self: &Arc<Self>) -> Arc<Self> {
		let mut children = self.children.lock();
		match *self.state.borrow() {
			State::Running => {
				let node = TreeNode::new();
				children.push(node.clone());
				node
			}
			// If we are cancelled already then we can just return
			// a clone of us
			State::Cancelled(_) => self.clone(),
		}
	}

	pub async fn cancelled(mut recv: Receiver<State<T>>) -> CancelReason<T>
	where
		T: Clone,
	{
		{
			let result = recv
				.wait_for(|state| match state {
					State::Running => false,
					State::Cancelled(_) => true,
				})
				.await;

			match result.as_deref() {
				Err(_) => {}
				Ok(State::Running) => unreachable!(),
				Ok(State::Cancelled(reason)) => return reason.clone(),
			};
		}

		std::future::pending::<()>().await;
		unreachable!();
	}

	pub async fn cancelled_or_dropped(mut recv: Receiver<State<T>>) -> Option<CancelReason<T>>
	where
		T: Clone,
	{
		let result = recv
			.wait_for(|state| match state {
				State::Running => false,
				State::Cancelled(_) => true,
			})
			.await;

		match result.as_deref() {
			Err(_) => None,
			Ok(State::Running) => unreachable!(),
			Ok(State::Cancelled(reason)) => Some(reason.clone()),
		}
	}

	fn cancel_with_reason(&self, reason: CancelReason<T>)
	where
		T: Clone,
	{
		// locking to prevent adding new children while we are cancelling
		let children = self.children.lock();

		let need_cancel = self.state.send_if_modified(|state| match state {
			State::Running => {
				*state = State::Cancelled(reason.clone());
				true
			}
			// do not re-cancel
			State::Cancelled(_) => false,
		});

		if need_cancel {
			for child in children.deref() {
				child.cancel_with_reason(reason.clone())
			}
		}
	}
}