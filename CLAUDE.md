# Actor12 Framework - Development Learnings

This document summarizes the key learnings and insights gained during the extraction and development of the Actor12 framework from the original Runy project.

## Project Overview

Actor12 is a standalone, lightweight actor framework for Rust extracted from the Runy project's `packages/actor` module. The goal was to create a self-contained, well-documented actor system that demonstrates modern Rust async patterns.

## Key Architecture Learnings

### 1. Actor Pattern Implementation

**Core Insight**: The framework uses a trait-based approach where actors are defined by implementing the `Actor` trait with associated types:

```rust
pub trait Actor: Sized + Send + Sync + 'static {
    type Message: ActorMessage<Self>;
    type Spec: Send;
    type Channel: ActorChannel<Message = Self::Message>;
    type Cancel: Clone + Debug + Default + Send + Sync + 'static;
    type State: Send + Sync + 'static;
    
    fn init(ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>>;
    async fn handle(&mut self, ctx: Exec<'_, Self>, msg: Self::Message);
}
```

**Learning**: Associated types provide compile-time guarantees while allowing flexibility in message types and channel implementations.

### 2. Message Handling Patterns

The framework supports two distinct messaging patterns:

#### Direct Envelope Pattern
```rust
type MyMessage = Envelope<Request, anyhow::Result<Response>>;
```
- Simple request-response pattern
- Used with `.send(message).await`
- Type safety guaranteed at compile time

#### Handler Trait Pattern  
```rust
impl Handler<String> for MyActor {
    type Reply = Result<String, anyhow::Error>;
    async fn handle(&mut self, ctx: Call<'_, Self, Self::Reply>, msg: String) -> Self::Reply {
        // Handle message
    }
}
```
- Supports polymorphic message handling
- Used with `.ask_dyn(message).await`
- Enables dynamic dispatch while maintaining type safety

**Learning**: The dual approach provides both simplicity for basic use cases and flexibility for complex scenarios.

### 3. Cancellation System

**Key Insight**: Hierarchical cancellation using a tree structure:

```rust
pub struct CancelToken<T: Clone> {
    inner: Arc<TreeNode<T>>,
}
```

- Parent tokens can cancel all child tokens
- Graceful shutdown propagation
- Each actor gets its own cancellation context

**Learning**: Tree-based cancellation provides clean resource management without complex cleanup logic.

### 4. Memory Management

The framework includes built-in memory tracking:

```rust
pub struct Count<T: 'static> {
    _phantom: PhantomData<T>,
}
```

- Automatic instance counting per type
- Debug assistance for memory leaks
- Zero-cost abstractions when not used

**Learning**: Built-in observability tools are crucial for actor system debugging.

## Development Challenges & Solutions

### 1. Dependency Extraction

**Challenge**: The original code had tight coupling with `runy-lib` workspace dependencies.

**Solution**: 
- Identified minimal required functionality (cancel tokens, counting)
- Embedded the code directly into the actor framework
- Removed unnecessary dependencies (specta, complex type definitions)

**Learning**: Dependency analysis is crucial before extraction - understand what's truly needed vs. what's convenient.

### 2. API Surface Design

**Challenge**: Balancing ease-of-use with power and flexibility.

**Solution**:
- Provided multiple messaging patterns for different use cases
- Created comprehensive examples showing real-world usage
- Documented the trade-offs between patterns

**Learning**: Examples are as important as the API itself - they teach users how to think about the problem domain.

### 3. Type Safety vs. Dynamic Behavior

**Challenge**: Rust's type system vs. the need for dynamic message dispatch.

**Solution**:
- `Multi<A>` type for polymorphic handling
- `Handler<T>` trait for type-safe dynamic dispatch
- Compile-time guarantees with runtime flexibility

**Learning**: Rust's trait system enables both static and dynamic dispatch patterns safely.

## Testing Strategy

The framework includes two test patterns:

1. **Regular Tests** (`tests/regular.rs`): Direct envelope messaging
2. **Dynamic Tests** (`tests/dynmsg.rs`): Handler trait pattern with `Multi<A>`

**Learning**: Test different usage patterns to ensure API robustness.

## Example Strategy

Created examples in order of complexity:

1. **echo_server.rs**: Basic request-response
2. **simple_counter.rs**: State management
3. **handler_pattern.rs**: Multiple message types
4. **dynamic_dispatch.rs**: Real-world router example
5. **bank_account.rs**: Error handling and transactions
6. **worker_pool.rs**: Concurrent processing patterns

**Learning**: Progressive complexity in examples helps users understand capabilities gradually.

## Performance Considerations

### 1. Zero-Copy Message Passing
- Messages moved, not copied, between actors
- `Arc` used for shared state where necessary
- Channels optimized for actor communication patterns

### 2. Async Runtime Integration
- Built on Tokio for proven async performance
- Futures composed efficiently
- No blocking operations in actor message loops

**Learning**: Performance comes from design choices, not micro-optimizations.

## Best Practices Discovered

### 1. Actor Design
- Keep actors focused on single responsibilities
- Use typed messages for compile-time safety
- Design for testability with dependency injection

### 2. Error Handling
- Use `Result<T, E>` for fallible operations
- Propagate errors through message responses
- Design for graceful degradation

### 3. State Management
- Minimize shared state between actors
- Use message passing for coordination
- Design for eventual consistency

## Future Enhancements

Potential areas for improvement identified:

1. **Supervision**: Actor restart strategies
2. **Networking**: Remote actor communication
3. **Persistence**: Actor state snapshots
4. **Metrics**: Built-in performance monitoring
5. **Tracing**: Distributed tracing support

## Conclusion

The Actor12 framework demonstrates that Rust's type system and async capabilities make it excellent for actor-based systems. The key is balancing compile-time safety with runtime flexibility through careful trait design and progressive API complexity.

The extraction process revealed that standalone components require careful dependency analysis, comprehensive examples, and clear documentation to be truly useful to others.