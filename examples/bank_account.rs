use runy_actor::prelude::*;
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

    async fn init((account_number, initial_balance): Self::Spec) -> anyhow::Result<Self> {
        println!("Bank account {} created with initial balance: ${:.2}", account_number, initial_balance);
        Ok(BankAccount {
            balance: initial_balance,
            account_number,
        })
    }
}

impl Handler<Deposit> for BankAccount {
    type Reply = f64;

    async fn exec(&mut self, msg: Deposit) -> anyhow::Result<Self::Reply> {
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
    type Reply = f64;

    async fn exec(&mut self, msg: Withdraw) -> anyhow::Result<Self::Reply> {
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
    type Reply = f64;

    async fn exec(&mut self, _msg: GetBalance) -> anyhow::Result<Self::Reply> {
        println!("Account {}: Balance inquiry: ${:.2}", self.account_number, self.balance);
        Ok(self.balance)
    }
}

impl Handler<Transfer> for BankAccount {
    type Reply = ();

    async fn exec(&mut self, msg: Transfer) -> anyhow::Result<Self::Reply> {
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
        msg.to_account.call(Deposit { amount: msg.amount }).await?;
        println!("Transfer of ${:.2} completed from {} to target account", 
                 msg.amount, self.account_number);

        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create two bank accounts
    let alice_account = BankAccount::spawn(("ALICE-001".to_string(), 1000.0));
    let bob_account = BankAccount::spawn(("BOB-002".to_string(), 500.0));

    // Perform some operations
    println!("\n=== Initial Operations ===");
    
    // Check initial balances
    let alice_balance = alice_account.call(GetBalance).await?;
    let bob_balance = bob_account.call(GetBalance).await?;
    println!("Alice: ${:.2}, Bob: ${:.2}", alice_balance, bob_balance);

    // Alice deposits money
    alice_account.call(Deposit { amount: 200.0 }).await?;

    // Bob withdraws money
    bob_account.call(Withdraw { amount: 100.0 }).await?;

    println!("\n=== Transfer Operation ===");
    
    // Alice transfers money to Bob
    alice_account.call(Transfer { 
        to_account: bob_account.clone(), 
        amount: 300.0 
    }).await?;

    println!("\n=== Final Balances ===");
    let alice_final = alice_account.call(GetBalance).await?;
    let bob_final = bob_account.call(GetBalance).await?;
    println!("Alice: ${:.2}, Bob: ${:.2}", alice_final, bob_final);

    println!("\n=== Error Handling ===");
    
    // Try to withdraw more than available (should fail)
    match alice_account.call(Withdraw { amount: 2000.0 }).await {
        Ok(_) => println!("Withdrawal succeeded unexpectedly!"),
        Err(e) => println!("Withdrawal failed as expected: {}", e),
    }

    // Wait a moment before shutting down
    sleep(Duration::from_millis(100)).await;

    Ok(())
}