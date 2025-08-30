use runy_actor::{Actor, Call, Handler, Init, MpscChannel, Multi, spawn};
use futures::future;
use std::collections::HashMap;
use std::future::Future;

// A router actor that can handle different types of requests dynamically
pub struct RouterActor {
    routes: HashMap<String, String>,
    request_count: u32,
}

// Different request types
#[derive(Debug)]
pub struct GetRouteRequest {
    pub path: String,
}

#[derive(Debug)]  
pub struct AddRouteRequest {
    pub path: String,
    pub handler: String,
}

#[derive(Debug)]
pub struct ListRoutesRequest;

#[derive(Debug)]
pub struct StatsRequest;

#[derive(Debug)]
pub struct ResetRequest;

// Response types
#[derive(Debug)]
pub struct RouteResponse {
    pub found: bool,
    pub handler: Option<String>,
}

impl Actor for RouterActor {
    type Spec = ();
    type Message = Multi<Self>;
    type Channel = MpscChannel<Self::Message>;  
    type Cancel = ();
    type State = ();

    fn state(_spec: &Self::Spec) -> Self::State {}

    fn init(_ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        println!("Router actor initialized");
        future::ready(Ok(RouterActor {
            routes: HashMap::new(),
            request_count: 0,
        }))
    }
}

// Handle route lookup
impl Handler<GetRouteRequest> for RouterActor {
    type Reply = Result<RouteResponse, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: GetRouteRequest) -> Self::Reply {
        self.request_count += 1;
        
        let handler = self.routes.get(&msg.path).cloned();
        let found = handler.is_some();
        
        println!("GET {}: {} (handler: {:?})", msg.path, if found { "FOUND" } else { "NOT_FOUND" }, handler);
        
        Ok(RouteResponse { found, handler })
    }
}

// Handle route registration
impl Handler<AddRouteRequest> for RouterActor {
    type Reply = Result<bool, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: AddRouteRequest) -> Self::Reply {
        self.request_count += 1;
        
        let was_existing = self.routes.contains_key(&msg.path);
        self.routes.insert(msg.path.clone(), msg.handler.clone());
        
        println!("ROUTE ADDED: {} -> {} ({})", 
                 msg.path, msg.handler, 
                 if was_existing { "UPDATED" } else { "NEW" });
        
        Ok(!was_existing) // true if it was a new route
    }
}

// Handle listing all routes
impl Handler<ListRoutesRequest> for RouterActor {
    type Reply = Result<Vec<(String, String)>, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _msg: ListRoutesRequest) -> Self::Reply {
        self.request_count += 1;
        
        let routes: Vec<(String, String)> = self.routes.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
            
        println!("LISTING {} routes:", routes.len());
        for (path, handler) in &routes {
            println!("  {} -> {}", path, handler);
        }
        
        Ok(routes)
    }
}

// Handle stats request  
impl Handler<StatsRequest> for RouterActor {
    type Reply = Result<(u32, usize), anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _msg: StatsRequest) -> Self::Reply {
        let stats = (self.request_count, self.routes.len());
        println!("STATS: {} total requests, {} registered routes", stats.0, stats.1);
        Ok(stats)
    }
}

// Handle reset request
impl Handler<ResetRequest> for RouterActor {
    type Reply = Result<(), anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _msg: ResetRequest) -> Self::Reply {
        let old_count = self.request_count;
        let old_routes = self.routes.len();
        
        self.routes.clear();
        self.request_count = 0;
        
        println!("RESET: Cleared {} routes, reset request count from {}", old_routes, old_count);
        Ok(())
    }
}

// Handle raw string commands (dynamic dispatch example)
impl Handler<String> for RouterActor {
    type Reply = Result<String, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: String) -> Self::Reply {
        self.request_count += 1;
        
        let parts: Vec<&str> = msg.split_whitespace().collect();
        match parts.as_slice() {
            ["help"] => {
                let help = "Available commands:\n\
                           - help: Show this help\n\
                           - status: Show current status\n\
                           - echo <text>: Echo back the text";
                println!("HELP requested");
                Ok(help.to_string())
            }
            ["status"] => {
                let status = format!("Router status: {} requests, {} routes", 
                                   self.request_count, self.routes.len());
                println!("STATUS requested");
                Ok(status)
            }
            ["echo", rest @ ..] => {
                let text = rest.join(" ");
                let response = format!("Echo: {}", text);
                println!("ECHO: {}", text);
                Ok(response)
            }
            _ => {
                let response = format!("Unknown command: '{}'. Try 'help'", msg);
                println!("UNKNOWN COMMAND: {}", msg);
                Ok(response)
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Dynamic Dispatch Example ===\n");

    let router = spawn::<RouterActor>(());

    // Test typed message handlers
    println!("1. Setting up routes:");
    
    let _: Result<bool, anyhow::Error> = router.ask_dyn(AddRouteRequest {
        path: "/api/users".to_string(),
        handler: "UserHandler".to_string(),
    }).await;
    
    let _: Result<bool, anyhow::Error> = router.ask_dyn(AddRouteRequest {
        path: "/api/posts".to_string(), 
        handler: "PostHandler".to_string(),
    }).await;

    let _: Result<bool, anyhow::Error> = router.ask_dyn(AddRouteRequest {
        path: "/health".to_string(),
        handler: "HealthHandler".to_string(), 
    }).await;

    println!("\n2. Testing route lookups:");
    
    let response: Result<RouteResponse, anyhow::Error> = router.ask_dyn(GetRouteRequest {
        path: "/api/users".to_string(),
    }).await;
    println!("Lookup result: {:?}", response);

    let response: Result<RouteResponse, anyhow::Error> = router.ask_dyn(GetRouteRequest {
        path: "/nonexistent".to_string(),
    }).await;
    println!("Lookup result: {:?}", response);

    println!("\n3. Listing all routes:");
    let routes: Result<Vec<(String, String)>, anyhow::Error> = router.ask_dyn(ListRoutesRequest).await;
    println!("Routes: {:?}", routes);

    println!("\n4. Testing dynamic string commands:");
    
    let help: Result<String, anyhow::Error> = router.ask_dyn("help".to_string()).await;
    println!("Help response: {:?}", help);

    let status: Result<String, anyhow::Error> = router.ask_dyn("status".to_string()).await;
    println!("Status response: {:?}", status);

    let echo: Result<String, anyhow::Error> = router.ask_dyn("echo Hello dynamic world!".to_string()).await;
    println!("Echo response: {:?}", echo);

    let unknown: Result<String, anyhow::Error> = router.ask_dyn("unknown command".to_string()).await;
    println!("Unknown command response: {:?}", unknown);

    println!("\n5. Final stats:");
    let stats: Result<(u32, usize), anyhow::Error> = router.ask_dyn(StatsRequest).await;
    println!("Final stats: {:?}", stats);

    Ok(())
}