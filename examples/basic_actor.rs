use runy_actor::{Actor, Envelope, Exec, Init, MpscChannel, spawn};
use std::future::Future;

// Define a simple counter actor
pub struct Counter {
    count: i32,
}

// Messages the counter can handle
#[derive(Debug)]
pub enum CounterMessage {
    Increment(Envelope<(), anyhow::Result<()>>),
    GetCount(Envelope<(), anyhow::Result<i32>>),
}

// Actor implementation
impl Actor for Counter {
    type Spec = i32; // initial count
    type Message = CounterMessage;
    type Channel = MpscChannel<Self::Message>;
    type Cancel = ();
    type State = ();

    fn state(_spec: &Self::Spec) -> Self::State {}

    fn init(ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        let initial_count = ctx.spec;
        println!("Counter actor initialized with count: {}", initial_count);
        futures::future::ready(Ok(Counter { count: initial_count }))
    }

    async fn handle(&mut self, _ctx: Exec<'_, Self>, msg: Self::Message) {
        match msg {
            CounterMessage::Increment(envelope) => {
                self.count += 1;
                println!("Count incremented to: {}", self.count);
                envelope.reply.send(Ok(())).unwrap();
            }
            CounterMessage::GetCount(envelope) => {
                println!("Current count requested: {}", self.count);
                envelope.reply.send(Ok(self.count)).unwrap();
            }
        }
    }
}

// Helper functions for easier message sending
impl Counter {
    pub async fn increment(link: &runy_actor::Link<Self>) -> anyhow::Result<()> {
        link.send(()).await
    }

    pub async fn get_count(link: &runy_actor::Link<Self>) -> anyhow::Result<i32> {
        link.send(()).await
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Spawn a counter actor with initial value 0
    let counter = spawn::<Counter>(0);

    // Send increment messages
    Counter::increment(&counter).await?;
    Counter::increment(&counter).await?;
    Counter::increment(&counter).await?;

    // Get the current count
    let count = Counter::get_count(&counter).await?;
    println!("Final count: {}", count);

    Ok(())
}