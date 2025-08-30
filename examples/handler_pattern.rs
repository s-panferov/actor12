use runy_actor::{Actor, Call, Handler, Init, MpscChannel, Multi, spawn};
use futures::future;
use std::future::Future;

// Actor that can handle multiple different message types using the Handler trait
pub struct MultiHandlerActor {
    counter: i32,
    name: String,
}

// Different message types
#[derive(Debug)]
pub struct IncrementMsg;

#[derive(Debug)]
pub struct GetCountMsg;

#[derive(Debug)]
pub struct SetNameMsg(pub String);

#[derive(Debug)]
pub struct GetNameMsg;

impl Actor for MultiHandlerActor {
    type Spec = String; // actor name
    type Message = Multi<Self>;
    type Channel = MpscChannel<Self::Message>;
    type Cancel = ();
    type State = ();

    fn state(_spec: &Self::Spec) -> Self::State {}

    fn init(ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        let name = ctx.spec;
        println!("MultiHandlerActor '{}' initialized", name);
        future::ready(Ok(MultiHandlerActor {
            counter: 0,
            name,
        }))
    }
}

// Implement Handler for each message type
impl Handler<IncrementMsg> for MultiHandlerActor {
    type Reply = Result<i32, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _msg: IncrementMsg) -> Self::Reply {
        self.counter += 1;
        println!("Actor '{}': Counter incremented to {}", self.name, self.counter);
        Ok(self.counter)
    }
}

impl Handler<GetCountMsg> for MultiHandlerActor {
    type Reply = Result<i32, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _msg: GetCountMsg) -> Self::Reply {
        println!("Actor '{}': Current counter is {}", self.name, self.counter);
        Ok(self.counter)
    }
}

impl Handler<SetNameMsg> for MultiHandlerActor {
    type Reply = Result<String, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: SetNameMsg) -> Self::Reply {
        let old_name = self.name.clone();
        self.name = msg.0;
        println!("Actor name changed from '{}' to '{}'", old_name, self.name);
        Ok(old_name)
    }
}

impl Handler<GetNameMsg> for MultiHandlerActor {
    type Reply = Result<String, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _msg: GetNameMsg) -> Self::Reply {
        println!("Actor '{}': Returning current name", self.name);
        Ok(self.name.clone())
    }
}

// String handler for dynamic messages
impl Handler<String> for MultiHandlerActor {
    type Reply = Result<String, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: String) -> Self::Reply {
        let response = format!("Actor '{}' received string: '{}'", self.name, msg);
        println!("{}", response);
        Ok(response)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Handler Pattern Example ===\n");

    // Spawn the actor
    let actor = spawn::<MultiHandlerActor>("CounterBot".to_string());

    // Use different message types with the same actor
    println!("1. Testing typed messages:");
    
    // Increment counter
    let count1: Result<i32, anyhow::Error> = actor.ask_dyn(IncrementMsg).await;
    println!("Result: {:?}", count1);

    let count2: Result<i32, anyhow::Error> = actor.ask_dyn(IncrementMsg).await;  
    println!("Result: {:?}", count2);

    // Get current count
    let current: Result<i32, anyhow::Error> = actor.ask_dyn(GetCountMsg).await;
    println!("Current count: {:?}", current);

    println!("\n2. Testing name operations:");
    
    // Change name
    let old_name: Result<String, anyhow::Error> = actor.ask_dyn(SetNameMsg("SuperBot".to_string())).await;
    println!("Previous name: {:?}", old_name);

    // Get current name
    let name: Result<String, anyhow::Error> = actor.ask_dyn(GetNameMsg).await;
    println!("Current name: {:?}", name);

    println!("\n3. Testing dynamic string messages:");
    
    // Send string messages dynamically
    let response1: Result<String, anyhow::Error> = actor.ask_dyn("Hello there!".to_string()).await;
    println!("Response: {:?}", response1);

    let response2: Result<String, anyhow::Error> = actor.ask_dyn("How are you doing?".to_string()).await;
    println!("Response: {:?}", response2);

    println!("\n4. Final state check:");
    let final_count: Result<i32, anyhow::Error> = actor.ask_dyn(GetCountMsg).await;
    let final_name: Result<String, anyhow::Error> = actor.ask_dyn(GetNameMsg).await;
    println!("Final count: {:?}, Final name: {:?}", final_count, final_name);

    Ok(())
}