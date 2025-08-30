---
title: Messages
description: Understanding message patterns in Actor12
---

# Messages

Messages are the only way actors communicate with each other in Actor12. They represent data sent from one actor to another and can optionally expect a response.

## Message Patterns

Actor12 supports three primary message patterns:

### 1. Handler-Style Messages (Recommended)

The most type-safe and flexible approach. Each message type implements a handler on the target actor:

```rust
use actor12::{Handler, Multi, Call, HandlerMessage};

// Define message types
#[derive(Debug)]
pub struct GetBalance(pub AccountId);

#[derive(Debug)]  
pub struct Deposit(pub AccountId, pub Amount);

// Actor with multi-message support
impl Actor for BankActor {
    type Message = Multi<Self>; // Enables handler-style messages
    // ... other types
}

// Implement handlers for each message type
impl Handler<GetBalance> for BankActor {
    type Reply = Result<Amount, BankError>;
    
    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: GetBalance) -> Self::Reply {
        self.accounts.get(&msg.0)
            .map(|account| account.balance)
            .ok_or(BankError::AccountNotFound)
    }
}

impl Handler<Deposit> for BankActor {
    type Reply = Result<Amount, BankError>;
    
    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: Deposit) -> Self::Reply {
        let account = self.accounts.get_mut(&msg.0)
            .ok_or(BankError::AccountNotFound)?;
        account.balance += msg.1;
        Ok(account.balance)
    }
}

// Usage
let bank = spawn::<BankActor>(initial_state);
let balance = bank.send_and_reply(HandlerMessage::new(GetBalance(account_id))).await?;
```

**Advantages:**
- ✅ Type-safe message routing at compile time
- ✅ Each message type has its own handler
- ✅ Easy to add new message types
- ✅ Clear separation of concerns

### 2. Envelope-Style Messages

Traditional request-response pattern where messages are wrapped in envelopes:

```rust
use actor12::{Envelope, EnvelopeMessage};

#[derive(Debug)]
pub struct ProcessRequest {
    pub data: Vec<u8>,
    pub priority: Priority,
}

impl Actor for ProcessorActor {
    type Message = Envelope<ProcessRequest, Result<ProcessedData, ProcessError>>;
    // ... other types
    
    async fn handle(&mut self, _ctx: Exec<'_, Self>, msg: Self::Message) {
        let result = self.process_data(msg.value.data, msg.value.priority);
        let _ = msg.reply.send(result);
    }
}

// Usage  
let processor = spawn::<ProcessorActor>(());
let request = ProcessRequest { data: vec![1, 2, 3], priority: Priority::High };
let result = processor.send_and_reply(EnvelopeMessage::new(request)).await?;
```

**Advantages:**
- ✅ Simple request-response pattern
- ✅ Good for single-message-type actors
- ✅ Direct mapping to traditional RPC patterns

### 3. Raw Messages

Direct message sending without wrappers, for performance-critical scenarios:

```rust
// Send raw message directly
let result = actor.send_raw_message(my_message).await.reply().await?;
```

## Message Wrapper Types

Actor12 provides wrapper types to unify different message patterns under the two-step API:

### `HandlerMessage<T>`
Wraps messages for handler-style actors:
```rust
let message = HandlerMessage::new(MyMessage { data: "hello".to_string() });
let handle = actor.send_message(message).await;
```

### `EnvelopeMessage<T>`  
Wraps messages for envelope-style actors:
```rust
let message = EnvelopeMessage::new(MyRequest { id: 42 });
let handle = actor.send_message(message).await;
```

### `RelayMessage<T, R>`
Relays existing envelopes (useful for forwarding):
```rust
let (envelope, _rx) = Envelope::new(MyMessage { data: "relay this" });
let relay = RelayMessage::new(envelope);
actor.send_and_forget(relay).await; // Fire-and-forget relay
```

## Message Traits

### `SendableMessage<A>`
Defines how to send a message to an actor:
```rust
pub trait SendableMessage<A: ActorLike> {
    type Reply: Send + Sync + 'static;
    
    fn send_to(self, link: &Link<A>) -> impl Future<Output = MessageHandle<Self::Reply>> + Send;
}
```

### `WeakSendableMessage<A>`  
For sending messages through WeakLinks:
```rust
pub trait WeakSendableMessage<A: ActorLike> {
    type Reply: Send + Sync + 'static;
    
    fn weak_send_to(self, weak_link: &WeakLink<A>) -> impl Future<Output = MessageHandle<Self::Reply>> + Send;
}
```

## Message Design Guidelines

### 1. Use Strong Types
```rust
// Good: descriptive types
#[derive(Debug)]
pub struct TransferMoney {
    pub from_account: AccountId,
    pub to_account: AccountId, 
    pub amount: Money,
    pub memo: Option<String>,
}

// Less good: primitive types
pub struct Transfer(String, String, f64, Option<String>);
```

### 2. Include All Necessary Context
```rust
// Good: self-contained message
#[derive(Debug)]
pub struct ProcessOrder {
    pub order_id: OrderId,
    pub customer_id: CustomerId,
    pub items: Vec<OrderItem>,
    pub shipping_address: Address,
    pub payment_method: PaymentMethod,
}

// Less good: requires additional lookups
#[derive(Debug)]
pub struct ProcessOrder {
    pub order_id: OrderId, // Actor must lookup all other data
}
```

### 3. Design for Error Handling
```rust
// Good: specific error types
#[derive(Debug, Error)]
pub enum PaymentError {
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("Invalid payment method")]
    InvalidPaymentMethod,
    #[error("Network error: {0}")]
    NetworkError(String),
}

impl Handler<ProcessPayment> for PaymentActor {
    type Reply = Result<PaymentReceipt, PaymentError>;
    // ...
}
```

### 4. Use Appropriate Patterns
```rust
// Use handler-style for multiple message types
impl Actor for UserService {
    type Message = Multi<Self>;
}
impl Handler<GetUser> for UserService { /* */ }
impl Handler<UpdateUser> for UserService { /* */ }
impl Handler<DeleteUser> for UserService { /* */ }

// Use envelope-style for single message type
impl Actor for HashCalculator {
    type Message = Envelope<HashRequest, HashResponse>;
}
```

## Message Serialization

Actor12 messages are in-memory by default, but you can add serialization for distributed scenarios:

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkMessage {
    pub payload: Vec<u8>,
    pub sender_id: NodeId,
}

// Can be sent over network, stored, etc.
```

## Testing Messages

Messages are easy to test since they're just data:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_message_creation() {
        let msg = TransferMoney {
            from_account: AccountId("alice".to_string()),
            to_account: AccountId("bob".to_string()),
            amount: Money::dollars(100),
            memo: Some("Coffee payment".to_string()),
        };
        
        assert_eq!(msg.amount, Money::dollars(100));
        assert!(msg.memo.is_some());
    }
}
```

## Best Practices

### Make Messages Immutable
```rust
// Good: all fields immutable
#[derive(Debug, Clone)]
pub struct OrderCreated {
    pub order_id: OrderId,
    pub items: Vec<OrderItem>,
    pub timestamp: SystemTime,
}

// Avoid: mutable messages can lead to confusion
```

### Use Builder Pattern for Complex Messages
```rust
#[derive(Debug)]
pub struct CreateUserMessage {
    pub name: String,
    pub email: String,
    pub role: UserRole,
    pub permissions: Vec<Permission>,
}

impl CreateUserMessage {
    pub fn builder(name: String, email: String) -> CreateUserMessageBuilder {
        CreateUserMessageBuilder::new(name, email)
    }
}

pub struct CreateUserMessageBuilder {
    name: String,
    email: String,
    role: UserRole,
    permissions: Vec<Permission>,
}

impl CreateUserMessageBuilder {
    pub fn new(name: String, email: String) -> Self {
        Self {
            name,
            email,
            role: UserRole::User,
            permissions: Vec::new(),
        }
    }
    
    pub fn role(mut self, role: UserRole) -> Self {
        self.role = role;
        self
    }
    
    pub fn permission(mut self, permission: Permission) -> Self {
        self.permissions.push(permission);
        self
    }
    
    pub fn build(self) -> CreateUserMessage {
        CreateUserMessage {
            name: self.name,
            email: self.email,
            role: self.role,
            permissions: self.permissions,
        }
    }
}

// Usage
let msg = CreateUserMessage::builder("Alice".to_string(), "alice@example.com".to_string())
    .role(UserRole::Admin)
    .permission(Permission::ReadUsers)
    .permission(Permission::WriteUsers)
    .build();
```

## Next Steps

- Learn about [Links](/concepts/links) - how to send messages to actors
- Explore [Handlers](/concepts/handlers) - implementing type-safe message processing
- See [Two-Step API](/api/two-step) - the ergonomic messaging interface