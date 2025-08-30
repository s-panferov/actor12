use actor12::{Actor, Envelope, Exec, Init, MpscChannel, spawn};
use std::future::Future;

// Echo server that returns whatever message it receives
pub struct EchoServer {
    message_count: u32,
}

type EchoMessage = Envelope<String, anyhow::Result<String>>;

impl Actor for EchoServer {
    type Spec = ();
    type Message = EchoMessage;
    type Channel = MpscChannel<Self::Message>;
    type Cancel = ();
    type State = ();

    fn state(_spec: &Self::Spec) -> Self::State {}

    fn init(_ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        println!("Echo server initialized");
        futures::future::ready(Ok(EchoServer { message_count: 0 }))
    }

    async fn handle(&mut self, _ctx: Exec<'_, Self>, msg: Self::Message) {
        self.message_count += 1;
        let response = format!("Echo #{}: {}", self.message_count, msg.value);
        println!("Echoing: {}", response);
        msg.reply.send(Ok(response)).unwrap();
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let echo_server = spawn::<EchoServer>(());

    let messages = vec!["Hello", "World", "How are you?", "Goodbye"];

    for message in messages {
        let response: anyhow::Result<String> = echo_server.send(message.to_string()).await;
        println!("Received: {}", response?);
    }

    Ok(())
}