use std::future::Future;

use actor12::Actor;
use actor12::Call;
use actor12::Handler;
use actor12::Init;
use actor12::MpscChannel;
use actor12::Multi;
use actor12::WeakLink;

struct Counter {
    count: i32,
}

struct Increment;

impl Actor for Counter {
    type Message = Multi<Self>;
    type Spec = ();
    type Channel = MpscChannel<Self::Message>;
    type Cancel = ();
    type State = ();

    fn state(_spec: &Self::Spec) -> Self::State {}

    fn init(
        _ctx: Init<'_, Self>,
    ) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        std::future::ready(Ok(Counter { count: 0 }))
    }
}

impl Handler<Increment> for Counter {
    type Reply = anyhow::Result<i32>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _msg: Increment) -> Self::Reply {
        self.count += 1;
        Ok(self.count)
    }
}

#[tokio::test]
async fn empty_link_behaves_as_dead() {
    let weak = WeakLink::<Counter>::default();

    assert!(!weak.is_set());
    assert!(weak.upgrade().is_none());
    assert!(weak.ask_dyn(Increment).await.is_err());
    weak.tell_dyn(Increment).await;
    weak.cancel(());

    assert!(WeakLink::<Counter>::new() == WeakLink::<Counter>::new());
}

#[tokio::test]
async fn set_and_replace() {
    let first = actor12::spawn::<Counter>(());
    let second = actor12::spawn::<Counter>(());

    let mut weak = WeakLink::new();
    weak.set(&first);
    assert!(weak.is_set());
    assert!(weak.upgrade().unwrap() == first);
    assert_eq!(weak.ask_dyn(Increment).await.unwrap(), 1);

    weak.set(&second);
    assert!(weak.upgrade().unwrap() == second);
    assert_eq!(weak.ask_dyn(Increment).await.unwrap(), 1);
}

#[tokio::test]
async fn try_set_only_once() {
    let first = actor12::spawn::<Counter>(());
    let second = actor12::spawn::<Counter>(());

    let mut weak = WeakLink::new();
    assert!(weak.try_set(&first));
    assert!(!weak.try_set(&second));
    assert!(weak.upgrade().unwrap() == first);
}

#[tokio::test]
async fn clear_take_and_dead_target() {
    let link = actor12::spawn::<Counter>(());

    let mut weak = WeakLink::from(&link);
    let taken = weak.take();
    assert!(!weak.is_set());
    assert!(taken.is_set());

    weak.set(&link);
    weak.clear();
    assert!(!weak.is_set());

    weak.set(&link);
    drop(link);
    assert!(weak.is_set());
    assert!(weak.upgrade().is_none());
}
