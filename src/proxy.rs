use std::sync::Arc;

use arc_swap::ArcSwap;
use crate::cancel::CancelToken;
use tokio::sync::Notify;
use tokio::sync::mpsc;

use crate::Actor;
use crate::Link;
use crate::channel::ActorChannel;
use crate::channel::ActorReceiver as _;

pub struct Proxy<A: Actor> {
	pub spec: A::Spec,
	pub state: ArcSwap<ProxyState<A>>,
	pub state_tx: mpsc::Sender<Arc<ProxyState<A>>>,
}

pub struct ProxyState<A: Actor> {
	pub token: CancelToken<A::Cancel>,
	pub link: Option<Link<A>>,
	pub stop: Notify,
}

impl<A: Actor> Proxy<A>
where
	A::Spec: Clone,
{
	pub fn new(spec: A::Spec) -> (Self, Link<A>) {
		let (tx, mut rx) = A::Channel::create(100);
		let token = CancelToken::new();

		let external_ref = Link::<A>::new(tx, token.clone(), A::state(&spec));
		let (state_tx, mut state_rx) = mpsc::channel::<Arc<ProxyState<A>>>(1);
		let state = ArcSwap::new(Arc::new(ProxyState {
			token,
			link: None,
			stop: Notify::new(),
		}));

		tokio::spawn({
			let mut state = state.load_full();
			async move {
				loop {
					#[rustfmt::skip]
          tokio::select! {
            new = state_rx.recv() => {
              match new {
                Some(new) => {
                  tracing::info!("Received a state update");
                  state = new;
                }
                None => {
                  state.token.cancel(A::Cancel::default());
                  tracing::info!("Proxy shutdown due to state channel close");
                  break;
                }
              }
            },
            reason = state.token.cancelled(), if !state.token.is_cancelled() => {
              if let Some(link) = state.link.as_ref() {
                tracing::info!("Proxy shutdown due to cancellation: {:?}", reason);
                link.cancel_and_wait(A::Cancel::default()).await;
              }
              state.stop.notify_waiters();
            }
            msg = rx.recv(), if state.link.as_ref().map(|l| l.alive()).unwrap_or(false) => {
              match msg {
                Some(msg) => {
                  if let Some(link) = state.link.as_ref() {
                    tracing::info!("Proxy message relayed");
                    let _ = link.send_raw(msg).await;
                  } else {
                    unreachable!("Proxy lost link unexpectedly"); 
                  }
                }
                None => {
                  tracing::warn!("Proxy lost input channel");
                  state.token.cancel(A::Cancel::default());
                  break;
                }
              }
            }
          }
				}
			}
		});

		(
			Self {
				spec,
				state_tx,
				state,
			},
			external_ref,
		)
	}

	pub fn init(&self) {
		let state = self.state.load();
		assert!(state.link.is_none(), "Proxy is already initialized");
		let new_state = Arc::new(ProxyState {
			token: CancelToken::new(),
			link: Some(A::spawn(self.spec.clone())),
			stop: Notify::new(),
		});

		self.state.store(new_state);
		self.state_tx.try_send(self.state.load_full()).unwrap();
	}

	pub async fn shutdown(&self) {
		let state = self.state.load();
		state.token.cancel(A::Cancel::default());
		if state.link.is_some() {
			state.stop.notified().await;
		}
	}

	pub fn reset(&self) {
		let state = self.state.load();
		assert!(state.token.is_cancelled(), "Proxy is not cancelled");
		let new_state = Arc::new(ProxyState {
			token: CancelToken::new(),
			link: Some(A::spawn(self.spec.clone())),
			stop: Notify::new(),
		});

		self.state.store(new_state);
		self.state_tx.try_send(self.state.load_full()).unwrap();
	}
}
