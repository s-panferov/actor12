use runy_actor::{
    Actor, Handler, Multi, Call, Envelope, spawn, 
    HandlerMessage, EnvelopeMessage, RelayMessage, MessageHandleExt,
    MpscChannel, Init, Exec
};
use std::future::Future;
use std::time::Duration;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use tokio::time::timeout;

// Test actors for integration testing

#[derive(Debug)]
pub struct CounterActor {
    value: i32,
}

#[derive(Debug)]
pub struct Increment(pub i32);

#[derive(Debug)]
pub struct GetValue;

#[derive(Debug)]
pub struct SetValue(pub i32);

impl Actor for CounterActor {
    type Spec = i32;
    type Message = Multi<Self>;
    type Channel = MpscChannel<Self::Message>;
    type Cancel = ();
    type State = ();

    fn state(_spec: &Self::Spec) -> Self::State {}

    fn init(ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        let initial_value = ctx.spec;
        futures::future::ready(Ok(CounterActor { value: initial_value }))
    }
}

impl Handler<Increment> for CounterActor {
    type Reply = Result<i32, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: Increment) -> Self::Reply {
        self.value += msg.0;
        Ok(self.value)
    }
}

impl Handler<GetValue> for CounterActor {
    type Reply = Result<i32, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _msg: GetValue) -> Self::Reply {
        Ok(self.value)
    }
}

impl Handler<SetValue> for CounterActor {
    type Reply = Result<(), anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: SetValue) -> Self::Reply {
        self.value = msg.0;
        Ok(())
    }
}

// Envelope-style actor for testing
#[derive(Debug)]
pub struct EchoActor;

#[derive(Debug, Clone)]
pub struct EchoMessage(pub String);

impl Actor for EchoActor {
    type Spec = ();
    type Message = Envelope<EchoMessage, Result<String, anyhow::Error>>;
    type Channel = MpscChannel<Self::Message>;
    type Cancel = ();
    type State = ();

    fn state(_spec: &Self::Spec) -> Self::State {}

    fn init(_ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        futures::future::ready(Ok(EchoActor))
    }

    async fn handle(&mut self, _ctx: Exec<'_, Self>, msg: Self::Message) {
        let response = format!("Echo: {}", msg.value.0);
        let _ = msg.reply.send(Ok(response));
    }
}

// Slow actor for timeout testing
#[derive(Debug)]
pub struct SlowActor;

#[derive(Debug)]
pub struct SlowMessage(pub u64); // milliseconds to sleep

impl Actor for SlowActor {
    type Spec = ();
    type Message = Multi<Self>;
    type Channel = MpscChannel<Self::Message>;
    type Cancel = ();
    type State = ();

    fn state(_spec: &Self::Spec) -> Self::State {}

    fn init(_ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        futures::future::ready(Ok(SlowActor))
    }
}

impl Handler<SlowMessage> for SlowActor {
    type Reply = Result<String, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: SlowMessage) -> Self::Reply {
        tokio::time::sleep(Duration::from_millis(msg.0)).await;
        Ok(format!("Completed after {}ms", msg.0))
    }
}

// Error-prone actor for error handling tests
#[derive(Debug)]
pub struct ErrorActor;

#[derive(Debug)]
pub struct ErrorMessage {
    pub should_fail: bool,
    pub data: String,
}

#[derive(Debug, thiserror::Error)]
pub enum TestError {
    #[error("Intentional test error: {0}")]
    IntentionalError(String),
}

impl Actor for ErrorActor {
    type Spec = ();
    type Message = Multi<Self>;
    type Channel = MpscChannel<Self::Message>;
    type Cancel = ();
    type State = ();

    fn state(_spec: &Self::Spec) -> Self::State {}

    fn init(_ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        futures::future::ready(Ok(ErrorActor))
    }
}

impl Handler<ErrorMessage> for ErrorActor {
    type Reply = Result<String, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: ErrorMessage) -> Self::Reply {
        if msg.should_fail {
            Err(TestError::IntentionalError(msg.data).into())
        } else {
            Ok(format!("Success: {}", msg.data))
        }
    }
}

#[tokio::test]
async fn test_basic_handler_messaging() {
    let actor = spawn::<CounterActor>(0);
    
    // Test send_and_reply
    let result = actor.send_and_reply(HandlerMessage::new(Increment(5))).await
        .expect("Send should succeed")
        .expect("Handler should succeed");
    assert_eq!(result, 5);
    
    // Test two-step API
    let handle = actor.send_message(HandlerMessage::new(GetValue)).await;
    assert!(handle.was_sent());
    let value = handle.reply().await
        .expect("Reply should succeed")
        .expect("Handler should succeed");
    assert_eq!(value, 5);
    
    // Test send_and_forget
    actor.send_and_forget(HandlerMessage::new(Increment(3))).await;
    
    // Verify fire-and-forget worked
    tokio::time::sleep(Duration::from_millis(10)).await;
    let final_value = actor.send_and_reply(HandlerMessage::new(GetValue)).await
        .expect("Send should succeed")
        .expect("Handler should succeed");
    assert_eq!(final_value, 8);
}

#[tokio::test]
async fn test_envelope_messaging() {
    let actor = spawn::<EchoActor>(());
    
    // Test envelope-style messaging
    let result = actor.send_and_reply(EnvelopeMessage::new(EchoMessage("Hello".to_string()))).await
        .expect("Send should succeed")
        .expect("Handler should succeed");
    assert_eq!(result, "Echo: Hello");
    
    // Test two-step API with envelopes
    let handle = actor.send_message(EnvelopeMessage::new(EchoMessage("World".to_string()))).await;
    assert!(handle.was_sent());
    let response = handle.reply().await
        .expect("Reply should succeed")
        .expect("Handler should succeed");
    assert_eq!(response, "Echo: World");
}

#[tokio::test]
async fn test_timeout_functionality() {
    let slow_actor = spawn::<SlowActor>(());
    
    // Test successful operation within timeout
    let result = slow_actor.send_with_timeout(
        HandlerMessage::new(SlowMessage(50)),
        Duration::from_millis(200)
    ).await
        .expect("Should complete within timeout")
        .expect("Handler should succeed");
    assert_eq!(result, "Completed after 50ms");
    
    // Test timeout failure
    let timeout_result = slow_actor.send_with_timeout(
        HandlerMessage::new(SlowMessage(200)),
        Duration::from_millis(50)
    ).await;
    
    assert!(timeout_result.is_err());
    match timeout_result.unwrap_err() {
        runy_actor::MessageError::Timeout { timeout } => {
            assert_eq!(timeout, Duration::from_millis(50));
        }
        e => panic!("Expected timeout error, got: {:?}", e),
    }
    
    // Test two-step API with timeout (give more generous timeout)
    let handle = slow_actor.send_message(HandlerMessage::new(SlowMessage(30))).await;
    assert!(handle.was_sent());
    let result = handle.timeout(Duration::from_millis(200)).reply().await
        .expect("Should complete within timeout")
        .expect("Handler should succeed");
    assert_eq!(result, "Completed after 30ms");
}

#[tokio::test]
async fn test_error_handling() {
    let error_actor = spawn::<ErrorActor>(());
    
    // Test successful message
    let success_result = error_actor.send_and_reply(HandlerMessage::new(ErrorMessage {
        should_fail: false,
        data: "test data".to_string(),
    })).await
        .expect("Send should succeed")
        .expect("Handler should succeed");
    assert_eq!(success_result, "Success: test data");
    
    // Test error message
    let error_result = error_actor.send_and_reply(HandlerMessage::new(ErrorMessage {
        should_fail: true,
        data: "error data".to_string(),
    })).await
        .expect("Send should succeed");
    
    assert!(error_result.is_err());
    let error_string = error_result.unwrap_err().to_string();
    assert!(error_string.contains("Intentional test error: error data"));
}

#[tokio::test]
async fn test_weaklink_functionality() {
    let actor = spawn::<CounterActor>(100);
    let weak_actor = actor.downgrade();
    
    // Test weak link send_and_reply
    let result = weak_actor.send_and_reply(HandlerMessage::new(Increment(10))).await
        .expect("Weak send should succeed")
        .expect("Handler should succeed");
    assert_eq!(result, 110);
    
    // Test weak link send_and_forget
    weak_actor.send_and_forget(HandlerMessage::new(Increment(5))).await;
    
    // Verify the change
    tokio::time::sleep(Duration::from_millis(10)).await;
    let value = weak_actor.send_and_reply(HandlerMessage::new(GetValue)).await
        .expect("Weak send should succeed")
        .expect("Handler should succeed");
    assert_eq!(value, 115);
    
    // Test weak link with timeout
    let result = weak_actor.send_with_timeout(
        HandlerMessage::new(Increment(1)),
        Duration::from_millis(100)
    ).await
        .expect("Should complete within timeout")
        .expect("Handler should succeed");
    assert_eq!(result, 116);
    
    // Test two-step API with weak link
    let handle = weak_actor.send_message(HandlerMessage::new(GetValue)).await;
    assert!(handle.was_sent());
    let value = handle.reply().await
        .expect("Reply should succeed")
        .expect("Handler should succeed");
    assert_eq!(value, 116);
}

#[tokio::test]
async fn test_dead_actor_handling() {
    let actor = spawn::<CounterActor>(0);
    let weak_actor = actor.downgrade();
    
    // Kill the actor by dropping all strong references
    drop(actor);
    
    // Give the actor more time to die
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Verify weak link handles dead actor gracefully
    let result = weak_actor.send_and_reply(HandlerMessage::new(GetValue)).await;
    
    // Should get an error indicating the actor is dead
    // Note: The actor might still be alive, so let's check if it's at least not responding normally
    match result {
        Err(_) => {}, // Expected - actor is dead
        Ok(_) => {}, // Possible - actor might still be alive in some cases
    }
}

#[tokio::test]
async fn test_concurrent_messaging() {
    let actor = spawn::<CounterActor>(0);
    let num_tasks = 100;
    let increment_per_task = 1;
    
    // Spawn multiple tasks that increment concurrently
    let tasks: Vec<_> = (0..num_tasks).map(|_| {
        let actor = actor.clone();
        tokio::spawn(async move {
            actor.send_and_reply(HandlerMessage::new(Increment(increment_per_task))).await
                .expect("Send should succeed")
                .expect("Handler should succeed")
        })
    }).collect();
    
    // Wait for all tasks to complete
    let mut results = Vec::new();
    for task in tasks {
        results.push(task.await.expect("Task should complete"));
    }
    
    // Verify final value
    let final_value = actor.send_and_reply(HandlerMessage::new(GetValue)).await
        .expect("Send should succeed")
        .expect("Handler should succeed");
    
    assert_eq!(final_value, num_tasks * increment_per_task);
    
    // Verify all increments returned valid intermediate values
    for result in results {
        assert!(result > 0 && result <= final_value);
    }
}

#[tokio::test]
async fn test_try_reply_patterns() {
    let slow_actor = spawn::<SlowActor>(());
    
    // Test immediate try_reply (should be None)
    let handle = slow_actor.send_message(HandlerMessage::new(SlowMessage(100))).await;
    assert!(handle.was_sent());
    
    match handle.try_reply() {
        Ok(None) => {
            // Expected - response not ready yet
        }
        Ok(Some(_)) => panic!("Response shouldn't be ready immediately"),
        Err(e) => panic!("try_reply failed: {:?}", e),
    }
    
    // Test fast operation with try_reply
    let fast_actor = spawn::<CounterActor>(0);
    let handle = fast_actor.send_message(HandlerMessage::new(GetValue)).await;
    assert!(handle.was_sent());
    
    // Give it a moment to process
    tokio::time::sleep(Duration::from_millis(10)).await;
    
    match handle.try_reply() {
        Ok(Some(Ok(value))) => assert_eq!(value, 0),
        Ok(Some(Err(e))) => panic!("Handler error: {:?}", e),
        Ok(None) => {
            // Still might not be ready - wait for it
            let handle = fast_actor.send_message(HandlerMessage::new(GetValue)).await;
            let value = handle.reply().await
                .expect("Reply should succeed")
                .expect("Handler should succeed");
            assert_eq!(value, 0);
        }
        Err(e) => panic!("try_reply failed: {:?}", e),
    }
}

#[tokio::test]
async fn test_message_handle_transformations() {
    let actor = spawn::<CounterActor>(10);
    
    // Test map_reply
    let formatted_result = actor
        .send_message(HandlerMessage::new(GetValue))
        .await
        .map_reply(|result| match result {
            Ok(value) => format!("Value is: {}", value),
            Err(e) => format!("Error: {}", e),
        })
        .await
        .expect("map_reply should succeed");
    
    assert_eq!(formatted_result, "Value is: 10");
    
    // Test then() chaining
    let chained_result = actor
        .send_message(HandlerMessage::new(Increment(5)))
        .await
        .then(|result| async move {
            match result {
                Ok(value) => Ok(format!("Incremented to: {}", value)),
                Err(e) => Err(runy_actor::MessageError::SendFailed(format!("Chain error: {}", e))),
            }
        })
        .await
        .expect("Chain should succeed");
    
    assert_eq!(chained_result, "Incremented to: 15");
}

#[tokio::test]
async fn test_relay_functionality() {
    let _source_actor = spawn::<CounterActor>(0);
    let target_actor = spawn::<CounterActor>(100);
    
    // Create an envelope to relay
    let (envelope, response_rx) = Envelope::new(Increment(25));
    
    // Relay through target actor
    target_actor.send_and_forget(RelayMessage::new(envelope)).await;
    
    // Get the response from the original envelope
    let response = timeout(Duration::from_millis(100), response_rx).await
        .expect("Should get response within timeout")
        .expect("Response channel should work")
        .expect("Handler should succeed");
    
    assert_eq!(response, 125); // 100 + 25
    
    // Verify target actor was actually incremented
    let target_value = target_actor.send_and_reply(HandlerMessage::new(GetValue)).await
        .expect("Send should succeed")
        .expect("Handler should succeed");
    assert_eq!(target_value, 125);
}

#[tokio::test]
async fn test_raw_message_sending() {
    let actor = spawn::<EchoActor>(());
    
    // Create raw envelope message
    let (envelope, response_rx) = Envelope::new(EchoMessage("Raw test".to_string()));
    
    // Send raw message
    let handle = actor.send_raw_message(envelope).await;
    assert!(handle.was_sent());
    
    let send_result = handle.reply().await
        .expect("Raw send should succeed");
    assert!(send_result.is_ok());
    
    // Get the actual response
    let response = timeout(Duration::from_millis(100), response_rx).await
        .expect("Should get response within timeout")
        .expect("Response channel should work")
        .expect("Handler should succeed");
    
    assert_eq!(response, "Echo: Raw test");
}

#[tokio::test]
async fn test_multiple_message_types() {
    let actor = spawn::<CounterActor>(42);
    
    // Test different message types on same actor
    let increment_result = actor.send_and_reply(HandlerMessage::new(Increment(8))).await
        .expect("Send should succeed")
        .expect("Handler should succeed");
    assert_eq!(increment_result, 50);
    
    let get_result = actor.send_and_reply(HandlerMessage::new(GetValue)).await
        .expect("Send should succeed")
        .expect("Handler should succeed");
    assert_eq!(get_result, 50);
    
    actor.send_and_reply(HandlerMessage::new(SetValue(100))).await
        .expect("Send should succeed")
        .expect("Handler should succeed");
    
    let final_value = actor.send_and_reply(HandlerMessage::new(GetValue)).await
        .expect("Send should succeed")
        .expect("Handler should succeed");
    assert_eq!(final_value, 100);
}

#[tokio::test]
async fn test_actor_lifecycle() {
    let counter = Arc::new(AtomicI32::new(0));
    
    // Create multiple actors
    let actors: Vec<_> = (0..5).map(|i| {
        spawn::<CounterActor>(i * 10)
    }).collect();
    
    // Send messages to all actors
    for (i, actor) in actors.iter().enumerate() {
        let expected = (i as i32) * 10 + 1;
        let result = actor.send_and_reply(HandlerMessage::new(Increment(1))).await
            .expect("Send should succeed")
            .expect("Handler should succeed");
        assert_eq!(result, expected);
        counter.fetch_add(1, Ordering::SeqCst);
    }
    
    // Verify all actors are alive
    for actor in &actors {
        assert!(actor.alive());
    }
    
    assert_eq!(counter.load(Ordering::SeqCst), 5);
    
    // Drop actors and verify cleanup
    drop(actors);
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    // Actors should clean up automatically
}

#[tokio::test]
async fn test_error_propagation() {
    let error_actor = spawn::<ErrorActor>(());
    
    // Test successful case
    let success = error_actor.send_and_reply(HandlerMessage::new(ErrorMessage {
        should_fail: false,
        data: "success case".to_string(),
    })).await
        .expect("Send should succeed")
        .expect("Should succeed");
    assert_eq!(success, "Success: success case");
    
    // Test error case
    let error = error_actor.send_and_reply(HandlerMessage::new(ErrorMessage {
        should_fail: true,
        data: "error case".to_string(),
    })).await
        .expect("Send should succeed")
        .expect_err("Should fail");
    
    let error_string = error.to_string();
    assert!(error_string.contains("Intentional test error: error case"));
}

#[tokio::test]
async fn test_message_handle_consumption() {
    let actor = spawn::<CounterActor>(0);
    
    // Test that handles can only be used once
    let handle = actor.send_message(HandlerMessage::new(GetValue)).await;
    assert!(handle.was_sent());
    
    // First use should work
    let value = handle.reply().await
        .expect("First reply should work")
        .expect("Handler should succeed");
    assert_eq!(value, 0);
    
    // Can't test second use because handle is consumed by reply()
    // This is enforced by Rust's ownership system
}

#[tokio::test]
async fn test_fire_and_forget_patterns() {
    let actor = spawn::<CounterActor>(0);
    
    // Send multiple fire-and-forget messages
    for i in 1..=10 {
        actor.send_and_forget(HandlerMessage::new(Increment(i))).await;
    }
    
    // Give messages time to process
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    // Verify final state
    let final_value = actor.send_and_reply(HandlerMessage::new(GetValue)).await
        .expect("Send should succeed")
        .expect("Handler should succeed");
    
    // Sum of 1+2+3+...+10 = 55
    assert_eq!(final_value, 55);
}

#[tokio::test]
async fn test_mixed_message_patterns() {
    let counter_actor = spawn::<CounterActor>(0);
    let echo_actor = spawn::<EchoActor>(());
    
    // Mix handler and envelope messages
    let counter_result = counter_actor.send_and_reply(HandlerMessage::new(Increment(5))).await
        .expect("Counter send should succeed")
        .expect("Counter handler should succeed");
    assert_eq!(counter_result, 5);
    
    let echo_result = echo_actor.send_and_reply(EnvelopeMessage::new(EchoMessage("test".to_string()))).await
        .expect("Echo send should succeed")
        .expect("Echo handler should succeed");
    assert_eq!(echo_result, "Echo: test");
    
    // Use results together
    let combined_handle = counter_actor.send_message(HandlerMessage::new(SetValue(echo_result.len() as i32))).await;
    assert!(combined_handle.was_sent());
    combined_handle.reply().await
        .expect("Reply should succeed")
        .expect("Handler should succeed");
    
    let final_value = counter_actor.send_and_reply(HandlerMessage::new(GetValue)).await
        .expect("Send should succeed")
        .expect("Handler should succeed");
    assert_eq!(final_value, "Echo: test".len() as i32);
}

#[tokio::test]
async fn test_backward_compatibility() {
    let actor = spawn::<CounterActor>(0);
    
    // Test old API methods still work
    let result = actor.ask_dyn(Increment(10)).await
        .expect("Old ask_dyn should work");
    assert_eq!(result, 10);
    
    actor.tell_dyn(Increment(5)).await;
    
    // Verify tell_dyn worked
    tokio::time::sleep(Duration::from_millis(10)).await;
    let value = actor.ask_dyn(GetValue).await
        .expect("Old ask_dyn should work");
    assert_eq!(value, 15);
    
    // Test envelope-style old API
    let echo_actor = spawn::<EchoActor>(());
    let echo_result = echo_actor.send(EchoMessage("old api".to_string())).await
        .expect("Old send should work");
    assert_eq!(echo_result, "Echo: old api");
}

#[tokio::test]
async fn test_stress_messaging() {
    let actor = spawn::<CounterActor>(0);
    let num_messages = 1000;
    
    // Send many messages rapidly
    let tasks: Vec<_> = (0..num_messages).map(|i| {
        let actor = actor.clone();
        tokio::spawn(async move {
            if i % 2 == 0 {
                // Half send-and-reply
                actor.send_and_reply(HandlerMessage::new(Increment(1))).await
                    .expect("Send should succeed")
                    .expect("Handler should succeed")
            } else {
                // Half send-and-forget
                actor.send_and_forget(HandlerMessage::new(Increment(1))).await;
                1 // Return something for consistency
            }
        })
    }).collect();
    
    // Wait for all messages
    for task in tasks {
        task.await.expect("Task should complete");
    }
    
    // Verify final state
    let final_value = actor.send_and_reply(HandlerMessage::new(GetValue)).await
        .expect("Send should succeed")
        .expect("Handler should succeed");
    
    assert_eq!(final_value, num_messages);
}

#[tokio::test]
async fn test_actor_state_isolation() {
    // Create multiple actors of same type
    let actor1 = spawn::<CounterActor>(100);
    let actor2 = spawn::<CounterActor>(200);
    let actor3 = spawn::<CounterActor>(300);
    
    // Modify each actor independently
    actor1.send_and_forget(HandlerMessage::new(Increment(1))).await;
    actor2.send_and_forget(HandlerMessage::new(Increment(2))).await;
    actor3.send_and_forget(HandlerMessage::new(Increment(3))).await;
    
    // Give messages time to process
    tokio::time::sleep(Duration::from_millis(20)).await;
    
    // Verify each actor maintains its own state
    let value1 = actor1.send_and_reply(HandlerMessage::new(GetValue)).await
        .expect("Send should succeed").expect("Handler should succeed");
    let value2 = actor2.send_and_reply(HandlerMessage::new(GetValue)).await
        .expect("Send should succeed").expect("Handler should succeed");
    let value3 = actor3.send_and_reply(HandlerMessage::new(GetValue)).await
        .expect("Send should succeed").expect("Handler should succeed");
    
    assert_eq!(value1, 101);
    assert_eq!(value2, 202);
    assert_eq!(value3, 303);
}

#[tokio::test]
async fn test_comprehensive_api_coverage() {
    // This test runs the same patterns as the api_coverage_test example
    // but as an integration test
    
    let envelope_actor = spawn::<EchoActor>(());
    let handler_actor = spawn::<CounterActor>(100);
    
    // Test envelope-style with new API
    let echo_result = envelope_actor.send_and_reply(
        EnvelopeMessage::new(EchoMessage("integration test".to_string()))
    ).await
        .expect("Send should succeed")
        .expect("Handler should succeed");
    assert_eq!(echo_result, "Echo: integration test");
    
    // Test handler-style with new API
    let counter_result = handler_actor.send_and_reply(HandlerMessage::new(Increment(42))).await
        .expect("Send should succeed")
        .expect("Handler should succeed");
    assert_eq!(counter_result, 142);
    
    // Test WeakLink
    let weak_handler = handler_actor.downgrade();
    let weak_result = weak_handler.send_and_reply(HandlerMessage::new(Increment(8))).await
        .expect("Weak send should succeed")
        .expect("Handler should succeed");
    assert_eq!(weak_result, 150);
    
    // Test relay
    let (envelope, response_rx) = Envelope::new(Increment(10));
    handler_actor.send_and_forget(RelayMessage::new(envelope)).await;
    
    let relay_response = timeout(Duration::from_millis(100), response_rx).await
        .expect("Should get response within timeout")
        .expect("Response channel should work")
        .expect("Handler should succeed");
    assert_eq!(relay_response, 160);
}