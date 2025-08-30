use runy_actor::prelude::*;
use runy_actor::{Multi, MpscChannel, Call, spawn};
use std::time::Duration;
use tokio::time::sleep;

// Worker actor that processes tasks
pub struct Worker {
    id: u32,
    tasks_processed: u32,
}

// Messages
#[derive(Debug)]
pub struct Task {
    pub id: u32,
    pub data: String,
    pub processing_time_ms: u64,
}

#[derive(Debug)]
pub struct TaskResult {
    pub task_id: u32,
    pub worker_id: u32,
    pub result: String,
}

#[derive(Debug)]
pub struct GetWorkerStats;

#[derive(Debug)]
pub struct WorkerStats {
    pub worker_id: u32,
    pub tasks_processed: u32,
}

// Worker implementation
impl Actor for Worker {
    type Spec = u32; // worker_id
    type Message = Multi<Self>;
    type Channel = MpscChannel<Self::Message>;
    type Cancel = ();
    type State = ();

    fn state(_spec: &Self::Spec) -> Self::State {}

    fn init(ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        let id = ctx.spec;
        async move {
            println!("Worker {} initialized", id);
            Ok(Worker {
                id,
                tasks_processed: 0,
            })
        }
    }
}

impl Handler<Task> for Worker {
    type Reply = Result<TaskResult, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: Task) -> Self::Reply {
        println!("Worker {} processing task {} ({}ms)", self.id, msg.id, msg.processing_time_ms);
        
        // Simulate work
        sleep(Duration::from_millis(msg.processing_time_ms)).await;
        
        self.tasks_processed += 1;
        let result = format!("Processed '{}' by worker {}", msg.data, self.id);
        
        println!("Worker {} completed task {} -> '{}'", self.id, msg.id, result);
        
        Ok(TaskResult {
            task_id: msg.id,
            worker_id: self.id,
            result,
        })
    }
}

impl Handler<GetWorkerStats> for Worker {
    type Reply = Result<WorkerStats, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _msg: GetWorkerStats) -> Self::Reply {
        Ok(WorkerStats {
            worker_id: self.id,
            tasks_processed: self.tasks_processed,
        })
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Worker Pool Example ===\n");
    
    // Create workers
    let worker1 = spawn::<Worker>(1);
    let worker2 = spawn::<Worker>(2);
    let worker3 = spawn::<Worker>(3);
    
    // Submit several tasks to different workers
    println!("Submitting tasks...\n");
    
    let tasks = vec![
        ("Calculate prime numbers", 500),
        ("Process image", 800),
        ("Analyze data", 300),
        ("Generate report", 1000),
        ("Send email", 200),
        ("Update database", 600),
    ];
    
    // Submit tasks concurrently using tokio::spawn
    let mut handles = vec![];
    
    for (i, (task_name, duration)) in tasks.into_iter().enumerate() {
        let worker = match i % 3 {
            0 => worker1.clone(),
            1 => worker2.clone(),
            _ => worker3.clone(),
        };
        
        let handle = tokio::spawn(async move {
            let task = Task {
                id: (i + 1) as u32,
                data: format!("{} #{}", task_name, i + 1),
                processing_time_ms: duration,
            };
            
            worker.ask_dyn(task).await
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    println!("Waiting for all tasks to complete...\n");
    for handle in handles {
        match handle.await? {
            Ok(result) => println!("✓ Task {} completed: {}", result.task_id, result.result),
            Err(e) => println!("✗ Task failed: {}", e),
        }
    }
    
    // Get worker statistics
    println!("\n=== Worker Statistics ===");
    for worker in [&worker1, &worker2, &worker3] {
        match worker.ask_dyn(GetWorkerStats).await {
            Ok(stats) => println!("Worker {}: {} tasks processed", stats.worker_id, stats.tasks_processed),
            Err(e) => println!("Failed to get stats: {}", e),
        }
    }
    
    println!("\n=== All tasks completed ===");
    
    // Wait a moment for any remaining output
    sleep(Duration::from_millis(500)).await;
    
    Ok(())
}