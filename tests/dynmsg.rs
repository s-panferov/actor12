use actor12::Actor;
use actor12::Call;
use actor12::Handler;
use actor12::Init;
use actor12::MpscChannel;
use actor12::Multi;
use actor12::prelude::InitFuture;
use futures::future;

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
    let link = actor12::spawn::<MultiActor>(());

    let value = link.ask_dyn("Test message".to_string()).await;
    assert_eq!(value.unwrap(), "Hello".to_string());
}
