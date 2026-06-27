use actor12::Actor;
use actor12::Call;
use actor12::Handler;
use actor12::Init;
use actor12::MpscChannel;
use actor12::Multi;
use actor12::prelude::InitFuture;
use actor12::spawn;
use criterion::{Criterion, criterion_group, criterion_main};
use futures::future;
use tokio::runtime::Builder;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

struct MyActor;

impl Actor for MyActor {
    type Cancel = ();
    type State = ();
    type Channel = MpscChannel<Self::Message>;
    type Message = Multi<Self>;
    type Spec = ();

    fn state(_: &Self::Spec) -> Self::State {}

    fn init(_: Init<'_, Self>) -> impl InitFuture<Self> {
        future::ready(Ok(MyActor))
    }
}

// Reply must be `anyhow::Result<_>`: `ActorReply` is only implemented for
// `anyhow::Result<T>` (src/handler.rs), unlike kameo's bare `u32`.
impl Handler<u32> for MyActor {
    type Reply = anyhow::Result<u32>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: u32) -> Self::Reply {
        Ok(msg)
    }
}

fn actor_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("actor12 Actor");

    // The only path that exists today: spawn hardcodes a bounded channel of 10.
    // Add bounded/unbounded variants once P2 (configurable capacity) lands.
    group.bench_function("bounded_ask", |b| {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();
        let _guard = rt.enter();
        let link = rt.block_on(async {
            let link = spawn::<MyActor>(());
            // Warm up so the actor is ready before measuring.
            link.ask_dyn(0u32).await.unwrap();
            link
        });
        b.to_async(&rt).iter(|| async {
            link.ask_dyn(0u32).await.unwrap();
        });
    });

    group.finish();
}

fn spawn_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("actor12 Lifecycle");

    // Full spawn -> ready -> shutdown cycle. This is where the per-actor task
    // count matters (currently 2 tokio tasks per actor: loop + monitor).
    // Multi-threaded runtime to reflect real many-actor scheduling.
    group.bench_function("spawn_shutdown", |b| {
        let rt = Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap();
        b.to_async(&rt).iter(|| async {
            let link = spawn::<MyActor>(());
            link.ask_dyn(0u32).await.unwrap(); // ensure fully started
            link.cancel_and_wait(()).await; // shutdown and wait for the loop to exit
        });
    });

    group.finish();
}

fn plain_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("Plain Tokio Task");

    // Theoretical floor: a hand-rolled mpsc + oneshot, mirroring kameo's baseline.
    group.bench_function("bounded_ask", |b| {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();
        let (tx, mut rx) = mpsc::channel::<(u32, oneshot::Sender<u32>)>(10);
        rt.spawn(async move {
            while let Some((msg, tx)) = rx.recv().await {
                tx.send(msg).unwrap();
            }
        });

        b.to_async(&rt).iter(|| async {
            let (reply_tx, reply_rx) = oneshot::channel();
            tx.send((0, reply_tx)).await.unwrap();
            reply_rx.await.unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, actor_benchmarks, spawn_benchmarks, plain_benchmarks);
criterion_main!(benches);
