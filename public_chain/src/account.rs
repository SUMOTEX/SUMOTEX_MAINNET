use std::collections::HashMap;
use secp256k1::{Secp256k1, PublicKey, SecretKey};

pub fn generate_keypair()->PublicKey {
    let secp = Secp256k1::new();
    let mut rng = secp256k1::rand::thread_rng();
    let (secret_key, public_key) = secp.generate_keypair(&mut rng);
    public_key
}

// Account structure
struct Account {
    public_address: PublicKey,
    balance: f64,
    nonce: u64,
}

impl Account {
    // Creates a new account
    fn new() -> Self {
        Account {
            public_address: generate_keypair(),
            balance: 0.0,
            nonce: 0,
        }
    }

    // Deposit an amount into the account
    fn deposit(&mut self, amount: f64) {
        self.balance += amount;
    }

    // Withdraw an amount from the account, returns true if successful
    fn withdraw(&mut self, amount: f64) -> bool {
        if self.balance >= amount {
            self.balance -= amount;
            true
        } else {
            false
        }
    }

    // Increment the nonce, typically called when a transaction is made
    fn increment_nonce(&mut self) {
        self.nonce += 1;
    }
}

// Sample Blockchain representation with accounts
struct Blockchain {
    accounts: HashMap<PublicKey, Account>,
}

impl Blockchain {
    // Creates a new blockchain instance
    fn new() -> Self {
        Blockchain {
            accounts: HashMap::new(),
        }
    }

    // Adds a new account to the blockchain
    fn add_account(&mut self) -> String {
        let account = Account::new();
        let address = account.public_address.clone();
        self.accounts.insert(address.clone(), account);
        address.to_string()
    }

    // Gets a reference to an account given an address
    fn get_account(&self, address: &String) -> Option<&Account> {
        // Step 1: Decode hex string to bytes
        let bytes = hex::decode(address).ok()?;

        // Step 2: Convert bytes to PublicKey
        let public_key = PublicKey::from_slice(&bytes).ok()?;
        self.accounts.get(&public_key)
    }
}
