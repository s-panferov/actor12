use runy_actor::prelude::*;
use runy_actor::Link;
use std::time::Duration;
use tokio::time::sleep;

// Ping actor
pub struct PingActor {
    pong_actor: Option<Link<PongActor>>,
    ping_count: u32,
}

// Pong actor  
pub struct PongActor {
    pong_count: u32,
}

// Messages
#[derive(Debug)]
pub struct StartPing(pub Link<PongActor>);

#[derive(Debug)]
pub struct Ping(pub u32);

#[derive(Debug)]
pub struct Pong(pub u32);

// Ping Actor implementation
impl Actor for PingActor {
    type Spec = ();

    async fn init(_: Self::Spec) -> anyhow::Result<Self> {
        println!("PingActor initialized");
        Ok(PingActor {
            pong_actor: None,
            ping_count: 0,
        })
    }
}

impl Handler<StartPing> for PingActor {
    type Reply = ();

    async fn exec(&mut self, msg: StartPing) -> anyhow::Result<Self::Reply> {
        self.pong_actor = Some(msg.0);
        
        // Start the ping-pong game
        if let Some(ref pong) = self.pong_actor {
            self.ping_count += 1;
            println!("Ping #{}", self.ping_count);
            pong.call(Ping(self.ping_count)).await?;
        }
        
        Ok(())
    }
}

impl Handler<Pong> for PingActor {
    type Reply = ();

    async fn exec(&mut self, msg: Pong) -> anyhow::Result<Self::Reply> {
        println!("Received Pong #{}", msg.0);
        
        // Continue ping-pong for a few rounds
        if self.ping_count < 5 {
            if let Some(ref pong) = self.pong_actor {
                self.ping_count += 1;
                println!("Ping #{}", self.ping_count);
                pong.call(Ping(self.ping_count)).await?;
            }
        } else {
            println!("Ping-pong game finished!");
        }
        
        Ok(())
    }
}

// Pong Actor implementation
impl Actor for PongActor {
    type Spec = ();

    async fn init(_: Self::Spec) -> anyhow::Result<Self> {
        println!("PongActor initialized");
        Ok(PongActor { pong_count: 0 })
    }
}

impl Handler<Ping> for PongActor {
    type Reply = ();

    async fn exec(&mut self, msg: Ping) -> anyhow::Result<Self::Reply> {
        self.pong_count += 1;
        println!("Received Ping #{}, sending Pong #{}", msg.0, self.pong_count);
        
        // We need the ping actor reference to send back Pong
        // For this example, we'll use the actor context to get the sender
        Ok(())
    }
}

// Since we need bidirectional communication, let's modify the approach
#[derive(Debug)]
pub struct PingPong {
    other_actor: Option<Link<PingPong>>,
    is_ping: bool,
    count: u32,
}

#[derive(Debug)]
pub struct Connect(pub Link<PingPong>);

#[derive(Debug)]
pub struct Ball(pub u32);

impl Actor for PingPong {
    type Spec = bool; // true for ping, false for pong

    async fn init(is_ping: Self::Spec) -> anyhow::Result<Self> {
        let name = if is_ping { "Ping" } else { "Pong" };
        println!("{} actor initialized", name);
        Ok(PingPong {
            other_actor: None,
            is_ping,
            count: 0,
        })
    }
}

impl Handler<Connect> for PingPong {
    type Reply = ();

    async fn exec(&mut self, msg: Connect) -> anyhow::Result<Self::Reply> {
        self.other_actor = Some(msg.0);
        
        // If this is the ping actor, start the game
        if self.is_ping {
            if let Some(ref other) = self.other_actor {
                self.count = 1;
                println!("Ping sends ball #{}", self.count);
                other.call(Ball(self.count)).await?;
            }
        }
        
        Ok(())
    }
}

impl Handler<Ball> for PingPong {
    type Reply = ();

    async fn exec(&mut self, msg: Ball) -> anyhow::Result<Self::Reply> {
        let name = if self.is_ping { "Ping" } else { "Pong" };
        println!("{} receives ball #{}", name, msg.0);
        
        if msg.0 < 10 {
            if let Some(ref other) = self.other_actor {
                let next_count = msg.0 + 1;
                println!("{} sends ball #{}", name, next_count);
                other.call(Ball(next_count)).await?;
            }
        } else {
            println!("{} stops the game at ball #{}", name, msg.0);
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Spawn ping and pong actors
    let ping = PingPong::spawn(true);
    let pong = PingPong::spawn(false);

    // Connect them
    ping.call(Connect(pong.clone())).await?;
    pong.call(Connect(ping)).await?;

    // Wait for the game to finish
    sleep(Duration::from_secs(2)).await;

    Ok(())
}