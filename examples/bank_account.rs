use actor12::prelude::*;
use actor12::{Multi, MpscChannel, Call, spawn, Link};
use std::time::Duration;
use tokio::time::sleep;

// Bank account actor that handles concurrent transactions safely
pub struct BankAccount {
    balance: f64,
    account_number: String,
}

// Messages for bank operations
#[derive(Debug)]
pub struct Deposit {
    pub amount: f64,
}

#[derive(Debug)]
pub struct Withdraw {
    pub amount: f64,
}

#[derive(Debug)]
pub struct GetBalance;

#[derive(Debug)]
pub struct Transfer {
    pub to_account: Link<BankAccount>,
    pub amount: f64,
}

// Custom error for insufficient funds
#[derive(Debug, thiserror::Error)]
pub enum BankError {
    #[error("Insufficient funds: balance {balance}, requested {requested}")]
    InsufficientFunds { balance: f64, requested: f64 },
    #[error("Invalid amount: {amount}")]
    InvalidAmount { amount: f64 },
}

impl Actor for BankAccount {
    type Spec = (String, f64); // (account_number, initial_balance)
    type Message = Multi<Self>;
    type Channel = MpscChannel<Self::Message>;
    type Cancel = ();
    type State = ();

    fn state(_spec: &Self::Spec) -> Self::State {}

    fn init(ctx: Init<'_, Self>) -> impl Future<Output = Result<Self, Self::Cancel>> + Send + 'static {
        let (account_number, initial_balance) = ctx.spec;
        async move {
            println!("Bank account {} created with initial balance: ${:.2}", account_number, initial_balance);
            Ok(BankAccount {
                balance: initial_balance,
                account_number,
            })
        }
    }
}

impl Handler<Deposit> for BankAccount {
    type Reply = Result<f64, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: Deposit) -> Self::Reply {
        if msg.amount <= 0.0 {
            return Err(BankError::InvalidAmount { amount: msg.amount }.into());
        }

        self.balance += msg.amount;
        println!("Account {}: Deposited ${:.2}, new balance: ${:.2}", 
                 self.account_number, msg.amount, self.balance);
        Ok(self.balance)
    }
}

impl Handler<Withdraw> for BankAccount {
    type Reply = Result<f64, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: Withdraw) -> Self::Reply {
        if msg.amount <= 0.0 {
            return Err(BankError::InvalidAmount { amount: msg.amount }.into());
        }

        if self.balance < msg.amount {
            return Err(BankError::InsufficientFunds { 
                balance: self.balance, 
                requested: msg.amount 
            }.into());
        }

        self.balance -= msg.amount;
        println!("Account {}: Withdrew ${:.2}, new balance: ${:.2}", 
                 self.account_number, msg.amount, self.balance);
        Ok(self.balance)
    }
}

impl Handler<GetBalance> for BankAccount {
    type Reply = Result<f64, anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, _msg: GetBalance) -> Self::Reply {
        println!("Account {}: Balance inquiry: ${:.2}", self.account_number, self.balance);
        Ok(self.balance)
    }
}

impl Handler<Transfer> for BankAccount {
    type Reply = Result<(), anyhow::Error>;

    async fn handle(&mut self, _ctx: Call<'_, Self, Self::Reply>, msg: Transfer) -> Self::Reply {
        if msg.amount <= 0.0 {
            return Err(BankError::InvalidAmount { amount: msg.amount }.into());
        }

        if self.balance < msg.amount {
            return Err(BankError::InsufficientFunds { 
                balance: self.balance, 
                requested: msg.amount 
            }.into());
        }

        // Withdraw from this account
        self.balance -= msg.amount;
        println!("Account {}: Transfer out ${:.2}, new balance: ${:.2}", 
                 self.account_number, msg.amount, self.balance);

        // Deposit to target account
        let _ = msg.to_account.ask_dyn(Deposit { amount: msg.amount }).await?;
        println!("Transfer of ${:.2} completed from {} to target account", 
                 msg.amount, self.account_number);

        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create two bank accounts
    let alice_account = spawn::<BankAccount>(("ALICE-001".to_string(), 1000.0));
    let bob_account = spawn::<BankAccount>(("BOB-002".to_string(), 500.0));

    // Perform some operations
    println!("\n=== Initial Operations ===");
    
    // Check initial balances
    let alice_balance = alice_account.ask_dyn(GetBalance).await?;
    let bob_balance = bob_account.ask_dyn(GetBalance).await?;
    println!("Alice: ${:.2}, Bob: ${:.2}", alice_balance, bob_balance);

    // Alice deposits money
    let _ = alice_account.ask_dyn(Deposit { amount: 200.0 }).await?;

    // Bob withdraws money
    let _ = bob_account.ask_dyn(Withdraw { amount: 100.0 }).await?;

    println!("\n=== Transfer Operation ===");
    
    // Alice transfers money to Bob
    alice_account.ask_dyn(Transfer { 
        to_account: bob_account.clone(), 
        amount: 300.0 
    }).await?;

    println!("\n=== Final Balances ===");
    let alice_final = alice_account.ask_dyn(GetBalance).await?;
    let bob_final = bob_account.ask_dyn(GetBalance).await?;
    println!("Alice: ${:.2}, Bob: ${:.2}", alice_final, bob_final);

    println!("\n=== Error Handling ===");
    
    // Try to withdraw more than available (should fail)
    match alice_account.ask_dyn(Withdraw { amount: 2000.0 }).await {
        Ok(_) => println!("Withdrawal succeeded unexpectedly!"),
        Err(e) => println!("Withdrawal failed as expected: {}", e),
    }

    // Wait a moment before shutting down
    sleep(Duration::from_millis(100)).await;

    Ok(())
}