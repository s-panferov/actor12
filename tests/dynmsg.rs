use futures::future;
use runy_actor::Actor;
use runy_actor::Call;
use runy_actor::Handler;
use runy_actor::Init;
use runy_actor::MpscChannel;
use runy_actor::Multi;
use runy_actor::prelude::InitFuture;

struct MultiActor {}

impl Actor for MultiActor {
	type Cancel = ();
	type State = ();
	type Channel = MpscChannel<Self::Message>;
	type Message = Multi<Self>;
	type Spec = ();

	fn state(_: &Self::Spec) -> Self::State {
		()
	}

	fn init(_: Init<'_, Self>) -> impl InitFuture<Self> {
		future::ready(Ok(MultiActor {}))
	}
}

impl Handler<String> for MultiActor {
	type Reply = Result<String, anyhow::Error>;

	async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _: String) -> Self::Reply {
		Ok("Hello".to_string())
	}
}

#[tokio::test]
async fn test() {
	let link = runy_actor::spawn::<MultiActor>(());

	let value = link.ask_dyn("Test message".to_string()).await;
	assert_eq!(value.unwrap(), "Hello".to_string());
}
