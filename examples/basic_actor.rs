use actor12::prelude::*;
use actor12::{spawn, Envelope, MpscChannel, Init, Exec};
use std::future::Future;

// Define a simple counter actor
pub struct Counter {
    count: i32,
}

// Messages the counter can handle
#[derive(Debug)]
pub struct Increment;

#[derive(Debug)]
pub struct GetCount;

// Actor implementation
impl Actor for Counter {
    type Spec = i32; // initial count
    type Message = Envelope<(), anyhow::Result<()>>; // Simple envelope for increment
    type Channel = MpscChannel<Self::Message>;
    type Cancel = ();
    type State = ();

    fn state(_spec: &Self::Spec) -> Self::State {}

    fn init(ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        let initial_count = ctx.spec;
        async move {
            println!("Counter actor initialized with count: {}", initial_count);
            Ok(Counter { count: initial_count })
        }
    }

    async fn handle(&mut self, _ctx: Exec<'_, Self>, msg: Self::Message) {
        self.count += 1;
        println!("Count incremented to: {}", self.count);
        let _ = msg.reply.send(Ok(()));
    }
}

// Define a separate counter for getting count
pub struct CounterReader {
    counter_value: i32,
}

impl Actor for CounterReader {
    type Spec = i32;
    type Message = Envelope<(), anyhow::Result<i32>>;
    type Channel = MpscChannel<Self::Message>;
    type Cancel = ();
    type State = ();

    fn state(_spec: &Self::Spec) -> Self::State {}

    fn init(ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        let initial_count = ctx.spec;
        async move {
            Ok(CounterReader { counter_value: initial_count })
        }
    }

    async fn handle(&mut self, _ctx: Exec<'_, Self>, msg: Self::Message) {
        println!("Current count requested: {}", self.counter_value);
        let _ = msg.reply.send(Ok(self.counter_value));
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Spawn a counter actor with initial value 0
    let counter = spawn::<Counter>(0);

    // Send increment messages
    counter.send(()).await?;
    counter.send(()).await?;
    counter.send(()).await?;

    println!("Sent 3 increment messages");

    // For demonstration, we'll just show that we can increment
    // A real multi-message actor would use the Multi<A> pattern
    // as shown in the integration tests

    Ok(())
}