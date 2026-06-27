# actor12

[![Documentation](https://docs.rs/actor12/badge.svg)](https://docs.rs/actor12)
[![Crates.io](https://img.shields.io/crates/v/actor12.svg)](https://crates.io/crates/actor12)

A small, type-safe actor framework for Rust on top of Tokio — designed to get out
of your way. Each actor is a single async task that owns its state; you talk to it
through a cheap, cloneable [`Link`]. What makes actor12 different is how much of
the actor you get to *choose*: the message style, the run loop, the state it
exposes, and how errors flow back to callers.

```toml
[dependencies]
actor12 = "0.0.9"
tokio = { version = "1", features = ["full"] }
anyhow = "1.0"
futures = "0.3"
```

## Why actor12?

Most actor crates lock you into one messaging style and one fixed event loop.
actor12 instead gives you a handful of small, overridable knobs on the [`Actor`]
trait, with sensible defaults for all of them, so the simple case stays a few
lines while the hard case stays *possible*:

- **Static *or* dynamic messages** — pick one statically-typed request enum per
  actor for maximum clarity and speed, or open the actor up to any number of
  message types via the `Handler` trait. Same actor model, your choice per actor.
- **Custom run loop (`tick` & `cycle`)** — drop in periodic background work with
  `tick`, or override `cycle` entirely to take full control of how the actor
  selects between messages, timers, and cancellation.
- **Custom state & props** — `Spec` is the typed input you spawn an actor *with*;
  `State` is what the actor exposes back through its `Link`, so callers can read a
  shared snapshot without sending a message.
- **First-class `anyhow`** — make your reply type `anyhow::Result<T>` and
  transport failures (dead actor, dropped reply) fold into `Err` automatically,
  while handlers use `?` like any other async code.
- **Hierarchical cancellation** — typed cancel reasons propagate to child tasks;
  dropping the last `Link` shuts the actor down cleanly.

## Quick start

Define an actor, implement a `Handler` per message type, and talk to it through
its `Link`:

```rust
use actor12::{spawn, Actor, Init, Handler, Call, Multi, MpscChannel};
use std::future::Future;

struct Counter { count: i64 }

struct Increment;
struct Get;

impl Actor for Counter {
    type Message = Multi<Self>;          // dynamic: many message types
    type Spec = ();                      // nothing needed to start
    type Channel = MpscChannel<Self::Message>;
    type Cancel = ();
    type State = ();

    fn state(_spec: &Self::Spec) -> Self::State {}

    fn init(_ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        std::future::ready(Ok(Counter { count: 0 }))
    }
}

impl Handler<Increment> for Counter {
    type Reply = anyhow::Result<()>;
    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _msg: Increment) -> Self::Reply {
        self.count += 1;
        Ok(())
    }
}

impl Handler<Get> for Counter {
    type Reply = anyhow::Result<i64>;
    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _msg: Get) -> Self::Reply {
        Ok(self.count)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let counter = spawn::<Counter>(());

    counter.tell_dyn(Increment).await;          // fire-and-forget
    let n: i64 = counter.ask_dyn(Get).await?;   // request/response
    assert_eq!(n, 1);
    Ok(())
}
```

## Static vs. dynamic messages

An actor's `type Message` decides how callers talk to it. actor12 supports two
styles and you choose per actor.

### Static — one typed request type

Set `Message = Envelope<Request, Reply>`. The actor handles a single message type
and you `match` on it in [`Actor::handle`]. This is the leanest, most explicit
option: one enum in, one reply out, no dynamic dispatch.

```rust
use actor12::{spawn, Actor, Init, Exec, Envelope, MpscChannel};
use std::future::Future;

enum Op { Inc, Get }

struct Counter { count: i64 }

impl Actor for Counter {
    type Message = Envelope<Op, anyhow::Result<i64>>;   // static: one request enum
    type Spec = ();
    type Channel = MpscChannel<Self::Message>;
    type Cancel = ();
    type State = ();

    fn state(_spec: &Self::Spec) -> Self::State {}

    fn init(_ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        std::future::ready(Ok(Counter { count: 0 }))
    }

    async fn handle(&mut self, _ctx: Exec<'_, Self>, msg: Self::Message) {
        let reply = match msg.value {
            Op::Inc => { self.count += 1; Ok(self.count) }
            Op::Get => Ok(self.count),
        };
        let _ = msg.reply.send(reply);
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let counter = spawn::<Counter>(());
    let n: i64 = counter.send::<Op, anyhow::Result<i64>>(Op::Inc).await?;
    assert_eq!(n, 1);
    Ok(())
}
```

### Dynamic — many message types via `Handler`

Set `Message = Multi<Self>` and implement [`Handler<T>`] once per message type.
Each message gets its own request and reply types, the actor stays open for
extension, and callers use `ask_dyn` / `tell_dyn` (see the [Quick start](#quick-start)
above). Reach for this when an actor naturally serves several distinct operations.

## Custom state & props

Two associated types separate *what you spawn an actor with* from *what it
exposes back*:

- **`Spec`** — the props/configuration passed to [`spawn`] and forwarded to
  `init`. Private to the actor.
- **`State`** — a value computed up front by `Actor::state` and stored in the
  [`Link`]. Any holder of the link can read it via [`Link::state`] *without
  sending a message* — ideal for a shared counter, health flag, or config
  snapshot.

```rust
use actor12::{spawn, Actor, Init, Handler, Call, Multi, MpscChannel};
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone)]
struct Shared { processed: Arc<AtomicU64> }

struct Worker { shared: Shared }
struct Job;

impl Actor for Worker {
    type Message = Multi<Self>;
    type Spec = String;                 // props: e.g. a worker name
    type Channel = MpscChannel<Self::Message>;
    type Cancel = ();
    type State = Shared;                // snapshot exposed on the link

    fn state(_spec: &Self::Spec) -> Self::State {
        Shared { processed: Arc::new(AtomicU64::new(0)) }
    }

    fn init(ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        // Grab the shared handle the framework already built from `state`.
        let shared = ctx.link.state().clone();
        std::future::ready(Ok(Worker { shared }))
    }
}

impl Handler<Job> for Worker {
    type Reply = anyhow::Result<()>;
    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _msg: Job) -> Self::Reply {
        self.shared.processed.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let worker = spawn::<Worker>("ingest".to_string());
    worker.ask_dyn(Job).await?;
    // Read live state straight off the link — no message required.
    assert_eq!(worker.state().processed.load(Ordering::Relaxed), 1);
    Ok(())
}
```

## Custom run loop

Every actor runs a `cycle` loop. By default each turn selects across three
things: a cancellation signal, a `tick`, and the next incoming message. You can
override either layer.

### Periodic work with `tick`

Override `tick` to run background work on the actor's own task, interleaved with
message handling. Return `ControlFlow::Continue(())` to keep going, or
`ControlFlow::Break(reason)` to stop the actor.

```rust
use std::ops::ControlFlow;
use std::time::Duration;

impl Actor for Reporter {
    // ... associated types, state(), init() as usual ...

    async fn tick(&mut self) -> ControlFlow<Self::Cancel> {
        tokio::time::sleep(Duration::from_secs(30)).await;
        // flush metrics, refresh a cache, send a heartbeat, ...
        ControlFlow::Continue(())
    }
}
```

### Full control with `cycle`

For complete control over scheduling — priority channels, custom timeouts,
draining behavior — override `cycle` itself. The default is a `tokio::select!`
over cancellation, `tick`, and `rx.recv()`; yours can do whatever you need, as
long as it returns `Continue` to loop again or `Break` to shut down.

```rust
use std::ops::ControlFlow;
use std::time::Duration;
use actor12::{Actor, ActorContext, Exec};
use actor12::cancel::CancelReason;

impl Actor for Server {
    // ... associated types, state(), init() as usual ...

    async fn cycle(
        &mut self,
        ctx: &mut ActorContext<Self>,
    ) -> ControlFlow<CancelReason<Self::Cancel>> {
        tokio::select! {
            reason = ctx.token.cancelled_or_dropped() => {
                ControlFlow::Break(reason.unwrap_or_default())
            }
            _ = tokio::time::sleep(Duration::from_secs(5)) => {
                self.flush();                       // periodic maintenance
                ControlFlow::Continue(())
            }
            msg = ctx.rx.recv() => match msg {
                Some(msg) => {
                    Actor::handle(self, Exec::new(ctx), msg).await;
                    ControlFlow::Continue(())
                }
                None => ControlFlow::Break(CancelReason::default()),
            }
        }
    }
}
```

Related lifecycle hooks: `mailbox_capacity` tunes the bounded mailbox (default
64), `termination_strategy` chooses whether to drain queued messages on shutdown,
and `terminate` runs custom cleanup with the cancel reason in hand.

## Error handling with `anyhow`

Make a handler's `Reply` an `anyhow::Result<T>` and error handling becomes
uniform end to end:

- **Inside the handler**, use `?` and `anyhow!` like in any async function.
- **At the call site**, transport failures — the actor is dead, or its reply was
  dropped — are converted into `Err` for you, so `ask_dyn`/`send` never panic on
  a gone actor; you just get a `Result`.

```rust
use actor12::{Actor, Handler, Call, Link, Multi};

struct Withdraw { amount: u64 }

impl Handler<Withdraw> for Account {
    type Reply = anyhow::Result<u64>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: Withdraw) -> Self::Reply {
        let balance = self.balance
            .checked_sub(msg.amount)
            .ok_or_else(|| anyhow::anyhow!("insufficient funds"))?;   // `?` just works
        self.balance = balance;
        Ok(self.balance)
    }
}

async fn run(account: Link<Account>) {
    // Both a handler error and a dead actor surface as `Err` here.
    match account.ask_dyn(Withdraw { amount: 100 }).await {
        Ok(remaining) => println!("balance: {remaining}"),
        Err(e) => eprintln!("withdraw failed: {e}"),
    }
}
```

## Cancellation & lifecycle

A [`Link`] is a cloneable, reference-counted handle. The actor lives as long as
at least one link does:

```rust
link.cancel(reason);                  // request shutdown with a typed reason
link.cancel_and_wait(reason).await;   // ...and wait until it has stopped
// Dropping the last `Link` cancels the actor automatically.
```

Cancellation is hierarchical: tasks spawned via the actor's context are cancelled
with it, and cancel reasons are typed (`type Cancel`) so shutdown can carry
meaning.

## Examples

The [`examples/`](examples/) directory has runnable programs for each pattern:

```bash
cargo run --example simple_counter      # static Envelope messages + state
cargo run --example handler_pattern     # dynamic Multi messages via Handler
cargo run --example dynamic_dispatch    # routing across many message types
cargo run --example echo_server         # request/response basics
cargo run --example ping_pong           # actors messaging each other
cargo run --example bank_account        # transactions with anyhow errors
cargo run --example worker_pool         # fan-out to a pool of workers
```

Full API documentation is on [docs.rs](https://docs.rs/actor12).

## Testing

```bash
cargo test
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

[`Actor`]: https://docs.rs/actor12/latest/actor12/trait.Actor.html
[`Actor::handle`]: https://docs.rs/actor12/latest/actor12/trait.Actor.html#method.handle
[`Handler`]: https://docs.rs/actor12/latest/actor12/trait.Handler.html
[`Handler<T>`]: https://docs.rs/actor12/latest/actor12/trait.Handler.html
[`Link`]: https://docs.rs/actor12/latest/actor12/struct.Link.html
[`Link::state`]: https://docs.rs/actor12/latest/actor12/struct.Link.html#method.state
[`spawn`]: https://docs.rs/actor12/latest/actor12/fn.spawn.html
```
