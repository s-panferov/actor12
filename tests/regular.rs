use std::future::Future;

use runy_actor::Actor;
use runy_actor::Envelope;
use runy_actor::Exec;
use runy_actor::Init;

struct MyActor {}

impl Actor for MyActor {
	type Cancel = ();
	type State = ();
	type Channel = runy_actor::MpscChannel<Self::Message>;
	type Message = Envelope<String, anyhow::Result<String>>;
	type Spec = ();

	async fn handle(&mut self, _ctx: Exec<'_, Self>, _msg: Self::Message) {
		_msg.reply.send(Ok("Hello".to_string())).unwrap();
	}

	fn state(_: &Self::Spec) -> Self::State {}

	fn init(_ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + 'static {
		futures::future::ready(Ok(MyActor {}))
	}
}

#[tokio::test]
async fn test() {
	let link = runy_actor::spawn::<MyActor>(());

	let value = link.send("Test message".to_string()).await;

	assert_eq!(value.unwrap(), "Hello".to_string());
}
