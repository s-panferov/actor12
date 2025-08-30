use runy_actor::{Actor, Envelope, Exec, Init, MpscChannel, Multi, Handler, Call, spawn};
use runy_actor::{HandlerMessage, MessageHandleExt, EnvelopeMessage};
use std::future::Future;
use std::time::Duration;

// =============================================================================
// Demo Actor for Envelope-style Messages
// =============================================================================

pub struct SimpleActor {
    counter: i32,
}

// Direct envelope messages
type IncrementMessage = Envelope<i32, anyhow::Result<i32>>;
// GetValueMessage removed as it's not used in this example

impl Actor for SimpleActor {
    type Spec = i32; // initial value
    type Message = IncrementMessage; // We'll demonstrate with one message type
    type Channel = MpscChannel<Self::Message>;
    type Cancel = ();
    type State = ();

    fn state(_spec: &Self::Spec) -> Self::State {}

    fn init(ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        let initial = ctx.spec;
        println!("SimpleActor initialized with counter: {}", initial);
        futures::future::ready(Ok(SimpleActor { counter: initial }))
    }

    async fn handle(&mut self, _ctx: Exec<'_, Self>, msg: Self::Message) {
        let increment = msg.value;
        self.counter += increment;
        println!("Counter incremented by {}, now: {}", increment, self.counter);
        let _ = msg.reply.send(Ok(self.counter)); // Don't panic on send error
    }
}

// =============================================================================
// Demo Actor for Handler-style Messages  
// =============================================================================

pub struct HandlerActor {
    name: String,
    count: u32,
}

#[derive(Debug)]
pub struct GreetMsg(pub String);

#[derive(Debug)]
pub struct CountMsg;

#[derive(Debug)]
pub struct SetNameMsg(pub String);

impl Actor for HandlerActor {
    type Spec = String; // name
    type Message = Multi<Self>;
    type Channel = MpscChannel<Self::Message>;
    type Cancel = ();
    type State = ();

    fn state(_spec: &Self::Spec) -> Self::State {}

    fn init(ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        let name = ctx.spec;
        println!("HandlerActor '{}' initialized", name);
        futures::future::ready(Ok(HandlerActor { name, count: 0 }))
    }
}

impl Handler<GreetMsg> for HandlerActor {
    type Reply = Result<String, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: GreetMsg) -> Self::Reply {
        self.count += 1;
        let response = format!("Hello {}! I'm {} (greeting #{})", msg.0, self.name, self.count);
        println!("{}", response);
        Ok(response)
    }
}

impl Handler<CountMsg> for HandlerActor {
    type Reply = Result<u32, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _msg: CountMsg) -> Self::Reply {
        println!("Current count: {}", self.count);
        Ok(self.count)
    }
}

impl Handler<SetNameMsg> for HandlerActor {
    type Reply = Result<String, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: SetNameMsg) -> Self::Reply {
        let old_name = self.name.clone();
        self.name = msg.0;
        println!("Name changed from '{}' to '{}'", old_name, self.name);
        Ok(old_name)
    }
}

// =============================================================================
// Main Demo Function
// =============================================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== New Two-Step API Demo ===\n");

    // =============================================================================
    // Demo 1: Envelope-style Messages
    // =============================================================================
    
    println!("1. Envelope-style Messages:");
    let simple_actor = spawn::<SimpleActor>(10);

    // New API: Request-Response Pattern
    let handle = simple_actor.send_message(EnvelopeMessage::new(5)).await; // increment by 5
    if handle.was_sent() {
        match handle.reply().await {
            Ok(Ok(value)) => println!("   New counter value: {}", value),
            Ok(Err(e)) => println!("   Actor error: {}", e),
            Err(e) => println!("   Message error: {}", e),
        }
    }

    // New API: Fire-and-Forget Pattern
    simple_actor.send_message(EnvelopeMessage::new(3)).await.forget(); // increment by 3, don't wait
    println!("   Fire-and-forget increment sent");

    // New API: Timeout Pattern
    match simple_actor.send_message(EnvelopeMessage::new(2)).await.timeout(Duration::from_secs(1)).reply().await {
        Ok(Ok(value)) => println!("   Timeout reply: {}", value),
        Ok(Err(e)) => println!("   Actor error: {}", e),
        Err(e) => println!("   Timeout error: {}", e),
    }

    println!();

    // =============================================================================
    // Demo 2: Handler-style Messages
    // =============================================================================
    
    println!("2. Handler-style Messages:");
    let handler_actor = spawn::<HandlerActor>("Alice".to_string());

    // New API with HandlerMessage wrapper
    let greet_handle = handler_actor.send_message(HandlerMessage::new(GreetMsg("Bob".to_string()))).await;
    match greet_handle.reply().await {
        Ok(Ok(greeting)) => println!("   Greeting response: {}", greeting),
        Ok(Err(e)) => println!("   Actor error: {}", e),
        Err(e) => println!("   Message error: {}", e),
    }

    // Count messages
    let count_handle = handler_actor.send_message(HandlerMessage::new(CountMsg)).await;
    match count_handle.reply().await {
        Ok(Ok(count)) => println!("   Count: {}", count),
        Ok(Err(e)) => println!("   Actor error: {}", e),
        Err(e) => println!("   Message error: {}", e),
    }

    // Set name with timeout
    let name_handle = handler_actor.send_message(HandlerMessage::new(SetNameMsg("Charlie".to_string()))).await;
    match name_handle.timeout(Duration::from_millis(500)).reply().await {
        Ok(Ok(old_name)) => println!("   Previous name was: {}", old_name),
        Ok(Err(e)) => println!("   Actor error: {}", e),
        Err(e) => println!("   Message error: {}", e),
    }

    println!();

    // =============================================================================
    // Demo 3: Advanced Patterns
    // =============================================================================
    
    println!("3. Advanced Patterns:");

    // Conditional reply pattern
    let handle = handler_actor.send_message(HandlerMessage::new(CountMsg)).await;
    if handle.was_sent() {
        println!("   Message sent successfully");
        match handle.reply().await {
            Ok(Ok(count)) => println!("   Final count: {}", count),
            Ok(Err(e)) => println!("   Actor error: {}", e),
            Err(e) => println!("   Message error: {}", e),
        }
    } else {
        println!("   Send failed: {:?}", handle.send_error());
        handle.forget();
    }

    // Extension trait usage - map reply
    let mapped_result = handler_actor
        .send_message(HandlerMessage::new(CountMsg))
        .await
        .map_reply(|result| match result {
            Ok(count) => format!("Count is: {}", count),
            Err(e) => format!("Error: {}", e),
        })
        .await;
    
    match mapped_result {
        Ok(formatted) => println!("   Mapped result: {}", formatted),
        Err(e) => println!("   Mapped error: {}", e),
    }

    // Try reply (non-blocking check)
    let handle = handler_actor.send_message(HandlerMessage::new(CountMsg)).await;
    match handle.try_reply() {
        Ok(Some(Ok(count))) => println!("   Immediate reply available: {}", count),
        Ok(Some(Err(e))) => println!("   Immediate actor error: {}", e),
        Ok(None) => {
            println!("   Reply not ready yet, waiting...");
            // Put the handle back and wait
            // Note: This is a simplified example - in practice you'd need to reconstruct the handle
        }
        Err(e) => println!("   Try reply error: {}", e),
    }

    println!();

    // =============================================================================
    // Demo 4: Batch Operations
    // =============================================================================
    
    println!("4. Batch Operations:");
    
    let messages = vec![
        HandlerMessage::new(GreetMsg("User1".to_string())),
        HandlerMessage::new(GreetMsg("User2".to_string())),
        HandlerMessage::new(GreetMsg("User3".to_string())),
    ];
    
    let mut handles = Vec::new();
    
    // Send all messages concurrently
    for msg in messages {
        handles.push(handler_actor.send_message(msg).await);
    }
    
    // Process responses
    for (i, handle) in handles.into_iter().enumerate() {
        match handle.timeout(Duration::from_secs(1)).reply().await {
            Ok(Ok(response)) => println!("   Batch response {}: {}", i + 1, response),
            Ok(Err(e)) => println!("   Batch actor error {}: {}", i + 1, e),
            Err(e) => println!("   Batch error {}: {}", i + 1, e),
        }
    }

    println!("\n=== Demo Complete ===");

    // Let actors finish processing
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    Ok(())
}

// =============================================================================
// Comparison with Old API (commented out for reference)
// =============================================================================

/*
// Old API usage (for comparison):

// Envelope messages
let result = simple_actor.send(5).await; // Direct result, no handle

// Handler messages  
let greeting = handler_actor.ask_dyn(GreetMsg("Bob".to_string())).await;
handler_actor.tell_dyn(CountMsg).await; // Fire-and-forget, wasteful

// No built-in timeout support
let result = tokio::time::timeout(
    Duration::from_secs(1), 
    handler_actor.ask_dyn(CountMsg)
).await??;

// No conditional patterns possible
// No batch operation helpers
// No non-blocking checks
*/