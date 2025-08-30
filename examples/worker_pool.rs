use runy_actor::prelude::*;
use runy_actor::{Link, Multi};
use std::time::Duration;
use tokio::time::sleep;

// Worker actor that processes tasks
pub struct Worker {
    id: u32,
    tasks_processed: u32,
}

// Task coordinator that distributes work to workers
pub struct TaskCoordinator {
    workers: Multi<Worker>,
    next_task_id: u32,
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
pub struct SubmitTask(pub String, pub u64);

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

    async fn init(id: Self::Spec) -> anyhow::Result<Self> {
        println!("Worker {} initialized", id);
        Ok(Worker {
            id,
            tasks_processed: 0,
        })
    }
}

impl Handler<Task> for Worker {
    type Reply = TaskResult;

    async fn exec(&mut self, msg: Task) -> anyhow::Result<Self::Reply> {
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
    type Reply = WorkerStats;

    async fn exec(&mut self, _msg: GetWorkerStats) -> anyhow::Result<Self::Reply> {
        Ok(WorkerStats {
            worker_id: self.id,
            tasks_processed: self.tasks_processed,
        })
    }
}

// Task Coordinator implementation
impl Actor for TaskCoordinator {
    type Spec = u32; // number of workers

    async fn init(num_workers: Self::Spec) -> anyhow::Result<Self> {
        println!("Initializing task coordinator with {} workers", num_workers);
        
        // Create worker pool
        let mut workers = Multi::new();
        for i in 0..num_workers {
            let worker = Worker::spawn(i);
            workers.add(worker);
        }
        
        Ok(TaskCoordinator {
            workers,
            next_task_id: 1,
        })
    }
}

impl Handler<SubmitTask> for TaskCoordinator {
    type Reply = TaskResult;

    async fn exec(&mut self, msg: SubmitTask) -> anyhow::Result<Self::Reply> {
        let task = Task {
            id: self.next_task_id,
            data: msg.0,
            processing_time_ms: msg.1,
        };
        
        self.next_task_id += 1;
        
        println!("Coordinator submitting task {} to worker pool", task.id);
        
        // Send task to any available worker (Multi handles load balancing)
        let result = self.workers.call(task).await?;
        
        println!("Coordinator received result for task {}", result.task_id);
        Ok(result)
    }
}

// Helper function to collect stats from all workers
async fn collect_worker_stats(workers: &Multi<Worker>) -> anyhow::Result<Vec<WorkerStats>> {
    // Note: In a real implementation, we'd need a way to iterate over all workers in Multi
    // For this example, we'll simulate collecting stats
    println!("Collecting worker statistics...");
    Ok(vec![]) // Placeholder
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Worker Pool Example ===\n");
    
    // Create a coordinator with 3 workers
    let coordinator = TaskCoordinator::spawn(3);
    
    // Submit several tasks concurrently
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
        let coordinator_clone = coordinator.clone();
        let handle = tokio::spawn(async move {
            let result = coordinator_clone
                .call(SubmitTask(format!("{} #{}", task_name, i + 1), duration))
                .await;
            result
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
    
    println!("\n=== All tasks completed ===");
    
    // Wait a moment for any remaining output
    sleep(Duration::from_millis(500)).await;
    
    Ok(())
}