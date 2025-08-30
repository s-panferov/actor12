use runy_actor::{Actor, Envelope, Exec, Init, MpscChannel, spawn};
use std::future::Future;

// Define a simple counter actor
pub struct Counter {
    count: i32,
}

// The message type is directly an Envelope
type CounterMessage = Envelope<CounterRequest, anyhow::Result<CounterResponse>>;

#[derive(Debug)]
pub enum CounterRequest {
    Increment,
    GetCount,
}

#[derive(Debug)]
pub enum CounterResponse {
    Unit,
    Count(i32),
}

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
        match msg.value {
            CounterRequest::Increment => {
                self.count += 1;
                println!("Count incremented to: {}", self.count);
                msg.reply.send(Ok(CounterResponse::Unit)).unwrap();
            }
            CounterRequest::GetCount => {
                println!("Current count requested: {}", self.count);
                msg.reply.send(Ok(CounterResponse::Count(self.count))).unwrap();
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Spawn a counter actor with initial value 0
    let counter = spawn::<Counter>(0);

    // Send increment messages
    let _: anyhow::Result<CounterResponse> = counter.send(CounterRequest::Increment).await;
    let _: anyhow::Result<CounterResponse> = counter.send(CounterRequest::Increment).await;
    let _: anyhow::Result<CounterResponse> = counter.send(CounterRequest::Increment).await;

    // Get the current count
    let response: anyhow::Result<CounterResponse> = counter.send(CounterRequest::GetCount).await;
    match response? {
        CounterResponse::Count(count) => println!("Final count: {}", count),
        _ => println!("Unexpected response"),
    }

    Ok(())
}