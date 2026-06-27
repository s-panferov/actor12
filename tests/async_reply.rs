use actor12::Actor;
use actor12::Call;
use actor12::Handler;
use actor12::Init;
use actor12::MpscChannel;
use actor12::Multi;
use actor12::prelude::InitFuture;
use futures::future;

struct AsyncActor {}

impl Actor for AsyncActor {
    type Cancel = ();
    type State = ();
    type Channel = MpscChannel<Self::Message>;
    type Message = Multi<Self>;
    type Spec = ();

    fn state(_: &Self::Spec) -> Self::State {}

    fn init(_: Init<'_, Self>) -> impl InitFuture<Self> {
        future::ready(Ok(AsyncActor {}))
    }
}

struct SyncMsg;
struct AsyncMsg;

// Sync reply: handler returns the value directly; the framework delivers it.
impl Handler<SyncMsg> for AsyncActor {
    type Reply = anyhow::Result<u32>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _: SyncMsg) -> Self::Reply {
        Ok(1)
    }
}

// Async reply: handler defers the reply to a spawned task via `reply_async`.
impl Handler<AsyncMsg> for AsyncActor {
    type Reply = anyhow::Result<u32>;

    async fn handle(&mut self, mut ctx: Call<'_, Self, Self::Reply>, _: AsyncMsg) -> Self::Reply {
        ctx.reply_async(async move {
            tokio::task::yield_now().await;
            Ok(42)
        })
    }
}

#[tokio::test]
async fn sync_reply_delivers() {
    let link = actor12::spawn::<AsyncActor>(());
    let value = link.ask_dyn(SyncMsg).await;
    assert_eq!(value.unwrap(), 1);
}

#[tokio::test]
async fn async_reply_delivers() {
    let link = actor12::spawn::<AsyncActor>(());
    // The reply arrives from a task spawned inside the handler, not the
    // handler's return value. Detection is via the take-state of the sender.
    let value = link.ask_dyn(AsyncMsg).await;
    assert_eq!(value.unwrap(), 42);
}
