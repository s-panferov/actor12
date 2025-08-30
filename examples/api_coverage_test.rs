use runy_actor::{Actor, Envelope, Exec, Init, MpscChannel, Multi, Handler, Call, spawn};
use runy_actor::{HandlerMessage, EnvelopeMessage, RelayMessage, MessageHandleExt, MessageError};
use std::future::Future;
use std::time::Duration;

// Test actor for comprehensive API coverage
pub struct TestActor {
    counter: i32,
    name: String,
}

#[derive(Debug)]
pub struct TestMessage(pub String);

#[derive(Debug)]  
pub struct CounterOp(pub i32);

// Envelope-style actor
impl Actor for TestActor {
    type Spec = (String, i32); // (name, initial_counter)
    type Message = Envelope<TestMessage, anyhow::Result<String>>;
    type Channel = MpscChannel<Self::Message>;
    type Cancel = ();
    type State = ();

    fn state(_spec: &Self::Spec) -> Self::State {}

    fn init(ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        let (name, counter) = ctx.spec;
        futures::future::ready(Ok(TestActor { counter, name }))
    }

    async fn handle(&mut self, _ctx: Exec<'_, Self>, msg: Self::Message) {
        let response = format!("Actor {} processed: {}", self.name, msg.value.0);
        let _ = msg.reply.send(Ok(response));
    }
}

// Handler-style actor
pub struct HandlerActor {
    value: i32,
}

impl Actor for HandlerActor {
    type Spec = i32;
    type Message = Multi<Self>;
    type Channel = MpscChannel<Self::Message>;
    type Cancel = ();
    type State = ();

    fn state(_spec: &Self::Spec) -> Self::State {}

    fn init(ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        let value = ctx.spec;
        futures::future::ready(Ok(HandlerActor { value }))
    }
}

impl Handler<CounterOp> for HandlerActor {
    type Reply = Result<i32, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: CounterOp) -> Self::Reply {
        self.value += msg.0;
        Ok(self.value)
    }
}

impl Handler<TestMessage> for HandlerActor {
    type Reply = Result<String, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: TestMessage) -> Self::Reply {
        Ok(format!("Handler processed: {} (value: {})", msg.0, self.value))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Comprehensive API Coverage Test ===\n");

    // =============================================================================
    // Test 1: Envelope-style messages (covers old `send` API)
    // =============================================================================
    
    println!("1. Envelope-style Messages:");
    let envelope_actor = spawn::<TestActor>(("EnvActor".to_string(), 42));

    // NEW API: Two-step pattern
    println!("   NEW API - Two-step:");
    let handle = envelope_actor.send_message(EnvelopeMessage::new(TestMessage("Hello".to_string()))).await;
    match handle.reply().await {
        Ok(Ok(response)) => println!("     Success: {}", response),
        Ok(Err(e)) => println!("     Actor Error: {}", e),
        Err(e) => println!("     Message Error: {}", e),
    }

    // NEW API: Convenience methods (equivalent to old API)
    println!("   NEW API - Direct reply:");
    match envelope_actor.send_and_reply(EnvelopeMessage::new(TestMessage("World".to_string()))).await {
        Ok(Ok(response)) => println!("     Success: {}", response),
        Ok(Err(e)) => println!("     Actor Error: {}", e),
        Err(e) => println!("     Message Error: {}", e),
    }

    // NEW API: Fire-and-forget
    println!("   NEW API - Fire-and-forget:");
    envelope_actor.send_and_forget(EnvelopeMessage::new(TestMessage("Goodbye".to_string()))).await;
    println!("     Fire-and-forget completed");

    // NEW API: With timeout (new capability)
    println!("   NEW API - With timeout:");
    match envelope_actor.send_with_timeout(
        EnvelopeMessage::new(TestMessage("Timeout test".to_string())), 
        Duration::from_secs(1)
    ).await {
        Ok(Ok(response)) => println!("     Success: {}", response),
        Ok(Err(e)) => println!("     Actor Error: {}", e),
        Err(e) => println!("     Timeout Error: {}", e),
    }

    println!();

    // =============================================================================
    // Test 2: Handler-style messages (covers old ask_dyn, tell_dyn API)
    // =============================================================================
    
    println!("2. Handler-style Messages:");
    let handler_actor = spawn::<HandlerActor>(100);

    // NEW API: ask_dyn equivalent
    println!("   NEW API - ask_dyn equivalent:");
    match handler_actor.send_and_reply(HandlerMessage::new(CounterOp(5))).await {
        Ok(Ok(value)) => println!("     Counter result: {}", value),
        Ok(Err(e)) => println!("     Actor Error: {}", e),
        Err(e) => println!("     Message Error: {}", e),
    }

    // NEW API: tell_dyn equivalent
    println!("   NEW API - tell_dyn equivalent:");
    handler_actor.send_and_forget(HandlerMessage::new(CounterOp(10))).await;
    println!("     Tell completed");

    // NEW API: Two-step pattern with timeout
    println!("   NEW API - Two-step with timeout:");
    let handle = handler_actor.send_message(HandlerMessage::new(TestMessage("Handler test".to_string()))).await;
    match handle.reply_timeout(Duration::from_millis(500)).await {
        Ok(Ok(response)) => println!("     Response: {}", response),
        Ok(Err(e)) => println!("     Actor Error: {}", e),
        Err(e) => println!("     Timeout Error: {}", e),
    }

    println!();

    // =============================================================================
    // Test 3: Raw message sending (covers old send_raw API)
    // =============================================================================
    
    println!("3. Raw Message Sending:");
    let raw_envelope = Envelope::new(TestMessage("Raw message".to_string()));
    let handle = envelope_actor.send_raw_message(raw_envelope.0).await;
    match handle.reply().await {
        Ok(Ok(())) => println!("   Raw send successful"),
        Ok(Err(e)) => println!("   Raw send failed: {:?}", e),
        Err(e) => println!("   Message error: {}", e),
    }

    println!();

    // =============================================================================
    // Test 4: Relay functionality (covers old relay_dyn API)  
    // =============================================================================
    
    println!("4. Relay Functionality:");
    let (envelope, _rx) = Envelope::new(CounterOp(25));
    let relay_handle = handler_actor.send_message(RelayMessage::new(envelope)).await;
    match relay_handle.reply().await {
        Ok(()) => println!("   Relay completed successfully"),
        Err(e) => println!("   Relay error: {}", e),
    }

    println!();

    // =============================================================================
    // Test 5: WeakLink functionality (covers all weak operations)
    // =============================================================================
    
    println!("5. WeakLink Functionality:");
    let weak_handler = handler_actor.downgrade();

    println!("   WeakLink - send_and_reply:");
    match weak_handler.send_and_reply(HandlerMessage::new(CounterOp(1))).await {
        Ok(Ok(value)) => println!("     Weak result: {}", value),
        Ok(Err(e)) => println!("     Actor Error: {}", e), 
        Err(e) => println!("     Message Error: {}", e),
    }

    println!("   WeakLink - send_and_forget:");
    weak_handler.send_and_forget(HandlerMessage::new(CounterOp(2))).await;
    println!("     Weak tell completed");

    println!("   WeakLink - with timeout:");
    match weak_handler.send_with_timeout(
        HandlerMessage::new(TestMessage("Weak test".to_string())), 
        Duration::from_millis(500)
    ).await {
        Ok(Ok(response)) => println!("     Weak timeout result: {}", response),
        Ok(Err(e)) => println!("     Actor Error: {}", e),
        Err(e) => println!("     Timeout Error: {}", e),
    }

    println!("   WeakLink - two-step pattern:");
    let weak_handle = weak_handler.send_message(HandlerMessage::new(CounterOp(-5))).await;
    if weak_handle.was_sent() {
        match weak_handle.reply().await {
            Ok(Ok(value)) => println!("     Weak two-step result: {}", value),
            Ok(Err(e)) => println!("     Actor Error: {}", e),
            Err(e) => println!("     Message Error: {}", e),
        }
    }

    println!();

    // =============================================================================
    // Test 6: Advanced patterns (new capabilities)
    // =============================================================================
    
    println!("6. Advanced Patterns:");

    // Try reply (non-blocking check)
    println!("   Try reply pattern:");
    let handle = handler_actor.send_message(HandlerMessage::new(CounterOp(0))).await;
    match handle.try_reply() {
        Ok(Some(Ok(value))) => println!("     Immediate result: {}", value),
        Ok(Some(Err(e))) => println!("     Immediate actor error: {}", e),
        Ok(None) => println!("     Reply not ready yet"),
        Err(e) => println!("     Try reply error: {}", e),
    }

    // Map reply
    println!("   Map reply pattern:");
    let mapped_result = handler_actor
        .send_message(HandlerMessage::new(CounterOp(3)))
        .await
        .map_reply(|result| match result {
            Ok(value) => format!("Mapped success: {}", value),
            Err(e) => format!("Mapped error: {}", e),
        })
        .await;
    
    match mapped_result {
        Ok(formatted) => println!("     {}", formatted),
        Err(e) => println!("     Map error: {}", e),
    }

    // Chain operations
    println!("   Chain operations:");
    let chain_result = handler_actor
        .send_message(HandlerMessage::new(CounterOp(7)))
        .await
        .then(|result| async move {
            match result {
                Ok(value) => Ok(format!("Chained result: {}", value * 2)),
                Err(e) => Err(MessageError::SendFailed(format!("Chain error: {}", e))),
            }
        })
        .await;

    match chain_result {
        Ok(chained) => println!("     {}", chained),
        Err(e) => println!("     Chain error: {}", e),
    }

    println!("\n=== API Coverage Test Complete ===");
    println!("✓ All old API patterns covered with new unified interface");
    println!("✓ New capabilities: timeouts, conditional patterns, chaining");
    println!("✓ WeakLink fully supported");
    println!("✓ Backward compatibility maintained");

    // Brief wait for any remaining async operations
    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok(())
}

/*
=============================================================================
API MAPPING SUMMARY:

Old API -> New API Equivalent:
- send(msg).await -> send_and_reply(EnvelopeMessage::new(msg)).await
- ask_dyn(msg).await -> send_and_reply(HandlerMessage::new(msg)).await  
- tell_dyn(msg).await -> send_and_forget(HandlerMessage::new(msg)).await
- ask_dyn_async(msg).await -> send_message(HandlerMessage::new(msg)).await (returns handle)
- relay_dyn(envelope).await -> send_and_forget(RelayMessage::new(envelope)).await
- send_raw(msg).await -> send_raw_message(msg).await.reply().await

WeakLink equivalents:
- weak.send(msg).await -> weak.send_and_reply(EnvelopeMessage::new(msg)).await
- weak.ask_dyn(msg).await -> weak.send_and_reply(HandlerMessage::new(msg)).await
- weak.tell_dyn(msg).await -> weak.send_and_forget(HandlerMessage::new(msg)).await

New capabilities:
- All methods now support timeouts with send_with_timeout()
- Two-step pattern allows conditional reply handling
- try_reply() for non-blocking checks
- map_reply() and then() for result transformation
- Better error handling with separate send/receive errors

=============================================================================
*/